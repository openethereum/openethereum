// Copyright 2020 Parity Technologies (UK) Ltd.
// This file is part of Parity Ethereum.

// Parity Ethereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Ethereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Ethereum.  If not, see <http://www.gnu.org/licenses/>.

// This program starts writing random data to the database with 100 (COLUMN_COUNT)
// columns and never stops until interrupted.

use ethereum_types::H256;
use keccak_hash::keccak;
use kvdb_rocksdb::{Database, DatabaseConfig};
use std::sync::{atomic::AtomicBool, atomic::Ordering as AtomicOrdering, Arc};
use sysinfo::{get_current_pid, ProcessExt, System, SystemExt};

const COLUMN_COUNT: u32 = 100;

#[derive(Clone)]
struct KeyValueSeed {
	seed: H256,
	key: H256,
	val: H256,
}

fn next(seed: H256) -> H256 {
	let mut buf = [0u8; 33];
	buf[0..32].copy_from_slice(&seed[..]);
	buf[32] = 1;

	keccak(&buf[..])
}

impl KeyValueSeed {
	fn with_seed(seed: H256) -> Self {
		KeyValueSeed { seed, key: next(seed), val: next(next(seed)) }
	}

	fn new() -> Self {
		Self::with_seed(H256::random())
	}
}

impl Iterator for KeyValueSeed {
	type Item = (H256, H256);

	fn next(&mut self) -> Option<Self::Item> {
		let result = (self.key, self.val);
		self.key = next(self.val);
		self.val = next(self.key);

		Some(result)
	}
}

fn proc_memory_usage() -> u64 {
	let mut sys = System::new();
	let self_pid = get_current_pid().ok();
	let memory = if let Some(self_pid) = self_pid {
		if sys.refresh_process(self_pid) {
			let proc = sys.get_process(self_pid).expect("Above refresh_process succeeds, this should be Some(), qed");
			proc.memory()
		} else {
			0
		}
	} else {
		0
	};

	memory
}

fn main() {
	let mb_per_col = std::env::args()
		.nth(1)
		.map(|arg| arg.parse().expect("Megabytes per col - should be integer or missing"))
		.unwrap_or(1);

	let exit = Arc::new(AtomicBool::new(false));
	let ctrlc_exit = exit.clone();

	ctrlc::set_handler(move || {
		println!("\nRemoving temp database...\n");
		ctrlc_exit.store(true, AtomicOrdering::Relaxed);
	})
	.expect("Error setting Ctrl-C handler");

	let mut config = DatabaseConfig::with_columns(COLUMN_COUNT);

	for c in 0..=COLUMN_COUNT {
		config.memory_budget.insert(c, mb_per_col);
	}
	let dir = tempdir::TempDir::new("rocksdb-example").unwrap();

	println!("Database is put in: {} (maybe check if it was deleted)", dir.path().to_string_lossy());
	let db = Database::open(&config, &dir.path().to_string_lossy()).unwrap();

	let mut step = 0;
	let mut keyvalues = KeyValueSeed::new();
	while !exit.load(AtomicOrdering::Relaxed) {
		let col = step % 100;

		let key_values: Vec<(H256, H256)> = keyvalues.clone().take(128).collect();
		let mut transaction = db.transaction();
		for (k, v) in key_values.iter() {
			transaction.put(col, k.as_ref(), v.as_ref());
		}
		db.write(transaction).expect("writing failed");

		let mut seed = H256::zero();
		for (k, _) in key_values.iter() {
			let mut buf = [0u8; 64];
			buf[0..32].copy_from_slice(seed.as_ref());
			let val = db.get(col, k.as_ref()).expect("Db fail").expect("Was put above");
			buf[32..64].copy_from_slice(val.as_ref());

			seed = keccak(&buf[..]);
		}

		let mut transaction = db.transaction();
		// delete all but one to avoid too much bloating
		for (k, _) in key_values.iter().take(127) {
			transaction.delete(col, k.as_ref());
		}
		db.write(transaction).expect("delete failed");

		keyvalues = KeyValueSeed::with_seed(seed);

		if step % 10000 == 9999 {
			let timestamp = time::strftime("%Y-%m-%d %H:%M:%S", &time::now()).expect("Error formatting log timestamp");

			println!("{}", timestamp);
			println!("\tData written: {} keys - {} Mb", step + 1, ((step + 1) * 64 * 128) / 1024 / 1024);
			println!("\tProcess memory used as seen by the OS: {} Mb", proc_memory_usage() / 1024);
			println!("\tMemory used as reported by rocksdb: {} Mb\n", parity_util_mem::malloc_size(&db) / 1024 / 1024);
		}

		step += 1;
	}
}
