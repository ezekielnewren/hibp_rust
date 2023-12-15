extern crate core;

mod util;
mod error;
mod bufferedio;
mod db;

use std::cmp::{max, min};
use std::env;
use std::ffi::c_void;

use std::io::{self, prelude::*};
use std::mem::{size_of, transmute};
use std::ops::{Index, Range};
use std::ptr::slice_from_raw_parts_mut;
use std::time::Instant;

use hex;
use rand::{random, Rng};
use ring::rand::SecureRandom;
use crate::db::HIBPDB;
use crate::util::{binary_search, binary_search_get_range, FileArray, HASH, HashFileArray, HashMemoryArray, HashMmapArray, IndexByCopy};





fn go2() {

    let args: Vec<_> = env::args().collect();

    let mut db = HIBPDB::new(&args[1]);

    let mut index = HashMemoryArray{
        arr: vec![0u8; db.index.arr.fd.metadata().unwrap().len() as usize],
    };

    let mut index_mmap = HashMmapArray::new(&db.index.arr.fd);

    let fsize = (&db.index.arr.fd).metadata().unwrap().len() as usize;
    let mut buff = vec![0; fsize];

    let rng = ring::rand::SystemRandom::new();
    let mut randpool = vec![0u8; 16*1000000];
    let mut off = randpool.len();


    let t: (&[u8], &[HASH], &[u8]) = unsafe { index.arr.align_to::<HASH>() };
    let mut index_slice: &[HASH] = t.1;

    let mut hrand = [0u8; 16];

    let mut loopit = 1;
    let mut timeit = 5.0;

    let method = 2;

    if method == 0 {
        print!("reading in file...");
        std::io::stdout().flush().unwrap();
        db.index.arr.fd.read_exact(buff.as_mut_slice()).unwrap();
        println!("done");
    }

    let mut elapsed = 0.0;
    loop {
        let percent = 0.2;
        let mut range: Range<u64> = 0..(db.index.len() as f64 * percent) as u64;
        range = 0..db.index.len();
        // index_slice = &index_slice[range.start as usize..range.end as usize];
        let beg = Instant::now();
        for _ in 0..loopit {
            if off >= randpool.len() {
                rng.fill(&mut randpool).unwrap();
                off = 0;
            }
            hrand.copy_from_slice(&randpool[off..off+size_of::<HASH>()]);
            off += size_of::<HASH>();

            match method {
                0 => {
                    let _ = index_slice.binary_search(&hrand);
                },
                1 => {
                    binary_search(&index, &range, &hrand);
                },
                2 => {
                    range = binary_search_get_range(&db.index_cache, &range, &hrand);
                    binary_search(&index_mmap, &range, &hrand);
                },
                3 => {
                    // let mut range = 0..self.index.len();
                    // db.find(hrand);
                    // range = binary_search_get_range(&db.index_cache, &range, &hrand);
                    binary_search(&db.index, &range, &hrand);
                },
                _ => panic!("invalid method")
            }
        }

        elapsed = beg.elapsed().as_secs_f64();
        if elapsed > timeit { break; }
        loopit += loopit*(timeit/elapsed) as u64;
    }
    let rate = (loopit as f64 / elapsed) as u64;

    println!("{} hashes/s", rate)
}

fn go3() {

    let args: Vec<_> = env::args().collect();

    let mut db = HIBPDB::new(&args[1]);

}

fn main() {
    go2();
}
