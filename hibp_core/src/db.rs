use std::{fs, io};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, ErrorKind, Read, Write};
use std::mem::size_of;
use memmap2::{MmapMut, MmapOptions};
use crate::{dir_list, download_range, extract_gz, extract_xz, HASH, HashRange, InterpolationSearch};
use bit_set::BitSet;

use futures::stream::{FuturesOrdered, FuturesUnordered};
use futures::StreamExt;
use regex::Regex;

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
    pub fn new(v: String) -> std::io::Result<Self> {
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

    pub fn save(self: &Self, hr: HashRange) -> std::io::Result<()> {
        let prefix: String = self.dbdir.clone()+"/range/";
        let fname = format!("{:05X}_{:016X}.{}", hr.range, hr.etag, HashRange::EXTENSION);

        let path_tmp = prefix.clone()+"tmp."+fname.as_str();
        let pathname = prefix+fname.as_str();
        {
            let mut fd = File::create(&path_tmp)?;
            fd.write_all(hr.buff.as_slice())?;
        }
        fs::rename(path_tmp, pathname)?;

        Ok(())
    }

    pub fn update<F>(self: &Self, mut f: F) -> io::Result<()> where F: FnMut(u32)  {
        let dir_range = self.dbdir.clone()+"/range/";
        fs::create_dir_all(dir_range.clone()).unwrap();

        let limit = 500;
        let client = reqwest::Client::new();

        let fut = async {
            let mut queue = FuturesUnordered::new();

            let ls = dir_list(dir_range.as_str()).unwrap();
            let mut bs = BitSet::new();
            for key in ls {
                let t = u32::from_str_radix(&key[0..5], 16).unwrap();
                bs.insert(t as usize);
            }

            let mut i = 0u32;
            loop {
                if i<(1<<20) && queue.len() < limit {
                    if !bs.contains(i as usize) {
                        queue.push(download_range(&client, i));
                    }
                    i += 1;
                    continue;
                }

                if let Some(result) = queue.next().await {
                    match result {
                        Ok(v) => {
                            f(v.range);
                            self.save(v).unwrap();
                        }
                        Err(err) => {
                            queue.push(download_range(&client, err.range));
                        }
                    }
                }

                if i >= (1<<20) && queue.is_empty() {
                    break;
                }
            }
        };

        self.rt.block_on(fut);

        Ok(())
    }


    pub fn range_map(&self) -> io::Result<Vec<String>> {
        let dir_range = self.dbdir.clone()+"/range/";

        let mut ls = dir_list(dir_range.as_str()).unwrap();
        ls.sort();
        let re = Regex::new("^([0-9a-fA-F]{5})_([0-9a-fA-F]{16})\\..z$").unwrap();

        let mut out: Vec<String> = Vec::new();

        let mut i = 0;
        for filename in ls {
            if ! re.is_match(&filename) {
                continue;
            }
            let cap = re.captures(filename.as_str()).unwrap();
            let range = u32::from_str_radix(cap.get(1).unwrap().as_str(), 16).unwrap();
            if range > i {
                return Err(io::Error::new(ErrorKind::NotFound, format!("missing {:05X}", i)));
            } else if range < i {
                return Err(io::Error::new(ErrorKind::AlreadyExists, format!("duplicate {:05X}", i)));
            }
            out.push(filename);
            i += 1;
        }

        if i < (1<<20) {
            return Err(io::Error::new(ErrorKind::NotFound, "not all ranges are present"));
        }

        return Ok(out);
    }


    async fn extract_range(&self, range_map: &Vec<String>, range: u32) -> io::Result<Vec<u8>> {
        let dir_range = self.dbdir.clone()+"/range/";

        let mut buff: Vec<u8> = Vec::new();
        let filename = &range_map[range as usize];
        let mut fd = File::open(dir_range.clone()+"/"+filename.as_str())?;
        fd.read_to_end(&mut buff)?;

        let plain = match HashRange::EXTENSION {
            "xz" => extract_xz(buff.as_slice()),
            "gz" => extract_gz(buff.as_slice()),
            _ => return Err(io::Error::new(ErrorKind::InvalidInput, "unsupported file type")),
        }?;

        buff.clear();
        let mut hash = vec![0u8; 16];
        for v in plain.lines() {
            let line = v?;
            let t = hex::decode_to_slice(format!("{:05X}{}", range, &line[0..(32-5)]), hash.as_mut_slice());
            match t {
                Ok(_) => buff.extend(&hash),
                Err(e) => return Err(io::Error::new(ErrorKind::InvalidInput, e.to_string())),
            }
        }

        Ok(buff)
    }

    pub fn construct_index<F>(&self, mut f: F) -> io::Result<()> where F: FnMut(u32) {
        let map = self.range_map().unwrap();

        let mut file_index = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(self.dbdir.clone()+"/index.bin")?;


        // let transformer: TransformSerial<u32, HashRange> = TransformSerial::new(|v| {});
        self.rt.block_on(async {
            let mut queue = FuturesOrdered::new();
            let limit = 1000;

            let mut wp = 0u32;
            let mut rp = 0u32;
            while rp < 1<<20 {
                if wp < (1<<20) && queue.len() < limit {
                    let fut = self.extract_range(&map, wp);
                    queue.push_back(fut);
                    wp += 1;
                } else {
                    let buff = queue.next().await.unwrap().unwrap();
                    file_index.write_all(buff.as_slice()).unwrap();
                    f(rp);
                    rp += 1;
                }
            }
        });

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






