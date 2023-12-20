use std::io::Write;
use std::rc::Rc;
use hibp_core::*;

use std::time::{Duration, Instant};
use thousands::Separable;


pub fn timeit<F>(min_runtime: Duration, mut inner: F) -> u64
    where F: FnMut(),
{
    let mut rate = 0u64;
    let mut loopit = 1;

    let total = Instant::now();
    loop {
        let start = Instant::now();
        for _i in 0..loopit {
            inner();
        }
        let elapsed = start.elapsed().as_secs_f64();
        rate = (loopit as f64 / elapsed) as u64;

        if elapsed > min_runtime.as_secs_f64() {
            break;
        } else if total.elapsed().as_secs_f64() > 2.0*min_runtime.as_secs_f64() {
            break;
        }
        loopit = (1.2*min_runtime.as_secs_f64()/elapsed) as u64;
    }

    return rate;
}

struct BenchmarkJob {
    name: String,
    func: Box<dyn FnMut() -> Box<dyn FnMut()>>,
}
struct Benchmarker {
    job: Vec<BenchmarkJob>,
}

impl Benchmarker {

    fn register<F>(&mut self, name: &str, closure: F)
    where
        F: FnMut() -> Box<dyn FnMut()> + 'static,
    {
        let job = BenchmarkJob {
            name: String::from(name),
            func: Box::new(closure),
        };

        self.job.push(job);
    }

    fn run_all(&mut self, min_runtime: Duration) {
        for i in 0..self.job.len() {
            let job = &mut self.job[i];

            print!("{}: ", job.name);
            std::io::stdout().flush().unwrap();
            let inner = (job.func)();
            let rate = timeit(min_runtime, inner);
            println!("{}", rate.separate_with_commas());
        }
    }
}


fn main() {
    let mut b = Benchmarker{job: Vec::new()};

    b.register("rng hash", || {
        let mut rng: RandomItemGenerator<HASH> = RandomItemGenerator::new(1000000);

        return Box::new(move || {
            rng.next_item();
        });
    });

    let min_runtime = Duration::from_secs_f64(3.0);
    b.run_all(min_runtime);
}
