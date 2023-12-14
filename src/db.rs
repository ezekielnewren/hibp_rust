use std::cmp::min;
use std::mem::size_of;
use crate::util::{binary_search, binary_search_generate_cache, binary_search_get_range, FileArray, HASH, HashFileArray, IndexByCopy};

pub struct HIBPDB {
    pub(crate) index: HashFileArray,
    pub(crate) index_cache: Vec<HASH>,
}

impl HIBPDB {
    pub fn new(v: &String) -> Self {
        let dbdir = v.clone();
        let mut file_index = dbdir.clone();
        file_index.push_str("/index.bin");
        let fa = FileArray::new(file_index, size_of::<HASH>() as u64);

        let log2 = (fa.len() as f64).log2().ceil() as u64;
        let depth = min(log2/2, log2);

        let mut hfa = HashFileArray {arr: fa};
        let mut cache: Vec<HASH> = Vec::new();
        binary_search_generate_cache(&hfa, 0..hfa.len(), &mut cache, depth);

        Self {
            index: hfa,
            index_cache: cache,
        }
    }

    pub(crate) fn find(self: &mut Self, key: HASH) -> i64 {
        let mut range = 0..self.index.len();
        range = binary_search_get_range(&self.index_cache, &(0..self.index.len()), &key);
        return binary_search(&mut self.index, &range, &key);
    }
}
