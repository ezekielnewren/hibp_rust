use clap::Parser;
use std::{env, io};
use std::io::{BufReader, prelude::*};
use std::ops::{Index, Range};
use std::time::Instant;

use hex;
use hibp_core::db::HIBPDB;
use hibp_core::*;

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

    let mut rng = RandomItemGenerator::<HASH>::new(1000);

    let mut stdin = BufReader::new(io::stdin());
    let mut buff: Vec<u8> = Vec::new();
    let mut queue: Vec<HashAndPassword> = Vec::new();

    let mut linecount = 0u64;
    let mut found = 0u64;
    let mut invalid_utf8 = 0u64;
    let mut miss = 0u64;

    let hashit = true;

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

        queue.push(HashAndPassword{
            hash: Default::default(),
            password: buff.clone(),
        });

        let mut t = queue.pop().unwrap();

        let HASH_NULL: HASH = Default::default();
        let key: &HASH;
        if hashit {
            match hash_password(&mut t) {
                Ok(_) => {},
                Err(_) => {
                    invalid_utf8 += 1;
                    continue
                }
            }
            key = &t.hash;
        } else {
            key = rng.next_item();
            // key = &HASH_NULL;
        }

        match db.find(&key) {
            Ok(_) => found += 1,
            Err(_) => miss += 1,
        }
    }

    let seconds = start.elapsed().as_secs_f64();
    let rate = (linecount as f64 / seconds) as u64;
    
    println!("lines: {}, invalid_utf8: {}, found: {}, miss: {}", linecount, invalid_utf8, found, miss);
    println!("rate: {}", rate)


}

fn main() {
    go3();
}