use std::collections::HashMap;
use std::env;
use std::io::Write;
use hibp_core::*;

use std::time::{Duration, Instant};
use md4::{Digest, Md4};
use rand::{Error, random, Rng, RngCore, SeedableRng};
use thousands::Separable;
// use rand::Fill;


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
        loopit = (min_runtime.as_secs_f64()/elapsed) as u64;
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
    let args: Vec<_> = env::args().collect();
    let min_runtime = Duration::from_secs_f64(3.0);
    let argv = &args[1..];
    let mut b = Benchmarker{job: HashMap::new()};

    b.register("StdRng", || {
        let mut rng = rand::rngs::StdRng::from_entropy();

        return Box::new(move || {
            rng.gen::<HASH>();
        });
    });

    b.register("rng bytes", || {
        let item_size = 16;
        let mut pool: Vec<u8> = vec![0u8; item_size* BUFFER_SIZE];
        let threshold = pool.len();
        let mut off = pool.len();

        // let mut rng = rand::rngs::OsRng::default();
        let mut rng = rand::rngs::StdRng::from_entropy();
        // let mut rng = xorshift64star{state: random()};


        return Box::new(move || {
            if off == threshold {
                rng.fill_bytes(pool.as_mut_slice());
                off = 0;
            }

            off += item_size;
        })
    });

    b.register("rng hash", || {
        let mut rng: RandomItemGenerator<HASH> = RandomItemGenerator::new(BUFFER_SIZE);

        return Box::new(move || {
            rng.next_item();
        });
    });

    b.register("utf8_to_utf16", || {
        return Box::new(move || {
            encode_to_utf16le("");
        });
    });

    b.register("md4_crate", || {
        let raw: HASH = Default::default();

        return Box::new(move || {
            let mut hasher = Md4::new();
            md4::Digest::update(&mut hasher, raw);
            let mut hash: &mut [u8; 16] = &mut Default::default();
            hash.copy_from_slice(hasher.finalize().as_slice());
        });
    });

    b.register("md4_rosettacode", || {
        let raw: HASH = Default::default();

        return Box::new(move || {
            md4_fast::md4(raw);
        });
    });

    if argv.len() == 0 || (argv.len() >= 0 && argv[0] == "all") {
        b.run_all(min_runtime);
    } else {
        for i in 1..args.len() {
            let test = &args[i];
            b.run(test.as_str(), min_runtime);
        }
    }
}
