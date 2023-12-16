// use crate::db::HIBPDB;
// use crate::util::{binary_search, binary_search_get_range, HASH, HashMemoryArray, IndexByCopy};

use std::ops;
use std::ops::{Index, Range};

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

    #[test]
    fn test_bad_add() {
        // This assert would fire and test will fail.
        // Please note, that private functions can be tested too!
        assert_eq!(bad_add(1, 2), 3);
    }
}



