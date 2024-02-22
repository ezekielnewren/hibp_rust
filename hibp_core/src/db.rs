use std::{fs, io};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::mem::size_of;
use memmap2::{MmapMut, MmapOptions};
use crate::{convert_range, dir_list, download_range, HASH, HashRange, InterpolationSearch};
use bit_set::BitSet;

use futures::stream::{FuturesUnordered};
use futures::StreamExt;
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

    pub fn save(&self, hr: HashRange) -> io::Result<()> {
        let prefix: String = self.dbdir.clone()+"/range/";
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






