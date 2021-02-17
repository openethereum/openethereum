// Copyright 2020 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

mod iter;
mod stats;

use std::{cmp, collections::HashMap, convert::identity, error, fs, io, mem, path::Path, result};

use parity_util_mem::MallocSizeOf;
use parking_lot::RwLock;
use rocksdb::{
	BlockBasedOptions, ColumnFamily, ColumnFamilyDescriptor, Error, Options, ReadOptions, WriteBatch, WriteOptions, DB,
};

use crate::iter::KeyValuePair;
use fs_swap::{swap, swap_nonatomic};
use kvdb::{DBOp, DBTransaction, DBValue, KeyValueDB};
use log::{debug, warn};

#[cfg(target_os = "linux")]
use regex::Regex;
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::process::Command;

fn other_io_err<E>(e: E) -> io::Error
where
	E: Into<Box<dyn error::Error + Send + Sync>>,
{
	io::Error::new(io::ErrorKind::Other, e)
}

// Used for memory budget.
type MiB = usize;

const KB: usize = 1_024;
const MB: usize = 1_024 * KB;

/// The default column memory budget in MiB.
pub const DB_DEFAULT_COLUMN_MEMORY_BUDGET_MB: MiB = 128;

/// The default memory budget in MiB.
pub const DB_DEFAULT_MEMORY_BUDGET_MB: MiB = 512;

/// Compaction profile for the database settings
/// Note, that changing these parameters may trigger
/// the compaction process of RocksDB on startup.
/// https://github.com/facebook/rocksdb/wiki/Leveled-Compaction#level_compaction_dynamic_level_bytes-is-true
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CompactionProfile {
	/// L0-L1 target file size
	/// The minimum size should be calculated in accordance with the
	/// number of levels and the expected size of the database.
	pub initial_file_size: u64,
	/// block size
	pub block_size: usize,
}

impl Default for CompactionProfile {
	/// Default profile suitable for most storage
	fn default() -> CompactionProfile {
		CompactionProfile::ssd()
	}
}

/// Given output of df command return Linux rotational flag file path.
#[cfg(target_os = "linux")]
pub fn rotational_from_df_output(df_out: Vec<u8>) -> Option<PathBuf> {
	use std::str;
	str::from_utf8(df_out.as_slice())
		.ok()
		// Get the drive name.
		.and_then(|df_str| {
			Regex::new(r"/dev/(sd[:alpha:]{1,2})")
				.ok()
				.and_then(|re| re.captures(df_str))
				.and_then(|captures| captures.get(1))
		})
		// Generate path e.g. /sys/block/sda/queue/rotational
		.map(|drive_path| {
			let mut p = PathBuf::from("/sys/block");
			p.push(drive_path.as_str());
			p.push("queue/rotational");
			p
		})
}

impl CompactionProfile {
	/// Attempt to determine the best profile automatically, only Linux for now.
	#[cfg(target_os = "linux")]
	pub fn auto(db_path: &Path) -> CompactionProfile {
		use std::io::Read;
		let hdd_check_file = db_path
			.to_str()
			.and_then(|path_str| Command::new("df").arg(path_str).output().ok())
			.and_then(|df_res| if df_res.status.success() { Some(df_res.stdout) } else { None })
			.and_then(rotational_from_df_output);
		// Read out the file and match compaction profile.
		if let Some(hdd_check) = hdd_check_file {
			if let Ok(mut file) = File::open(hdd_check.as_path()) {
				let mut buffer = [0; 1];
				if file.read_exact(&mut buffer).is_ok() {
					// 0 means not rotational.
					if buffer == [48] {
						return Self::ssd();
					}
					// 1 means rotational.
					if buffer == [49] {
						return Self::hdd();
					}
				}
			}
		}
		// Fallback if drive type was not determined.
		Self::default()
	}

	/// Just default for other platforms.
	#[cfg(not(target_os = "linux"))]
	pub fn auto(_db_path: &Path) -> CompactionProfile {
		Self::default()
	}

	/// Default profile suitable for SSD storage
	pub fn ssd() -> CompactionProfile {
		CompactionProfile { initial_file_size: 64 * MB as u64, block_size: 16 * KB }
	}

	/// Slow HDD compaction profile
	pub fn hdd() -> CompactionProfile {
		CompactionProfile { initial_file_size: 256 * MB as u64, block_size: 64 * KB }
	}
}

