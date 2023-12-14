extern crate core;

mod util;
mod error;
mod bufferedio;
mod db;

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
use crate::db::HIBPDB;
use crate::util::{binary_search, binary_search_get_range, HASH, HashMemoryArray, IndexByCopy};





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


    let t: (&[u8], &[HASH], &[u8]) = unsafe { index.arr.align_to::<HASH>() };
    let mut index_slice: &[HASH] = t.1;

    let mut hrand = [0u8; 16];

    let mut loopit = 1;
    let mut timeit = 5.0;

    let ondisk = true;
    let mut elapsed = 0.0;
    loop {
        let percent = 1.0;
        let mut range = 0..(db.index.len() as f64 * percent) as usize;
        range = 0..100;
        index_slice = &index_slice[range];
        let beg = Instant::now();
        for i in 0..loopit {
            if off >= randpool.len() {
                rng.fill(&mut randpool).unwrap();
                off = 0;
            }
            hrand.copy_from_slice(&randpool[off..off+size_of::<HASH>()]);
            off += size_of::<HASH>();

            if ondisk {
                db.find(hrand);
            } else {
                // range = binary_search_get_range(&db.index_cache, range, &hrand);
                // binary_search(&index, &range, &hrand);
                let _ = index_slice.binary_search(&hrand);
            }
        }

        elapsed = beg.elapsed().as_secs_f64();
        if elapsed > timeit { break; }
        loopit += loopit*(timeit/elapsed) as u64;
    }
    let rate = (loopit as f64 / elapsed) as u64;

    println!("{} hashes/s", rate)
}

fn main() {
    go2();
}
