use std::fs::File;
use std::mem::size_of;
use memmap2::{Mmap, MmapOptions};
use crate::{HASH, InterpolationSearch};

pub struct FileArray {
    pub pathname: String,
    pub fd: File,
    pub mmap: Mmap,
}

pub struct HIBPDB<'a> {
    pub index: FileArray,
    pub index_slice: &'a [HASH],
}

impl<'a> HIBPDB<'a> {
    pub fn new(v: String) -> Self {
        let dbdir = v.clone();
        let mut file_index = dbdir.clone();
        file_index.push_str("/index.bin");

        let fd = File::open(file_index).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&fd).unwrap() };

        let mut fa = FileArray {
            pathname: v.clone(),
            fd,
            mmap,
        };

        let index_slice: &'a [HASH] = unsafe {
            let ptr = fa.mmap.as_ptr() as *const HASH;
            std::slice::from_raw_parts(ptr, fa.mmap.len()/size_of::<HASH>())
        };

        Self {
            index: fa,
            index_slice,
        }
    }

    #[inline]
    pub fn index(self: &Self) -> &[HASH] {
        return self.index_slice;
    }

    pub fn find(self: &mut Self, key: &HASH) -> Result<usize, usize> {
        self.index().interpolation_search(key)
    }

    pub fn len(&self) -> usize {
        self.index().len()
    }
}
