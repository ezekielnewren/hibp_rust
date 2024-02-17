use std::any::Any;
use std::fs;
use std::fs::File;
use std::future::Future;
use std::io::{Read, Write};
use std::mem::size_of;
use flate2::Compression;
use flate2::write::GzEncoder;
use memmap2::{Mmap, MmapOptions};
use reqwest::Error;
use tokio::select;
use crate::{compress_gz, dirMap, DownloadError, extract_gz, GenericError, HASH, InterpolationSearch};
use bit_set::BitSet;

use futures::future::{self, BoxFuture, select_all};
use futures::stream::FuturesUnordered;
use futures::StreamExt;

pub struct FileArray {
    pub pathname: String,
    pub fd: File,
    pub mmap: Mmap,
}

pub struct HashRange {
    pub range: u32,
    pub etag: u64,
    pub compressed: Vec<u8>,
}

async fn download_range(client: &reqwest::Client, range: u32) -> Result<HashRange, DownloadError> {
    let base_url = "https://api.pwnedpasswords.com/range/X?mode=ntlm";
    let t = format!("{:05X}", range);
    let url = base_url.replace("X", t.as_str());

    let r = client.get(url)
        .header(reqwest::header::ACCEPT_ENCODING, "gzip")
        .send()
        .await;

    if r.is_err() {
        return Err(DownloadError{ range });
    }
    let response = r.unwrap();

    let h = response.headers();
    let mut etag = h.get("etag").unwrap().to_str().unwrap();
    let prefix = "W/\"0x";
    if etag.starts_with(prefix) {
        etag = &etag[prefix.len()..etag.len()-1]
    }
    let etag_u64 = u64::from_str_radix(etag, 16).unwrap();

    let mut t: Vec<String> = vec![];
    for v in h.keys() {
        t.push(v.to_string());
    }

    let r = response.bytes().await;
    if r.is_err() {
        return Err(DownloadError{range});
    }

    let content: Vec<u8> = r.unwrap().to_vec();

    // extract the payload, delete carriage returns, recompress
    let mut plain = extract_gz(content.as_slice()).unwrap();
    plain.retain(|&x| x != b'\r');
    let compressed = compress_gz(plain.as_slice()).unwrap();

    Ok(HashRange{
        range,
        etag: etag_u64,
        compressed,
    })
}

pub struct HIBPDB<'a> {
    pub dbdir: String,
    pub index: FileArray,
    pub index_slice: &'a [HASH],
    pub rt: tokio::runtime::Runtime,
}

impl<'a> HIBPDB<'a> {
    pub fn new(v: String) -> Self {
        let dbdir = v.clone();
        let mut file_index = dbdir.clone();
        file_index.push_str("/index.bin");

        let fd = File::open(file_index).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&fd).unwrap() };

        let mut fa = FileArray {
            pathname: v.clone(),
            fd,
            mmap,
        };

        let index_slice: &'a [HASH] = unsafe {
            let ptr = fa.mmap.as_ptr() as *const HASH;
            std::slice::from_raw_parts(ptr, fa.mmap.len()/size_of::<HASH>())
        };

        Self {
            dbdir,
            index: fa,
            index_slice,
            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
        }
    }

    pub fn save(self: &Self, hr: HashRange) -> Result<(), GenericError> {
        let prefix: String = self.dbdir.clone()+"/range/";
        let fname = format!("{:05X}_{:016X}.gz", hr.range, hr.etag);

        let path_tmp = prefix.clone()+"tmp."+fname.as_str();
        let pathname = prefix+fname.as_str();
        {
            let mut fd = File::create(&path_tmp)?;
            fd.write_all(hr.compressed.as_slice())?;
        }
        fs::rename(path_tmp, pathname)?;

        Ok(())
    }

    pub fn update<F>(self: &Self, mut f: F) -> Result<(), GenericError> where F: FnMut(u32)  {
        let map = dirMap((self.dbdir.clone()+"/range/").as_str())?;

        let limit = 1000;

        let client = reqwest::Client::new();

        let fut = async {
            let mut queue = FuturesUnordered::new();

            let map = dirMap(format!("{}/range", self.dbdir).as_str()).unwrap();
            let mut bs = BitSet::new();
            for key in map.keys() {
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

    #[inline]
    pub fn index(self: &Self) -> &[HASH] {
        return self.index_slice;
    }

    pub fn find(self: &mut Self, key: &HASH) -> Result<usize, usize> {
        self.index().interpolation_search(key)
    }

    pub fn len(&self) -> usize {
        self.index().len()
    }
}
