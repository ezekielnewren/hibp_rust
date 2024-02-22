use std::collections::VecDeque;

pub trait Transform<From, To> {

    fn add(&mut self, item: From);

    fn take(&mut self) -> To;

}

pub struct TransformSerial<From, To> {
    queue: VecDeque<From>,
    transform: Box<dyn FnMut(From) -> To>,
}

impl<From, To> TransformSerial<From, To> {
    pub fn new<F>(t: F) -> Self
        where F: FnMut(From) -> To + 'static
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
