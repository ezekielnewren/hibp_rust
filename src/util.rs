use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::mem::size_of;
use std::ops::Range;
use std::os::unix::fs::FileExt;

use crate::error::ValueError;
pub(crate) struct FileArray {
    pathname: String,
    fd: File,
    element_size: u64,
}

impl FileArray {

    pub fn new(pathname: String, element_size: u64) -> FileArray {
        let fd = File::options()
            .read(true)
            .write(true)
            .open(pathname.as_str()).unwrap();
        FileArray{
            pathname,
            fd,
            element_size,
        }
    }

    pub fn get(self: &mut Self, index: u64, value: &mut [u8]) -> std::io::Result<()> {
        if !(0 <= index && index < self.len()) {
            panic!("index out of bounds")
        }

        if value.len() as u64 != self.element_size {
            panic!("value isn't the same size as the element size")
        }

        self.fd.read_exact_at(value, index*self.element_size)
    }

    pub fn set(self: &mut Self, index: u64, value: &[u8]) -> std::io::Result<()> {
        if !(0 <= index && index < self.len()) {
            panic!("index out of bounds")
        }

        if value.len() as u64 != self.element_size {
            panic!("value isn't the same size as the element size")
        }

        self.fd.write_all_at(value, index*self.element_size)
    }

    pub fn len(self: &mut Self) -> u64 {
        return self.fd.metadata().unwrap().len()/self.element_size;
    }

}

trait CompactArray: IndexByCopy<u64> {

}

struct FileCompactArray {
    arr: FileArray,
    range: Range<u64>,
}

impl IndexByCopy<u64> for FileCompactArray {
    fn get(&mut self, index: u64) -> u64 {
        let mut buff = [0u8; size_of::<u64>()];
        self.arr.get(index, &mut buff);

        return 0u64;
    }

    fn set(&mut self, index: u64, value: &u64) {

    }

    fn len(&mut self) -> u64 {
        todo!()
    }
}

pub trait IndexByCopy<T: Clone> {

    fn get(&mut self, index: u64) -> T;

    fn set(&mut self, index: u64, value: &T);

    fn len(&mut self) -> u64;
}

pub fn swap<T: Clone>(v: &mut dyn IndexByCopy<T>, i: u64, j: u64) {
    if i == j {return;}

    let a = v.get(i).clone();
    let b = v.get(j).clone();
    v.set(i, &b);
    v.set(j, &a);
}

pub fn binary_search<T: Clone, F>(v: &mut dyn IndexByCopy<T>, range: Range<u64>, key: &T, mut is_less: F) -> i64
    where
        F: FnMut(&T, &T) -> bool,
{

    let mut lo = range.start;
    let mut hi = range.end-1;

    loop {
        let mid = (hi+lo)>>1;
        let midval = v.get(mid);

        if is_less(&key, &midval) {
            hi = mid-1;
        } else if is_less(&midval, &key) {
            lo = mid+1;
        } else {
            return mid as i64;
        }

        if lo > hi {break;}
    }

    return -1;
}

pub fn bubble_sort<T: Clone, F>(v: &mut dyn IndexByCopy<T>, mut is_less: F)
    where
        F: FnMut(&T, &T) -> bool,
{

    let mut change = 0;

    loop {
        change = 0;

        for i in 0..v.len()-1 {
            let a = v.get(i+1);
            let b = v.get(i);
            if is_less(&a, &b) {
                swap(v, i+1, i);
                change += 1;
            }
        }

        if change == 0 {break;}
    }
}

