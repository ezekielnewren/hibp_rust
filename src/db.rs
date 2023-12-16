use std::cmp::min;
use std::fs::File;
use std::mem::size_of;
use memmap2::{Mmap, MmapOptions};
use crate::util::{HASH, binary_search_generate_cache, binary_search_get_range};

pub struct FileArray {
    pub pathname: String,
    pub fd: File,
    pub mmap: Mmap,
}

pub struct HIBPDB {
    pub(crate) index: FileArray,
    pub(crate) index_cache: Vec<HASH>,
}


impl HIBPDB {
    pub fn new(v: &String) -> HIBPDB {
        let dbdir = v.clone();
        let mut file_index = dbdir.clone();
        file_index.push_str("/index.bin");
        // let fa = FileArray::new(file_index, size_of::<HASH>() as u64);

        let fd = File::open(file_index).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&fd).unwrap() };

        let fa = FileArray {
            pathname: v.clone(),
            fd,
            mmap,
        };

        let index = unsafe { (&fa.mmap).align_to::<HASH>().1 };

        let log2 = (index.len() as f64).log2().ceil() as u64;
        let depth = min(log2/2, log2);
        let mut index_cache: Vec<HASH> = Vec::new();
        binary_search_generate_cache(&index, 0..index.len() as u64, &mut index_cache, depth);

        Self {
            index: fa,
            index_cache: Vec::new(),
        }
    }

    pub(crate) fn index(self: &Self) -> &[HASH] {
        return unsafe { self.index.mmap.align_to::<HASH>().1 };
    }

    pub(crate) fn find(self: &mut Self, key: HASH) -> Result<usize, usize> {
        let mut range = 0..self.index().len() as u64;
        range = binary_search_get_range(&self.index_cache, &range, &key);
        return self.index().binary_search(&key);
    }
}


