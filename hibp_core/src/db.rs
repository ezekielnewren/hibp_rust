use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::mem::size_of;
use flate2::Compression;
use flate2::write::GzEncoder;
use memmap2::{Mmap, MmapOptions};
use reqwest::Error;
use crate::{dirMap, GenericError, HASH, InterpolationSearch};

pub struct FileArray {
    pub pathname: String,
    pub fd: File,
    pub mmap: Mmap,
}

pub struct HashRange {
    pub range: u32,
    pub etag: u64,
    pub plain: Vec<u8>,
}

fn download_range(rt: &tokio::runtime::Runtime, range: u32) -> Result<HashRange, GenericError> {
    let base_url = "https://api.pwnedpasswords.com/range/X?mode=ntlm";
    let t = format!("{:05X}", range);
    let url = base_url.replace("X", t.as_str());

    rt.block_on(async {
        let client = reqwest::Client::new();
        let response = client.get(url)
            .header(reqwest::header::ACCEPT_ENCODING, "gzip")
            .send()
            .await?
            .error_for_status()?;

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

        let content: Vec<u8> = response.bytes().await?.to_vec();
        let mut decoder = flate2::read::GzDecoder::new(content.as_slice());
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;
        decompressed_data.retain(|&x| x != b'\r');
        Ok(HashRange{
            range,
            etag: etag_u64,
            plain: decompressed_data,
        })
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

    pub fn download(self: &Self, range: u32) -> Result<(), GenericError> {
        let prefix: String = self.dbdir.clone()+"/range/";
        let hr = download_range(&self.rt, range)?;
        let fname = format!("{:05X}_{:016X}.gz", hr.range, hr.etag);
        // let pathname = prefix+fname.as_str();

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(hr.plain.as_slice())?;
        let compressed_data = encoder.finish()?;

        let path_tmp = prefix.clone()+"tmp."+fname.as_str();
        let pathname = prefix+fname.as_str();
        {
            let mut fd = File::create(&path_tmp)?;
            fd.write_all(compressed_data.as_slice())?;
        }
        fs::rename(path_tmp, pathname)?;

        Ok(())
    }

    pub fn update<F>(self: &Self, mut f: F) -> Result<(), GenericError> where F: FnMut(u32)  {
        let map = dirMap((self.dbdir.clone()+"/range/").as_str())?;

        for i in 0..(1<<20) {
            let prefix = format!("{:05X}", i);
            let m = map.keys().filter(|&key| key.starts_with(&prefix));
            if m.count() == 0 {
                f(i);
                self.download(i)?;
            }
        }
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
