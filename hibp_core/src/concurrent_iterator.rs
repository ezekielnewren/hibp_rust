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
}

pub struct ConcurrentIterator<In, Out> {
    pool: ThreadPool,
    data: Arc<Mutex<ConcurrentIteratorData<In, Out>>>,
    signal: Arc<Condvar>,
}


impl<In: 'static, Out: 'static> ConcurrentIterator<In, Out> {

    pub fn new(thread_count: usize, output_limit: usize, t: Arc<dyn Fn(In) -> Out>) -> Self {
        let mut it = Self {
            pool: ThreadPool::new(thread_count),
            data: Arc::new(Mutex::new(ConcurrentIteratorData {
                queue_in: VecDeque::new(),
                queue_out: VecDeque::new(),
            })),
            signal: Arc::new(Condvar::new()),
        };

        for i in 0..thread_count {
            let _data = it.data.clone();
            let signal = it.signal.clone();
            let transform = t.clone();
            it.pool.submit(move || {
                loop {
                    let mut data = _data.lock().unwrap();

                    let result_pop = data.queue_in.pop_front();
                    if result_pop.is_none() || output_limit > 0 && data.queue_out.len() >= output_limit {
                        let _unused = signal.wait(data).unwrap();
                        continue;
                    }

                    data.queue_out.push_back(transform(result_pop.unwrap()));
                }
            });
        }

        it
    }

}



