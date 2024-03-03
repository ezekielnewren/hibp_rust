use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fmt::Display;
use std::io;
use std::io::{Error, ErrorKind};
use std::os::fd::RawFd;
use std::path::{Path, PathBuf};
use std::time::Instant;
use crate::BitSet;


pub fn get_errno_message() -> String {
    unsafe {
        let errno = *libc::__errno_location();
        let ptr = libc::strerror(errno);
        return String::from(CStr::from_ptr(ptr).to_str().unwrap());
    }
}

pub struct Page {
    ptr: *mut libc::c_void,
    timestamp: Instant,
    dirty: bool,
}

impl Page {

    pub fn new() -> io::Result<Self> {
        let mut mem_ptr: *mut libc::c_void = std::ptr::null_mut();
        unsafe {
            let ret = libc::posix_memalign(&mut mem_ptr, Page::size() as libc::size_t, 1);
            if ret != 0 {
                return Err(io::Error::new(ErrorKind::Other, get_errno_message()));
            }
        }

        Ok(Self{
            ptr: mem_ptr,
            timestamp: Instant::now(),
            dirty: false,
        })
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            self.timestamp = Instant::now();
            self.dirty = true;
            let ptr = self.ptr as *mut u8;
            std::slice::from_raw_parts_mut(ptr, Page::size() as usize)
        }
    }

    pub fn as_slice(&mut self) -> &[u8] {
        unsafe {
            self.timestamp = Instant::now();
            let ptr = self.ptr as *const u8;
            std::slice::from_raw_parts(ptr, Page::size() as usize)
        }
    }

    pub fn zero(&mut self) {
        self.as_mut_slice().fill(0u8);
    }

    pub fn size() -> u64 {
        libc::_SC_PAGESIZE as u64
    }

}

impl Drop for Page {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.ptr);
        }
    }
}

struct DirectIO {
    pathname: PathBuf,
    fd: RawFd,
    inactive: Vec<Page>,
    active: HashMap<usize, Page>,
}

impl Drop for DirectIO {
    fn drop(&mut self) {
        unsafe {
            let mut err_list = Vec::<io::Result<()>>::new();

            let sync_error = self.sync();
            if sync_error.is_err() {
                err_list.push(sync_error);
            }

            let r = libc::close(self.fd);
            if r < 0 {
                err_list.push(Err(Error::new(ErrorKind::Other, get_errno_message())));
            }

            if !err_list.is_empty() {
                let dump: Vec<String> = err_list.into_iter().map(|e| return format!("{}", e.err().unwrap())).collect();
                let msg = dump.join("\n");
                panic!("{}", msg);
            }
        }
    }
}

impl DirectIO {

    pub fn open(pathname: &Path) -> io::Result<Self> {
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

        let mut it = Self{
            pathname: PathBuf::from(pathname),
            fd,
            inactive: Vec::new(),
            active: HashMap::new(),
        };

        it.inactive.push(Page::new()?);

        Ok(it)
    }

    fn _read_page(&self, page_id: usize, page: &mut Page) -> io::Result<()> {
        unsafe {
            let page_size = Page::size() as libc::size_t;
            let off = page_id as libc::off64_t * page_size as libc::off64_t;
            let r = libc::pread64(self.fd, page.ptr, page_size, off);
            if r < 0 {
                return Err(io::Error::new(ErrorKind::Other, get_errno_message()));
            }
            if r < Page::size() as libc::ssize_t {
                return Err(io::Error::new(ErrorKind::UnexpectedEof, format!{"{}", r as isize}))
            }
            page.timestamp = Instant::now();
            Ok(())
        }
    }

    fn _write_page(&self, page_id: usize, page: &mut Page) -> io::Result<()> {
        unsafe {
            let page_size = Page::size() as libc::size_t;
            let off = page_id as libc::off64_t * page_size as libc::off64_t;
            let w = libc::pwrite64(self.fd, page.ptr, page_size, off);
            if w < 0 {
                return Err(io::Error::new(ErrorKind::Other, get_errno_message()));
            }
            if w < Page::size() as libc::ssize_t {
                return Err(io::Error::new(ErrorKind::UnexpectedEof, format!{"{}", w as isize}))
            }
            page.timestamp = Instant::now();
            page.dirty = false;
            Ok(())
        }
    }

    pub fn resize(&mut self, new_size: u64) -> io::Result<()> {
        unsafe {
            let r = libc::ftruncate64(self.fd, (Page::size()*new_size) as libc::off64_t);
            if r < 0 {
                return Err(Error::new(ErrorKind::Other, get_errno_message()));
            }
        }

        Ok(())
    }

    fn _page_fault(&mut self, page_id: usize) -> io::Result<()> {
        // allocate more memory if possible
        if self.inactive.is_empty() {
            self.inactive.push(Page::new()?);
        }
        // if we can't allocate more memory then reclaim clean pages
        let keys: Vec<usize> = self.active.keys().map(|k: &usize| return *k).collect();
        if self.inactive.is_empty() {
            for k in keys.iter() {
                if !self.active[&k].dirty {
                    let t = self.active.remove(&k).unwrap();
                    self.inactive.push(t);
                    break;
                }
            }
        }
        if self.inactive.is_empty() {
            let mut victim = keys[0];
            let mut oldest = self.active[&victim].timestamp;

            for k in &keys[1..] {
                if self.active[&k].timestamp < oldest {
                    oldest = self.active[&k].timestamp;
                    victim = *k;
                }
            }

            let mut p: Page = self.active.remove(&victim).unwrap();
            self._write_page(victim, &mut p)?;
            self.inactive.push(p);
        }

        let mut p = self.inactive.remove(self.inactive.len()-1);
        self._read_page(page_id, &mut p)?;
        self.active.insert(page_id, p);

        Ok(())
    }

    pub fn at_mut(&mut self, page_id: usize) -> io::Result<&mut [u8]> {
        if !self.active.contains_key(&page_id) {
            self._page_fault(page_id)?;
        }


        let p = self.active.get_mut(&page_id).unwrap();
        Ok(p.as_mut_slice())
    }

    pub fn at(&mut self, page_id: usize) -> io::Result<&[u8]> {
        if !self.active.contains_key(&page_id) {
            self._page_fault(page_id)?;
        }

        let p = self.active.get_mut(&page_id).unwrap();
        Ok(p.as_slice())
    }

    pub fn sync(&mut self) -> io::Result<()> {
        let keys: Vec<usize> = self.active.keys().map(|k| return *k).collect();

        for page_id in keys {
            let mut page = self.active.remove(&page_id).unwrap();
            if page.dirty {
                self._write_page(page_id, &mut page)?;
            }
            self.active.insert(page_id, page);
        }

        unsafe {
            let r = libc::fsync(self.fd);
            if r < 0 {
                return Err(Error::new(ErrorKind::Other, get_errno_message()));
            }
        }

        Ok(())
    }
}