/// Database configuration
#[derive(Clone)]
pub struct DatabaseConfig {
	/// Max number of open files.
	pub max_open_files: i32,
	/// Memory budget (in MiB) used for setting block cache size and
	/// write buffer size for each column including the default one.
	/// If the memory budget of a column is not specified,
	/// `DB_DEFAULT_COLUMN_MEMORY_BUDGET_MB` is used for that column.
	pub memory_budget: HashMap<u32, MiB>,
	/// Compaction profile.
	pub compaction: CompactionProfile,
	/// Set number of columns.
	///
	/// # Safety
	///
	/// The number of columns must not be zero.
	pub columns: u32,
	/// Specify the maximum number of info/debug log files to be kept.
	pub keep_log_file_num: i32,
	/// Enable native RocksDB statistics.
	/// Disabled by default.
	///
	/// It can have a negative performance impact up to 10% according to
	/// https://github.com/facebook/rocksdb/wiki/Statistics.
	pub enable_statistics: bool,
	/// Open the database as a secondary instance.
	/// Specify a path for the secondary instance of the database.
	/// Secondary instances are read-only and kept updated by tailing the rocksdb MANIFEST.
	/// It is up to the user to call `catch_up_with_primary()` manually to update the secondary db.
	/// Disabled by default.
	///
	/// `max_open_files` is overridden to always equal `-1`.
	/// May have a negative performance impact on the secondary instance
	/// if the secondary instance reads and applies state changes before the primary instance compacts them.
	/// More info: https://github.com/facebook/rocksdb/wiki/Secondary-instance
	pub secondary: Option<String>,
        /// Limit the size of write ahead logs
        /// More info: 
        /// https://github.com/facebook/rocksdb/wiki/Write-Ahead-Log
        /// https://github.com/facebook/rocksdb/blob/48bfca38f6f175435052a59791922a1a453d9609/include/rocksdb/options.h
        pub max_total_wal_size: Option<u64>,
}

impl DatabaseConfig {
	/// Create new `DatabaseConfig` with default parameters and specified set of columns.
	/// Note that cache sizes must be explicitly set.
	///
	/// # Safety
	///
	/// The number of `columns` must not be zero.
	pub fn with_columns(columns: u32) -> Self {
		assert!(columns > 0, "the number of columns must not be zero");

		Self { columns, ..Default::default() }
	}

	/// Returns the total memory budget in bytes.
	pub fn memory_budget(&self) -> MiB {
		(0..self.columns).map(|i| self.memory_budget.get(&i).unwrap_or(&DB_DEFAULT_COLUMN_MEMORY_BUDGET_MB) * MB).sum()
	}

	/// Returns the memory budget of the specified column in bytes.
	fn memory_budget_for_col(&self, col: u32) -> MiB {
		self.memory_budget.get(&col).unwrap_or(&DB_DEFAULT_COLUMN_MEMORY_BUDGET_MB) * MB
	}

	// Get column family configuration with the given block based options.
	fn column_config(&self, block_opts: &BlockBasedOptions, col: u32) -> Options {
		let column_mem_budget = self.memory_budget_for_col(col);
		let mut opts = Options::default();

		opts.set_level_compaction_dynamic_level_bytes(true);
		opts.set_block_based_table_factory(block_opts);
		opts.optimize_level_style_compaction(column_mem_budget);
		opts.set_target_file_size_base(self.compaction.initial_file_size);
		opts.set_compression_per_level(&[]);

		opts
	}
}

impl Default for DatabaseConfig {
	fn default() -> DatabaseConfig {
		DatabaseConfig {
			max_open_files: 512,
			memory_budget: HashMap::new(),
			compaction: CompactionProfile::default(),
			columns: 1,
			keep_log_file_num: 1,
			enable_statistics: false,
			secondary: None,
                        max_total_wal_size: None,
		}
	}
}

struct DBAndColumns {
	db: DB,
	column_names: Vec<String>,
}

impl MallocSizeOf for DBAndColumns {
	fn size_of(&self, ops: &mut parity_util_mem::MallocSizeOfOps) -> usize {
		let mut total = self.column_names.size_of(ops)
			// we have at least one column always, so we can call property on it
			+ self.db
				.property_int_value_cf(self.cf(0), "rocksdb.block-cache-usage")
				.unwrap_or(Some(0))
				.map(|x| x as usize)
				.unwrap_or(0);

		for v in 0..self.column_names.len() {
			total += self.static_property_or_warn(v, "rocksdb.estimate-table-readers-mem");
			total += self.static_property_or_warn(v, "rocksdb.cur-size-all-mem-tables");
		}

		total
	}
}

impl DBAndColumns {
	fn cf(&self, i: usize) -> &ColumnFamily {
		self.db.cf_handle(&self.column_names[i]).expect("the specified column name is correct; qed")
	}

	fn static_property_or_warn(&self, col: usize, prop: &str) -> usize {
		match self.db.property_int_value_cf(self.cf(col), prop) {
			Ok(Some(v)) => v as usize,
			_ => {
				warn!("Cannot read expected static property of RocksDb database: {}", prop);
				0
			}
		}
	}
}

/// Key-Value database.
#[derive(MallocSizeOf)]
pub struct Database {
	db: RwLock<Option<DBAndColumns>>,
	#[ignore_malloc_size_of = "insignificant"]
	config: DatabaseConfig,
	path: String,
	#[ignore_malloc_size_of = "insignificant"]
	opts: Options,
	#[ignore_malloc_size_of = "insignificant"]
	write_opts: WriteOptions,
	#[ignore_malloc_size_of = "insignificant"]
	read_opts: ReadOptions,
	#[ignore_malloc_size_of = "insignificant"]
	block_opts: BlockBasedOptions,
	#[ignore_malloc_size_of = "insignificant"]
	stats: stats::RunningDbStats,
}

