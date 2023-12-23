use std::collections::VecDeque;
use std::sync::{Arc, Condvar, LockResult, Mutex, MutexGuard};
use std::thread;
use std::thread::JoinHandle;
use crate::{Job};

struct MutexAlwaysNotifyAll<T> {
    lock: Mutex<T>,
    signal: Condvar,
}

impl<E> MutexAlwaysNotifyAll<E> {

    pub fn new(resource: E) -> Self {
        Self {
            lock: Mutex::<E>::new(resource),
            signal: Condvar::new(),
        }
    }

    pub fn lock(&self) -> LockResult<MutexGuard<'_, E>> {
        let lr = self.lock.lock();
        self.signal.notify_all();
        return lr;
    }

    pub fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> LockResult<MutexGuard<'a, T>> {
        self.signal.wait(guard)
    }

}

pub trait BatchTransform<From, To> {
    fn add(&mut self, item: From);

    fn take(&mut self, queue: &mut VecDeque<To>);

    fn close(&mut self);
}


struct ConcurrentBatchTransformData<From, To> {
    queue_in: VecDeque<From>,
    queue_out: VecDeque<To>,
    unprocessed: usize,
    open: bool,
}

pub struct ConcurrentBatchTransform<From, To> {
    pool: VecDeque<JoinHandle<()>>,
    data: Arc<MutexAlwaysNotifyAll<ConcurrentBatchTransformData<From, To>>>,
    batch_size: usize,
}

impl<From: 'static, To: 'static> ConcurrentBatchTransform<From, To> {

    pub fn new<F>(thread_count: usize, t: F) -> Self
        where F: Fn(From) -> Option<To> + 'static
    {
        let mut it = Self {
            pool: VecDeque::new(),
            data: Arc::new(MutexAlwaysNotifyAll::new(ConcurrentBatchTransformData {
                queue_in: VecDeque::new(),
                queue_out: VecDeque::new(),
                unprocessed: 0,
                open: true,
            })),
            batch_size: thread_count,
        };

        let at = Arc::new(t);

        for i in 0..thread_count {
            let _data = it.data.clone();
            let transform = at.clone();

            let job = Job::new(move || {
                loop {
                    let element: From;
                    {
                        // 1. get the next element to process
                        let mut data = _data.lock().unwrap();
                        if !data.open { break; }

                        element = match data.queue_in.pop_front() {
                            None => {
                                let _unused = _data.wait(data);
                                continue;
                            }
                            Some(v) => v
                        }
                    }
                    // 2. process that element outside the mutex
                    let result = transform(element);
                    {
                        // 3. add the result to the processed queue
                        let mut data = _data.lock().unwrap();
                        if result.is_some() {
                            data.queue_out.push_back(result.unwrap());
                        }
                        data.unprocessed -= 1;
                    }
                }
            });
            let handle = thread::spawn(move || {
                job.invoke();
            });
            it.pool.push_back(handle);
        }

        it
    }
}

impl<From, To> Drop for ConcurrentBatchTransform<From, To> {
    fn drop(&mut self) {
        {
            let mut data = self.data.lock().unwrap();
            data.open = false;
        }
        for handle in self.pool.drain(..) {
            handle.join().unwrap();
        }
    }
}

impl<From, To> BatchTransform<From, To> for ConcurrentBatchTransform<From, To> {

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
            let threshold: usize = if data.open { self.batch_size } else { 0 };
            if data.unprocessed > threshold {
                let _ = self.data.wait(data);
                continue;
            }

            queue.extend(data.queue_out.drain(..));
            break;
        }
    }

    fn close(&mut self) {
        let mut data = self.data.lock().unwrap();
        data.open = false;
    }
}


