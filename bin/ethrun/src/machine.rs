// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use crate::{
    backend,
    db::{insert_block, new_chain},
};
use common_types::encoded::Block;
use ethcore::{machine::EthereumMachine, spec::CommonParams};
use ethcore_blockchain::{BlockChain, BlockChainDB};
use ethcore_builtin::Builtin;
use ethereum_types::{Address, H256, U256};
use ethjson::spec::Spec as JsonSpec;
use std::{collections::HashMap, convert::TryFrom, error::Error, sync::Arc};
use vm::Ext;

pub struct SmallMachine {
    storage: HashMap<H256, H256>,
    blockhashes: HashMap<U256, H256>,
    balances: HashMap<Address, U256>,
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
            storage: HashMap::new(),
            balances: HashMap::new(),
            blockhashes: HashMap::new(),
            database: database.clone(),
            blockchain: new_chain(genesis, database.clone()),
            _machine: machine,
        })
    }

    pub fn consume_block(&mut self, block: Block) -> Result<Block, Box<dyn Error>> {
        insert_block(&self.database, &self.blockchain, block.clone(), vec![]);
        Ok(block)
    }
}
