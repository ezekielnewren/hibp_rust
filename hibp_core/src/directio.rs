use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fmt::Display;
use std::io;
use std::io::{Error, ErrorKind};
use std::os::fd::RawFd;
use std::path::{Path, PathBuf};
use std::time::Instant;
use sysinfo::{MemoryRefreshKind, RefreshKind, System};


pub fn get_errno_message() -> String {
    unsafe {
        let errno = *libc::__errno_location();
        let ptr = libc::strerror(errno);
        return String::from(CStr::from_ptr(ptr).to_str().unwrap());
    }
}

#[derive(PartialEq, PartialOrd)]
enum MemoryPressure {
    Minimal,
    Moderate,
    Extreme,
}

impl MemoryPressure {
    fn current() -> MemoryPressure {
        let rk = RefreshKind::new()
            .with_memory(MemoryRefreshKind::new().without_swap());
        let mut sys = System::new_with_specifics(rk);
        sys.refresh_all();
        let percent = (sys.used_memory()*100) as f64 / sys.total_memory() as f64;

        return if percent >= 95.0 {
            Self::Extreme
        } else if percent >= 80.0 {
            Self::Moderate
        } else {
            Self::Minimal
        }
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
        self.dirty = true;
        self.timestamp = Instant::now();
        unsafe { std::slice::from_raw_parts_mut(self.ptr as *mut u8, Page::size() as usize) }
    }

    pub fn as_slice(&mut self) -> &[u8] {
        self.timestamp = Instant::now();
        unsafe { std::slice::from_raw_parts(self.ptr as *const u8, Page::size() as usize) }
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

    fn _get_page(&mut self) -> io::Result<Page> {
        use MemoryPressure::*;

        if self.inactive.is_empty() {
            self._reclaim_pages(MemoryPressure::current())?;
            let pressure = MemoryPressure::current();
            if pressure > Minimal {
                self.free_memory(pressure)?;
            }
        }

        return Ok(self.inactive.pop().unwrap());
    }

    pub fn free_memory(&mut self, pressure: MemoryPressure) -> io::Result<()> {
        use MemoryPressure::*;

        self._reclaim_pages(pressure)?;
        if self.inactive.len() > 1 {
            self.inactive.truncate(1);
        }

        Ok(())
    }

    fn _reclaim_pages(&mut self, mut pressure: MemoryPressure) -> io::Result<()> {
        use MemoryPressure::*;

        if !self.inactive.is_empty() && pressure == Minimal {
            return Ok(())
        }
        let mut oldest = Instant::now();
        let mut victim = usize::MAX;

        for page_id in self.active.keys() {
            if !self.active[page_id].dirty {
                victim = *page_id;
                break;
            } else {
                let t = self.active[page_id].timestamp;
                if t < oldest {
                    oldest = t;
                    victim = *page_id;
                }
            }
        }

        let mut page: Page;
        if victim == usize::MAX {
            page = Page::new().unwrap();
        } else {
            page = self.active.remove(&victim).unwrap();
            if page.dirty {
                self._write_page(victim, &mut page)?;
            }
        };

        self.inactive.push(page);

        if pressure >= Moderate {
            let mut clean = Vec::new();
            let mut dirty = Vec::new();

            for page_id in self.active.keys() {
                if !self.active[page_id].dirty {
                    clean.push(*page_id);
                } else {
                    dirty.push(*page_id);
                }
            }

            for page_id in clean {
                let page = self.active.remove(&page_id).unwrap();
                self.inactive.push(page);
            }

            if pressure == Extreme {
                for page_id in dirty {
                    let mut page = self.active.remove(&page_id).unwrap();
                    self._write_page(page_id, &mut page)?;
                    self.inactive.push(page);
                }
            }
        }

        Ok(())
    }

    fn _page_fault(&mut self, page_id: usize) -> io::Result<()> {
        let mut page = self._get_page()?;
        self._read_page(page_id, &mut page)?;
        self.active.insert(page_id, page);

        Ok(())
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


