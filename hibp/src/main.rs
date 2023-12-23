use std::{env, io, thread};
use std::collections::VecDeque;
use std::io::{BufReader, prelude::*};
use std::ops::{Index, Range};
use std::sync::{Arc, Mutex};
use std::time::Instant;


use concurrent_queue::{ConcurrentQueue, PushError};
use clap::Parser;
use hex;
use hibp_core::db::HIBPDB;
use hibp_core::*;
use hibp_core::batch_transform::{BatchTransform, ConcurrentBatchTransform};

fn go2() {

    let args: Vec<_> = env::args().collect();

    let mut db = HIBPDB::new(&args[1]);

    let mut rng: RandomItemGenerator<HASH> = RandomItemGenerator::new(1000000);

    let mut loopit = 1;
    let mut timeit = 5.0;

    let method = 0;

    let mem = unsafe { UnsafeMemory::new(db.index().len()) }.unwrap();
    if method == 1 {
        print!("reading in file...");
        std::io::stdout().flush().unwrap();
        // let buff = unsafe { arr.align_to_mut::<u8>().1 };
        let buff = unsafe { mem.as_slice_mut() };
        db.index.fd.read_exact(buff).unwrap();
        println!("done");
    }

    let mut elapsed = 0.0;
    loop {
        let beg = Instant::now();
        for _i in 0..loopit {
            let hrand = rng.next_item();
            match method {
                0 => {
                    let _ = unsafe { mem.as_slice::<HASH>() }.binary_search(&hrand);
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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    dbdirectory: String,

    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

fn go3() {

    let args = Args::parse();

    let mut db = HIBPDB::new(&args.dbdirectory);

    let mut stdin = BufReader::new(io::stdin());
    let mut buff: Vec<u8> = Vec::new();

    let mut linecount = 0u64;
    let mut found = 0u64;
    let mut miss = 0u64;

    let mut transformer = ConcurrentBatchTransform::<Vec<u8>, HashAndPassword>::new(0, 0, |v| {
        let mut out = HashAndPassword {
            hash: Default::default(),
            password: v,
        };

        return match hash_password(&mut out) {
            Ok(_) => Some(out),
            Err(_) => None,
        };
    });

    let mut batch = |it: &mut VecDeque<HashAndPassword>| {
        for v in it.drain(..) {
            let key = v.hash;
            match db.find(&key) {
                Ok(_) => found += 1,
                Err(_) => miss += 1,
            }
        }
        assert!(it.is_empty())
    };

    let mut batch_queue = VecDeque::<HashAndPassword>::new();

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
        if buff.len() > 0 && buff[buff.len()-1] == b'\n' {
            buff.pop();
        }
        linecount += 1;

        transformer.add(buff.clone());
        transformer.take(&mut batch_queue);
        batch(&mut batch_queue);

    }

    transformer.close();
    transformer.take(&mut batch_queue);
    batch(&mut batch_queue);

    let seconds = start.elapsed().as_secs_f64();
    let rate = (linecount as f64 / seconds) as u64;

    let invalid_utf8 = linecount - (found + miss);
    println!("lines: {}, invalid_utf8: {}, found: {}, miss: {}", linecount, invalid_utf8, found, miss);
    println!("rate: {}", rate)


}

fn main() {
    go3();
}