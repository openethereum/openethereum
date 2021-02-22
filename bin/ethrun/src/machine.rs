// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use std::{convert::TryFrom, error::Error, sync::Arc};

use ethcore::{machine::EthereumMachine, spec::CommonParams};
use TypedTransaction::Legacy;

use ethcore_blockchain::{BlockChain, BlockChainDB, BlockProvider};
use ethjson::spec::Spec as JsonSpec;

use ethcore_builtin::Builtin;

use common_types::{
    encoded::Block,
    transaction::{Action, Transaction, TypedTransaction, UnverifiedTransaction},
};

use crate::{
    backend,
    db::{insert_block, new_chain},
};

pub struct SmallMachine {
    database: Arc<dyn BlockChainDB>,
    blockchain: BlockChain,
    _machine: EthereumMachine,
}

impl SmallMachine {
    pub fn new(spec: JsonSpec, genesis: Block) -> Result<Self, Box<dyn Error>> {
        let database = Arc::new(backend::LiteBackend::new(&spec, &genesis)?);
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

        Ok(SmallMachine {
            database: database.clone(),
            blockchain: new_chain(genesis, database.clone()),
            _machine: machine,
        })
    }

    pub fn consume_block(&mut self, block: Block) -> Result<Option<Block>, Box<dyn Error>> {
        insert_block(&self.database, &self.blockchain, block, vec![]);
        let header = self.blockchain.best_block_header();
        let block = self.blockchain.block(&header.hash()).unwrap();
        match wasm_contracts(&block).len() {
            0 => Ok(None),
            _ => Ok(Some(block)),
        }
    }
}

pub fn is_wasm_creation_transaction<'a>(tx: &'a &UnverifiedTransaction) -> bool {
    match tx.as_unsigned() {
        Legacy(tx) => match (&tx.action, tx.data.starts_with(b"\0asm")) {
            (Action::Create, true) => true,
            _ => false,
        },
        TypedTransaction::AccessList(_) => false,
    }
}

fn wasm_contracts(block: &Block) -> Vec<Transaction> {
    block
        .transactions()
        .iter()
        .filter(is_wasm_creation_transaction)
        .map(|ut| match ut.as_unsigned() {
            Legacy(tx) => tx.clone(),
            _ => unreachable!(),
        })
        .collect()
}
