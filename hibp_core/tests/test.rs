#![feature(test)]


use std::{fs, thread};
use std::ops::{Index, IndexMut, Range};
use std::sync::Arc;
use std::thread::available_parallelism;
use std::time::Duration;
use hibp_core::{HashAndPassword};
use hibp_core::concurrent_iterator::ConcurrentIterator;
use hibp_core::thread_pool::ThreadPool;

extern crate test;

const DIR_SRC_DATA: &str = "src/data";
const DIR_TESTS_DATA: &str = "tests/data";


#[test]
fn test_test_data_directory() {
    let cd = fs::canonicalize(".");

    let result = fs::metadata(DIR_TESTS_DATA);
    assert!(result.is_ok());

    assert!(result.unwrap().is_dir());
}


#[test]
fn test_unordered_queue() {

    // let mut pool = ThreadPool::new(12);
    // pool.submit(move || {
    //     thread::sleep(Duration::from_secs(10));
    // });
    //
    // pool.close();

    let thread_count = num_cpus::get();
    let mut it: ConcurrentIterator<Vec<u8>, HashAndPassword> = ConcurrentIterator::new(thread_count, thread_count*2, Arc::new(|input| {

        HashAndPassword {
            hash: Default::default(),
            password: input,
        }
    }));



}
