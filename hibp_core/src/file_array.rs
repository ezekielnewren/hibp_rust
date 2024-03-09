use std::{fs, io};
use std::fs::File;
use std::marker::PhantomData;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use memmap2::{Mmap, MmapMut, MmapOptions};
use crate::divmod;
use crate::indexbycopy::IndexByCopy;

pub struct FileArrayMut<'a, T> {
    pub pathname: PathBuf,
    pub fd: File,
    pub mmap: MmapMut,
    pub slice: &'a mut [T],
}

impl<'a, T> FileArrayMut<'a, T> {

    pub fn open(_pathname: &Path, size: usize) -> std::io::Result<Self> {
        let fd = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(_pathname)?;

        if size != 0 {
            fd.set_len((size * size_of::<T>()) as u64)?;
        }

        let mut mmap_mut = unsafe { MmapOptions::new().map_mut(&fd)? };

        let slice = unsafe {
            let ptr = mmap_mut.as_mut_ptr() as *mut T;
            std::slice::from_raw_parts_mut(ptr, mmap_mut.len()/size_of::<T>())
        };

        Ok(Self {
            pathname: PathBuf::from(_pathname),
            fd,
            mmap: mmap_mut,
            slice,
        })
    }

    pub fn sync(&mut self) -> io::Result<()> {
        self.mmap.flush()
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        return self.slice;
    }

    pub fn as_slice(&self) -> &[T] {
        return self.slice;
    }

    pub fn len(&self) -> usize {
        return self.mmap.len()/size_of::<T>();
    }

}

pub struct FileArray<'a, T> {
    pub pathname: PathBuf,
    pub fd: File,
    pub mmap: Mmap,
    pub slice: &'a [T],
}

impl<'a, T> FileArray<'a, T> {

    pub fn open(_pathname: &Path) -> io::Result<Self> {
        let fd = fs::OpenOptions::new()
            .read(true)
            .open(_pathname)?;

        let mmap = unsafe { MmapOptions::new().map(&fd)? };

        let slice = unsafe {
            let ptr = mmap.as_ptr() as *const T;
            std::slice::from_raw_parts(ptr, mmap.len()/size_of::<T>())
        };

        Ok(Self {
            pathname: PathBuf::from(_pathname),
            fd,
            mmap,
            slice,
        })
    }

    pub fn as_slice(&self) -> &[T] {
        return self.slice;
    }

    pub fn len(&self) -> usize {
        return self.mmap.len()/size_of::<T>();
    }

}







