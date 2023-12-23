use std::collections::VecDeque;
use std::ops::Deref;
use std::sync::{Arc, Condvar, LockResult, Mutex, MutexGuard};
use std::thread;
use std::thread::JoinHandle;
use crate::{Job, Transform};
use crate::thread_pool::ThreadPool;


struct ConcurrentTransformData<In, Out> {
    queue_in: VecDeque<In>,
    queue_out: VecDeque<Out>,
    unprocessed: usize,
    open: bool,
}

pub struct ConcurrentTransform<In, Out> {
    pool: ThreadPool,
    data: Arc<Mutex<ConcurrentTransformData<In, Out>>>,
    signal: Arc<Condvar>,
    batch_size: usize,
}

pub trait BatchTransform<In, Out> {
    fn add(&mut self, item: In);

    fn take(&mut self, queue: &mut VecDeque<Out>);

    fn close(&mut self);
}


impl<In: 'static, Out: 'static> ConcurrentTransform<In, Out> {

    pub fn new<F>(thread_count: usize, t: F) -> Self
        where F: Fn(In) -> Option<Out> + 'static
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
                    let element: In;
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

impl<In, Out> BatchTransform<In, Out> for ConcurrentTransform<In, Out> {

    fn add(&mut self, item: In) {
        let mut data = self.data.lock().unwrap();
        if !data.open {
            panic!("closed! cannot add item")
        }

        data.queue_in.push_back(item);
        data.unprocessed += 1;
    }

    fn take(&mut self, queue: &mut VecDeque<Out>) {
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


