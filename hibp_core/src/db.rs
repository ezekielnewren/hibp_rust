use std::{fs, io};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, ErrorKind, Read, Write};
use std::mem::size_of;
use std::path::Path;
use memmap2::{MmapMut, MmapOptions};
use crate::{compress_xz, convert_range, dir_list, download_range, DownloadError, extract_gz, extract_xz, HASH, HashRange, InterpolationSearch};
use bit_set::BitSet;

use futures::stream::{FuturesUnordered};
use futures::StreamExt;
use rayon::prelude::*;
use crate::transform::{Transform, TransformConcurrent};

pub struct FileArray<'a, T> {
    pub pathname: String,
    pub fd: File,
    pub mmap: MmapMut,
    pub slice: &'a mut [T],
}

impl<'a, T> FileArray<'a, T> {

    pub fn new(_pathname: String, size: usize) -> std::io::Result<Self> {
        let fd = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(_pathname.clone())?;

        fd.set_len((size * size_of::<T>()) as u64)?;

        let mut mmap_mut = unsafe { MmapOptions::new().map_mut(&fd)? };

        let slice = unsafe {
            let ptr = mmap_mut.as_mut_ptr() as *mut T;
            std::slice::from_raw_parts_mut(ptr, mmap_mut.len()/size_of::<T>())
        };

        Ok(Self {
            pathname: _pathname,
            fd,
            mmap: mmap_mut,
            slice,
        })
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        return self.slice;
    }

    pub fn as_slice(&self) -> &[T] {
        return self.slice;
    }

    pub fn len(&self) -> usize {
        return self.mmap.len();
    }

}



pub struct HIBPDB<'a> {
    pub dbdir: String,
    pub index: Option<FileArray<'a, HASH>>,
    pub rt: tokio::runtime::Runtime,
}

impl<'a> HIBPDB<'a> {
    pub fn new(v: String) -> io::Result<Self> {
        let dbdir = v.clone();
        let mut file_index = dbdir.clone();
        file_index.push_str("/index.bin");

        Ok(Self {
            dbdir,
            index: None,
            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
        })
    }

    pub fn save(prefix: String, hr: HashRange) -> io::Result<()> {
        let file_name = HashRange::name(hr.range);

        let path_tmp = prefix.clone()+"tmp."+file_name.as_str();
        let pathname = prefix+file_name.as_str();
        {
            let mut fd = File::create(&path_tmp)?;
            let out = hr.serialize();
            fd.write_all(out.as_slice())?;
        }
        fs::rename(path_tmp, pathname)?;

        Ok(())
    }

    pub fn load(prefix: String, range: u32) -> io::Result<HashRange> {
        let file_name = HashRange::name(range);

        let mut buff: Vec<u8> = Vec::new();
        let mut fd = File::open(prefix+file_name.as_str())?;
        fd.read_to_end(&mut buff)?;

        return HashRange::deserialize(buff.as_slice());
    }

    pub fn compact(hr: &HashRange) -> io::Result<HashRange> {
        let mut copy = HashRange {
            range: hr.range,
            etag: hr.etag,
            timestamp: hr.timestamp,
            len: 0,
            sum: 0,
            format: hr.format.clone(),
            buff: Vec::new(),
        };

        copy.buff = match hr.format.as_str() {
            "xz" => extract_xz(hr.buff.as_slice())?,
            "gz" => extract_gz(hr.buff.as_slice())?,
            "txt" => hr.buff.clone(),
            _ => return Err(io::Error::new(ErrorKind::InvalidInput, "unsupported file type")),
        };

        copy.buff.retain(|&x| x != b'\r');

        for v in copy.buff.lines() {
            let line = v?;
            let t = u64::from_str_radix(&line[33-5..], 16);
            match t {
                Ok(v) => copy.sum += 1,
                Err(e) => return Err(io::Error::new(ErrorKind::InvalidInput, e.to_string())),
            }

            copy.len += 1;
        }

        copy.buff = compress_xz(copy.buff.as_slice())?;
        copy.format = String::from("xz");

        Ok(copy)
    }

    pub fn update<F>(self: &Self, mut f: F) -> io::Result<()> where F: FnMut(u32)  {
        let dir_range = self.dbdir.clone()+"/range/";
        fs::create_dir_all(dir_range.clone()).unwrap();

        let limit0 = 500;
        let limit1 = rayon::current_num_threads()*10;
        let client = reqwest::Client::new();

        let fut = async {
            let prefix = Path::new(dir_range.as_str());

            let mut queue0 = FuturesUnordered::new();
            let mut queue1: Vec<HashRange> = Vec::new();

            let mut range = 0;
            loop {
                if queue0.len() < limit0 && range < (1<<20) {
                    if !prefix.join(HashRange::name(range)).exists() {
                        queue0.push(download_range(&client, range));
                    }
                    range += 1;
                } else {
                    let result = queue0.next().await.unwrap();
                    match result {
                        Ok(hr) => queue1.push(hr),
                        Err(e) => queue0.push(download_range(&client, e.range)),
                    }
                }

                let downloaded = range >= (1<<20) && queue0.is_empty();
                if downloaded || queue1.len() >= limit1 {
                    queue1.par_iter().for_each(|hr| {
                        let compact = Self::compact(&hr).unwrap();
                        Self::save(dir_range.clone(), compact).unwrap();
                    });
                    for r in &queue1 {
                        f(r.range);
                    }
                    queue1.clear();
                }

                if downloaded {
                    break;
                }
            }

            Ok(())
        };

        self.rt.block_on(fut)
    }

    pub fn construct_index<F>(&self, mut f: F) -> io::Result<()> where F: FnMut(u32) {
        let mut file_index = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(self.dbdir.clone()+"/index.bin")?;


        let prefix: String = self.dbdir.clone()+"/range/";
        let mut transformer: TransformConcurrent<u32, io::Result<Vec<u8>>> = TransformConcurrent::new(move |range| {
            return convert_range(Self::load(prefix.clone(), range)?);
        }, 0);

        let limit = 1000;

        let mut wp = 0u32;
        let mut rp = 0u32;
        while rp < 1<<20 {
            if wp < (1<<20) && wp-rp < limit {
                transformer.add(wp);
                wp += 1;
            } else {
                let buff = transformer.take()?;
                file_index.write_all(buff.as_slice()).unwrap();
                f(rp);
                rp += 1;
            }
        }

        Ok(())
    }

    #[inline]
    pub fn index(&self) -> &[HASH] {
        let fa: &FileArray<HASH> = self.index.as_ref().unwrap();
        return fa.as_slice();
    }

    pub fn find(self: &mut Self, key: &HASH) -> Result<usize, usize> {
        self.index().interpolation_search(key)
    }

    pub fn len(&self) -> usize {
        self.index.as_ref().unwrap().len()
    }
}






