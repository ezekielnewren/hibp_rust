

pub trait IndexByCopy<T: Copy> {
    fn get(&mut self, index: usize) -> T;
    fn len(&mut self) -> usize;
}

pub trait IndexByCopyMut<T: Copy> {
    fn set(&mut self, index: usize, value: T);
}

