// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use std::sync::Arc;

use tempdir::TempDir;

use kvdb::{
  DBTransaction, 
  KeyValueDB
};

use common_types::{
    encoded,
    receipt::TypedReceipt,
};
use ethcore_blockchain::{
    BlockChain, BlockChainDB, Config, 
    ExtrasInsert, ImportRoute,
    InTransactionBlockProvider,
};

pub struct InMemoryBlockChainDB {
    _blooms_dir: TempDir,
    _trace_blooms_dir: TempDir,
    blooms: blooms_db::Database,
    trace_blooms: blooms_db::Database,
    key_value: Arc<dyn KeyValueDB>,
}

impl BlockChainDB for InMemoryBlockChainDB {
    fn key_value(&self) -> &Arc<dyn KeyValueDB> {
        &self.key_value
    }

    fn blooms(&self) -> &blooms_db::Database {
        &self.blooms
    }

    fn trace_blooms(&self) -> &blooms_db::Database {
        &self.trace_blooms
    }
}

pub fn new_db() -> Arc<dyn BlockChainDB> {
    let blooms_dir = TempDir::new("").unwrap();
    let trace_blooms_dir = TempDir::new("").unwrap();

    let db = InMemoryBlockChainDB {
        blooms: blooms_db::Database::open(blooms_dir.path()).unwrap(),
        trace_blooms: blooms_db::Database::open(trace_blooms_dir.path()).unwrap(),
        _blooms_dir: blooms_dir,
        _trace_blooms_dir: trace_blooms_dir,
        key_value: Arc::new(kvdb_memorydb::create(ethcore_db::NUM_COLUMNS.unwrap())),
    };

    Arc::new(db)
}

pub fn new_chain(genesis: encoded::Block, db: Arc<dyn BlockChainDB>) -> BlockChain {
    BlockChain::new(Config::default(), genesis.raw(), db)
}

pub fn insert_block(
    db: &Arc<dyn BlockChainDB>,
    bc: &BlockChain,
    block: encoded::Block,
    receipts: Vec<TypedReceipt>,
) -> ImportRoute {
    insert_block_commit(db, bc, block, receipts, true)
}

fn insert_block_commit(
    db: &Arc<dyn BlockChainDB>,
    bc: &BlockChain,
    block: encoded::Block,
    receipts: Vec<TypedReceipt>,
    commit: bool,
) -> ImportRoute {
    let mut batch = db.key_value().transaction();
    let res = insert_block_batch(&mut batch, bc, block, receipts);
    db.key_value().write(batch).unwrap();
    if commit {
        bc.commit();
    }
    res
}

fn insert_block_batch(
    batch: &mut DBTransaction,
    bc: &BlockChain,
    block: encoded::Block,
    receipts: Vec<TypedReceipt>,
) -> ImportRoute {
    let fork_choice = {
        let header = block.header_view();
        let parent_hash = header.parent_hash();
        let parent_details = bc
            .uncommitted_block_details(&parent_hash)
            .unwrap_or_else(|| panic!("Invalid parent hash: {:?}", parent_hash));
        let block_total_difficulty = parent_details.total_difficulty + header.difficulty();
        if block_total_difficulty > bc.best_block_total_difficulty() {
            common_types::engines::ForkChoice::New
        } else {
            common_types::engines::ForkChoice::Old
        }
    };

    bc.insert_block(
        batch,
        block,
        receipts,
        ExtrasInsert {
            fork_choice: fork_choice,
            is_finalized: false,
        },
    )
}

// impl InTransactionBlockProvider for BlockChain {
//     fn uncommitted_block_details(&self, hash: &H256) -> Option<BlockDetails> {
//         let result = self.db.key_value().read_with_two_layer_cache(
//             ethcore_db::COL_EXTRA,
//             &self.pending_block_details,
//             &self.block_details,
//             hash,
//         )?;
//         self.cache_man
//             .lock()
//             .note_used(CacheId::BlockDetails(*hash));
//         Some(result)
//     }
// }
