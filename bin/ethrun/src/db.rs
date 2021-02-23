// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use kvdb::DBTransaction;
use std::sync::Arc;

use common_types::{encoded, receipt::TypedReceipt};
use ethcore_blockchain::{
    BlockChain, BlockChainDB, Config, ExtrasInsert, ImportRoute, InTransactionBlockProvider,
};

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
