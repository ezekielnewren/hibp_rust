extern crate core;

mod util;
mod error;
mod bufferedio;
mod db;

use std::env;
use std::io::{prelude::*};
use std::mem::{size_of};
use std::ops::{Index, Range};
use std::time::Instant;

use hex;
use rand::{Rng};
use ring::rand::SecureRandom;
use crate::db::HIBPDB;
use crate::util::{HASH, binary_search_get_range};





fn go2() {

    let args: Vec<_> = env::args().collect();

    let mut db = HIBPDB::new(&args[1]);



    let rng = ring::rand::SystemRandom::new();
    let mut randpool = vec![0u8; 16*1000000];
    let mut off = randpool.len();


    let mut hrand = [0u8; 16];

    let mut loopit = 1;
    let mut timeit = 5.0;

    let method = 0;

    let mut arr: Vec<HASH> = vec![[0u8; 16]; db.index().len()];
    if method == 0 {
        print!("reading in file...");
        std::io::stdout().flush().unwrap();
        let buff = unsafe { arr.align_to_mut::<u8>().1 };
        db.index.fd.read_exact(buff).unwrap();
        println!("done");
    }

    let mut elapsed = 0.0;
    loop {
        let percent = 0.2;
        let mut range: Range<u64> = 0..(db.index().len() as f64 * percent) as u64;
        // index_slice = &index_slice[range.start as usize..range.end as usize];
        let beg = Instant::now();
        for _i in 0..loopit {
            if off >= randpool.len() {
                rng.fill(&mut randpool).unwrap();
                off = 0;
            }
            hrand.copy_from_slice(&randpool[off..off+size_of::<HASH>()]);
            off += size_of::<HASH>();

            match method {
                0 => {
                    let _ = arr.binary_search(&hrand);
                },
                1 => {
                    let _ = db.index().binary_search(&hrand);
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
