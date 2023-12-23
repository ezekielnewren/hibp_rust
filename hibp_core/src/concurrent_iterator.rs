use std::collections::VecDeque;
use std::ops::Deref;
use std::sync::{Arc, Condvar, LockResult, Mutex, MutexGuard};
use std::thread;
use std::thread::JoinHandle;
use crate::{Job, Transform};
use crate::thread_pool::ThreadPool;


struct ConcurrentTransformData<From, To> {
    queue_in: VecDeque<From>,
    queue_out: VecDeque<To>,
    unprocessed: usize,
    open: bool,
}

pub struct ConcurrentTransform<From, To> {
    pool: ThreadPool,
    data: Arc<Mutex<ConcurrentTransformData<From, To>>>,
    signal: Arc<Condvar>,
    batch_size: usize,
}

pub trait BatchTransform<From, To> {
    fn add(&mut self, item: From);

    fn take(&mut self, queue: &mut VecDeque<To>);

    fn close(&mut self);
}


impl<From: 'static, To: 'static> ConcurrentTransform<From, To> {

    pub fn new<F>(thread_count: usize, t: F) -> Self
        where F: Fn(From) -> Option<To> + 'static
    {
        let mut it = Self {
            pool: ThreadPool::new(thread_count),
            data: Arc::new(Mutex::new(ConcurrentTransformData {
                queue_in: VecDeque::new(),
                queue_out: VecDeque::new(),
                unprocessed: 0,
                open: true,
            })),
            signal: Arc::new(Condvar::new()),
            batch_size: thread_count,
        };

        let at = Arc::new(t);

        for i in 0..thread_count {
            let _data = it.data.clone();
            let signal = it.signal.clone();
            let transform = at.clone();
            it.pool.submit(move || {
                loop {
                    let element: From;
                    {
                        let mut data = _data.lock().unwrap();

                        element = match data.queue_in.pop_front() {
                            None => {
                                let _unused = signal.wait(data).unwrap();
                                continue;
                            }
                            Some(v) => v
                        }
                    }
                    match transform(element) {
                        None => {},
                        Some(v) => {
                            let mut data = _data.lock().unwrap();
                            data.queue_out.push_back(v);
                            data.unprocessed -= 1;
                        }
                    }
                }
            });
        }

        it
    }
}

impl<From, To> BatchTransform<From, To> for ConcurrentTransform<From, To> {

    fn add(&mut self, item: From) {
        let mut data = self.data.lock().unwrap();
        if !data.open {
            panic!("closed! cannot add item")
        }

        data.queue_in.push_back(item);
        data.unprocessed += 1;
    }

    fn take(&mut self, queue: &mut VecDeque<To>) {
        loop {
            let mut data = self.data.lock().unwrap();
            if data.queue_out.len() > 0 {
                queue.extend(data.queue_out.drain(..));
                break;
            } else {
                let threshold: usize = if data.open { self.batch_size } else { 0 };
                if data.unprocessed > threshold {
                    let _ = self.signal.wait(data);
                    continue;
                }
                break;
            }
        }
    }

    fn close(&mut self) {
        let mut data = self.data.lock().unwrap();
        data.open = false;
    }
}


