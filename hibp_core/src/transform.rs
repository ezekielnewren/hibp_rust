use std::cmp::Ordering;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::collections::BinaryHeap;

pub trait Transform<From, To> {

    fn add(&mut self, item: From);

    fn take(&mut self) -> To;

}

pub struct MinHeapItem<T> {
    pub priority: u64,
    pub item: T,
}

impl<T> PartialEq<Self> for MinHeapItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl<T> Eq for MinHeapItem<T> {}

impl<T> PartialOrd for MinHeapItem<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // reverse comparison
        other.priority.partial_cmp(&self.priority)
    }
}

impl<T> Ord for MinHeapItem<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority)
    }
}

pub struct TransformConcurrentData<From, To> {
    pub in_queue: VecDeque<From>,
    pub out_queue: BinaryHeap<MinHeapItem<To>>,
    pub wp: u64,
    pub rp: u64,
}

pub struct TransformConcurrent<From, To> {
    pub pool: VecDeque<JoinHandle<()>>,
    pub mutex: Arc<Mutex<TransformConcurrentData<From, To>>>,
    pub in_cond: Arc<Condvar>,
    pub out_cond: Arc<Condvar>,
}

impl<From, To> Transform<From, To> for TransformConcurrent<From, To> {
    fn add(&mut self, item: From) {
        let mut data = self.mutex.lock().unwrap();
        data.in_queue.push_back(item);
        self.in_cond.notify_one();
    }

    fn take(&mut self) -> To {
        let mut data = self.mutex.lock().unwrap();
        if data.rp >= data.wp {
            panic!("transform item under flow");
        }

        while data.out_queue.is_empty() || data.out_queue.peek().unwrap().priority != data.rp {
            data = self.out_cond.wait(data).unwrap();
        }

        data.rp += 1;
        return data.out_queue.pop().unwrap().item;
    }
}


impl<From: 'static, To: 'static> TransformConcurrent<From, To> {
    pub fn new<F>(t: F, thread_count: u32) -> Self
        where F: Fn(From) -> To + 'static
    {
        let at = Arc::new(t);

        let tc = match thread_count {
            0 => num_cpus::get_physical() as u32,
            _ => thread_count,
        };

        let mut inst = Self {
            pool: VecDeque::new(),
            mutex: Arc::new(Mutex::new(TransformConcurrentData {
                in_queue: Default::default(),
                out_queue: Default::default(),
                wp: 0,
                rp: 0,
            })),
            in_cond: Arc::new(Condvar::new()),
            out_cond: Arc::new(Condvar::new()),
        };

        for _ in 0..tc {
            let mutex = inst.mutex.clone();
            let in_cond = inst.in_cond.clone();
            let out_cond = inst.out_cond.clone();
            let transform = at.clone();
            let w = Worker::new(move || {
                loop {
                    let item: From;
                    {
                        let mut data = mutex.lock().unwrap();

                        while data.in_queue.is_empty() {
                            data = in_cond.wait(data).unwrap();
                        }

                        item = data.in_queue.pop_front().unwrap();
                    }
                    let out = transform(item);
                    {
                        let mut data = mutex.lock().unwrap();
                        let priority = data.wp;
                        data.out_queue.push(MinHeapItem{
                            priority,
                            item: out,
                        });
                        data.wp += 1;
                        out_cond.notify_one();
                    }
                }
            });

            inst.pool.push_back(thread::spawn(move || w.invoke()));
        }

        return inst;
    }
}






pub struct Worker {
    pub closure: Box<dyn FnOnce()>,
}

unsafe impl Send for Worker {}

impl Worker {

    pub fn new<F>(closure: F) -> Self where F: FnOnce() + 'static {
        Self {
            closure: Box::new(closure),
        }
    }
    pub fn invoke(self) {
        (self.closure)();
    }
}


pub struct TransformSerial<From, To> {
    queue: VecDeque<From>,
    transform: Box<dyn Fn(From) -> To>,
}

impl<From, To> TransformSerial<From, To> {
    pub fn new<F>(t: F) -> Self
        where F: Fn(From) -> To + 'static
    {
        Self {
            queue: VecDeque::new(),
            transform: Box::new(t),
        }
    }
}

impl<From, To> Transform<From, To> for TransformSerial<From, To> {
    fn add(&mut self, item: From) {
        self.queue.push_back(item);
    }

    fn take(&mut self) -> To {
        let item = self.queue.pop_front().unwrap();
        return (self.transform)(item);
    }
}
