use std::collections::VecDeque;
use std::fmt::Display;
use std::ops::{Index, Range};

pub type HASH = [u8; 16];
pub const HASH_NULL: HASH = [0u8; 16];

pub fn binary_search_generate_cache<T: Copy>(v: &[T], range: Range<u64>, cache: &mut Vec<T>, max_depth: u64) -> i64 {

    let mut depth = 0;
    let mut queue: VecDeque<(u64, u64)> = VecDeque::new();

    if !range.is_empty() {
        queue.push_back((range.start, range.end-1))
    } else {
        queue.push_back((0, v.len() as u64));
    }

    loop {
        let mut size = queue.len();
        for _i in 0..size {
            let (lo, hi) = queue.pop_front().unwrap();

            let mid = (hi+lo)>>1;
            let mid_val = v.get(mid as usize).unwrap();

            cache.push(*mid_val);

            if depth < max_depth {
                queue.push_back((lo, mid-1));
                queue.push_back((mid+1, hi));
            }
        }

        depth += 1;
        if depth >= max_depth {break;}
    }

    return -1;
}

pub fn binary_search_get_range<T: Copy + PartialOrd>(cache: &Vec<T>, range: &Range<u64>, key: &T) -> Range<u64> {
    // can't do any bounds checking because we're working with the cache not the original array
    let mut lo = range.start;
    let mut hi = range.end;

    let mut mvi = 0;

    loop {
        let mid = (hi+lo)>>1;
        let midval: &T = &cache[mvi];

        if &key < &midval {
            mvi = ((mvi+1)<<1)+0;
            hi = mid-1;
        } else if &key > &midval {
            mvi = ((mvi+1)<<1)+0;
            lo = mid+1;
        }

        if lo > hi || mvi >= cache.len() {break;}
    }

    return lo..hi;
}
