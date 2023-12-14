extern crate core;

mod util;
mod error;

use std::cmp::{max, min};
use std::collections::btree_map::Range;
use std::env;
use std::ffi::c_void;

use std::io::{self, prelude::*};
use std::mem::{size_of, transmute};
use std::ops::{Index};
use std::ptr::slice_from_raw_parts_mut;
use std::time::Instant;

use hex;
use rand::{random, Rng};
use ring::rand::SecureRandom;
use crate::util::{binary_search, binary_search_generate_cache, binary_search_get_range, cmp_default, FileArray, IndexByCopy};

type HASH = [u8; 16];

struct HashFileArray {
    pub arr: FileArray,
}
impl IndexByCopy<HASH> for HashFileArray {
    fn get(&self, index: u64) -> HASH {
        let mut tmp = [0u8; 16];
        self.arr.get(index, &mut tmp).unwrap();
        return tmp;
    }

    fn set(&mut self, index: u64, value: &HASH) {
        self.arr.set(index, value).expect("TODO: panic message");
    }

    fn len(&self) -> u64 {
        return self.arr.len();
    }
}

struct HashMemoryArray {
    arr: Vec<u8>,
}

impl IndexByCopy<HASH> for HashMemoryArray {
    fn get(&self, index: u64) -> HASH {
        let start: usize = (index * size_of::<HASH>() as u64) as usize;
        let end: usize = start+size_of::<HASH>();
        let mut junk = [0u8; 16];
        let mut value = junk.as_mut_slice();
        let element = &self.arr.as_slice()[start..end];
        // value = element
        // value.copy_from_slice(&element);
        junk.as_mut_slice().copy_from_slice(&element);
        return junk;
    }

    fn set(&mut self, index: u64, value: &HASH) {
        let start = (index*size_of::<HASH>() as u64) as usize;
        let end = start+size_of::<HASH>();
        let element: &mut [u8] = &mut self.arr.as_mut_slice()[start..end];
        // element = value
        element.copy_from_slice(value);
    }

    fn len(&self) -> u64 {
        (self.arr.len() / size_of::<HASH>()) as u64
    }
}

struct HIBPDB {
    index: HashFileArray,
    index_cache: Vec<HASH>,
}

impl HIBPDB {
    pub fn new(v: &String) -> Self {
        let dbdir = v.clone();
        let mut file_index = dbdir.clone();
        file_index.push_str("/index.bin");
        let fa = FileArray::new(file_index, size_of::<HASH>() as u64);

        let log2 = (fa.len() as f64).log2().ceil() as u64;
        let depth = min(log2/2, log2);

        let mut hfa = HashFileArray {arr: fa};
        let mut cache: Vec<HASH> = Vec::new();
        binary_search_generate_cache(&hfa, 0..hfa.len(), &mut cache, depth);

        Self {
            index: hfa,
            index_cache: cache,
        }
    }

    fn find(self: &mut Self, key: HASH) -> i64 {
        let cmp = cmp_default();
        let mut range = 0..self.index.len();
        range = binary_search_get_range(&self.index_cache, 0..self.index.len(), cmp, &key);
        return binary_search(&mut self.index, range, cmp, &key);
    }
}

fn go2() {

    let args: Vec<_> = env::args().collect();

    let mut db = HIBPDB::new(&args[1]);

    let mut index = HashMemoryArray{
        arr: vec![0u8; db.index.arr.fd.metadata().unwrap().len() as usize],
    };

    let fsize = db.index.arr.fd.metadata().unwrap().len() as usize;
    let mut buff = vec![0; fsize];

    print!("reading in file...");
    std::io::stdout().flush().unwrap();
    db.index.arr.fd.read_exact(buff.as_mut_slice()).unwrap();
    println!("done");

    let rng = ring::rand::SystemRandom::new();
    let mut randpool = vec![0u8; 16*1000000];
    let mut off = randpool.len();

    let mut ondisk = true;
    // ondisk = false;

    let cmp = cmp_default::<HASH>();
    // let mut asdfg = 0..db.index.len();

    let mut hrand = [0u8; 16];
    let mut count = 0u64;
    let beg = Instant::now();
    loop {
        if off >= randpool.len() {
            rng.fill(&mut randpool).unwrap();
            off = 0;
        }
        hrand.copy_from_slice(&randpool[off..off+size_of::<HASH>()]);
        off += size_of::<HASH>();

        if ondisk {
            db.find(hrand);
        } else {
            let mut range = 0..db.index.len();
            // range = binary_search_get_range(&db.index_cache, 0..db.index.len(), cmp, &hrand);
            binary_search(&index, range, cmp_default(), &hrand);
        }
        count += 1;
        if (count&0xff) == 0 {
            rng.fill(&mut hrand).unwrap();
            if beg.elapsed().as_millis() > 10000 {
                break;
            }
        }
    }
    let rate = (count as f64 / beg.elapsed().as_secs_f64()) as u64;

    println!("{} hashes/s", rate)
}

fn main() {
    go2();
}
