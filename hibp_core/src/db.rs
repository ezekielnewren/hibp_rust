use std::{fs, io};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, ErrorKind, Read, Write};
use std::mem::size_of;
use std::path::{Path, PathBuf};
use memmap2::{MmapMut, MmapOptions};
use crate::{compress_xz, convert_range, download_range, extract_gz, extract_xz, HASH, HashRange, InterpolationSearch};

use futures::stream::{FuturesUnordered};
use futures::StreamExt;
use tokio::runtime::Runtime;
use rayon::prelude::*;
use crate::transform::{Transform, TransformConcurrent};

pub struct FileArray<'a, T> {
    pub pathname: PathBuf,
    pub fd: File,
    pub mmap: MmapMut,
    pub slice: &'a mut [T],
}

impl<'a, T> FileArray<'a, T> {

    pub fn new(_pathname: &Path, size: usize) -> std::io::Result<Self> {
        let fd = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(_pathname)?;

        if size != 0 {
            fd.set_len((size * size_of::<T>()) as u64)?;
        }

        let mut mmap_mut = unsafe { MmapOptions::new().map_mut(&fd)? };

        let slice = unsafe {
            let ptr = mmap_mut.as_mut_ptr() as *mut T;
            std::slice::from_raw_parts_mut(ptr, mmap_mut.len()/size_of::<T>())
        };

        Ok(Self {
            pathname: PathBuf::from(_pathname),
            fd,
            mmap: mmap_mut,
            slice,
        })
    }

    pub fn sync(&mut self) -> io::Result<()> {
        self.mmap.flush()
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
    pub dbdir: PathBuf,
    pub index: Option<FileArray<'a, HASH>>,
    pub rt: tokio::runtime::Runtime,
}

impl<'a> HIBPDB<'a> {
    pub fn new(v: &Path) -> io::Result<Self> {
        Ok(Self {
            dbdir: PathBuf::from(v),
            index: None,
            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
        })
    }

    pub fn save(prefix: &Path, hr: HashRange) -> io::Result<()> {
        let file_name = HashRange::name(hr.range);

        let path_tmp = prefix.join(String::from("tmp.")+file_name.as_str());
        let pathname = prefix.join(file_name);
        {
            let mut fd = File::create(&path_tmp)?;
            let out = hr.serialize();
            fd.write_all(out.as_slice())?;
        }
        fs::rename(path_tmp, pathname)?;

        Ok(())
    }

    pub fn load(prefix: &Path, range: u32) -> io::Result<HashRange> {
        let file_name = HashRange::name(range);

        let mut buff: Vec<u8> = Vec::new();
        let mut fd = File::open(prefix.join(file_name))?;
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
                Ok(_) => copy.sum += 1,
                Err(e) => return Err(io::Error::new(ErrorKind::InvalidInput, e.to_string())),
            }

            copy.len += 1;
        }

        copy.buff = compress_xz(copy.buff.as_slice())?;
        copy.format = String::from("xz");

        Ok(copy)
    }

    pub fn update<F>(rt: &Runtime, dbdir: &Path, mut f: F) -> io::Result<()> where F: FnMut(u32)  {
        let dir_range = dbdir.join("range/");
        fs::create_dir_all(dir_range.clone()).unwrap();

        let limit0 = 500;
        let limit1 = rayon::current_num_threads()*10;
        let client = reqwest::Client::new();

        let fut = async {
            let mut queue0 = FuturesUnordered::new();
            let mut queue1: Vec<HashRange> = Vec::new();

            let mut range = 0;
            loop {
                if queue0.len() < limit0 && range < (1<<20) {
                    if !dir_range.join(HashRange::name(range)).exists() {
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
                        Self::save(dir_range.as_path(), compact).unwrap();
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

            Ok::<(), io::Error>(())
        };

        rt.block_on(fut)
    }

    pub fn sort_freq(&mut self) -> io::Result<()> {
        let file_freq = self.dbdir.join("frequency.bin");
        let file_freq_index = self.dbdir.join("freq_index.bin");

        let db_len = self.len()?;

        let slice_index = self.index()?;

        let fa_freq: FileArray<u64> = FileArray::new(file_freq.as_path(), 0)?;
        let slice_freq = fa_freq.as_slice();

        let mut fa_freq_index: FileArray<u64> = FileArray::new(file_freq_index.as_path(), db_len)?;

        let slice_freq_index = fa_freq_index.as_mut_slice();
        for i in 0..slice_freq_index.len() {
            slice_freq_index[i] = i as u64;
        }

        slice_freq_index.par_sort_unstable_by(|i, j| {
            let mut cmp = slice_freq[*j as usize].cmp(&slice_freq[*i as usize]);
            if cmp.is_eq() {
                cmp = slice_index[*i as usize].cmp(&slice_index[*j as usize]);
            }
            return cmp;
        });

        fa_freq_index.sync()?;

        Ok(())
    }

    pub fn construct_index<F>(&self, mut f: F) -> io::Result<()> where F: FnMut(u32) {
        let dir_range: PathBuf = self.dbdir.join("range/");

        let mut file_index = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(self.dbdir.join("index.bin"))?;

        let mut file_frequency = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(self.dbdir.join("frequency.bin"))?;

        let prefix = dir_range.clone();
        let mut transformer: TransformConcurrent<u32, io::Result<(Vec<u8>, Vec<u8>)>> = TransformConcurrent::new(move |range| {
            let r = convert_range(Self::load(prefix.as_path(), range)?);
            return match r {
                Ok((buff0, freq)) => {
                    let mut buff1: Vec<u8> = Vec::new();
                    for i in freq {
                        let x = i.to_le_bytes();
                        buff1.extend_from_slice(&x);
                    }
                    Ok((buff0, buff1))
                }
                Err(e) => Err(e),
            }
        }, 0);

        let limit = 1000;

        let mut wp = 0u32;
        let mut rp = 0u32;
        while rp < 1<<20 {
            if wp < (1<<20) && wp-rp < limit {
                transformer.add(wp);
                wp += 1;
            } else {
                let (buff, freq) = transformer.take()?;
                file_index.write_all(buff.as_slice()).unwrap();
                file_frequency.write_all(freq.as_slice()).unwrap();
                f(rp);
                rp += 1;
            }
        }

        Ok(())
    }

    #[inline]
    pub fn index(&mut self) -> io::Result<&[HASH]> {
        if self.index.is_none() {
            let file_index = self.dbdir.join("index.bin");
            self.index = Some(FileArray::new(file_index.as_path(), 0)?);
        }
        let fa: &FileArray<HASH> = self.index.as_ref().unwrap();
        return Ok(fa.as_slice());
    }

    pub fn find(self: &mut Self, key: &HASH) -> io::Result<Result<usize, usize>> {
        Ok(self.index()?.interpolation_search(key))
    }

    pub fn len(&mut self) -> io::Result<usize> {
        Ok(self.index()?.len())
    }
}






