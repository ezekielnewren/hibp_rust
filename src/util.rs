use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::mem::size_of;
use std::ops::{Index, IndexMut, Range};
use md4::{Digest, Md4};
use rand::Error;
use ring::rand::{SecureRandom, SystemRandom};

pub struct HASH([u8; 16]);
pub const HASH_NULL: HASH = HASH([0u8; 16]);

impl Clone for HASH {
    fn clone(&self) -> Self {
        let mut item = HASH_NULL;
        item.0.copy_from_slice(&self.0);
        item
    }
}

impl Copy for HASH {}

impl PartialEq<Self> for HASH {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl PartialOrd for HASH {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Eq for HASH {

}

impl Ord for HASH {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl From<&[u8]> for HASH {
    fn from(value: &[u8]) -> Self {
        let mut item = HASH_NULL;
        item.0.copy_from_slice(value);
        item
    }
}

impl From<Vec<u8>> for HASH {
    fn from(value: Vec<u8>) -> Self {
        value.as_slice().into()
    }
}

impl Display for HASH {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for byte in &(self.0) {
            write!(f, "{:02x}", byte)?;
        }

        Ok(())
    }
}


pub(crate) struct RandomItemGenerator<T: Copy + for<'a> From<&'a [u8]>> {
    rng: SystemRandom,
    pool: Vec<u8>,
    off: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Copy + for<'a> From<&'a [u8]>> RandomItemGenerator<T> {
    pub fn new(buffer_size: usize) -> Self {
        let size: usize = size_of::<T>() * buffer_size;
        Self {
            rng: SystemRandom::new(),
            pool: vec![0u8; size],
            off: size,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn next_item(&mut self) -> &T {
        if self.off == self.pool.len() {
            self.rng.fill(&mut self.pool.as_mut_slice()).unwrap();
        }

        unsafe {
            let t = self.pool.as_ptr().offset(self.off as isize);
            let ret: &T = unsafe { &*std::mem::transmute::<*const u8, *const T>(t) };
            self.off += size_of::<T>();
            return ret;
        }
    }
}

impl<T: Copy + for<'a> From<&'a [u8]>> Iterator for RandomItemGenerator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_item().clone())
    }
}


pub fn encode_to_utf16le(line: &str) -> Vec<u8> {
    return line.encode_utf16().flat_map(|v| v.to_le_bytes()).collect();
}

pub struct HashAndPassword {
    buff: Vec<u8>,
    off: Vec<u64>,
    order: Vec<u64>,
}

impl Index<u64> for HashAndPassword {
    type Output = [u8];

    fn index(&self, index: u64) -> &Self::Output {
        let range = self.range_of_index(self.order[index as usize]);
        return &self.buff[range];
    }
}

impl IndexMut<u64> for HashAndPassword {
    fn index_mut(&mut self, index: u64) -> &mut Self::Output {
        let range = self.range_of_index(self.order[index as usize]);
        return &mut self.buff[range];
    }
}

impl HashAndPassword {

    pub fn new() -> HashAndPassword {
        HashAndPassword {
            buff: Vec::new(),
            off: Vec::new(),
            order: Vec::new(),
        }
    }

    pub fn add_password(&mut self, line: &str) {
        let raw = line.as_bytes();

        let off_old = self.buff.len();
        let add = size_of::<HASH>() + raw.len();

        self.order.push(self.off.len() as u64);
        self.off.push(self.buff.len() as u64);
        unsafe {
            self.buff.reserve(add);
            self.buff.set_len(off_old + add);
        }
        let off_password = off_old + size_of::<HASH>();
        self.buff[off_password..off_password+raw.len()].copy_from_slice(line.as_bytes());
    }

    fn range_of_index(&self, index: u64) -> Range<usize> {
        let start = self.off[index as usize];
        let mut end = 0u64;
        if ((index+1) as usize) < self.off.len() {
            end = self.off[(index+1) as usize];
        } else {
            end = self.buff.len() as u64;
        }
        return start as usize..end as usize;
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

    pub fn hash_passwords(&mut self) {
        for i in 0..self.len() {
            // get the password
            let e_password = self.index_password(i);

            let password: &str = std::str::from_utf8(e_password).unwrap();
            let raw = encode_to_utf16le(password);

            let mut hasher = Md4::new();
            md4::Digest::update(&mut hasher, raw);
            let hash: HASH = hasher.finalize().to_vec().into();
            // let hash: HASH = HASH::try_from(hasher.finalize().to_vec()).unwrap();

            // update the hash
            let e_hash = self.index_hash_mut(i);
            e_hash.copy_from_slice(&hash.0);
        }
    }

    pub fn sort(&mut self) {
        let mut order: Vec<u64> = self.order.clone();
        order.sort_unstable_by_key(|index| self.index_hash(*index));
        self.order.copy_from_slice(order.as_slice());
    }

    pub fn clear(&mut self) {
        self.buff.clear();
        self.off.clear();
        self.order.clear();
    }

    pub fn hash_and_sort(&mut self) {
        self.hash_passwords();
        self.sort();
    }

    pub fn len(&self) -> u64 {
        return self.off.len() as u64;
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
