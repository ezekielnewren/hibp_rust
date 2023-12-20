// use crate::db::HIBPDB;
// use crate::util::{binary_search, binary_search_get_range, HASH, HashMemoryArray, IndexByCopy};

#![feature(test)]


use std::{ops, thread};
use std::ops::{Index, IndexMut, Range};
use std::time::{Duration, Instant};
use test::bench::iter;

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[allow(dead_code)]
fn bad_add(a: i32, b: i32) -> i32 {
    a - b
}


// use hibp_rust::{HASH, RandomItemGenerator};

extern crate test;
use test::Bencher;
use hibp_core::{HASH, RandomItemGenerator};




#[test]
fn test_random_item_generator() {
    let buff_elements = 1000000;
    let mut rng_hash: RandomItemGenerator<HASH> = RandomItemGenerator::new(buff_elements);
    let mut rng_array: RandomItemGenerator<[u8; 16]> = RandomItemGenerator::new(buff_elements);

    let mut rate = 0u64;

    let min_runtime = Duration::from_secs_f64(1.0);

    // without explicit copy
    rate = timeit(min_runtime, || {
        test::black_box(rng_array.next_item());
    });
    assert_eq!(0, 0);

    // with explicit copy
    rate = timeit(min_runtime, || {
        test::black_box(rng_hash.next_item().clone());
    });
    assert_eq!(0, 0);

    // rate = timeit(min_runtime, || {
    //     test::black_box(rng_hash.next());
    // });
    // assert_eq!(0, 0);
}

#[test]
fn test_random_item_generator_borrow() {
    let buff_elements = 1;
    let mut rng: RandomItemGenerator<HASH> = RandomItemGenerator::new(buff_elements);

    let a = rng.next_item().clone();
    let b = rng.next_item().clone();

    assert_ne!(a, b);
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