#[inline]
fn check_for_corruption<T, P: AsRef<Path>>(path: P, res: result::Result<T, Error>) -> io::Result<T> {
	if let Err(ref s) = res {
		if is_corrupted(s) {
			warn!("DB corrupted: {}. Repair will be triggered on next restart", s);
			let _ = fs::File::create(path.as_ref().join(Database::CORRUPTION_FILE_NAME));
		}
	}

	res.map_err(other_io_err)
}

fn is_corrupted(err: &Error) -> bool {
	err.as_ref().starts_with("Corruption:")
		|| err.as_ref().starts_with("Invalid argument: You have to open all column families")
}

/// Generate the options for RocksDB, based on the given `DatabaseConfig`.
fn generate_options(config: &DatabaseConfig) -> Options {
	let mut opts = Options::default();

	opts.set_report_bg_io_stats(true);
	if config.enable_statistics {
		opts.enable_statistics();
	}
	opts.set_use_fsync(false);
	opts.create_if_missing(true);
	if config.secondary.is_some() {
		opts.set_max_open_files(-1)
	} else {
		opts.set_max_open_files(config.max_open_files);
	}
	opts.set_bytes_per_sync(1 * MB as u64);
	opts.set_keep_log_file_num(1);
	opts.increase_parallelism(cmp::max(1, num_cpus::get() as i32 / 2));
        if let Some(m) = config.max_total_wal_size {
            opts.set_max_total_wal_size(m);
        }

	opts
}

fn generate_read_options() -> ReadOptions {
	let mut read_opts = ReadOptions::default();
	read_opts.set_verify_checksums(false);
	read_opts
}

/// Generate the block based options for RocksDB, based on the given `DatabaseConfig`.
fn generate_block_based_options(config: &DatabaseConfig) -> io::Result<BlockBasedOptions> {
	let mut block_opts = BlockBasedOptions::default();
	block_opts.set_block_size(config.compaction.block_size);
	// See https://github.com/facebook/rocksdb/blob/a1523efcdf2f0e8133b9a9f6e170a0dad49f928f/include/rocksdb/table.h#L246-L271 for details on what the format versions are/do.
	block_opts.set_format_version(5);
	block_opts.set_block_restart_interval(16);
	// Set cache size as recommended by
	// https://github.com/facebook/rocksdb/wiki/Setup-Options-and-Basic-Tuning#block-cache-size
	let cache_size = config.memory_budget() / 3;
	if cache_size == 0 {
		block_opts.disable_cache()
	} else {
		let cache = rocksdb::Cache::new_lru_cache(cache_size).map_err(other_io_err)?;
		block_opts.set_block_cache(&cache);
		// "index and filter blocks will be stored in block cache, together with all other data blocks."
		// See: https://github.com/facebook/rocksdb/wiki/Memory-usage-in-RocksDB#indexes-and-filter-blocks
		block_opts.set_cache_index_and_filter_blocks(true);
		// Don't evict L0 filter/index blocks from the cache
		block_opts.set_pin_l0_filter_and_index_blocks_in_cache(true);
	}
	block_opts.set_bloom_filter(10, true);

	Ok(block_opts)
}

impl Database {
	const CORRUPTION_FILE_NAME: &'static str = "CORRUPTED";

	/// Open database file. Creates if it does not exist.
	///
	/// # Safety
	///
	/// The number of `config.columns` must not be zero.
	pub fn open(config: &DatabaseConfig, path: &str) -> io::Result<Database> {
		assert!(config.columns > 0, "the number of columns must not be zero");

		let opts = generate_options(config);
		let block_opts = generate_block_based_options(config)?;

		// attempt database repair if it has been previously marked as corrupted
		let db_corrupted = Path::new(path).join(Database::CORRUPTION_FILE_NAME);
		if db_corrupted.exists() {
			warn!("DB has been previously marked as corrupted, attempting repair");
			DB::repair(&opts, path).map_err(other_io_err)?;
			fs::remove_file(db_corrupted)?;
		}

		let column_names: Vec<_> = (0..config.columns).map(|c| format!("col{}", c)).collect();
		let write_opts = WriteOptions::default();
		let read_opts = generate_read_options();

		let db = if let Some(secondary_path) = &config.secondary {
			Self::open_secondary(&opts, path, secondary_path.as_str(), column_names.as_slice())?
		} else {
			let column_names: Vec<&str> = column_names.iter().map(|s| s.as_str()).collect();
			Self::open_primary(&opts, path, config, column_names.as_slice(), &block_opts)?
		};

		Ok(Database {
			db: RwLock::new(Some(DBAndColumns { db, column_names })),
			config: config.clone(),
			path: path.to_owned(),
			opts,
			read_opts,
			write_opts,
			block_opts,
			stats: stats::RunningDbStats::new(),
		})
	}

