pub mod db;
pub mod transform;
pub mod minbitrep;
pub mod indexbycopy;
pub mod file_array;

use std::fmt::{Debug, Formatter};
use std::mem::{size_of};
use std::{io, slice};
use std::io::{BufRead, ErrorKind, Read, Write};
use std::ops::Deref;
use std::str::Utf8Error;
use std::sync::Arc;
use chrono::DateTime;
use flate2::Compression;
use flate2::write::GzEncoder;
use xz2::read::XzDecoder;

use md4::{Digest, Md4};
use rand::{RngCore, SeedableRng};
use xz2::write::XzEncoder;
use serde::{Serialize, Deserialize};

use tokio::runtime::Runtime;

pub type HASH = [u8; 16];

#[macro_export]
macro_rules! divmod {
    ($dividend:expr, $divisor:expr) => {
        (($dividend / $divisor), ($dividend % $divisor))
    };
}

#[allow(non_snake_case)]
pub fn HASH_to_hex(v: &HASH) -> String {
    hex::encode_upper(v)
}


static TOKIO_RUNTIME: once_cell::sync::Lazy<Arc<Runtime>> = once_cell::sync::Lazy::new(|| Arc::new(tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .unwrap()));

pub fn get_runtime() -> &'static Runtime {
     TOKIO_RUNTIME.deref()
}

pub fn compute_offset(slice: &[HASH], bit_len: u32) -> Vec<usize> {
    let mut offset: Vec<usize> = Vec::new();
    let mut prev = (1<<bit_len)-1;
    (0..slice.len()).into_iter().for_each(|i| {
        let v = u128::from_be_bytes(slice[i]);
        let cur = (v>>(128-bit_len)) as usize;
        if prev != cur {
            if prev < cur {
                for _ in prev..cur {
                    offset.push(i);
                }
            } else {
                offset.push(i);
            }
            prev = cur;
        }
    });
    offset.push(slice.len());
    return offset;
}


pub struct DownloadError {
    range: u32,
}

impl Debug for DownloadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:05X}", self.range).as_str())
    }
}

pub fn extract_gz(compressed: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = flate2::read::GzDecoder::new(compressed);
    let mut plain = Vec::new();
    decoder.read_to_end(&mut plain)?;
    return Ok(plain);
}

pub fn compress_gz(plain: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(plain)?;
    encoder.finish()
}

pub fn extract_xz(compressed: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = XzDecoder::new(compressed);
    let mut plain = Vec::new();
    decoder.read_to_end(&mut plain)?;
    return Ok(plain);
}

pub fn compress_xz(plain: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut compressor = XzEncoder::new(Vec::new(), 6);
    compressor.write_all(plain)?;
    return compressor.finish();
}

pub struct RandomItemGenerator<'a, T: Default + Copy> {
    rng: rand::rngs::StdRng,
    pool: Vec<T>,
    memory: &'a mut [u8],
    threshold: usize,
    off: usize,
}

impl<'a, T: Default + Copy> RandomItemGenerator<'a, T> {
    pub fn new(buffer_size: usize) -> Self {
        let mut pool: Vec<T> = vec![Default::default(); buffer_size];
        let slice = pool.as_mut_slice();
        let ptr: *mut u8 = slice.as_mut_ptr() as *mut u8;
        let len: usize = size_of::<T>()*slice.len();

        let memory: &mut [u8] = unsafe { slice::from_raw_parts_mut(ptr, len) };

        Self {
            rng: rand::rngs::StdRng::from_entropy(),
            pool,
            memory,
            threshold: buffer_size,
            off: buffer_size,
        }
    }

    #[inline]
    pub fn next_item(&mut self) -> &T {
        assert_eq!(self.pool.as_ptr() as *const u8, self.memory.as_ptr());
        assert_eq!(self.pool.len() * size_of::<T>(), self.memory.len());
        assert!(self.off <= self.pool.len());

        if self.off == self.threshold {
            self.rng.fill_bytes(self.memory);
            self.off = 0;
        }

        let t = self.off;
        self.off += 1;
        return &self.pool[t];
    }
}

pub fn encode_to_utf16le(line: &str) -> Vec<u8> {
    return line.encode_utf16().flat_map(|v| v.to_le_bytes()).collect();
}

pub struct HashAndPassword {
    pub hash: HASH,
    pub password: Vec<u8>,
}

pub fn hash_password(v: &mut HashAndPassword) -> Result<(), Utf8Error> {
    let password: &str = std::str::from_utf8(v.password.as_slice())?;
    let raw = encode_to_utf16le(password);

    let mut hasher = Md4::new();
    hasher.update(raw);
    v.hash = hasher.finalize().into();

    Ok(())
}

pub trait InterpolationSearch<T> {
    fn interpolation_search(&self, key: &T) -> Result<usize, usize>;
}

