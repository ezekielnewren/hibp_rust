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
    open: bool,
}

pub struct ConcurrentIterator<In, Out> {
    pool: ThreadPool,
    data: Arc<Mutex<ConcurrentIteratorData<In, Out>>>,
    signal: Arc<Condvar>,
}


impl<In: 'static, Out: 'static> ConcurrentIterator<In, Out> {

    pub fn new<F>(thread_count: usize, t: F) -> Self
        where F: Fn(In) -> Out + 'static
    {
        let mut it = Self {
            pool: ThreadPool::new(thread_count),
            data: Arc::new(Mutex::new(ConcurrentIteratorData {
                queue_in: VecDeque::new(),
                queue_out: VecDeque::new(),
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
                    let mut data = _data.lock().unwrap();

                    let result_pop = data.queue_in.pop_front();
                    if result_pop.is_none() {
                        let _unused = signal.wait(data).unwrap();
                        continue;
                    }

                    data.queue_out.push_back(transform(result_pop.unwrap()));
                }
            });
        }

        it
    }

    pub fn push(&mut self, item: In) {
        let data = self.data.lock().unwrap();

        data.queue_in.push_back(item);
    }

}


impl<In, Out> Iterator for ConcurrentIterator<In, Out> {
    type Item = Out;

    fn next(&mut self) -> Option<Self::Item> {
        let mut result_pop;
        loop {
            let mut data = self.data.lock().unwrap();

            result_pop = data.queue_out.pop_front();
            if result_pop.is_none() && !data.open {
                let _unused = self.signal.wait(data).unwrap();
                continue;
            }
            break;
        }

        return result_pop;
    }
}


