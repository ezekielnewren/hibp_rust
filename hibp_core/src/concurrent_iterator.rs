use std::collections::VecDeque;
use std::ops::Deref;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::JoinHandle;
use crate::{Job, Transform};
use crate::thread_pool::ThreadPool;


struct ConcurrentIteratorData<In, Out> {
    queue_in: VecDeque<In>,
    queue_out: VecDeque<Out>,
    job_count: u64,
    open: bool,
}

pub struct ConcurrentIterator<In, Out> {
    pool: ThreadPool,
    data: Arc<Mutex<ConcurrentIteratorData<In, Out>>>,
    signal: Arc<Condvar>,
}


impl<In: 'static, Out: 'static> ConcurrentIterator<In, Out> {

    pub fn new<F>(thread_count: usize, t: F) -> Self
        where F: Fn(In) -> Option<Out> + 'static
    {
        let mut it = Self {
            pool: ThreadPool::new(thread_count),
            data: Arc::new(Mutex::new(ConcurrentIteratorData {
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

    pub fn close(&mut self) {
        let mut data = self.data.lock().unwrap();
        data.open = false;
    }

}


impl<In, Out> Iterator for ConcurrentIterator<In, Out> {
    type Item = Out;

    fn next(&mut self) -> Option<Self::Item> {
        let mut result_pop: Option<Self::Item>;
        loop {
            let mut data = self.data.lock().unwrap();
            result_pop = data.queue_out.pop_front();
            match result_pop {
                None => {
                    match !data.open && data.job_count > 0 {
                        true => {
                            let _unused = self.signal.wait(data).unwrap();
                            continue
                        },
                        false => break,
                    }
                }
                Some(_) => break,
            }
        }

        return result_pop;
    }
}


