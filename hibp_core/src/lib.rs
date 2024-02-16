pub mod db;
pub mod batch_transform;

use std::fmt::{Debug, Display, Formatter};
use std::mem::{size_of};
use std::ops::{Index, IndexMut};
use std::{slice};
use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::fs::DirEntry;
use std::io::{Read, Write};
use std::panic::UnwindSafe;
use std::path::PathBuf;
use std::str::Utf8Error;
use flate2::Compression;
use flate2::write::GzEncoder;

use md4::{Digest, Md4};
use rand::{RngCore, SeedableRng};

pub type HASH = [u8; 16];

pub fn HASH_to_hex(v: &HASH) -> String {
    hex::encode_upper(v)
}


pub struct DownloadError {
    range: u32,
}

pub struct GenericError {
    err: Box<dyn Any>,
}

impl Debug for GenericError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // f.write_str(self.err.as)
        Ok(())
    }
}

impl From<reqwest::Error> for GenericError {
    fn from(value: reqwest::Error) -> Self {
        GenericError{
            err: Box::new(value),
        }
    }
}

impl From<std::io::Error> for GenericError {
    fn from(value: std::io::Error) -> Self {
        GenericError{
            err: Box::new(value),
        }
    }
}


pub fn extract(compressed: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = flate2::read::GzDecoder::new(compressed);
    let mut plain = Vec::new();
    decoder.read_to_end(&mut plain)?;
    return Ok(plain);
}

pub fn compress(plain: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(plain)?;
    encoder.finish()
}



pub struct Job {
    pub closure: Box<dyn FnOnce()>,
}

unsafe impl Send for Job {}
impl UnwindSafe for Job {}

impl Job {

    pub fn new<F>(job: F) -> Job where F: FnOnce() + 'static {
        Self {
            closure: Box::new(job),
        }
    }

    pub fn invoke(self) {
        (self.closure)();
    }
}

pub struct Transform<From, To> {
    lambda: Box<dyn Fn(From) -> To>,
}

unsafe impl<From, To> Send for Transform<From, To> {}

impl<From, To> Transform<From, To> {

    pub fn new<F>(transform: F) -> Transform<From, To> where F: Fn(From) -> To + 'static {
        Self {
            lambda: Box::new(transform),
        }
    }

    pub fn call_lambda(self, item: From) -> To {
        (self.lambda)(item)
    }
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


pub fn dirMap(path: &str) -> Result<BTreeMap<String, DirEntry>, GenericError> {
    let mut map: BTreeMap<String, DirEntry> = BTreeMap::new();

    for entry in std::fs::read_dir(path)? {
        let dir_entry = entry?;
        let path = dir_entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                map.insert(filename.to_owned(), dir_entry);
            }
        }
    }

    return Ok(map);
}


