#![feature(unboxed_closures)]

pub mod db;
pub mod thread_pool;
pub mod batch_transform;

use std::alloc::Layout;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::mem::{MaybeUninit, size_of, transmute};
use std::ops::{Index, IndexMut, Range};
use std::{slice, thread};
use std::panic::UnwindSafe;
use std::str::Utf8Error;

use std::thread::JoinHandle;
use md4::{Digest, Md4};
use rand::{Error, RngCore, SeedableRng};

// pub struct HASH([u8; 16]);
pub type HASH = [u8; 16];

pub struct Job {
    pub closure: Box<dyn FnOnce()>,
}

unsafe impl Send for Job {}
impl UnwindSafe for Job {}

impl Job {

    pub fn new<F>(job: F) -> Job where F: FnOnce() + 'static {
        Self {
            closure: Box::new(job),
        }
    }

    pub fn invoke(self) {
        (self.closure)();
    }
}

pub struct Transform<From, To> {
    lambda: Box<dyn Fn(From) -> To>,
}

unsafe impl<From, To> Send for Transform<From, To> {}

impl<From, To> Transform<From, To> {

    pub fn new<F>(transform: F) -> Transform<From, To> where F: Fn(From) -> To + 'static {
        Self {
            lambda: Box::new(transform),
        }
    }

    pub fn call_lambda(self, item: From) -> To {
        (self.lambda)(item)
    }
}

pub struct UnsafeMemory {
    pub ptr: *mut u8,
    pub len: usize,
}

impl UnsafeMemory {
    pub unsafe fn new(len: usize) -> Result::<Self, String> {
        let ptr: *mut u8 = std::alloc::alloc(Layout::array::<u8>(len).unwrap());
        if ptr.is_null() {
            Err(String::from("allocation failed"))?;
        }
        Ok(Self {ptr, len})
    }

    pub unsafe fn cast<T>(&self) -> *const T {
        return transmute::<*mut u8, *const T>(self.ptr);
    }

    pub unsafe fn cast_mut<T>(&self) -> *mut T {
        return transmute::<*mut u8, *mut T>(self.ptr);
    }

    pub unsafe fn at<T>(&self, index: isize) -> &T {
        return &*self.cast::<T>().offset(index);
    }

    pub unsafe fn at_mut<T>(&self, index: isize) -> &mut T {
        return &mut *self.cast_mut::<T>().offset(index);
    }

    pub unsafe fn as_slice<T>(&self) -> &[T] {
        return slice::from_raw_parts(self.cast::<T>(), self.len/size_of::<T>());
    }

    pub unsafe fn as_slice_mut<T>(&self) -> &mut [T] {
        return slice::from_raw_parts_mut(self.cast_mut::<T>(), self.len/size_of::<T>());
    }

}

impl Drop for UnsafeMemory {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(self.ptr, Layout::array::<u8>(self.len).unwrap());
        }
    }
}

pub struct RandomItemGenerator<'a, T: Default + Copy> {
    rng: rand::rngs::StdRng,
    pool: Vec<T>,
    memory: &'a mut [u8],
    threshold: usize,
    off: usize,
}

impl<'a, T: Default + Copy> RandomItemGenerator<'a, T> {
    pub fn new(buffer_size: usize) -> Self {
        let mut pool: Vec<T> = vec![Default::default(); buffer_size];
        let slice = pool.as_mut_slice();
        let ptr: *mut u8 = slice.as_mut_ptr() as *mut u8;
        let len: usize = size_of::<T>()*slice.len();

        let memory: &mut [u8] = unsafe { slice::from_raw_parts_mut(ptr, len) };

        Self {
            rng: rand::rngs::StdRng::from_entropy(),
            pool,
            memory,
            threshold: buffer_size,
            off: buffer_size,
        }
    }

    #[inline]
    pub fn next_item(&mut self) -> &T {
        assert_eq!(self.pool.as_ptr() as *const u8, self.memory.as_ptr());
        assert_eq!(self.pool.len() * size_of::<T>(), self.memory.len());
        assert!(self.off <= self.pool.len());

        if self.off == self.threshold {
            self.rng.fill_bytes(self.memory);
            self.off = 0;
        }

        let t = self.off;
        self.off += 1;
        return &self.pool[t];
    }
}

pub fn encode_to_utf16le(line: &str) -> Vec<u8> {
    return line.encode_utf16().flat_map(|v| v.to_le_bytes()).collect();
}

pub struct HashAndPassword {
    pub hash: HASH,
    pub password: Vec<u8>,
}

pub fn hash_password(v: &mut HashAndPassword) -> Result<(), Utf8Error> {
    let password: &str = std::str::from_utf8(v.password.as_slice())?;
    let raw = encode_to_utf16le(password);

    let mut hasher = Md4::new();
    hasher.update(raw);
    v.hash = hasher.finalize().into();

    Ok(())
}

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


