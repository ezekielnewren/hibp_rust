use std::io;
use std::io::{BufReader, prelude::*};
use std::time::Instant;

use clap::Parser;
use hibp_core::db::HIBPDB;
use hibp_core::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    dbdirectory: String,

    #[arg(short, long)]
    update: bool,

    #[arg(short, long)]
    ingest: bool,

    #[arg(short, long)]
    construct: bool,

    #[arg(short, long)]
    test: bool,
}

fn ingest(args: Args) {
    let mut db = HIBPDB::new(args.dbdirectory).unwrap();

    let mut stdin = BufReader::new(io::stdin());
    let mut buff: Vec<u8> = Vec::new();

    let mut linecount = 0u64;
    let mut found = 0u64;
    let mut miss = 0u64;

    let mut hp = HashAndPassword{
        hash: [0u8; 16],
        password: vec![],
    };

    let start = Instant::now();
    loop {
        buff.clear();
        let _buff_size = buff.len();
        let _buff_capacity = buff.capacity();
        match stdin.read_until('\n' as u8, &mut buff) {
            Ok(0) => break, // EOF
            Err(_) => {
                break;
            }
            _ => {}
        }
        if buff.len() > 0 && buff[buff.len()-1] == b'\n' {
            buff.pop();
        }

        hp.password.clear();
        hp.password.extend(&buff);
        if hash_password(&mut hp).is_err() {
            continue;
        }

        match db.find(&hp.hash) {
            Ok(_) => found += 1,
            Err(_) => miss += 1,
        }

        linecount += 1;

    }

    let seconds = start.elapsed().as_secs_f64();
    let rate = (linecount as f64 / seconds) as u64;

    let invalid_utf8 = linecount - (found + miss);
    println!("lines: {}, invalid_utf8: {}, found: {}, miss: {}", linecount, invalid_utf8, found, miss);
    println!("rate: {}", rate)
}

fn update(args: Args) {
    let mut db = HIBPDB::new(args.dbdirectory).unwrap();

    let status: fn(u32) = |range| {
        println!("{:05X}", range);
    };

    db.update(status).unwrap();
}

fn construct(args: Args) {
    let db = HIBPDB::new(args.dbdirectory).unwrap();

    let status: fn(u32) = |range| {
        println!("{:05X}", range);
    };

    db.construct_index(status).unwrap();
}

fn test(args: Args) {
    let mut db = HIBPDB::new(args.dbdirectory).unwrap();

    let status: fn(u32) = |range| {
        println!("{:05X}", range);
    };

    db.sort_freq().unwrap();
}

fn main() {
    let args = Args::parse();

    if args.ingest {
        ingest(args);
    } else if args.update {
        update(args);
    } else if args.construct {
        construct(args);
    } else if args.test {
        test(args);
    }
}

















