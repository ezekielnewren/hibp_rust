use std::ffi::{CStr, CString};
use std::io;
use std::io::{Error, ErrorKind};
use std::os::fd::RawFd;
use std::path::{Path};
use crate::{BitSet, divmod};

pub fn get_errno_message() -> String {
    unsafe {
        let errno = *libc::__errno_location();
        let ptr = libc::strerror(errno);
        return String::from(CStr::from_ptr(ptr).to_str().unwrap());
    }
}

#[macro_export]
macro_rules! errno_to_error {
    ($err:expr) => {
        if $err < 0 {
            return Err(Error::new(ErrorKind::Other, get_errno_message()));
        }
    };
}

macro_rules! check_bounds {
    ($value:expr, $range:expr) => {
        if !($range.start <= $value && $value < $range.end) {
            panic!("{} is out of bounds {},{}", $value, $range.start, $range.end);
        }
    }
}

pub struct Segment {
    ptr: *mut u8,
    len: usize,
}

impl Segment {

    pub fn new(len: usize) -> io::Result<Self> {
        if UserFileCache::PAGESIZE%len != 0 {
            panic!("len must be a multiple of PAGESIZE");
        }

        let mut mem_ptr: *mut libc::c_void = std::ptr::null_mut();
        unsafe {
            let ret = libc::posix_memalign(&mut mem_ptr, UserFileCache::PAGESIZE as libc::size_t, 1);
            errno_to_error!(ret);
        }

        Ok(Self{
            ptr: mem_ptr as *mut u8,
            len,
        })
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    // pub fn as_slice(&self) -> &[u8] {
    //     unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    // }
}

impl Drop for Segment {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.ptr as *mut libc::c_void);
        }
    }
}


pub struct UserFileCache {
    fd: RawFd,
    pages: usize,
    segment_len: usize,
    dirty: BitSet,
    inactive: Vec<Segment>,
    active: Vec<Option<Segment>>,
}

impl UserFileCache {
    pub const PAGESIZE: usize = libc::_SC_PAGESIZE as usize;


    pub fn open(pathname: &Path, number_of_pages: usize) -> io::Result<Self> {
        let path = CString::new(pathname.as_os_str().to_str().unwrap()).unwrap();
        let fd: RawFd = unsafe {
            libc::open(path.as_ptr(),
                       libc::O_DIRECT | libc::O_RDWR | libc::O_CREAT,
                       libc::S_IRUSR
            )
        };
        if fd < 0 {
            return Err(io::Error::new(ErrorKind::Other, get_errno_message()))
        }

        unsafe {
            let mut stat64: libc::stat64 = std::mem::zeroed();
            let r = libc::fstat64(fd, &mut stat64);
            if r < 0 {
                libc::close(fd);
                errno_to_error!(r);
            }
            let fsize = stat64.st_size as usize;
            let t = number_of_pages*Self::PAGESIZE;
            if fsize < t {
                let r = libc::ftruncate64(fd, t as libc::off64_t);
                if r < 0 {
                    libc::close(fd);
                    errno_to_error!(r);
                }
            }
        }

        let it = Self {
            fd,
            pages: number_of_pages*Self::PAGESIZE,
            segment_len: 1<<21,
            dirty: BitSet::new(),
            inactive: Vec::new(),
            active: Vec::new()
        };

        Ok(it)
    }

    fn _read(&self, buff: &mut [u8], off: usize) -> io::Result<()> {
        unsafe {
            let r = libc::pread64(self.fd, buff.as_mut_ptr() as *mut libc::c_void, buff.len(), off as libc::off64_t);
            errno_to_error!(r);
            if r < buff.len() as libc::ssize_t {
                return Err(io::Error::new(ErrorKind::UnexpectedEof, format!{"{}", r as isize}))
            }
            Ok(())
        }
    }


    fn _segfault(&mut self, segment_id: usize) {
        if self.inactive.is_empty() {
            self.inactive.push(Segment::new(self.segment_len).unwrap());
        }

        let off = segment_id*self.segment_len;
        let len = std::cmp::min(self.segment_len, self.file_size()-off);
        let mut seg = self.inactive.pop().unwrap();

        let slice = seg.as_mut_slice();
        self._read(&mut slice[0..len], off).unwrap();
        slice[len..].fill(0);
    }

    fn _at(&mut self, page_id: usize) -> &mut [u8] {
        let (q, r) = divmod!(page_id, self.segment_len);
        if self.active[q].is_none() {
            self._segfault(q);
        }

        let seg = self.active[q].as_mut().unwrap();
        let slice = &mut seg.as_mut_slice()[r*Self::PAGESIZE..(r+1)*Self::PAGESIZE];
        return slice;
    }

    pub fn at(&mut self, page_id: usize) -> &[u8] {
        check_bounds!(page_id, 0..self.pages);
        self._at(page_id)
    }

    pub fn at_mut(&mut self, page_id: usize) -> &mut [u8] {
        check_bounds!(page_id, 0..self.pages);
        self.dirty.set(page_id as u64);
        self._at(page_id)
    }


    pub fn len(&self) -> usize {
        self.pages
    }

    pub fn file_size(&self) -> usize {
        self.pages*Self::PAGESIZE
    }

}


