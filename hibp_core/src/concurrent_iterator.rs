use std::collections::VecDeque;
use std::ops::Deref;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::JoinHandle;
use crate::{Job, Transform};
use crate::thread_pool::ThreadPool;


struct ConcurrentTransformData<In, Out> {
    queue_in: VecDeque<In>,
    queue_out: VecDeque<Out>,
    job_count: u64,
    open: bool,
}

pub struct ConcurrentTransform<In, Out> {
    pool: ThreadPool,
    data: Arc<Mutex<ConcurrentTransformData<In, Out>>>,
    signal: Arc<Condvar>,
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
                job_count: 0,
                open: true,
            })),
            signal: Arc::new(Condvar::new()),
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
                            data.job_count -= 1;
                        }
                    }
                }
            });
        }

        it
    }

    pub fn add(&mut self, item: In) {
        let mut data = self.data.lock().unwrap();
        if !data.open {
            panic!("closed! cannot add item")
        }

        data.queue_in.push_back(item);
        data.job_count += 1;
    }

    pub fn take(&mut self, queue: &mut VecDeque<Out>) {
        loop {
            let mut data = self.data.lock().unwrap();
            match data.queue_out.len() {
                0 => {
                    match !data.open && data.job_count > 0 {
                        true => {
                            // wait for all jobs to finish
                            let _unused = self.signal.wait(data).unwrap();
                            continue;
                        }
                        false => return,
                    }
                },
                _ => queue.extend(data.queue_out.drain(..)),
            }
            break;
        }
    }

    pub fn close(&mut self) {
        let mut data = self.data.lock().unwrap();
        data.open = false;
    }
}


