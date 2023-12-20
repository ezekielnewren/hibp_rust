use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;
use hibp_core::*;

use std::time::{Duration, Instant};
use ring::rand::SecureRandom;
use thousands::Separable;


pub fn timeit<F>(min_runtime: Duration, mut inner: F) -> u64
    where F: FnMut(),
{
    let mut rate = 0u64;
    let mut loopit = 1;

    let total = Instant::now();
    loop {
        let start = Instant::now();
        let mut _i = 0u64;
        loop {
            if _i >= loopit { break; }
            inner();
            _i += 1;
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
    job: HashMap<String, BenchmarkJob>,
}

impl Benchmarker {

    fn register<F>(&mut self, name: &str, closure: F)
    where
        F: FnMut() -> Box<dyn FnMut()> + 'static,
    {
        let _name = String::from(name);

        let job = BenchmarkJob {
            name: _name.clone(),
            func: Box::new(closure),
        };

        self.job.insert(_name, job);
        // self.job.push(job.name, job);
    }

    fn run(&mut self, name: &str, min_runtime: Duration) {
        let job: &mut BenchmarkJob = self.job.get_mut(name).unwrap();

        print!("{}: ", job.name);
        std::io::stdout().flush().unwrap();
        let inner = (job.func)();
        let rate = timeit(min_runtime, inner);
        println!("{}", rate.separate_with_commas());
    }

    fn run_all(&mut self, min_runtime: Duration) {
        let mut job_list: Vec<String> = Vec::new();
        for name in self.job.keys() {
            job_list.push(name.clone());
        }

        for name in job_list {
            self.run(name.as_str(), min_runtime);
        }
    }
}

const BUFFER_SIZE: usize = 1000;

fn main() {
    let mut b = Benchmarker{job: HashMap::new()};



    b.register("rng bytes", || {
        let item_size = 16;
        let mut pool: Vec<u8> = vec![0u8; item_size* BUFFER_SIZE];
        let mut off = pool.len();

        let rng = ring::rand::SystemRandom::new();

        return Box::new(move || {
            if off == pool.len() {
                rng.fill(pool.as_mut_slice()).unwrap();
                off = 0;
            }

            off += item_size;
        })
    });

    b.register("rng hash ", || {
        let mut rng: RandomItemGenerator<HASH> = RandomItemGenerator::new(BUFFER_SIZE);

        return Box::new(move || {
            rng.next_item();
        });
    });

    let min_runtime = Duration::from_secs_f64(1.0);

    b.run("rng bytes", min_runtime);
    // b.run_all(min_runtime);
}