	/// Internal api to open a database in primary mode.
	fn open_primary(
		opts: &Options,
		path: &str,
		config: &DatabaseConfig,
		column_names: &[&str],
		block_opts: &BlockBasedOptions,
	) -> io::Result<rocksdb::DB> {
		let cf_descriptors: Vec<_> = (0..config.columns)
			.map(|i| ColumnFamilyDescriptor::new(column_names[i as usize], config.column_config(&block_opts, i)))
			.collect();

		let db = match DB::open_cf_descriptors(&opts, path, cf_descriptors) {
			Err(_) => {
				// retry and create CFs
				match DB::open_cf(&opts, path, &[] as &[&str]) {
					Ok(mut db) => {
						for (i, name) in column_names.iter().enumerate() {
							let _ = db
								.create_cf(name, &config.column_config(&block_opts, i as u32))
								.map_err(other_io_err)?;
						}
						Ok(db)
					}
					err => err,
				}
			}
			ok => ok,
		};

		Ok(match db {
			Ok(db) => db,
			Err(ref s) if is_corrupted(s) => {
				warn!("DB corrupted: {}, attempting repair", s);
				DB::repair(&opts, path).map_err(other_io_err)?;

				let cf_descriptors: Vec<_> = (0..config.columns)
					.map(|i| {
						ColumnFamilyDescriptor::new(column_names[i as usize], config.column_config(&block_opts, i))
					})
					.collect();

				DB::open_cf_descriptors(&opts, path, cf_descriptors).map_err(other_io_err)?
			}
			Err(s) => return Err(other_io_err(s)),
		})
	}

	/// Internal api to open a database in secondary mode.
	/// Secondary database needs a seperate path to store its own logs.
	fn open_secondary(
		opts: &Options,
		path: &str,
		secondary_path: &str,
		column_names: &[String],
	) -> io::Result<rocksdb::DB> {
		let db = DB::open_cf_as_secondary(&opts, path, secondary_path, column_names);

		Ok(match db {
			Ok(db) => db,
			Err(ref s) if is_corrupted(s) => {
				warn!("DB corrupted: {}, attempting repair", s);
				DB::repair(&opts, path).map_err(other_io_err)?;
				DB::open_cf_as_secondary(&opts, path, secondary_path, column_names).map_err(other_io_err)?
			}
			Err(s) => return Err(other_io_err(s)),
		})
	}

	/// Helper to create new transaction for this database.
	pub fn transaction(&self) -> DBTransaction {
		DBTransaction::new()
	}

	/// Commit transaction to database.
	pub fn write(&self, tr: DBTransaction) -> io::Result<()> {
		match *self.db.read() {
			Some(ref cfs) => {
				let mut batch = WriteBatch::default();
				let ops = tr.ops;

				self.stats.tally_writes(ops.len() as u64);
				self.stats.tally_transactions(1);

				let mut stats_total_bytes = 0;

				for op in ops {
					let cf = cfs.cf(op.col() as usize);

					match op {
						DBOp::Insert { col: _, key, value } => {
							stats_total_bytes += key.len() + value.len();
							batch.put_cf(cf, &key, &value);
						}
						DBOp::Delete { col: _, key } => {
							// We count deletes as writes.
							stats_total_bytes += key.len();
							batch.delete_cf(cf, &key);
						}
						DBOp::DeletePrefix { col, prefix } => {
							let end_prefix = kvdb::end_prefix(&prefix[..]);
							let no_end = end_prefix.is_none();
							let end_range = end_prefix.unwrap_or_else(|| vec![u8::max_value(); 16]);
							batch.delete_range_cf(cf, &prefix[..], &end_range[..]);
							if no_end {
								use crate::iter::IterationHandler as _;

								let prefix = if prefix.len() > end_range.len() { &prefix[..] } else { &end_range[..] };
								// We call `iter_with_prefix` directly on `cfs` to avoid taking a lock twice
								// See https://github.com/paritytech/parity-common/pull/396.
								let read_opts = generate_read_options();
								for (key, _) in cfs.iter_with_prefix(col, prefix, read_opts) {
									batch.delete_cf(cf, &key[..]);
								}
							}
						}
					};
				}
				self.stats.tally_bytes_written(stats_total_bytes as u64);

				check_for_corruption(&self.path, cfs.db.write_opt(batch, &self.write_opts))
			}
			None => Err(other_io_err("Database is closed")),
		}
	}

	/// Get value by key.
	pub fn get(&self, col: u32, key: &[u8]) -> io::Result<Option<DBValue>> {
		match *self.db.read() {
			Some(ref cfs) => {
				if cfs.column_names.get(col as usize).is_none() {
					return Err(other_io_err("column index is out of bounds"));
				}
				self.stats.tally_reads(1);
				let value = cfs
					.db
					.get_pinned_cf_opt(cfs.cf(col as usize), key, &self.read_opts)
					.map(|r| r.map(|v| v.to_vec()))
					.map_err(other_io_err);

				match value {
					Ok(Some(ref v)) => self.stats.tally_bytes_read((key.len() + v.len()) as u64),
					Ok(None) => self.stats.tally_bytes_read(key.len() as u64),
					_ => {}
				};

				value
			}
			None => Ok(None),
		}
	}

	/// Get value by partial key. Prefix size should match configured prefix size.
	pub fn get_by_prefix(&self, col: u32, prefix: &[u8]) -> Option<Box<[u8]>> {
		self.iter_with_prefix(col, prefix).next().map(|(_, v)| v)
	}

