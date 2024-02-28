use std::collections::HashMap;
use std::io::{Write};
use std::path::PathBuf;
use hibp_core::*;

use std::time::{Duration, Instant};
use md4::{Digest, Md4};
use rand::{Rng, RngCore, SeedableRng};
use thousands::Separable;
use clap::Parser;
use hibp_core::db::HIBPDB;
use rayon::prelude::*;

pub fn timeit<F>(min_runtime: Duration, mut inner: F) -> u64
    where F: FnMut(),
{
    let mut rate;
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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    dbdirectory: String,

    #[arg(short, long)]
    sandbox: bool,

    #[arg(short, long, default_value_t = 3.0)]
    runtime: f64,

    benchmark: Vec<String>,
}


struct BenchmarkJob {
    name: String,
    func: Box<dyn Fn(&Args) -> Box<dyn FnMut()>>,
}
struct Benchmarker {
    job: HashMap<String, BenchmarkJob>,
    args: Args,
}

impl Benchmarker {

    fn register<F>(&mut self, name: &str, closure: F)
    where
        F: Fn(&Args) -> Box<dyn FnMut()> + 'static,
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
        let args: &Args = &self.args;

        let job: &mut BenchmarkJob = self.job.get_mut(name).unwrap();

        print!("{}: ", job.name);
        std::io::stdout().flush().unwrap();
        let inner = (job.func)(args);
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

pub fn mean<T>(slice: &[T], c: fn(&T) -> f64) -> f64
    where T: Send + Sync
{
    slice.par_iter().map(c).sum::<f64>()/slice.len() as f64
}


pub fn linear_regression<T>(slice: &[T], c: fn(&T) -> f64) -> (f64, f64)
    where T: Send + Sync
{
    let n = slice.len() as f64;

    let y_mean: f64 = mean(slice, c);
    let x_mean = (n-1.0)/2.0;

    let numerator: f64 = (0..slice.len()).into_par_iter().map(|i| {
        let x = i as f64;
        let y = c(&slice[i]);
        (x - x_mean) * (y - y_mean)
    }).sum();

    let denominator = n*(n-1.0)*(2.0*n-1.0)/6.0 - x_mean*n*(n-1.0) + n*x_mean*x_mean;
    let m = numerator / denominator;

    let b = y_mean - (m * x_mean);
    return (m, b);
}

fn sandbox(args: &Args) {
    let dbdir = PathBuf::from(args.dbdirectory.clone());
    let db = HIBPDB::open(dbdir.as_path()).unwrap();

    let hash_col = db.hash_col.as_slice();

    let (m, b) = linear_regression(hash_col, |&hash| u128::from_be_bytes(hash) as f64);

    println!("y = {}*x + {}", m as u128, b as u128);
}

fn main() {
    let args = Args::parse();

    if args.sandbox {
        let start = Instant::now();
        sandbox(&args);
        let elapsed = start.elapsed().as_secs_f64();
        println!("elapsed: {}", elapsed);
        return;
    }

    let min_runtime = Duration::from_secs_f64(args.runtime);
    let mut b = Benchmarker{job: HashMap::new(), args: Args::parse()};

    b.register("StdRng", |_| {
        let mut rng = rand::rngs::StdRng::from_entropy();

        return Box::new(move || {
            rng.gen::<HASH>();
        });
    });

    b.register("rng_bytes", |_| {
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

    b.register("rng_hash", |_| {
        let mut rng: RandomItemGenerator<HASH> = RandomItemGenerator::new(BUFFER_SIZE);

        return Box::new(move || {
            rng.next_item();
        });
    });

    b.register("utf8_to_utf16", |_| {
        return Box::new(move || {
            encode_to_utf16le("");
        });
    });

    b.register("md4_crate", |_| {
        let raw: HASH = Default::default();

        return Box::new(move || {
            let mut hasher = Md4::new();
            md4::Digest::update(&mut hasher, raw);
            let hash: &mut [u8; 16] = &mut Default::default();
            hash.copy_from_slice(hasher.finalize().as_slice());
        });
    });

    b.register("dbquery_inmemory", |args| {
        let dbdir = PathBuf::from(args.dbdirectory.clone());
        let db = HIBPDB::open(dbdir.as_path()).unwrap();
        let mut rng = RandomItemGenerator::new(BUFFER_SIZE);

        let mut index_slice: Vec<HASH> = Vec::new();
        unsafe {
            index_slice.reserve_exact(db.hash().len());
            index_slice.set_len(db.hash().len());
        }
        index_slice.copy_from_slice(db.hash());
        // let index_slice = db.hash().clone();

        return Box::new(move || {
            let key = rng.next_item();
            let _ = index_slice.binary_search(key);
            // let _ = db.find(key);
        })
    });

    b.register("dbquery_miss_binary_search", |args| {
        let dbdir = PathBuf::from(args.dbdirectory.clone());
        let db = HIBPDB::open(dbdir.as_path()).unwrap();
        let mut rng = RandomItemGenerator::new(BUFFER_SIZE);

        return Box::new(move || {
            let key = rng.next_item();
            let _ = db.hash().binary_search(key);
        })
    });

    b.register("dbquery_miss_interpolation_search", |args| {
        let dbdir = PathBuf::from(args.dbdirectory.clone());
        let db = HIBPDB::open(dbdir.as_path()).unwrap();
        let mut rng = RandomItemGenerator::new(BUFFER_SIZE);

        return Box::new(move || {
            let key = rng.next_item();
            let slice = db.hash();
            let _ = slice.interpolation_search(key);
        })
    });

    b.register("dbquery_hit_binary_search", |args| {
        let dbdir = PathBuf::from(args.dbdirectory.clone());
        let db = HIBPDB::open(dbdir.as_path()).unwrap();
        let mut rng = RandomItemGenerator::<usize>::new(BUFFER_SIZE);

        return Box::new(move || {
            let array = db.hash();
            let index = rng.next_item()%array.len();
            let key: &HASH = &array[index];
            let _ = array.binary_search(key);
        })
    });

    b.register("dbquery_hit_interpolation_search", |args| {
        let dbdir = PathBuf::from(args.dbdirectory.clone());
        let db = HIBPDB::open(dbdir.as_path()).unwrap();
        let mut rng = RandomItemGenerator::<usize>::new(BUFFER_SIZE);

        return Box::new(move || {
            let array = db.hash();
            let index = rng.next_item()%array.len();
            let key: &HASH = &array[index];
            let slice = db.hash();
            let _ = slice.interpolation_search(key);
        })
    });

    b.register("range_read", |args| {
        let prefix = PathBuf::from(args.dbdirectory.clone()+"/range");
        // let mut rng = RandomItemGenerator::<usize>::new(BUFFER_SIZE);

        let mut i = 0;

        return Box::new(move || {
            if i < (1<<20) {
                let _ = HIBPDB::load(prefix.as_path(), i);
            }
            i += 1;
        })
    });

    let args = Args::parse();
    if args.benchmark.is_empty() {
        b.run_all(min_runtime);
    } else {
        for name in &args.benchmark {
            b.run(name.as_str(), min_runtime);
        }
    }
}
