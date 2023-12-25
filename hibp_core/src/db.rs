use std::cmp::min;
use std::ffi::c_void;
use std::fs::File;
use std::mem::size_of;
use memmap2::{Mmap, MmapOptions};
use crate::{binary_search_generate_cache, binary_search_get_range, HASH};

pub struct FileArray {
    pub pathname: String,
    pub fd: File,
    pub mmap: Mmap,
    mlock_result: i32,
}

pub struct HIBPDB<'a> {
    pub index: FileArray,
    pub index_slice: &'a [HASH],
    pub index_cache: Vec<HASH>,
}


impl<'a> HIBPDB<'a> {
    pub fn new(v: String, prefer_locking: bool) -> Self {
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

        // let index = unsafe { (&fa.mmap).align_to::<HASH>().1 };
        let index_slice: &'a [HASH] = unsafe {
            let ptr = fa.mmap.as_ptr() as *const HASH;
            std::slice::from_raw_parts(ptr, fa.mmap.len()/size_of::<HASH>())
        };

        if prefer_locking {
            unsafe {
                let ptr = fa.mmap.as_ptr() as *const c_void;
                let flags = libc::MLOCK_ONFAULT;
                libc::mlock2(ptr, fa.mmap.len(), flags);
                fa.mlock_result = *libc::__errno_location();
            }
        }

        let log2 = (index_slice.len() as f64).log2().ceil() as u64;
        let depth = min(log2/2, log2);
        let mut index_cache: Vec<HASH> = Vec::new();
        binary_search_generate_cache(&index_slice, 0..index_slice.len() as u64, &mut index_cache, depth);

        Self {
            index: fa,
            index_slice,
            index_cache,
        }
    }

    pub fn index(self: &Self) -> &[HASH] {
        return self.index_slice;
    }

    pub fn find(self: &mut Self, key: &HASH) -> Result<usize, usize> {
        return if self.index.mlock_result == 0 {
            self.index().binary_search(&key)
        } else {
            let mut range = 0..self.index().len() as u64;
            range = binary_search_get_range(&self.index_cache, &range, &key);
            self.index().binary_search(&key)
        }
    }
}


impl Drop for HIBPDB<'_> {
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

