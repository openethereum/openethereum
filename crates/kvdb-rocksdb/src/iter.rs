// Copyright 2020 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This module contains an implementation of a RocksDB iterator
//! wrapped inside a `RwLock`. Since `RwLock` "owns" the inner data,
//! we're using `owning_ref` to work around the borrowing rules of Rust.
//!
//! Note: this crate does not use "Prefix Seek" mode which means that the prefix iterator
//! will return keys not starting with the given prefix as well (as long as `key >= prefix`).
//! To work around this we set an upper bound to the prefix successor.
//! See https://github.com/facebook/rocksdb/wiki/Prefix-Seek-API-Changes for details.

use crate::DBAndColumns;
use owning_ref::{OwningHandle, StableAddress};
use parking_lot::RwLockReadGuard;
use rocksdb::{DBIterator, Direction, IteratorMode, ReadOptions};
use std::ops::{Deref, DerefMut};

/// A tuple holding key and value data, used as the iterator item type.
pub type KeyValuePair = (Box<[u8]>, Box<[u8]>);

/// Iterator with built-in synchronization.
pub struct ReadGuardedIterator<'a, I, T> {
	inner: OwningHandle<UnsafeStableAddress<'a, Option<T>>, DerefWrapper<Option<I>>>,
}

// We can't implement `StableAddress` for a `RwLockReadGuard`
// directly due to orphan rules.
#[repr(transparent)]
struct UnsafeStableAddress<'a, T>(RwLockReadGuard<'a, T>);

impl<'a, T> Deref for UnsafeStableAddress<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0.deref()
	}
}

// RwLockReadGuard dereferences to a stable address; qed
unsafe impl<'a, T> StableAddress for UnsafeStableAddress<'a, T> {}

struct DerefWrapper<T>(T);

impl<T> Deref for DerefWrapper<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T> DerefMut for DerefWrapper<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<'a, I: Iterator, T> Iterator for ReadGuardedIterator<'a, I, T> {
	type Item = I::Item;

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.deref_mut().as_mut().and_then(|iter| iter.next())
	}
}

/// Instantiate iterators yielding `KeyValuePair`s.
pub trait IterationHandler {
	type Iterator: Iterator<Item = KeyValuePair>;

	/// Create an `Iterator` over a `ColumnFamily` corresponding to the passed index. Takes
	/// `ReadOptions` to allow configuration of the new iterator (see
	/// https://github.com/facebook/rocksdb/blob/master/include/rocksdb/options.h#L1169).
	fn iter(&self, col: u32, read_opts: ReadOptions) -> Self::Iterator;
	/// Create an `Iterator` over a `ColumnFamily` corresponding to the passed index. Takes
	/// `ReadOptions` to allow configuration of the new iterator (see
	/// https://github.com/facebook/rocksdb/blob/master/include/rocksdb/options.h#L1169).
	/// The `Iterator` iterates over keys which start with the provided `prefix`.
	fn iter_with_prefix(&self, col: u32, prefix: &[u8], read_opts: ReadOptions) -> Self::Iterator;
}

impl<'a, T> ReadGuardedIterator<'a, <&'a T as IterationHandler>::Iterator, T>
where
	&'a T: IterationHandler,
{
	/// Creates a new `ReadGuardedIterator` that maps `RwLock<RocksDB>` to `RwLock<DBIterator>`,
	/// where `DBIterator` iterates over all keys.
	pub fn new(read_lock: RwLockReadGuard<'a, Option<T>>, col: u32, read_opts: ReadOptions) -> Self {
		Self { inner: Self::new_inner(read_lock, |db| db.iter(col, read_opts)) }
	}

	/// Creates a new `ReadGuardedIterator` that maps `RwLock<RocksDB>` to `RwLock<DBIterator>`,
	/// where `DBIterator` iterates over keys which start with the given prefix.
	pub fn new_with_prefix(
		read_lock: RwLockReadGuard<'a, Option<T>>,
		col: u32,
		prefix: &[u8],
		read_opts: ReadOptions,
	) -> Self {
		Self { inner: Self::new_inner(read_lock, |db| db.iter_with_prefix(col, prefix, read_opts)) }
	}

	fn new_inner(
		rlock: RwLockReadGuard<'a, Option<T>>,
		f: impl FnOnce(&'a T) -> <&'a T as IterationHandler>::Iterator,
	) -> OwningHandle<UnsafeStableAddress<'a, Option<T>>, DerefWrapper<Option<<&'a T as IterationHandler>::Iterator>>> {
		OwningHandle::new_with_fn(UnsafeStableAddress(rlock), move |rlock| {
			let rlock = unsafe { rlock.as_ref().expect("initialized as non-null; qed") };
			DerefWrapper(rlock.as_ref().map(f))
		})
	}
}

impl<'a> IterationHandler for &'a DBAndColumns {
	type Iterator = DBIterator<'a>;

	fn iter(&self, col: u32, read_opts: ReadOptions) -> Self::Iterator {
		self.db.iterator_cf_opt(self.cf(col as usize), read_opts, IteratorMode::Start)
	}

	fn iter_with_prefix(&self, col: u32, prefix: &[u8], read_opts: ReadOptions) -> Self::Iterator {
		self.db.iterator_cf_opt(self.cf(col as usize), read_opts, IteratorMode::From(prefix, Direction::Forward))
	}
}
