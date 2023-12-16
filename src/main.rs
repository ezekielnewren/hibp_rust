extern crate core;

mod util;
mod error;
mod bufferedio;
mod db;

use md4::{Digest, Md4};
use std::{env, io};
use std::io::{BufReader, prelude::*};
use std::mem::{size_of};
use std::ops::{Index, Range};
use std::str::Utf8Error;
use std::time::Instant;

use hex;
use md4::digest::Update;
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

fn encode_to_utf16le(line: &str) -> Vec<u8> {
    let utf16: Vec<u16> = line.encode_utf16().collect();
    let bytes: Vec<u8> = utf16.iter().flat_map(|&v| v.to_le_bytes()).collect();
    return bytes;
}

fn go3() {

    let args: Vec<_> = env::args().collect();

    let mut db = HIBPDB::new(&args[1]);

    let mut stdin = BufReader::new(io::stdin());

    let mut buff: Vec<u8> = Vec::new();

    let mut linecount = 0;
    let mut asciicount = 0;
    let mut unreadable = 0;

    let start = Instant::now();
    loop {
        buff.clear();
        let _buff_size = buff.len();
        let _buff_capacity = buff.capacity();
        match stdin.read_until('\n' as u8, &mut buff) {
            Ok(0) => break, // EOF
            Err(err) => {
                break;
            }
            _ => {}
        }
        match std::str::from_utf8(buff.as_slice()) {
            Ok(v) => {
                let mut line: &str = v.trim_end_matches('\n');
                let mut hash_input: &[u8] = line.as_bytes();

                // if line.is_empty() { continue; }
                // let t = line.chars().filter(|&c| c != '\0').collect::<String>();
                // line = t.as_str();
                // if line.is_empty() { continue; }

                let raw = encode_to_utf16le(&line);
                hash_input = raw.as_slice();

                let mut hasher = Md4::new();
                md4::Digest::update(&mut hasher, hash_input);
                let hash: HASH = hasher.finalize().into();

                let is_ascii = line.chars().all(|c| c.is_ascii());
                if is_ascii {
                    asciicount += 1;
                }

                linecount += 1;
            }
            Err(err) => {
                unreadable += 1;
                continue;
            }
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    let rate = (linecount as f64 / seconds) as u64;


    println!("lines: {}, ascii: {}, unreadables: {}", linecount, asciicount, unreadable);
    println!("rate: {}", rate)


}

fn main() {
    go3();
}
