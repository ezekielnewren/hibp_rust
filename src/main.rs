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
use crate::util::{binary_search, HASH, HashMemoryArray, IndexByCopy};





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

    let ondisk = false;

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
            binary_search(&index, range, &hrand);
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
