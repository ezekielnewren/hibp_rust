extern crate core;

mod test;
mod util;
mod error;

use std::env;

use std::io::{self, prelude::*};
use std::mem::size_of;
use std::ops::{Index};

use hex;
use crate::util::{binary_search, FileArray, IndexByCopy};

type HASH = [u8; 16];

struct HashFileArray {
    arr: FileArray,
}

impl IndexByCopy<HASH> for HashFileArray {
    fn get(&mut self, index: u64) -> HASH {
        let mut tmp = [0u8; 16];
        self.arr.get(index, &mut tmp).unwrap();
        return tmp;
    }

    fn set(&mut self, index: u64, value: &HASH) {
        self.arr.set(index, value).expect("TODO: panic message");
    }

    fn len(&mut self) -> u64 {
        self.arr.len()
    }
}

struct HIBPDB {
    index: HashFileArray,
}

impl HIBPDB {
    pub fn new(v: &String) -> Self {
        let dbdir = v.clone();
        let mut file_index = dbdir.clone();
        file_index.push_str("/index.bin");
        let fa = FileArray::new(file_index, size_of::<HASH>() as u64);
        Self {
            index: HashFileArray {arr: fa},
        }
    }

    fn find(self: &mut Self, key: HASH) -> i64 {
        let len = self.index.len();
        return binary_search(&mut self.index, 0..len, &key, |lhs: &HASH, rhs: &HASH| lhs < rhs);
    }
}

fn go2() {

    let args: Vec<_> = env::args().collect();


    let mut db = HIBPDB::new(&args[1]);

    let mut stdin = io::stdin();
    let mut buff: HASH = [0u8; 16];

    loop {
        stdin.read_exact(&mut buff).expect("TODO: panic message");
        db.find(buff);
    }


}

fn main() {
    go2();
}
