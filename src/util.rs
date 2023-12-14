use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Range;
use std::os::unix::fs::FileExt;

pub(crate) struct FileArray {
    pub pathname: String,
    pub fsize: u64,
    pub fd: File,
    pub element_size: u64,
}

impl FileArray {

    pub fn new(pathname: String, element_size: u64) -> FileArray {
        let fd = File::options()
            .read(true)
            .write(true)
            .open(pathname.as_str()).unwrap();
        FileArray{
            pathname,
            fsize: &fd.metadata().unwrap().len()/element_size,
            fd,
            element_size,
        }
    }

    pub fn get(self: &Self, index: u64, value: &mut [u8]) -> std::io::Result<()> {
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

    pub fn len(self: &Self) -> u64 {
        return self.fsize;
    }

}

pub trait IndexByCopy<T: Clone> {

    fn get(&self, index: u64) -> T;

    fn set(&mut self, index: u64, value: &T);

    fn len(&self) -> u64;
}

pub fn swap<T: Clone>(v: &mut dyn IndexByCopy<T>, i: u64, j: u64) {
    if i == j {return;}

    let a = v.get(i).clone();
    let b = v.get(j).clone();
    v.set(i, &b);
    v.set(j, &a);
}

pub fn cmp_default<T: PartialOrd>() -> fn(&T, &T) -> bool {
    return |lhs: &T, rhs: &T| lhs < rhs;
}

pub fn binary_search<T: Clone, F>(v: &dyn IndexByCopy<T>, range: Range<u64>, mut is_less: F, key: &T) -> i64
    where
        F: FnMut(&T, &T) -> bool,
{

    let mut lo = 0;
    let mut hi = v.len()-1;

    if !range.is_empty() {
        lo = range.start;
        hi = range.end-1;
    }

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

pub fn binary_search_generate_cache<T: Clone>(v: &dyn IndexByCopy<T>, range: Range<u64>, cache: &mut Vec<T>, max_depth: u64) -> i64
{

    let mut depth = 0;
    let mut queue: VecDeque<(u64, u64)> = VecDeque::new();

    if !range.is_empty() {
        queue.push_back((range.start, range.end-1))
    } else {
        queue.push_back((0, v.len()));
    }


    loop {
        let mut size = queue.len();
        for _i in 0..size {
            let (lo, hi) = queue.pop_front().unwrap();

            let mid = (hi+lo)>>1;
            let mid_val = v.get(mid);

            cache.push(mid_val);

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

pub fn binary_search_get_range<T: Clone, F>(cache: &Vec<T>, range: Range<u64>, mut is_less: F, key: &T) -> Range<u64>
    where
        F: FnMut(&T, &T) -> bool,
{

    let mut lo = range.start;
    let mut hi = range.end-1;
    let mut mvi = 0;

    loop {
        let mid = (hi+lo)>>1;
        let midval: &T = &cache[mvi];

        if is_less(&key, &midval) {
            mvi = ((mvi+1)<<1)+0;
            hi = mid-1;
        } else if is_less(&midval, &key) {
            mvi = ((mvi+1)<<1)+0;
            lo = mid+1;
        }

        if lo > hi || mvi >= cache.len() {break;}
    }

    return lo..hi;
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

