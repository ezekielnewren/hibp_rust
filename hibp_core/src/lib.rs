pub mod db;

use std::alloc::Layout;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::mem::{MaybeUninit, size_of, transmute};
use std::ops::{Index, IndexMut, Range};
use std::slice;
use md4::{Digest, Md4};
use rand::{Error, RngCore, SeedableRng};

// pub struct HASH([u8; 16]);
pub type HASH = [u8; 16];
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
    buff: Vec<u8>,
    off: Vec<usize>,
}

impl Index<usize> for HashAndPassword {
    type Output = [u8];

    fn index(&self, index: usize) -> &Self::Output {
        let range = self.range_of_index(index);
        return &self.buff[range];
    }
}

impl IndexMut<usize> for HashAndPassword {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let range = self.range_of_index(index);
        return &mut self.buff[range];
    }
 }


impl HashAndPassword {

    pub fn new() -> HashAndPassword {
        HashAndPassword {
            buff: Vec::new(),
            off: Vec::new(),
        }
    }

    fn range_of_index(&self, index: usize) -> Range<usize> {
        let start = self.off[index];
        let mut end = 0usize;
        if ((index+1)) < self.off.len() {
            end = self.off[(index+1) as usize];
        } else {
            end = self.buff.len();
        }
        return start..end;
    }

    pub fn index_hash(&self, index: usize) -> &HASH {
        let t = &self[index][0..size_of::<HASH>()];
        assert_eq!(t.len(), size_of::<HASH>());
        unsafe {
            return &*transmute::<*const u8, *const HASH>(t.as_ptr());
        }
    }

    pub fn index_hash_mut(&mut self, index: usize) -> &mut HASH {
        let mut t = &mut self[index][0..size_of::<HASH>()];
        assert_eq!(t.len(), size_of::<HASH>());
        unsafe {
            let ptr = t.as_mut_ptr();
            return &mut *transmute::<*mut u8, *mut HASH>(ptr);
        }
    }

    pub fn index_password(&self, index: usize) -> &[u8] {
        let e = &self[index];
        return &e[size_of::<HASH>()..e.len()];
    }

    pub fn index_password_mut(&mut self, index: usize) -> &mut [u8] {
        let range = self.range_of_index(index);
        let e = &mut self.buff[range];
        let end = e.len();
        return &mut e[size_of::<HASH>()..end];
    }

    pub fn add_password(&mut self, line: &str) {
        let raw = line.as_bytes();

        let off_old = self.buff.len();
        let add = size_of::<HASH>() + raw.len();

        self.off.push(self.buff.len());
        unsafe {
            self.buff.reserve(add);
            self.buff.set_len(off_old + add);
        }
        let off_password = off_old + size_of::<HASH>();
        self.buff[off_password..off_password+raw.len()].copy_from_slice(line.as_bytes());
    }

    pub fn hash_passwords(&mut self) {
        for i in 0..self.len() {
            // get the password
            let e_password = self.index_password(i);

            let password: &str = std::str::from_utf8(e_password).unwrap();
            let raw = encode_to_utf16le(password);

            let mut hasher = Md4::new();
            md4::Digest::update(&mut hasher, raw);
            let mut hash: &mut [u8; 16] = &mut Default::default();
            hash.copy_from_slice(hasher.finalize().as_slice());
            // let hash: HASH = HASH::try_from(hasher.finalize().to_vec()).unwrap();

            // update the hash
            let e_hash = self.index_hash_mut(i);
            e_hash.copy_from_slice(hash);
        }
    }

    pub fn sort(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.off.len()).collect();
        order.sort_unstable_by_key(|index| self.index_hash(*index));
        return order;
    }

    pub fn clear(&mut self) {
        self.buff.clear();
        self.off.clear();
    }

    pub fn hash_and_sort(&mut self) {
        self.hash_passwords();
        self.sort();
    }

    pub fn len(&self) -> usize {
        return self.off.len();
    }

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

fn main() {}