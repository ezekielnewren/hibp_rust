// #![feature(test)]


use std::{fs, thread};
use std::ops::{Index, IndexMut, Range};
use std::str::Utf8Error;
use std::sync::Arc;
use std::thread::{available_parallelism, JoinHandle};
use std::time::Duration;
use reqwest::Error;
use hibp_core::{HASH, hash_password, HashAndPassword};
// use hibp_core::batch_transform::ConcurrentIterator;
// use hibp_core::thread_pool::ThreadPool;

// extern crate test;

const DIR_SRC_DATA: &str = "src/data";
const DIR_TESTS_DATA: &str = "tests/data";


use std::io::Read;
use reqwest::header::HeaderName;


#[test]
fn test_test_data_directory() {
    let cd = fs::canonicalize(".");

    let result = fs::metadata(DIR_TESTS_DATA);
    assert!(result.is_ok());

    assert!(result.unwrap().is_dir());
}


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

mod tests {
    use std::env;
    use hibp_core::db::HIBPDB;
    use hibp_core::{HASH_to_hex, InterpolationSearch};

    fn db_directory() -> String {
        env::var("DB_DIRECTORY").unwrap()
    }

    #[test]
    fn test_interpolation_search() {
        let mut db = HIBPDB::new(db_directory());

        let mut view = String::from("");

        let percent: usize = (0.23 * (db.len() as f64)) as usize;
        let t = db.index()[percent];
        view = HASH_to_hex(&t);

        match db.index().interpolation_search(&t) {
            Ok(v) => assert_eq!(percent, v),
            Err(_) => assert!(false),
        }

        let percent: usize = (0.90 * (db.len() as f64)) as usize;
        let t = db.index()[percent];
        view = HASH_to_hex(&t);

        match db.index().interpolation_search(&t) {
            Ok(v) => assert_eq!(percent, v),
            Err(_) => assert!(false),
        }
    }


}



