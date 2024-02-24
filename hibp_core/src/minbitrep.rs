use std::cmp::min;
use crate::divmod;

pub struct MinBitRep<'a> {
    pub a: &'a mut[u8],
    pub bit_len: usize,
}


impl<'a> MinBitRep<'a> {

    pub fn calculate_array_size(len: usize, bit_len: u8) -> usize {
        (len*bit_len as usize+7)/8
    }

    pub fn wrap(a: &'a mut[u8], bit_len: u8) -> Self {
        Self {
            a,
            bit_len: bit_len as usize,
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






