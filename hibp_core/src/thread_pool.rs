use std::{thread};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

struct Job {
    lambda: Box<dyn FnOnce()>,
}

unsafe impl Send for Job {}

impl Job {
    pub fn call_lambda(self) {
        (self.lambda)();
    }
}

struct ThreadPoolData {
    queue: VecDeque<Job>,
    open: bool,
}

pub struct ThreadPool {
    pool: VecDeque<JoinHandle<()>>,
    signal: Arc<Condvar>,
    data: Arc<Mutex<ThreadPoolData>>,
}

impl ThreadPool {

    pub fn new(size: usize) -> Self {
        let mut tp = ThreadPool{
            pool: VecDeque::new(),
            signal: Arc::new(Condvar::new()),
            data: Arc::new(Mutex::new(ThreadPoolData{
                queue: VecDeque::new(),
                open: true,
            })),
        };

        for _ in 0..size {
            let _data = tp.data.clone();
            let signal = tp.signal.clone();
            let t = thread::spawn(move || {
                loop {
                    let mut data = _data.lock().unwrap();
                    if !data.open { break; }
                    let job = match data.queue.pop_front() {
                        None =>  {
                            let _x = signal.wait(data).unwrap();
                            continue
                        },
                        Some(xxx) => xxx,
                    };
                    job.call_lambda();
                }
            });

            tp.pool.push_back(t);
        }

        tp
    }

    pub fn submit<F>(&mut self, job: F) where F: FnOnce() + 'static {
        let mut data = self.data.lock().unwrap();
        data.queue.push_front(Job{
            lambda: Box::new(job),
        });

        self.signal.notify_all();
    }

    pub fn close(&mut self) {
        {
            let mut data = self.data.lock().unwrap();
            data.open = false;
            self.signal.notify_all();
        }
        while let Some(handle) = self.pool.pop_front() {
            handle.join().unwrap();
        }
    }
}


