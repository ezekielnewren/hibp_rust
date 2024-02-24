use std::cmp::{max, min};
use crate::divmod;

pub struct MinBitRep<'a> {
    pub a: &'a mut[u8],
    pub bit_len: usize,
}


impl<'a> MinBitRep<'a> {
    pub fn minbit(x: u64) -> u64 {
        max(64-u64::leading_zeros(x) as u64, 1)
    }

    pub fn calculate_array_size(len: usize, max_value: u64) -> usize {
        let bit_len = Self::minbit(max_value) as usize;
        (len*bit_len+7)/8
    }

    pub fn wrap(a: &'a mut[u8], max_value: u64) -> Self {
        let bit_len = Self::minbit(max_value) as usize;
        Self {
            a,
            bit_len,
        }
    }


    pub fn get(&self, index: usize) -> u64 {
        if index >= self.len() {
            panic!("index out of bounds");
        }

        let mut out = 0u64;
        let mut shift = 0usize;
        while shift < self.bit_len {
            let (q, r) = divmod!(index*self.bit_len+shift, 8);
            let read = min(8-r, self.bit_len-shift);
            let mask = match read as u8 {
                8u8 => 0xff,
                _ => (1<<read)-1,
            };
            out |= (((self.a[q]>>r)&mask) as u64) << shift;
            shift += read;
        }

        return out;
    }

    pub fn set(&mut self, index: usize, mut value: u64) {
        if index >= self.len() {
            panic!("index out of bounds");
        }

        if self.bit_len < 64 && value >= (1<<self.bit_len) {
            panic!("value too large");
        }

        let mut shift = 0usize;
        while shift < self.bit_len {
            let (q, r) = divmod!(index*self.bit_len+shift, 8);
            let write = min(8-r, self.bit_len-shift);
            let mask = match write as u8 {
                8u8 => 0xff,
                _ => (1<<write)-1,
            };
            self.a[q] &= !(mask<<r);
            self.a[q] |= (value<<r) as u8;
            value >>= write;
            shift += write;
        }
    }

    pub fn len(&self) -> usize {
        self.a.len()*8/self.bit_len
    }
}






