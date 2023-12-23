use std::{thread};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use crate::Job;

struct ThreadPoolData {
    queue: VecDeque<Job>,
    open: bool,
}

pub struct ThreadPool {
    pool: VecDeque<JoinHandle<()>>,
    data: Arc<Mutex<ThreadPoolData>>,
    signal: Arc<Condvar>,
}

impl ThreadPool {

    pub fn new(size: usize) -> Self {
        let mut tp = ThreadPool{
            pool: VecDeque::new(),
            data: Arc::new(Mutex::new(ThreadPoolData{
                queue: VecDeque::new(),
                open: true,
            })),
            signal: Arc::new(Condvar::new()),
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
        data.queue.push_front(Job::new(job));
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


