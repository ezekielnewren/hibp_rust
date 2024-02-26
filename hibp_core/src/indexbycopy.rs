use std::cmp::Ordering;

pub trait IndexByCopy<T: Copy> {
    fn get(&mut self, index: usize) -> T;
    fn len(&mut self) -> usize;
}

pub trait IndexByCopyMut<T: Copy> {
    fn set(&mut self, index: usize, value: T);
}

pub fn binary_search_by<T: Ord + Copy, F>(arr: &mut dyn IndexByCopy<T>, mut f: F) -> Result<usize, usize>
    where
        F: FnMut(T) -> Ordering,
{
    let mut lo = 0;
    let mut hi = arr.len()-1;

    while lo <= hi {
        let mid = (lo+hi)>>1;
        let cmp = f(arr.get(mid));

        if cmp.is_eq() {
            return Ok(mid);
        } else if cmp.is_lt() {
            lo = mid+1;
        } else {
            hi = mid-1;
        }
    }

    Err(lo)
}

pub fn binary_search<T: Copy + Ord>(arr: &mut dyn IndexByCopy<T>, key: T) -> Result<usize, usize> {
    return binary_search_by(arr, |p| p.cmp(&key));
}