impl InterpolationSearch<HASH> for [HASH] {
    #[allow(non_snake_case)]
    fn interpolation_search(&self, key: &HASH) -> Result<usize, usize> {
        #[cfg(debug_assertions)]
        let _key__hash = HASH_to_hex(key);
        #[cfg(debug_assertions)]
        let _view_hash = String::from("");

        let slope: u128 = u128::MAX/self.len() as u128;
        let key_as_u128 = u128::from_be_bytes(*key);

        let guess: usize = (key_as_u128/slope) as usize;
        let mut step = 1usize;

        let mut lo = 0usize;
        let mut hi = self.len()-1;
        let _len = self.len();

        #[cfg(debug_assertions)]
        let _view_hash = HASH_to_hex(&self[guess]);

        let mut i = guess;
        if key < &self[i] {
            while key < &self[i] && i < _len {
                #[cfg(debug_assertions)]
                let _view_hash = HASH_to_hex(&self[i]);
                hi = i;
                i = match i.checked_sub(step) {
                    None => break,
                    Some(v) => v,
                };
                step <<= 1;
            }
            #[cfg(debug_assertions)]
            let _view_hash = HASH_to_hex(&self[i]);
            lo = i;
        } else {
            while key > &self[i] {
                #[cfg(debug_assertions)]
                let _view_hash = HASH_to_hex(&self[i]);
                lo = i;
                i += step;
                step <<= 1;
            }
            #[cfg(debug_assertions)]
            let _view_hash = HASH_to_hex(&self[i]);
            hi = i;
        }

        if i < _len {
            match self[lo..hi+1].binary_search(key) {
                Ok(v) => Ok(lo+v),
                Err(v) => Err(lo+v),
            }
        } else if i == _len {
            Err(i)
        } else {
            Err(0)
        }
    }
}



#[derive(Serialize, Deserialize, Debug)]
pub struct HashRange {
    pub range: u32,
    pub etag: u64,
    pub timestamp: i64,
    pub len: u64,
    pub sum: u64,
    pub format: String,
    #[serde(skip)]
    pub buff: Vec<u8>,
}

impl HashRange {
    pub fn name(range: u32) -> String {
        return format!("{:05X}.dat", range);
    }

    pub fn deserialize(buff: &[u8]) -> io::Result<Self> {
        let size = u16::from_le_bytes(buff[0..2].try_into().unwrap()) as usize;
        let meta = serde_cbor::from_slice::<HashRange>(&buff[2usize..2usize+size]);
        return match meta {
            Ok(mut hr) => {
                hr.buff.extend_from_slice(&buff[2+size..]);
                Ok(hr)
            }
            Err(e) => Err(io::Error::new(ErrorKind::InvalidInput, e)),
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut payload: Vec<u8> = Vec::new();

        let meta = serde_cbor::to_vec(&self).unwrap();
        let size = meta.len() as u16;
        payload.extend_from_slice(&size.to_le_bytes());
        payload.extend(&meta);
        payload.extend(&self.buff);

        payload
    }
}

pub async fn download_range(client: &reqwest::Client, range: u32) -> Result<HashRange, DownloadError> {
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

    let t = h.get("last-modified").unwrap().to_str().unwrap();
    let timestamp = DateTime::parse_from_rfc2822(t).unwrap().timestamp();

    let r = response.bytes().await;
    if r.is_err() {
        return Err(DownloadError{range});
    }

    let content: Vec<u8> = r.unwrap().to_vec();

    Ok(HashRange{
        range,
        etag: etag_u64,
        timestamp,
        len: 0,
        sum: 0,
        format: String::from("gz"),
        buff: content,
    })
}

pub fn convert_range(hr: HashRange) -> io::Result<(Vec<u8>, Vec<u64>)> {
    let plain = match hr.format.as_str() {
        "xz" => extract_xz(hr.buff.as_slice()),
        "gz" => extract_gz(hr.buff.as_slice()),
        _ => return Err(io::Error::new(ErrorKind::InvalidInput, "unsupported file type")),
    }?;

    let mut buff: Vec<u8> = Vec::new();
    let mut freq: Vec<u64> = Vec::new();

    let mut hash = vec![0u8; 16];
    for v in plain.lines() {
        let line = v?;
        let t = hex::decode_to_slice(format!("{:05X}{}", hr.range, &line[0..(32-5)]), hash.as_mut_slice());
        match t {
            Ok(_) => buff.extend(&hash),
            Err(e) => return Err(io::Error::new(ErrorKind::InvalidInput, e.to_string())),
        }
        let t = u64::from_str_radix(&line[33-5..], 16);
        match t {
            Ok(v) => freq.push(v),
            Err(e) => return Err(io::Error::new(ErrorKind::InvalidInput, e.to_string())),
        }
    }

    Ok((buff, freq))
}

pub fn dir_list(path: &str) -> std::io::Result<Vec<String>> {
    let mut list: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(path)? {
        let t = entry?.file_name();
        let name = t.to_str().unwrap();
        list.push(String::from(name));
    }

    return Ok(list);
}

