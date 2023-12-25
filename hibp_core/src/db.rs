use std::cmp::min;
use std::ffi::c_void;
use std::fs::File;
use std::mem::size_of;
use std::ops::Deref;
use memmap2::{Mmap, MmapOptions};
use crate::{binary_search_generate_cache, binary_search_get_range, HASH};
// use crate::lib::{HASH, binary_search_generate_cache, binary_search_get_range};

pub struct FileArray {
    pub pathname: String,
    pub fd: File,
    pub mmap: Mmap,
    mlock_result: i32,
}

pub struct HIBPDB {
    pub index: FileArray,
    pub index_cache: Vec<HASH>,
}


impl HIBPDB {
    pub fn new(v: &String, prefer_locking: bool) -> HIBPDB {
        let dbdir = v.clone();
        let mut file_index = dbdir.clone();
        file_index.push_str("/index.bin");
        // let fa = FileArray::new(file_index, size_of::<HASH>() as u64);

        let fd = File::open(file_index).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&fd).unwrap() };

        let mut fa = FileArray {
            pathname: v.clone(),
            fd,
            mmap,
            mlock_result: 0i32,
        };

        let index = unsafe { (&fa.mmap).align_to::<HASH>().1 };

        if prefer_locking {
            unsafe {
                let ptr = fa.mmap.as_ptr() as *const c_void;
                let flags = libc::MLOCK_ONFAULT;
                libc::mlock2(ptr, fa.mmap.len(), flags);
                fa.mlock_result = *libc::__errno_location();
            }
        }

        let log2 = (index.len() as f64).log2().ceil() as u64;
        let depth = min(log2/2, log2);
        let mut index_cache: Vec<HASH> = Vec::new();
        binary_search_generate_cache(&index, 0..index.len() as u64, &mut index_cache, depth);

        Self {
            index: fa,
            index_cache,
        }
    }

    pub fn index(self: &Self) -> &[HASH] {
        return unsafe { self.index.mmap.align_to::<HASH>().1 };
    }

    pub fn find(self: &mut Self, key: &HASH) -> Result<usize, usize> {
        let mut range = 0..self.index().len() as u64;
        range = binary_search_get_range(&self.index_cache, &range, &key);
        return self.index().binary_search(&key);
    }
}


impl Drop for HIBPDB {
    fn drop(&mut self) {
        if self.index.mlock_result == 0 {
            unsafe {
                let ptr = self.index.mmap.as_ptr() as *const c_void;
                let ret = libc::munlock(ptr, self.index.mmap.len());
                if ret != 0 {
                    panic!("munlock returned with {}", ret)
                }
            }
        }
    }
}

