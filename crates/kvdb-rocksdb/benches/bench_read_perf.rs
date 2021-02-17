// Copyright 2020 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Benchmark RocksDB read performance.
//! The benchmark setup consists in writing `NEEDLES * NEEDLES_TO_HAYSTACK_RATIO` 32-bytes random
//! keys with random values 150 +/- 30 bytes long. With 10 000 keys and a ratio of 100 we get one
//! million keys; ideally the db should be deleted for each benchmark run but in practice it has
//! little impact on the performance numbers for these small database sizes.
//! Allocations (on the Rust side) are counted and printed.
//!
//! Note that this benchmark is not a good way to measure the performance of the database itself;
//! its purpose is to be a tool to gauge the performance of the glue code, or work as a starting point
//! for a more elaborate benchmark of a specific workload.

const NEEDLES: usize = 10_000;
const NEEDLES_TO_HAYSTACK_RATIO: usize = 100;

use std::io;
use std::time::{Duration, Instant};

use alloc_counter::{count_alloc, AllocCounterSystem};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ethereum_types::H256;
use rand::{distributions::Uniform, seq::SliceRandom, Rng};

use kvdb_rocksdb::{Database, DatabaseConfig};

#[global_allocator]
static A: AllocCounterSystem = AllocCounterSystem;

criterion_group!(benches, get, iter);
criterion_main!(benches);

/// Opens (or creates) a RocksDB database in the `benches/` folder of the crate with one column
/// family and default options. Needs manual cleanup.
fn open_db() -> Database {
	let tempdir_str = "./benches/_rocksdb_bench_get";
	let cfg = DatabaseConfig::with_columns(1);
	let db = Database::open(&cfg, tempdir_str).expect("rocksdb works");
	db
}

/// Generate `n` random bytes +/- 20%.
/// The variability in the payload size lets us simulate payload allocation patterns: `DBValue` is
/// an `ElasticArray128` so sometimes we save on allocations.
fn n_random_bytes(n: usize) -> Vec<u8> {
	let mut rng = rand::thread_rng();
	let variability: i64 = rng.gen_range(0, (n / 5) as i64);
	let plus_or_minus: i64 = if variability % 2 == 0 { 1 } else { -1 };
	let range = Uniform::from(0..u8::max_value());
	rng.sample_iter(&range).take((n as i64 + plus_or_minus * variability) as usize).collect()
}

/// Writes `NEEDLES * NEEDLES_TO_HAYSTACK_RATIO` keys to the DB. Keys are random, 32 bytes long and
/// values are random, 120-180 bytes long. Every `NEEDLES_TO_HAYSTACK_RATIO` keys are kept and
/// returned in a `Vec` for and used to benchmark point lookup performance. Keys are sorted
/// lexicographically in the DB, and the benchmark keys are random bytes making the needles are
/// effectively random points in the key set.
fn populate(db: &Database) -> io::Result<Vec<H256>> {
	let mut needles = Vec::with_capacity(NEEDLES);
	let mut batch = db.transaction();
	for i in 0..NEEDLES * NEEDLES_TO_HAYSTACK_RATIO {
		let key = H256::random();
		if i % NEEDLES_TO_HAYSTACK_RATIO == 0 {
			needles.push(key.clone());
			if i % 100_000 == 0 && i > 0 {
				println!("[populate] {} keys", i);
			}
		}
		// In ethereum keys are mostly 32 bytes and payloads ~140bytes.
		batch.put(0, &key.as_bytes(), &n_random_bytes(140));
	}
	db.write(batch)?;
	Ok(needles)
}

fn get(c: &mut Criterion) {
	let db = open_db();
	let needles = populate(&db).expect("rocksdb works");

	let mut total_iterations = 0;
	let mut total_allocs = 0;

	c.bench_function("get key", |b| {
		b.iter_custom(|iterations| {
			total_iterations += iterations;
			let mut elapsed = Duration::new(0, 0);
			// NOTE: counts allocations on the Rust side only
			let (alloc_stats, _) = count_alloc(|| {
				let start = Instant::now();
				for _ in 0..iterations {
					// This has no measurable impact on performance (~30ns)
					let needle = needles.choose(&mut rand::thread_rng()).expect("needles is not empty");
					black_box(db.get(0, needle.as_bytes()).unwrap());
				}
				elapsed = start.elapsed();
			});
			total_allocs += alloc_stats.0;
			elapsed
		});
	});
	if total_iterations > 0 {
		println!(
			"[get key] total: iterations={}, allocations={}; allocations per iter={:.2}\n",
			total_iterations,
			total_allocs,
			total_allocs as f64 / total_iterations as f64
		);
	}

	total_iterations = 0;
	total_allocs = 0;
	c.bench_function("get key by prefix", |b| {
		b.iter_custom(|iterations| {
			total_iterations += iterations;
			let mut elapsed = Duration::new(0, 0);
			// NOTE: counts allocations on the Rust side only
			let (alloc_stats, _) = count_alloc(|| {
				let start = Instant::now();
				for _ in 0..iterations {
					// This has no measurable impact on performance (~30ns)
					let needle = needles.choose(&mut rand::thread_rng()).expect("needles is not empty");
					black_box(db.get_by_prefix(0, &needle.as_bytes()[..8]).unwrap());
				}
				elapsed = start.elapsed();
			});
			total_allocs += alloc_stats.0;
			elapsed
		});
	});
	if total_iterations > 0 {
		println!(
			"[get key by prefix] total: iterations={}, allocations={}; allocations per iter={:.2}\n",
			total_iterations,
			total_allocs,
			total_allocs as f64 / total_iterations as f64
		);
	}
}

fn iter(c: &mut Criterion) {
	let db = open_db();
	let mut total_iterations = 0;
	let mut total_allocs = 0;

	c.bench_function("iterate over 1k keys", |b| {
		b.iter_custom(|iterations| {
			total_iterations += iterations;
			let mut elapsed = Duration::new(0, 0);
			// NOTE: counts allocations on the Rust side only
			let (alloc_stats, _) = count_alloc(|| {
				let start = Instant::now();
				for _ in 0..iterations {
					black_box(db.iter(0).take(1000).collect::<Vec<_>>());
				}
				elapsed = start.elapsed();
			});
			total_allocs += alloc_stats.0;
			elapsed
		});
	});
	if total_iterations > 0 {
		println!(
			"[iterate over 1k keys] total: iterations={}, allocations={}; allocations per iter={:.2}\n",
			total_iterations,
			total_allocs,
			total_allocs as f64 / total_iterations as f64
		);
	}

	total_allocs = 0;
	total_iterations = 0;
	c.bench_function("single key from iterator", |b| {
		b.iter_custom(|iterations| {
			total_iterations += iterations;
			let mut elapsed = Duration::new(0, 0);
			// NOTE: counts allocations on the Rust side only
			let (alloc_stats, _) = count_alloc(|| {
				let start = Instant::now();
				for _ in 0..iterations {
					black_box(db.iter(0).next().unwrap());
				}
				elapsed = start.elapsed();
			});
			total_allocs += alloc_stats.0;
			elapsed
		});
	});
	if total_iterations > 0 {
		println!(
			"[single key from iterator] total: iterations={}, allocations={}; allocations per iter={:.2}\n",
			total_iterations,
			total_allocs,
			total_allocs as f64 / total_iterations as f64
		);
	}
}
