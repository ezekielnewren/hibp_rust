// #![feature(unboxed_closures)]

pub mod db;
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

pub fn HASH_to_hex(v: &HASH) -> String {
    hex::encode_upper(v)
}

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

pub trait InterpolationSearch<T> {
    fn interpolation_search(&self, key: &T) -> Result<usize, usize>;
}

impl InterpolationSearch<HASH> for [HASH] {
    fn interpolation_search(&self, key: &HASH) -> Result<usize, usize> {
        #[cfg(debug_assertions)]
        let _key__hash = HASH_to_hex(key);
        #[cfg(debug_assertions)]
        let _view_hash = String::from("");

        let slope: u128 = u128::MAX/self.len() as u128;
        let key_as_u128 = u128::from_be_bytes(*key);

        let guess: usize = (key_as_u128/slope) as usize;
        let mut step = 1usize;

        let mut lo = 0usize;
        let mut hi = self.len()-1;
        let _len = self.len();

        #[cfg(debug_assertions)]
        let _view_hash = HASH_to_hex(&self[guess]);

        let mut i = guess;
        if key < &self[i] {
            while key < &self[i] && i < _len {
                #[cfg(debug_assertions)]
                let _view_hash = HASH_to_hex(&self[i]);
                hi = i;
                i = match i.checked_sub(step) {
                    None => break,
                    Some(v) => v,
                };
                step <<= 1;
            }
            #[cfg(debug_assertions)]
            let _view_hash = HASH_to_hex(&self[i]);
            lo = i;
        } else {
            while key > &self[i] {
                #[cfg(debug_assertions)]
                let _view_hash = HASH_to_hex(&self[i]);
                lo = i;
                i += step;
                step <<= 1;
            }
            #[cfg(debug_assertions)]
            let _view_hash = HASH_to_hex(&self[i]);
            hi = i;
        }

        if i < _len {
            match self[lo..hi+1].binary_search(key) {
                Ok(v) => Ok(lo+v),
                Err(v) => Err(lo+v),
            }
        } else if i == _len {
            Err(i)
        } else {
            Err(0)
        }
    }
}

