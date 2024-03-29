extern crate byteorder;
extern crate time;
extern crate zebra_chain;
extern crate zebra_db;
extern crate zebra_network;
extern crate zebra_primitives;
extern crate zebra_storage;
extern crate zebra_test_data;
extern crate zebra_verification;

mod database;
mod verifier;

use std::io::Write;
use std::str;
use time::{Duration, PreciseTime};

#[derive(Default)]
pub struct Benchmark {
    start: Option<PreciseTime>,
    end: Option<PreciseTime>,
    samples: Option<usize>,
}

impl Benchmark {
    pub fn start(&mut self) {
        self.start = Some(PreciseTime::now());
    }

    pub fn stop(&mut self) {
        self.end = Some(PreciseTime::now());
    }

    pub fn evaluate(&self) -> Duration {
        self.start
            .expect("benchmarch never ended")
            .to(self.end.expect("benchmark never started"))
    }

    pub fn samples(&mut self, samples: usize) {
        self.samples = Some(samples);
    }
}

fn decimal_mark(s: String) -> String {
    let bytes: Vec<_> = s.bytes().rev().collect();
    let chunks: Vec<_> = bytes
        .chunks(3)
        .map(|chunk| str::from_utf8(chunk).unwrap())
        .collect();
    let result: Vec<_> = chunks.join(",").bytes().rev().collect();
    String::from_utf8(result).unwrap()
}

fn run_benchmark<F>(name: &str, f: F)
where
    F: FnOnce(&mut Benchmark),
{
    print!("{}: ", name);
    ::std::io::stdout().flush().unwrap();

    let mut benchmark = Benchmark::default();
    f(&mut benchmark);
    if let Some(samples) = benchmark.samples {
        println!(
            "{} ns/sample",
            decimal_mark(format!(
                "{}",
                benchmark.evaluate().num_nanoseconds().unwrap() / samples as i64
            )),
        );
    } else {
        println!(
            "{} ns",
            decimal_mark(format!(
                "{}",
                benchmark.evaluate().num_nanoseconds().unwrap()
            ))
        );
    }
}

macro_rules! benchmark {
    ($t:expr) => {
        run_benchmark(stringify!($t), $t);
    };
}

fn main() {
    benchmark!(database::fetch);
    benchmark!(database::write);
    benchmark!(database::reorg_short);
    benchmark!(database::write_heavy);
    benchmark!(verifier::main);
}