	/// Iterator over the data in the given database column index.
	/// Will hold a lock until the iterator is dropped
	/// preventing the database from being closed.
	pub fn iter<'a>(&'a self, col: u32) -> impl Iterator<Item = KeyValuePair> + 'a {
		let read_lock = self.db.read();
		let optional = if read_lock.is_some() {
			let read_opts = generate_read_options();
			let guarded = iter::ReadGuardedIterator::new(read_lock, col, read_opts);
			Some(guarded)
		} else {
			None
		};
		optional.into_iter().flat_map(identity)
	}

	/// Iterator over data in the `col` database column index matching the given prefix.
	/// Will hold a lock until the iterator is dropped
	/// preventing the database from being closed.
	fn iter_with_prefix<'a>(&'a self, col: u32, prefix: &'a [u8]) -> impl Iterator<Item = iter::KeyValuePair> + 'a {
		let read_lock = self.db.read();
		let optional = if read_lock.is_some() {
			let mut read_opts = generate_read_options();
			// rocksdb doesn't work with an empty upper bound
			if let Some(end_prefix) = kvdb::end_prefix(prefix) {
				read_opts.set_iterate_upper_bound(end_prefix);
			}
			let guarded = iter::ReadGuardedIterator::new_with_prefix(read_lock, col, prefix, read_opts);
			Some(guarded)
		} else {
			None
		};
		optional.into_iter().flat_map(identity)
	}

	/// Close the database
	fn close(&self) {
		*self.db.write() = None;
	}

	/// Restore the database from a copy at given path.
	pub fn restore(&self, new_db: &str) -> io::Result<()> {
		self.close();

		// swap is guaranteed to be atomic
		match swap(new_db, &self.path) {
			Ok(_) => {
				// ignore errors
				let _ = fs::remove_dir_all(new_db);
			}
			Err(err) => {
				debug!("DB atomic swap failed: {}", err);
				match swap_nonatomic(new_db, &self.path) {
					Ok(_) => {
						// ignore errors
						let _ = fs::remove_dir_all(new_db);
					}
					Err(err) => {
						warn!("Failed to swap DB directories: {:?}", err);
						return Err(io::Error::new(
							io::ErrorKind::Other,
							"DB restoration failed: could not swap DB directories",
						));
					}
				}
			}
		}

		// reopen the database and steal handles into self
		let db = Self::open(&self.config, &self.path)?;
		*self.db.write() = mem::replace(&mut *db.db.write(), None);
		Ok(())
	}

	/// The number of column families in the db.
	pub fn num_columns(&self) -> u32 {
		self.db
			.read()
			.as_ref()
			.and_then(|db| if db.column_names.is_empty() { None } else { Some(db.column_names.len()) })
			.map(|n| n as u32)
			.unwrap_or(0)
	}

	/// The number of keys in a column (estimated).
	pub fn num_keys(&self, col: u32) -> io::Result<u64> {
		const ESTIMATE_NUM_KEYS: &str = "rocksdb.estimate-num-keys";
		match *self.db.read() {
			Some(ref cfs) => {
				let cf = cfs.cf(col as usize);
				match cfs.db.property_int_value_cf(cf, ESTIMATE_NUM_KEYS) {
					Ok(estimate) => Ok(estimate.unwrap_or_default()),
					Err(err_string) => Err(other_io_err(err_string)),
				}
			}
			None => Ok(0),
		}
	}

	/// Remove the last column family in the database. The deletion is definitive.
	pub fn remove_last_column(&self) -> io::Result<()> {
		match *self.db.write() {
			Some(DBAndColumns { ref mut db, ref mut column_names }) => {
				if let Some(name) = column_names.pop() {
					db.drop_cf(&name).map_err(other_io_err)?;
				}
				Ok(())
			}
			None => Ok(()),
		}
	}

	/// Add a new column family to the DB.
	pub fn add_column(&self) -> io::Result<()> {
		match *self.db.write() {
			Some(DBAndColumns { ref mut db, ref mut column_names }) => {
				let col = column_names.len() as u32;
				let name = format!("col{}", col);
				let col_config = self.config.column_config(&self.block_opts, col as u32);
				let _ = db.create_cf(&name, &col_config).map_err(other_io_err)?;
				column_names.push(name);
				Ok(())
			}
			None => Ok(()),
		}
	}

	/// Get RocksDB statistics.
	pub fn get_statistics(&self) -> HashMap<String, stats::RocksDbStatsValue> {
		if let Some(stats) = self.opts.get_statistics() {
			stats::parse_rocksdb_stats(&stats)
		} else {
			HashMap::new()
		}
	}

	/// Try to catch up a secondary instance with
	/// the primary by reading as much from the logs as possible.
	///
	/// Guaranteed to have changes up to the the time that `try_catch_up_with_primary` is called
	/// if it finishes succesfully.
	///
	/// Blocks until the MANIFEST file and any state changes in the corresponding Write-Ahead-Logs
	/// are applied to the secondary instance. If the manifest files are very large
	/// this method could take a long time.
	///
	/// If Write-Ahead-Logs have been purged by the primary instance before the secondary
	/// is able to open them, the secondary will not be caught up
	/// until this function is called again and new Write-Ahead-Logs are identified.
	///
	/// If called while the primary is writing, the catch-up may fail.
	///
	/// If the secondary is unable to catch up because of missing logs,
	/// this method fails silently and no error is returned.
	///
	/// Calling this as primary will return an error.
	pub fn try_catch_up_with_primary(&self) -> io::Result<()> {
		match self.db.read().as_ref() {
			Some(DBAndColumns { db, .. }) => db.try_catch_up_with_primary().map_err(other_io_err),
			None => Ok(()),
		}
	}
}

