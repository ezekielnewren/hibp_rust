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
    let buff_elements = 1000000;
    let mut rng_hash: RandomItemGenerator<HASH> = RandomItemGenerator::new(buff_elements);
    let mut rng_array: RandomItemGenerator<[u8; 16]> = RandomItemGenerator::new(buff_elements);

    let mut rate = 0u64;

    let min_runtime = Duration::from_secs_f64(1.0);

    rate = timeit(min_runtime, || {
        rng_array.next_item();
    });
    assert_eq!(0, 0);

    rate = timeit(min_runtime, || {
        test::black_box(rng_hash.next_item());
    });
    assert_eq!(0, 0);

    // rate = timeit(min_runtime, || {
    //     test::black_box(rng_hash.next());
    // });
    // assert_eq!(0, 0);
}



#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(1, 2), 3);
    }
}



