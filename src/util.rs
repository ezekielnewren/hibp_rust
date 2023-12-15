use std::collections::VecDeque;
use std::fmt::Display;
use std::fs::File;
use std::io::{Read, Write};
use std::mem::size_of;
use std::ops;
use std::ops::{Index, Range};
use std::os::unix::fs::FileExt;
use std::rc::Rc;
use memmap2::{Mmap, MmapMut, MmapOptions};

pub type HASH = [u8; 16];

pub fn check_bounds<T: PartialOrd + Display>(lo: T, value: T, hi: T) {
    if !(lo <= value && value <= hi) {
        // let msg = format!("{} <= {} <= {} check failed", lo.to_string(), value.to_string(), hi.to_string());
        panic!("{} <= {} <= {} check failed", lo, value, hi);
    }
}

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

pub struct HashFileArray {
    pub arr: FileArray,
}

impl IndexByCopy<HASH> for HashFileArray {
    fn get(&self, index: u64) -> HASH {
        let mut tmp = [0u8; 16];
        self.arr.get(index, &mut tmp).unwrap();
        return tmp;
    }

    fn len(&self) -> u64 {
        return self.arr.len();
    }
}

impl IndexMutByCopy<HASH> for HashFileArray {
    fn set(&mut self, index: u64, value: &HASH) {
        self.arr.set(index, value).unwrap();
    }
}

pub struct HashFileArraySlice<'a> {
    arr: Rc<&'a HashFileArray>,
    lo: u64,
    hi: u64,
}

impl HashFileArraySlice<'_> {
    fn bounds_check(&self, index: u64) {
        if !(self.lo <= index && index <= self.hi) { panic!("index of out bounds") }
    }
}

// impl IndexByCopy<HASH> for HashFileArraySlice<'_> {
//
//     fn get(&self, index: u64) -> HASH {
//         self.bounds_check(index);
//         self.arr.get(self.lo+index)
//     }
//
//     fn set(&mut self, index: u64, value: &HASH) {
//         self.bounds_check(index);
//         self.arr.set(index, value);
//     }
//
//     fn len(&self) -> u64 {
//         self.hi-self.lo+1
//     }
// }

impl HashFileArray {

    pub fn slice(&self, range: Range<u64>) -> HashFileArraySlice {
        HashFileArraySlice{
            arr: Rc::new(self),
            lo: range.start,
            hi: range.end,
        }
    }

}

// impl Constrainable for HashFileArray {
//     fn constrain(self: &mut Self, range: Range<u64>) {
//         self.range = range;
//     }
// }

pub struct HashMemoryArray {
    pub(crate) arr: Vec<u8>,
}

impl IndexByCopy<HASH> for HashMemoryArray {
    fn get(&self, index: u64) -> HASH {
        let start: usize = (index * size_of::<HASH>() as u64) as usize;
        let end: usize = start+size_of::<HASH>();
        let mut junk = [0u8; 16];
        let mut value = junk.as_mut_slice();
        let element = &self.arr.as_slice()[start..end];
        // value = element
        // value.copy_from_slice(&element);
        junk.as_mut_slice().copy_from_slice(&element);
        return junk;
    }

    fn len(&self) -> u64 {
        (self.arr.len() / size_of::<HASH>()) as u64
    }
}

impl IndexMutByCopy<HASH> for HashMemoryArray {
    fn set(&mut self, index: u64, value: &HASH) {
        let start = (index*size_of::<HASH>() as u64) as usize;
        let end = start+size_of::<HASH>();
        let element: &mut [u8] = &mut self.arr.as_mut_slice()[start..end];
        // element = value
        element.copy_from_slice(value);
    }
}

pub struct HashMmapArray {
    mmap: Mmap,
}

impl HashMmapArray {
    pub fn new(fd: &File) -> HashMmapArray {
        let mmap = unsafe { MmapOptions::new().map(fd).unwrap() };

        HashMmapArray {
            mmap,
        }
    }
}

impl IndexByCopy<HASH> for HashMmapArray {
    fn get(&self, index: u64) -> HASH {
        let arr = unsafe { self.mmap[..].align_to::<HASH>().1 };
        return arr[index as usize];
    }


    fn len(&self) -> u64 {
        let arr = unsafe { self.mmap.as_ref().align_to::<HASH>().1 };
        return arr.len() as u64;
    }
}

// impl IndexMutByCopy<HASH> for HashMmapArray {
//     fn set(&mut self, index: u64, value: &HASH) {
//         let mut arr = unsafe { self.mmap[..].align_to::<HASH>().1 };
//         return &arr[index as usize] = value;
//     }
// }

pub trait IndexByCopy<T: Copy> {

    fn get(&self, index: u64) -> T;
    fn len(&self) -> u64;
}

pub trait IndexMutByCopy<T: Copy>: IndexByCopy<T> {
    fn set(&mut self, index: u64, value: &T);
}

pub fn swap<T: Copy>(v: &mut dyn IndexMutByCopy<T>, i: u64, j: u64) {
    if i == j {return;}

    let a = v.get(i).clone();
    let b = v.get(j).clone();
    v.set(i, &b);
    v.set(j, &a);
}

pub fn binary_search<T: Copy + PartialOrd>(v: &dyn IndexByCopy<T>, range: &Range<u64>, key: &T) -> i64 {

    // let mut lo = 0;
    // let mut hi = v.len()-1;
    //
    // if !range.is_empty() {
    //     lo = range.start;
    //     hi = range.end-1;
    // }

    let mut lo = range.start;
    let mut hi = range.end;

    if hi == v.len() {
        hi -= 1;
    }

    loop {
        let mid = (hi+lo)>>1;
        let midval: &T = &v.get(mid);

        if &key < &midval {
            hi = mid-1;
        } else if &key > &midval {
            lo = mid+1;
        } else {
            return mid as i64;
        }

        if lo > hi {break;}
    }

    return -1;
}

pub fn binary_search_generate_cache<T: Copy>(v: &dyn IndexByCopy<T>, range: Range<u64>, cache: &mut Vec<T>, max_depth: u64) -> i64 {

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
