// Copyright 2015-2020 Parity Technologies (UK) Ltd.
// This file is part of OpenEthereum.

// OpenEthereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// OpenEthereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with OpenEthereum.  If not, see <http://www.gnu.org/licenses/>.

use parking_lot::RwLock;
use std::{collections::HashMap, fmt, sync::Arc};

/// Generic cache implementation
#[derive(Debug, Clone)]
pub struct Cache<K, V> {
    values: Arc<RwLock<HashMap<K, V>>>,
    limit: usize,
    name: String,
}

impl<K, V> Cache<K, V> {
    /// Create new named cache with a limit of `limit` entries.
    pub fn new(name: &str, limit: usize) -> Self {
        Self {
            values: Arc::new(RwLock::new(HashMap::with_capacity(limit / 2))),
            limit,
            name: name.to_string(),
        }
    }

    /// Retrieve a cached value for given key.
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<V>
    where
        K: std::hash::Hash + std::cmp::Eq + std::borrow::Borrow<Q>,
        Q: std::hash::Hash + std::cmp::Eq,
        V: Clone,
    {
        self.values.read().get(key).cloned()
    }

    /// Inserts a value computed from f into the cache if it is not already cached,
    /// then returns the value.
    pub fn get_or_insert<F>(&self, key: K, f: F) -> V
    where
        K: std::hash::Hash + std::cmp::Eq,
        V: Clone,
        F: FnOnce() -> V,
    {
        if let Some(value) = self.get(&key) {
            // The corresponding value was cached.
            return value;
        }

        // We don't check again if cache has been populated.
        // We assume that it's not THAT expensive to fetch required data from state.
        let mut cache = self.values.write();
        let value = f();
        cache.insert(key, value.clone());

        if cache.len() < self.limit {
            return value;
        }

        debug!(target: "txpool", "{}Cache: reached limit.", self.name().to_string());
        trace_time!("txpool_cache:clear");

        // Remove excessive amount of entries from the cache.
        let remaining: Vec<_> = cache.drain().skip(self.limit / 2).collect();
        for (k, v) in remaining {
            cache.insert(k, v);
        }

        value
    }

    /// Clear all entries from the cache.
    pub fn clear(&self) {
        self.values.write().clear();
    }

    /// Returns the number of elements in the cache.
    pub fn len(&self) -> usize {
        self.values.read().len()
    }

    /// Returns maximum number of elements the cache can keep.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Returns the name of the cache
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub(super) struct CachedClient<'a, C: 'a, K, V> {
    client: &'a C,
    cache: &'a Cache<K, V>,
}

impl<'a, C: 'a, K, V> Clone for CachedClient<'a, C, K, V> {
    fn clone(&self) -> Self {
        Self {
            client: self.client,
            cache: self.cache,
        }
    }
}

impl<'a, C: 'a, K, V> fmt::Debug for CachedClient<'a, C, K, V> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("CachedClient")
            .field("name", &self.cache.name())
            .field("cache", &self.cache.len())
            .field("limit", &self.cache.limit())
            .finish()
    }
}

impl<'a, C: 'a, K, V> CachedClient<'a, C, K, V> {
    pub fn new(client: &'a C, cache: &'a Cache<K, V>) -> Self {
        Self { client, cache }
    }

    pub fn cache(&self) -> &'a Cache<K, V> {
        self.cache
    }

    pub fn client(&self) -> &'a C {
        self.client
    }
}
