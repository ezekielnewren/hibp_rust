// #![feature(test)]


use std::{fs, thread};
use std::ops::{Index, IndexMut, Range};
use std::str::Utf8Error;
use std::sync::Arc;
use std::thread::{available_parallelism, JoinHandle};
use std::time::Duration;
use hibp_core::{hash_password, HashAndPassword};
// use hibp_core::batch_transform::ConcurrentIterator;
// use hibp_core::thread_pool::ThreadPool;

// extern crate test;

const DIR_SRC_DATA: &str = "src/data";
const DIR_TESTS_DATA: &str = "tests/data";


#[test]
fn test_test_data_directory() {
    let cd = fs::canonicalize(".");

    let result = fs::metadata(DIR_TESTS_DATA);
    assert!(result.is_ok());

    assert!(result.unwrap().is_dir());
}


// #[test]
// fn test_unordered_queue() {
//     let thread_count = num_cpus::get();
//     let mut it: ConcurrentIterator<Vec<u8>, HashAndPassword> = ConcurrentIterator::new(thread_count, |input| {
//         let mut hp = HashAndPassword {
//             hash: Default::default(),
//             password: input,
//         };
//
//         return match hash_password(&mut hp) {
//             Ok(_) => Some(hp),
//             Err(_) => None,
//         };
//     });
//
//     it.close();
//
//     for hap in it {
//
//     }
//
// }


struct ThreadPool {
    pool: Vec<JoinHandle<()>>,
}

impl ThreadPool {
    pub fn new(size: usize) {
        let mut inst = Self{
            pool: Vec::new(),
        };

        for _ in 0..size {
            inst.pool.push(thread::spawn(move || {
                // do something
            }));
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // for handle in self.pool.into_iter() {
        //     let _ = handle.join();
        // }
        for handle in self.pool.drain(..) {
            let _ = handle.join();
        }
        while let Some(handle) = self.pool.pop() {
            let _ = handle.join();
        }
    }
}


#[test]
fn test_arbitrary_code_snippet() {

    // let mut handles = Vec::new();
    let mut tp = ThreadPool{
        pool: Vec::new(),
    };
    tp.pool.push(thread::spawn(move || {
        // let mut data = snd_rx.recv().unwrap();
        // data += 1;
        // let _ = rcv_tx.send(data);
    }));

    // for handle in tp.pool.into_iter().rev() {
    //     let _ = handle.join();
    // }

    while let Some(handle) = tp.pool.pop() {
        let _ = handle.join();
    }



}