// duplicate declaration of methods here to avoid trait import in certain existing cases
// at time of addition.
impl KeyValueDB for Database {
	fn get(&self, col: u32, key: &[u8]) -> io::Result<Option<DBValue>> {
		Database::get(self, col, key)
	}

	fn get_by_prefix(&self, col: u32, prefix: &[u8]) -> Option<Box<[u8]>> {
		Database::get_by_prefix(self, col, prefix)
	}

	fn write(&self, transaction: DBTransaction) -> io::Result<()> {
		Database::write(self, transaction)
	}

	fn iter<'a>(&'a self, col: u32) -> Box<dyn Iterator<Item = KeyValuePair> + 'a> {
		let unboxed = Database::iter(self, col);
		Box::new(unboxed.into_iter())
	}

	fn iter_with_prefix<'a>(&'a self, col: u32, prefix: &'a [u8]) -> Box<dyn Iterator<Item = KeyValuePair> + 'a> {
		let unboxed = Database::iter_with_prefix(self, col, prefix);
		Box::new(unboxed.into_iter())
	}

	fn restore(&self, new_db: &str) -> io::Result<()> {
		Database::restore(self, new_db)
	}

	fn io_stats(&self, kind: kvdb::IoStatsKind) -> kvdb::IoStats {
		let rocksdb_stats = self.get_statistics();
		let cache_hit_count = rocksdb_stats.get("block.cache.hit").map(|s| s.count).unwrap_or(0u64);
		let overall_stats = self.stats.overall();
		let old_cache_hit_count = overall_stats.raw.cache_hit_count;

		self.stats.tally_cache_hit_count(cache_hit_count - old_cache_hit_count);

		let taken_stats = match kind {
			kvdb::IoStatsKind::Overall => self.stats.overall(),
			kvdb::IoStatsKind::SincePrevious => self.stats.since_previous(),
		};

		let mut stats = kvdb::IoStats::empty();

		stats.reads = taken_stats.raw.reads;
		stats.writes = taken_stats.raw.writes;
		stats.transactions = taken_stats.raw.transactions;
		stats.bytes_written = taken_stats.raw.bytes_written;
		stats.bytes_read = taken_stats.raw.bytes_read;
		stats.cache_reads = taken_stats.raw.cache_hit_count;
		stats.started = taken_stats.started;
		stats.span = taken_stats.started.elapsed();

		stats
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use kvdb_shared_tests as st;
	use std::io::{self, Read};
	use tempdir::TempDir;

	fn create(columns: u32) -> io::Result<Database> {
		let tempdir = TempDir::new("")?;
		let config = DatabaseConfig::with_columns(columns);
		Database::open(&config, tempdir.path().to_str().expect("tempdir path is valid unicode"))
	}

	#[test]
	fn get_fails_with_non_existing_column() -> io::Result<()> {
		let db = create(1)?;
		st::test_get_fails_with_non_existing_column(&db)
	}

	#[test]
	fn put_and_get() -> io::Result<()> {
		let db = create(1)?;
		st::test_put_and_get(&db)
	}

	#[test]
	fn delete_and_get() -> io::Result<()> {
		let db = create(1)?;
		st::test_delete_and_get(&db)
	}

	#[test]
	fn delete_prefix() -> io::Result<()> {
		let db = create(st::DELETE_PREFIX_NUM_COLUMNS)?;
		st::test_delete_prefix(&db)
	}

	#[test]
	fn iter() -> io::Result<()> {
		let db = create(1)?;
		st::test_iter(&db)
	}

	#[test]
	fn iter_with_prefix() -> io::Result<()> {
		let db = create(1)?;
		st::test_iter_with_prefix(&db)
	}

	#[test]
	fn complex() -> io::Result<()> {
		let db = create(1)?;
		st::test_complex(&db)
	}

	#[test]
	fn stats() -> io::Result<()> {
		let db = create(st::IO_STATS_NUM_COLUMNS)?;
		st::test_io_stats(&db)
	}

	#[test]
	fn secondary_db_get() -> io::Result<()> {
		let primary = TempDir::new("")?;
		let config = DatabaseConfig::with_columns(1);
		let db = Database::open(&config, primary.path().to_str().expect("tempdir path is valid unicode"))?;

		let key1 = b"key1";
		let mut transaction = db.transaction();
		transaction.put(0, key1, b"horse");
		db.write(transaction)?;

		let config = DatabaseConfig {
			secondary: TempDir::new("")?.path().to_str().map(|s| s.to_string()),
			..DatabaseConfig::with_columns(1)
		};
		let second_db = Database::open(&config, primary.path().to_str().expect("tempdir path is valid unicode"))?;
		assert_eq!(&*second_db.get(0, key1)?.unwrap(), b"horse");
		Ok(())
	}

	#[test]
	fn secondary_db_catch_up() -> io::Result<()> {
		let primary = TempDir::new("")?;
		let config = DatabaseConfig::with_columns(1);
		let db = Database::open(&config, primary.path().to_str().expect("tempdir path is valid unicode"))?;

		let config = DatabaseConfig {
			secondary: TempDir::new("")?.path().to_str().map(|s| s.to_string()),
			..DatabaseConfig::with_columns(1)
		};
		let second_db = Database::open(&config, primary.path().to_str().expect("tempdir path is valid unicode"))?;

		let mut transaction = db.transaction();
		transaction.put(0, b"key1", b"mule");
		transaction.put(0, b"key2", b"cat");
		db.write(transaction)?;

		second_db.try_catch_up_with_primary()?;
		assert_eq!(&*second_db.get(0, b"key2")?.unwrap(), b"cat");
		Ok(())
	}

	#[test]
	fn mem_tables_size() {
		let tempdir = TempDir::new("").unwrap();

		let config = DatabaseConfig {
			max_open_files: 512,
			memory_budget: HashMap::new(),
			compaction: CompactionProfile::default(),
			columns: 11,
			keep_log_file_num: 1,
			enable_statistics: false,
			secondary: None,
		};

		let db = Database::open(&config, tempdir.path().to_str().unwrap()).unwrap();

		let mut batch = db.transaction();
		for i in 0u32..10000u32 {
			batch.put(i / 1000 + 1, &i.to_le_bytes(), &(i * 17).to_le_bytes());
		}
		db.write(batch).unwrap();

		{
			let db = db.db.read();
			db.as_ref().map(|db| {
				assert!(db.static_property_or_warn(0, "rocksdb.cur-size-all-mem-tables") > 512);
			});
		}
	}

	#[test]
	#[cfg(target_os = "linux")]
	fn df_to_rotational() {
		use std::path::PathBuf;
		// Example df output.
		let example_df = vec![
			70, 105, 108, 101, 115, 121, 115, 116, 101, 109, 32, 32, 32, 32, 32, 49, 75, 45, 98, 108, 111, 99, 107,
			115, 32, 32, 32, 32, 32, 85, 115, 101, 100, 32, 65, 118, 97, 105, 108, 97, 98, 108, 101, 32, 85, 115, 101,
			37, 32, 77, 111, 117, 110, 116, 101, 100, 32, 111, 110, 10, 47, 100, 101, 118, 47, 115, 100, 97, 49, 32,
			32, 32, 32, 32, 32, 32, 54, 49, 52, 48, 57, 51, 48, 48, 32, 51, 56, 56, 50, 50, 50, 51, 54, 32, 32, 49, 57,
			52, 52, 52, 54, 49, 54, 32, 32, 54, 55, 37, 32, 47, 10,
		];
		let expected_output = Some(PathBuf::from("/sys/block/sda/queue/rotational"));
		assert_eq!(rotational_from_df_output(example_df), expected_output);
	}

	#[test]
	#[should_panic]
	fn db_config_with_zero_columns() {
		let _cfg = DatabaseConfig::with_columns(0);
	}

	#[test]
	#[should_panic]
	fn open_db_with_zero_columns() {
		let cfg = DatabaseConfig { columns: 0, ..Default::default() };
		let _db = Database::open(&cfg, "");
	}

	#[test]
	fn add_columns() {
		let config_1 = DatabaseConfig::default();
		let config_5 = DatabaseConfig::with_columns(5);

		let tempdir = TempDir::new("").unwrap();

		// open 1, add 4.
		{
			let db = Database::open(&config_1, tempdir.path().to_str().unwrap()).unwrap();
			assert_eq!(db.num_columns(), 1);

			for i in 2..=5 {
				db.add_column().unwrap();
				assert_eq!(db.num_columns(), i);
			}
		}

		// reopen as 5.
		{
			let db = Database::open(&config_5, tempdir.path().to_str().unwrap()).unwrap();
			assert_eq!(db.num_columns(), 5);
		}
	}

	#[test]
	fn remove_columns() {
		let config_1 = DatabaseConfig::default();
		let config_5 = DatabaseConfig::with_columns(5);

		let tempdir = TempDir::new("drop_columns").unwrap();

		// open 5, remove 4.
		{
			let db = Database::open(&config_5, tempdir.path().to_str().unwrap()).expect("open with 5 columns");
			assert_eq!(db.num_columns(), 5);

			for i in (1..5).rev() {
				db.remove_last_column().unwrap();
				assert_eq!(db.num_columns(), i);
			}
		}

		// reopen as 1.
		{
			let db = Database::open(&config_1, tempdir.path().to_str().unwrap()).unwrap();
			assert_eq!(db.num_columns(), 1);
		}
	}

	#[test]
	fn test_num_keys() {
		let tempdir = TempDir::new("").unwrap();
		let config = DatabaseConfig::with_columns(1);
		let db = Database::open(&config, tempdir.path().to_str().unwrap()).unwrap();

		assert_eq!(db.num_keys(0).unwrap(), 0, "database is empty after creation");
		let key1 = b"beef";
		let mut batch = db.transaction();
		batch.put(0, key1, key1);
		db.write(batch).unwrap();
		assert_eq!(db.num_keys(0).unwrap(), 1, "adding a key increases the count");
	}

	#[test]
	fn default_memory_budget() {
		let c = DatabaseConfig::default();
		assert_eq!(c.columns, 1);
		assert_eq!(c.memory_budget(), DB_DEFAULT_COLUMN_MEMORY_BUDGET_MB * MB, "total memory budget is default");
		assert_eq!(
			c.memory_budget_for_col(0),
			DB_DEFAULT_COLUMN_MEMORY_BUDGET_MB * MB,
			"total memory budget for column 0 is the default"
		);
		assert_eq!(
			c.memory_budget_for_col(999),
			DB_DEFAULT_COLUMN_MEMORY_BUDGET_MB * MB,
			"total memory budget for any column is the default"
		);
	}

	#[test]
	fn memory_budget() {
		let mut c = DatabaseConfig::with_columns(3);
		c.memory_budget = [(0, 10), (1, 15), (2, 20)].iter().cloned().collect();
		assert_eq!(c.memory_budget(), 45 * MB, "total budget is the sum of the column budget");
	}

	#[test]
	fn test_stats_parser() {
		let raw = r#"rocksdb.row.cache.hit COUNT : 1
rocksdb.db.get.micros P50 : 2.000000 P95 : 3.000000 P99 : 4.000000 P100 : 5.000000 COUNT : 0 SUM : 15
"#;
		let stats = stats::parse_rocksdb_stats(raw);
		assert_eq!(stats["row.cache.hit"].count, 1);
		assert!(stats["row.cache.hit"].times.is_none());
		assert_eq!(stats["db.get.micros"].count, 0);
		let get_times = stats["db.get.micros"].times.unwrap();
		assert_eq!(get_times.sum, 15);
		assert_eq!(get_times.p50, 2.0);
		assert_eq!(get_times.p95, 3.0);
		assert_eq!(get_times.p99, 4.0);
		assert_eq!(get_times.p100, 5.0);
	}

	#[test]
	fn rocksdb_settings() {
		const NUM_COLS: usize = 2;
		let mut cfg = DatabaseConfig { enable_statistics: true, ..DatabaseConfig::with_columns(NUM_COLS as u32) };
		cfg.max_open_files = 123; // is capped by the OS fd limit (typically 1024)
		cfg.compaction.block_size = 323232;
		cfg.compaction.initial_file_size = 102030;
		cfg.memory_budget = [(0, 30), (1, 300)].iter().cloned().collect();

		let db_path = TempDir::new("config_test").expect("the OS can create tmp dirs");
		let db = Database::open(&cfg, db_path.path().to_str().unwrap()).expect("can open a db");
		let mut rocksdb_log = std::fs::File::open(format!("{}/LOG", db_path.path().to_str().unwrap()))
			.expect("rocksdb creates a LOG file");
		let mut settings = String::new();
		let statistics = db.get_statistics();
		assert!(statistics.contains_key("block.cache.hit"));

		rocksdb_log.read_to_string(&mut settings).unwrap();
		// Check column count
		assert!(settings.contains("Options for column family [default]"), "no default col");
		assert!(settings.contains("Options for column family [col0]"), "no col0");
		assert!(settings.contains("Options for column family [col1]"), "no col1");

		// Check max_open_files
		assert!(settings.contains("max_open_files: 123"));

		// Check block size
		assert!(settings.contains(" block_size: 323232"));

		// LRU cache (default column)
		assert!(settings.contains("block_cache_options:\n    capacity : 8388608"));
		// LRU cache for non-default columns is ⅓ of memory budget (including default column)
		let lru_size = (330 * MB) / 3;
		let needle = format!("block_cache_options:\n    capacity : {}", lru_size);
		let lru = settings.match_indices(&needle).collect::<Vec<_>>().len();
		assert_eq!(lru, NUM_COLS);

		// Index/filters share cache
		let include_indexes = settings.matches("cache_index_and_filter_blocks: 1").collect::<Vec<_>>().len();
		assert_eq!(include_indexes, NUM_COLS);
		// Pin index/filters on L0
		let pins = settings.matches("pin_l0_filter_and_index_blocks_in_cache: 1").collect::<Vec<_>>().len();
		assert_eq!(pins, NUM_COLS);

		// Check target file size, aka initial file size
		let l0_sizes = settings.matches("target_file_size_base: 102030").collect::<Vec<_>>().len();
		assert_eq!(l0_sizes, NUM_COLS);
		// The default column uses the default of 64Mb regardless of the setting.
		assert!(settings.contains("target_file_size_base: 67108864"));

		// Check compression settings
		let snappy_compression = settings.matches("Options.compression: Snappy").collect::<Vec<_>>().len();
		// All columns use Snappy
		assert_eq!(snappy_compression, NUM_COLS + 1);
		// …even for L7
		let snappy_bottommost = settings.matches("Options.bottommost_compression: Disabled").collect::<Vec<_>>().len();
		assert_eq!(snappy_bottommost, NUM_COLS + 1);

		// 7 levels
		let levels = settings.matches("Options.num_levels: 7").collect::<Vec<_>>().len();
		assert_eq!(levels, NUM_COLS + 1);

		// Don't fsync every store
		assert!(settings.contains("Options.use_fsync: 0"));

		// We're using the new format
		assert!(settings.contains("format_version: 5"));
	}
}
