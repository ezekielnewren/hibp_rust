use std::env;
use std::fs::File;
use regex::{Error, Regex};

use std::io::{self, prelude::*, BufReader, SeekFrom};

use hex;

fn go0() {
    let args: Vec<_> = env::args().collect();
    let mut re = Regex::new(r"^([0-9a-fA-F]{32}):([0-9]+)$").unwrap();
    // re = Regex::new(r"^([0-9a-fA-F]+).*").unwrap();

    let ntlm_raw = File::open(args[1].as_str()).expect("no such file");
    let reader = BufReader::new(ntlm_raw);

    let mut file_index = File::create(args[2].as_str()).expect("no such file");

    let mut i = 0;

    for v in reader.lines() {
        // if i >= 10 {break;}
        let line = v.unwrap();
        let m = re.captures(line.as_str()).unwrap();
        let hash = m.get(1).unwrap().as_str();
        let count = m.get(2).unwrap().as_str();

        let bin = hex::decode(hash).expect("failed to decode");
        let c = count.parse::<u32>().expect("failed to parse int");

        file_index.write_all(bin.as_slice()).expect("TODO: panic message");

        // println!("{} {} {}", line, hash, c);
        i += 1;
    }



}


fn go1() {
    let args: Vec<_> = env::args().collect();

    let mut buff: [u8; 4096] = [0u8; 4096];
    let mut file_index = File::open(args[2].as_str()).unwrap();

    let off = 16;
    let len = 16*7;

    file_index.seek(SeekFrom::Start(16)).unwrap();

    let read = file_index.read(&mut buff[off..off+len]).unwrap();
    println!("{}", read);

}

fn main() {
    go1();
}
