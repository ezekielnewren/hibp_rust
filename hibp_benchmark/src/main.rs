use hibp_core::*;

use std::time::{Duration, Instant};
use thousands::Separable;

pub fn timeit<T, F>(min_runtime: Duration, mut inner: F) -> u64
    where F: FnMut() -> T,
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


fn main() {

    let mut rng: RandomItemGenerator<HASH> = RandomItemGenerator::new(1000000);
    let min_runtime = Duration::from_secs_f64(3.0);
    let mut rate: u64 = 0;


    rate = timeit(min_runtime, || {
        rng.next_item();
    });
    println!("rng hash: {}", rate.separate_with_commas());

}
