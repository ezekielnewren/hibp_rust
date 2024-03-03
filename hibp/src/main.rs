use std::io;
use std::io::{BufReader, BufWriter, prelude::*};
use std::path::PathBuf;
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

    #[arg(long)]
    left: bool,

    #[arg(short, long)]
    ingest: bool,

    #[arg(short, long)]
    construct: bool,

    #[arg(short, long)]
    test: bool,
}

fn ingest(args: Args) {
    let dbdir = PathBuf::from(args.dbdirectory);
    let mut db = HIBPDB::open(dbdir.as_path()).unwrap();

    let mut stdin = BufReader::new(io::stdin());
    let mut buff: Vec<u8> = Vec::new();

    let mut linecount = 0u64;
    let mut new_password = 0u64;
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
            Ok(i) => {
                if ! db.password_bitset.get(i as u64) {
                    hp.password.push(b'\n');
                    db.submit(i, hp.password.as_slice()).unwrap();
                    new_password += 1;
                }

                found += 1
            },
            Err(_) => miss += 1,
        }

        linecount += 1;
    }

    db.commit().unwrap();


    let seconds = start.elapsed().as_secs_f64();
    let rate = (linecount as f64 / seconds) as u64;

    let invalid_utf8 = linecount - (found + miss);
    println!("lines: {}, invalid_utf8: {}, new: {}, found: {}, miss: {}", linecount, invalid_utf8, new_password, found, miss);
    println!("rate: {}", rate)
}

fn update(args: Args) {
    let dbdir = PathBuf::from(args.dbdirectory);

    let status: fn(u32) = |range| {
        println!("{:05X}", range);
    };

    HIBPDB::update_download_missing(get_runtime(), dbdir.as_path(), status).unwrap();
    HIBPDB::update_password_metadata(dbdir.as_path()).unwrap();
}

fn left(args: Args) {
    let dbdir = PathBuf::from(args.dbdirectory.clone());
    let db = HIBPDB::open(dbdir.as_path()).unwrap();

    let lock = io::stdout().lock();
    let mut writer = BufWriter::new(lock);

    let mut dump = [0u8; 32];
    for i in 0..db.len() {
        let row = db.frequency_idx.as_slice()[i] as usize;
        if db.password_bitset.get(row as u64) {
            continue;
        }
        let h = &db.hash()[row];
        hex::encode_to_slice(h, &mut dump).unwrap();
        let f = db.frequency_col.as_slice()[row];

        writeln!(writer, "{}:{}", std::str::from_utf8(&dump).unwrap(), f).unwrap();
    }
    writer.flush().unwrap();
}

fn construct(args: Args) {
    let dbdir = PathBuf::from(args.dbdirectory);

    let status: fn(u32) = |range| {
        println!("{:05X}", range);
    };

    HIBPDB::update_construct_columns(dbdir.as_path(), status).unwrap();
}

fn main() {
    let args = Args::parse();

    if args.ingest {
        ingest(args);
    } else if args.update {
        update(args);
    } else if args.construct {
        construct(args);
    } else if args.left {
        left(args);
    }
}

















