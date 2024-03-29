use std::cmp::min;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use hibp_core::*;

use std::time::{Duration, Instant};
use md4::{Digest, Md4};
use rand::{Rng, RngCore, SeedableRng};
use thousands::Separable;
use clap::Parser;
use hibp_core::db::HIBPDB;

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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    dbdirectory: String,

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

fn main() {
    let args = Args::parse();

    let min_runtime = Duration::from_secs_f64(args.runtime);
    let mut b = Benchmarker{job: HashMap::new(), args: Args::parse()};

    b.register("StdRng", |args| {
        let mut rng = rand::rngs::StdRng::from_entropy();

        return Box::new(move || {
            rng.gen::<HASH>();
        });
    });

    b.register("rng_bytes", |args| {
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

    b.register("rng_hash", |args| {
        let mut rng: RandomItemGenerator<HASH> = RandomItemGenerator::new(BUFFER_SIZE);

        return Box::new(move || {
            rng.next_item();
        });
    });

    b.register("utf8_to_utf16", |args| {
        return Box::new(move || {
            encode_to_utf16le("");
        });
    });

    b.register("md4_crate", |args| {
        let raw: HASH = Default::default();

        return Box::new(move || {
            let mut hasher = Md4::new();
            md4::Digest::update(&mut hasher, raw);
            let mut hash: &mut [u8; 16] = &mut Default::default();
            hash.copy_from_slice(hasher.finalize().as_slice());
        });
    });

    b.register("dbquery_inmemory", |args| {
        let mut db = HIBPDB::new(args.dbdirectory.clone()).unwrap();
        let mut rng = RandomItemGenerator::new(BUFFER_SIZE);

        let mut index_slice: Vec<HASH> = Vec::new();
        unsafe {
            index_slice.reserve_exact(db.index().len());
            index_slice.set_len(db.index().len());
        }
        index_slice.copy_from_slice(db.index());
        // let index_slice = db.index().clone();

        return Box::new(move || {
            let key = rng.next_item();
            let _ = index_slice.binary_search(key);
            // let _ = db.find(key);
        })
    });

    b.register("dbquery_miss_binary_search", |args| {
        let mut db = HIBPDB::new(args.dbdirectory.clone()).unwrap();
        let mut rng = RandomItemGenerator::new(BUFFER_SIZE);

        return Box::new(move || {
            let key = rng.next_item();
            let _ = db.index().binary_search(key);
        })
    });

    b.register("dbquery_miss_interpolation_search", |args| {
        let mut db = HIBPDB::new(args.dbdirectory.clone()).unwrap();
        let mut rng = RandomItemGenerator::new(BUFFER_SIZE);

        return Box::new(move || {
            let key = rng.next_item();
            let slice = db.index();
            let _ = slice.interpolation_search(key);
        })
    });

    b.register("dbquery_hit_binary_search", |args| {
        let mut db = HIBPDB::new(args.dbdirectory.clone()).unwrap();
        let mut rng = RandomItemGenerator::<usize>::new(BUFFER_SIZE);

        return Box::new(move || {
            let array = db.index();
            let index = rng.next_item()%array.len();
            let key: &HASH = &array[index];
            let _ = array.binary_search(key);
        })
    });

    b.register("dbquery_hit_interpolation_search", |args| {
        let mut db = HIBPDB::new(args.dbdirectory.clone()).unwrap();
        let mut rng = RandomItemGenerator::<usize>::new(BUFFER_SIZE);

        return Box::new(move || {
            let array = db.index();
            let index = rng.next_item()%array.len();
            let key: &HASH = &array[index];
            let slice = db.index();
            let _ = slice.interpolation_search(key);
        })
    });

    b.register("range_extract", |args| {
        let mut db = HIBPDB::new(args.dbdirectory.clone()).unwrap();
        let mut rng = RandomItemGenerator::<usize>::new(BUFFER_SIZE);

        let map = db.range_map().unwrap();
        let mut buff: Vec<u8> = Vec::new();

        return Box::new(move || {
            buff.clear();
            let idx = rng.next_item()&0xFFFFF;
            let pathname = db.dbdir.clone()+"/range/"+&map[idx];
            let mut fd = File::open(&pathname).unwrap();
            fd.read_to_end(&mut buff).unwrap();
            if pathname.ends_with("xz") {
                extract_xz(buff.as_slice()).unwrap();
            } else {
                extract_gz(buff.as_slice()).unwrap();
            }
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

fn preload(arr: &[u8]) {
    let mut buff = [0u8; 1<<16];

    let mut off = 0;
    while off < arr.len() {
        let len = min(arr.len()-off, buff.len());
        buff[0..len].copy_from_slice(&arr[off..off+len]);
        off += len;
    }
}
