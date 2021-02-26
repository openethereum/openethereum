// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

mod nibble;
mod trie;

use ethereum_types::{H256, U256};
use keccak_hasher::KeccakHasher;
use memory_db::MemoryDB;
use patricia_trie_ethereum::trie::TrieDB;

/// Public Storage API
pub use trie::MerklePatriciaTree;

pub struct WorldState {
    storage: MemoryDB<KeccakHasher, Vec<u8>>,
    //accounts: TrieDB<>
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct Account {
    nonce: U256,
    balance: U256,
    code_hash: H256,
    storage_root: H256,
}

impl Account {
    pub fn nonce(&self) -> U256 {
        self.nonce
    }
    pub fn balance(&self) -> U256 {
        self.balance
    }
}
