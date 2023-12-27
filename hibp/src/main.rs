use std::{io};
use std::collections::VecDeque;
use std::io::{BufReader, prelude::*};
use std::time::Instant;

use clap::Parser;
use hex;
use hibp_core::db::HIBPDB;
use hibp_core::*;
use hibp_core::batch_transform::{BatchTransform, ConcurrentBatchTransform};

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

    let mut db = HIBPDB::new(args.dbdirectory);

    let mut stdin = BufReader::new(io::stdin());
    let mut buff: Vec<u8> = Vec::new();

    let mut linecount = 0u64;
    let mut found = 0u64;
    let mut miss = 0u64;

    let thread_count = num_cpus::get_physical();

    let cl = |v| {
        let mut out = HashAndPassword {
            hash: Default::default(),
            password: v,
        };

        return match hash_password(&mut out) {
            Ok(_) => Some(out),
            Err(_) => None,
        };
    };

    let mut transformer = ConcurrentBatchTransform::<Vec<u8>, HashAndPassword>::new(thread_count, 1, cl);
    // let mut transformer = SerialBatchTransform::<Vec<u8>, HashAndPassword>::new(500000, cl);

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

    let mut batch_in = VecDeque::<Vec<u8>>::new();
    let mut batch_out = VecDeque::<HashAndPassword>::new();

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

        batch_in.push_back(buff.clone());

        if batch_in.len() >= transformer.get_batch_size() {
            transformer.add(&mut batch_in);
            transformer.take(&mut batch_out);
            batch(&mut batch_out);
        }

    }

    transformer.close();
    transformer.take(&mut batch_out);
    batch(&mut batch_out);

    let seconds = start.elapsed().as_secs_f64();
    let rate = (linecount as f64 / seconds) as u64;

    let invalid_utf8 = linecount - (found + miss);
    println!("lines: {}, invalid_utf8: {}, found: {}, miss: {}", linecount, invalid_utf8, found, miss);
    println!("rate: {}", rate)


}

fn main() {
    go3();
}