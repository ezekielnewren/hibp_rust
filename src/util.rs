use std::collections::VecDeque;
use std::fmt::Display;
use std::mem::size_of;
use std::ops::{Index, IndexMut, Range};

pub type HASH = [u8; 16];
pub const HASH_NULL: HASH = [0u8; 16];

pub fn encode_to_utf16le(line: &str) -> Vec<u8> {
    let utf16: Vec<u16> = line.encode_utf16().collect();
    let bytes: Vec<u8> = utf16.iter().flat_map(|&v| v.to_le_bytes()).collect();
    return bytes;
}

pub struct HashAndPassword {
    buff: Vec<u8>,
    off: Vec<usize>,
}

// pub fn index_hash_and_password(inst: &mut HashAndPassword, index: u64) {
//     let e = inst[index];
//     let e_hash = &mut inst[index][0..size_of::<HASH>()];
//
//     let e = &mut self[index];
//     let end = e.len();
//     return &mut e[size_of::<HASH>()..end];
// }

impl HashAndPassword {

    pub fn new() -> HashAndPassword {
        HashAndPassword {
            buff: Vec::new(),
            off: Vec::new(),
        }
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

    fn range_of_index(&self, index: u64) -> Range<usize> {
        let start: usize = self.off[index as usize];
        let mut end: usize = 0;
        if ((index+1) as usize) < self.off.len() {
            end = self.off[(index+1) as usize];
        } else {
            end = self.buff.len();
        }
        return start..end;
    }

    pub fn index_hash_mut(&mut self, index: u64) -> &mut [u8] {
        return &mut self[index][0..size_of::<HASH>()];
    }

    pub fn index_password_mut(&mut self, index: u64) -> &mut [u8] {
        let e = &mut self[index];
        let end = e.len();
        return &mut e[size_of::<HASH>()..end];
    }

    pub fn index_hash(&self, index: u64) -> &[u8] {
        return &self[index][0..size_of::<HASH>()];
    }

    pub fn index_password(&self, index: u64) -> &[u8] {
        let e = &self[index];
        return &e[size_of::<HASH>()..e.len()];
    }

    pub fn len(&self) -> u64 {
        return self.off.len() as u64;
    }

}

impl Index<u64> for HashAndPassword {
    type Output = [u8];

    fn index(&self, index: u64) -> &Self::Output {
        return &self.buff[self.range_of_index(index)];
    }
}

impl IndexMut<u64> for HashAndPassword {
    fn index_mut(&mut self, index: u64) -> &mut Self::Output {
        let range = self.range_of_index(index);
        return &mut self.buff[range];
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
