// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use std::{path::PathBuf, sync::Arc};

use common_types::encoded::Block;
use elastic_array::ElasticArray128;
use ethcore_blockchain::BlockChainDB;
use ethcore_db::NUM_COLUMNS;
use ethjson::spec::Spec;
use kvdb::{DBTransaction, KeyValueDB};

struct KeyValueBackend {
    kv_forrest: sled::Db,
    trees: Vec<sled::Tree>,
}

/// This backend implements a disk-backed persistance for blockchain key-value-db.
/// Its implemented on top of sled crate, supports transactions and implements the KeyValueDB
/// trait. Used to store blockchain blocks.
pub struct LiteBackend {
    blooms: blooms_db::Database,
    trace_blooms: blooms_db::Database,
    kv_backend: Arc<dyn KeyValueDB>,
    storeroot: PathBuf,
}

impl LiteBackend {
    pub fn new(spec: &Spec, genesis: &Block) -> sled::Result<Self> {
        let state_root = hex::encode(&genesis.state_root()[0..6]);
        let dirname = format!("{}-{}", &spec.name, state_root);
        let dirpath = std::env::temp_dir().join(dirname);

        let bloomspath = dirpath.join("blooms");
        let tracespath = dirpath.join("trace_blooms");

        std::fs::create_dir_all(&dirpath)?;
        std::fs::create_dir_all(&bloomspath)?;
        std::fs::create_dir_all(&tracespath)?;
        
        Ok(LiteBackend {
            kv_backend: Arc::new(KeyValueBackend::new(&dirpath, NUM_COLUMNS.unwrap())?),
            blooms: blooms_db::Database::open(bloomspath)?,
            trace_blooms: blooms_db::Database::open(tracespath)?,
            storeroot: dirpath,
        })
    }
}

impl Drop for LiteBackend {
    fn drop(&mut self) {
        self.kv_backend
            .flush()
            .expect("failed to flush pending ops");
        std::fs::remove_dir_all(&self.storeroot).expect("failed to cleanup temp storage dir")
    }
}

impl BlockChainDB for LiteBackend {
    fn key_value(&self) -> &Arc<dyn KeyValueDB> {
        &self.kv_backend
    }

    fn blooms(&self) -> &blooms_db::Database {
        &self.blooms
    }

    fn trace_blooms(&self) -> &blooms_db::Database {
        &self.trace_blooms
    }
}

impl KeyValueBackend {
    pub fn new<P: AsRef<std::path::Path>>(path: P, columns: u32) -> sled::Result<Self> {
        let database = sled::open(path.as_ref())?;
        let trees = (0..columns)
            .map(|c| database.open_tree(format!("col_{}", c)).unwrap())
            .collect();
        database.flush()?;

        Ok(KeyValueBackend {
            kv_forrest: database,
            trees: trees,
        })
    }

    fn col(&self, column: Option<u32>) -> &sled::Tree {
        &self.trees[column.unwrap() as usize]
    }
}

impl Drop for KeyValueBackend {
    fn drop(&mut self) {}
}

impl KeyValueDB for KeyValueBackend {
    fn get(&self, col: Option<u32>, key: &[u8]) -> std::io::Result<Option<kvdb::DBValue>> {
        match self.col(col).get(key)? {
            None => Ok(None),
            Some(val) => Ok(Some(ElasticArray128::<u8>::from_vec(val.to_vec()))),
        }
    }

    fn get_by_prefix(&self, col: Option<u32>, prefix: &[u8]) -> Option<Box<[u8]>> {
        match self.col(col).scan_prefix(prefix).next() {
            None => None,
            Some(Err(_)) => panic!("read error"),
            Some(Ok((_, v))) => Some(v.to_vec().into_boxed_slice()),
        }
    }

    fn write_buffered(&self, transaction: DBTransaction) {
        for op in transaction.ops {
            match op {
                kvdb::DBOp::Insert { col, key, value } => {
                    self.col(col)
                        .insert(key.as_ref(), value.as_ref())
                        .expect("insertion failed");
                }
                kvdb::DBOp::Delete { col, key } => {
                    self.col(col).remove(key).expect("insertion failed");
                }
            }
        }
    }

    fn flush(&self) -> std::io::Result<()> {
        self.kv_forrest.flush()?;
        Ok(())
    }

    fn iter<'a>(
        &'a self,
        col: Option<u32>,
    ) -> Box<dyn Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a> {
        let converted = self.col(col).iter().map(|kv| match kv {
            Ok((k, v)) => (k.to_vec().into_boxed_slice(), v.to_vec().into_boxed_slice()),
            Err(_) => panic!("read error"),
        });
        Box::new(converted)
    }

    fn iter_from_prefix<'a>(
        &'a self,
        col: Option<u32>,
        prefix: &'a [u8],
    ) -> Box<dyn Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a> {
        let result = self.col(col).scan_prefix(prefix).map(|kv| match kv {
            Ok((k, v)) => (k.to_vec().into_boxed_slice(), v.to_vec().into_boxed_slice()),
            Err(_) => panic!("read error"),
        });
        Box::new(result)
    }

    fn restore(&self, new_db: &str) -> std::io::Result<()> {
        unimplemented!("db restore: {}", new_db);
    }
}
