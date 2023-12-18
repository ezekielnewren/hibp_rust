// use crate::db::HIBPDB;
// use crate::util::{binary_search, binary_search_get_range, HASH, HashMemoryArray, IndexByCopy};

#![feature(test)]


use std::{ops, thread};
use std::ops::{Index, Range};
use std::time::{Duration, Instant};
use test::bench::iter;

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[allow(dead_code)]
fn bad_add(a: i32, b: i32) -> i32 {
    a - b
}


struct AbsArray<T> {
    arr: Vec<T>,
}

impl<T> ops::Index<u64> for AbsArray<T> {
    type Output = T;

    fn index(&self, index: u64) -> &Self::Output {
        return &self.arr[index as usize];
    }
}

struct MyStruct {
    data: Vec<[u8; 16]>,
}

impl MyStruct {
    fn new(size: usize) -> MyStruct {
        let HASH_NULL = [0u8; 16];
        let v: Vec<[u8; 16]> = vec![HASH_NULL; size];
        // let data = &v[..]; // Take a slice of the whole Vec

        MyStruct {
            data: v
        }
    }
}

use hibp_rust::{HASH, RandomItemGenerator};

extern crate test;
use test::Bencher;


pub fn timeit<T, F>(min_runtime: Duration, mut inner: F) -> u64
    where F: FnMut() -> T,
{
    let mut rate = 0u64;
    let mut loopit = 1;

    loop {
        let start = Instant::now();
        for _i in 0..loopit {
            inner();
        }
        let elapsed = start.elapsed().as_secs_f64();
        rate = (loopit as f64 / elapsed) as u64;

        if elapsed > min_runtime.as_secs_f64() {
            break;
        }
        loopit += (loopit as f64 * 2.0*(min_runtime.as_secs_f64()/elapsed)) as u64;
    }

    return rate;
}



#[test]
fn test_random_item_generator() {
    let mut rng: RandomItemGenerator<HASH> = RandomItemGenerator::new(1000000);

    let mut rate = 0u64;

    let min_runtime = Duration::from_secs_f64(5.0);

    rate = timeit(min_runtime, || {
        rng.next_item();
    });
    assert_eq!(0, 0);

    rate = timeit(min_runtime, || {
        test::black_box(rng.next_item());
    });
    assert_eq!(0, 0);

    rate = timeit(min_runtime, || {
        test::black_box(rng.next());
    });
    assert_eq!(0, 0);
}



#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    fn test_vector() {
        // let x: Vec<u8> = vec![0u8; 1000];
        // let b = &x[0..5];

        let arr = AbsArray{
            arr: vec![0u8; 1000],
        };

        let a: &u8 = &arr[0];
    }

    #[test]
    fn test_add() {
        assert_eq!(add(1, 2), 3);
    }
}



