extern crate core;

mod test;
mod util;
mod error;

use std::env;
use std::fs::File;
use regex::{Error, Regex};

use std::io::{self, prelude::*, BufReader, SeekFrom};
use std::mem::size_of;
use std::ops::{Index, IndexMut, Range};

use hex;
use crate::util::{binary_search, bubble_sort, CloneableSlice, FileArray, IndexByCopy};

type HASH = [u8; 16];

struct HashArray {
    arr: FileArray,
}

impl IndexByCopy<HASH> for HashArray {
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

impl CloneableSlice<HASH> for HashArray {
    fn slice(self: &mut Self, range: Range<u64>) {
        todo!()
    }
}

struct HIBPDB {
    index: HashArray,
}

impl HIBPDB {
    pub fn new(v: &String) -> Self {
        let dbdir = v.clone();
        let mut file_index = dbdir.clone();
        file_index.push_str("/index.bin");
        let fa = FileArray::new(file_index, size_of::<HASH>() as u64);
        Self {
            index: HashArray{arr: fa},
        }
    }

    fn find(self: &mut Self, key: HASH) -> i64 {
        let len = self.index.len();
        return binary_search(&mut self.index, 0..len, &key, |lhs: &HASH, rhs: &HASH| lhs < rhs);
    }
}

// pub struct FileSlice<'a> {
//     fd: &'a File,
//     element_size: u64,
//     range: Range<u64>,
// }
//
// impl<T: Clone> CloneableSlice<T> for FileSlice<'_> {
//     fn slice(self: &mut Self, range: Range<u64>) {
//         todo!()
//     }
// }
//
// impl FileSlice<'_> {
//
//     pub fn new<T: Clone>(fd: &File,) -> FileSlice {
//         let element_size = size_of::<T>() as u64;
//         let size: u64 = fd.metadata().unwrap().len()/element_size;
//
//         FileSlice{
//             fd,
//             element_size,
//             range: 0..size,
//         }
//     }
//
// }
//
// impl<T: Clone> IndexByCopy<T> for FileSlice<'_> {
//     fn get(self: &mut Self, index: u64) -> T {
//         let mut tmp = [0u8; 16];
//
//         self.fd.seek(SeekFrom::Start(16*index)).unwrap();
//         self.fd.read(&mut tmp).unwrap();
//
//         return tmp;
//     }
//
//     fn set(self: &mut Self, index: u64, value: &T) {
//         self.fd.seek(SeekFrom::Start(16*index)).unwrap();
//         self.fd.write(value).unwrap();
//     }
//
//     fn len(self: &mut Self) -> u64 {
//         return self.fd.metadata().unwrap().len()/size_of::<T>() as u64;
//     }
// }


fn go1() {
    let args: Vec<_> = env::args().collect();

    let mut stdin = io::stdin();

    let mut buff = [0u8; 1<<20];

    loop {
        let read = stdin.read(&mut buff).expect("TODO: panic message");
        if read == 0 {break;}
    }

    // let pathname = String::from(args[2].as_str());
    // let mut db = HIBPDB::new(&pathname);
    //
    // // let hash: HASH = hex::decode("00000011059407D743D40689940F858C").unwrap().as_slice().try_into().unwrap();
    // // let index = db.find(hash);
    //
    // bubble_sort(&mut db.index, |lhs: &HASH, rhs: &HASH| lhs < rhs);
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
