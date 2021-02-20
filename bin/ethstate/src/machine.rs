// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use std::{collections::BTreeMap, convert::TryFrom, error::Error, sync::Arc};

use ethcore::{
    factory::Factories, machine::EthereumMachine, spec::CommonParams, state::State,
    state_db::StateDB,
};
use TypedTransaction::Legacy;

use ethcore_blockchain::{BlockChain, BlockChainDB, BlockProvider};
use ethjson::{
    hash::Address,
    spec::{Account, Spec as JsonSpec},
};

use ethcore_builtin::Builtin;
use ethereum_types::{H256, U256};
use journaldb::new_memory_db;
use keccak_hasher::KeccakHasher;
use kvdb::DBValue;
use memory_db::MemoryDB;

// use trie_db::TrieMut;
// use patricia_trie_ethereum::TrieDBMut;

use common_types::{
    encoded::Block,
    transaction::{Action, Transaction, TypedTransaction},
};

use crate::db::{insert_block, new_chain, new_db};

struct StateTree {
    pub root_hash: H256,
    pub root_node: MemoryDB<KeccakHasher, DBValue>,
}

impl StateTree {
    fn new() -> Self {
        StateTree {
            root_hash: H256::new(),
            root_node: new_memory_db(),
        }
    }
}

pub struct SmallMachine {
    db: Arc<dyn BlockChainDB>,
    state: State<StateDB>,
    blockchain: BlockChain,
    machine: EthereumMachine,
}

fn create_statedb(
    machine: &EthereumMachine,
    accounts: BTreeMap<Address, Account>,
) -> Result<State<StateDB>, Box<dyn Error>> {
    let number_of_columns = 7;
    let column_for_state = Some(0);
    let kvdb = kvdb_memorydb::create(number_of_columns);
    let journaldb = journaldb::new(
        Arc::new(kvdb),
        journaldb::Algorithm::Archive,
        column_for_state,
    );

    let mut state = State::new(
        StateDB::new(journaldb, 1024 * 1024),
        machine.account_start_nonce(0),
        Factories::default(),
    );

    for (address, account) in accounts {
        state.add_balance(
            &address.0,
            &match account.balance {
                Some(acctstate) => acctstate.0,
                None => U256::from(0),
            },
            ethcore::state::CleanupMode::ForceCreate,
        )?;
    }

    Ok(state)
}

impl SmallMachine {
    pub fn new(spec: JsonSpec, genesis: Block) -> Self {
        let machine = EthereumMachine::regular(
            CommonParams::from(spec.params),
            spec.accounts
                .builtins()
                .into_iter()
                .map(|p| {
                    (
                        p.0.into(),
                        Builtin::try_from(p.1).expect("invalid chainspec"),
                    )
                })
                .collect(),
        );

        let db = new_db();
        SmallMachine {
            db: db.clone(),
            state: create_statedb(&machine, spec.accounts.into_iter().collect()).unwrap(),
            blockchain: new_chain(genesis, db.clone()),
            machine: machine,
        }
    }

    pub fn consume_block(&mut self, block: Block) -> Result<Option<Block>, Box<dyn Error>> {
        insert_block(&self.db, &self.blockchain, block, vec![]);
        let header = self.blockchain.best_block_header();
        let block = self.blockchain.block(&header.hash()).unwrap();
        match wasm_contracts(&block).len() {
            0 => Ok(None),
            _ => Ok(Some(block)),
        }
    }
}

fn wasm_contracts(block: &Block) -> Vec<Transaction> {
    block
        .transactions()
        .iter()
        .filter(|&t| match t.as_unsigned() {
            Legacy(tx) => match (&tx.action, tx.data.starts_with(b"\0asm")) {
                (Action::Create, true) => true,
                _ => false,
            },
            TypedTransaction::AccessList(_) => false,
        })
        .map(|ut| -> Transaction {
            if let Legacy(tx) = ut.as_unsigned() {
                tx.clone()
            } else {
                unreachable!()
            }
        })
        .collect()
}
