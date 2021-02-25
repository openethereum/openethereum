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

impl Ext for SmallMachine {
    fn initial_storage_at(&self, key: &H256) -> vm::Result<H256> {
        Ok(H256::new())
    }

    fn storage_at(&self, key: &H256) -> vm::Result<H256> {
        Ok(self.storage.get(key).unwrap_or(&H256::new()).clone())
    }

    fn set_storage(&mut self, key: H256, value: H256) -> vm::Result<()> {
        self.storage.insert(key, value);
        Ok(())
    }

    fn exists(&self, address: &ethereum_types::Address) -> vm::Result<bool> {
        Ok(self.balances.contains_key(address))
    }

    fn exists_and_not_null(&self, address: &ethereum_types::Address) -> vm::Result<bool> {
        Ok(self.balances.get(address).map_or(false, |b| !b.is_zero()))
    }

    fn origin_balance(&self) -> vm::Result<ethereum_types::U256> {
        unimplemented!();
    }

    fn balance(&self, address: &ethereum_types::Address) -> vm::Result<ethereum_types::U256> {
        Ok(self.balances.get(address).cloned().unwrap_or(U256::zero()))
    }

    fn blockhash(&mut self, number: &ethereum_types::U256) -> H256 {
        self.blockhashes.get(number).unwrap_or(&H256::new()).clone()
    }

    fn create(
        &mut self,
        gas: &ethereum_types::U256,
        value: &ethereum_types::U256,
        code: &[u8],
        address: ethcore::CreateContractAddress,
        trap: bool,
    ) -> Result<evm::ContractCreateResult, vm::TrapKind> {
        todo!()
    }

    fn calc_address(
        &self,
        code: &[u8],
        address: ethcore::CreateContractAddress,
    ) -> Option<ethereum_types::Address> {
        todo!()
    }

    fn call(
        &mut self,
        gas: &ethereum_types::U256,
        sender_address: &ethereum_types::Address,
        receive_address: &ethereum_types::Address,
        value: Option<ethereum_types::U256>,
        data: &[u8],
        code_address: &ethereum_types::Address,
        call_type: evm::CallType,
        trap: bool,
    ) -> Result<evm::MessageCallResult, vm::TrapKind> {
        todo!()
    }

    fn extcode(&self, address: &ethereum_types::Address) -> vm::Result<Option<Arc<Vec<u8>>>> {
        todo!()
    }

    fn extcodehash(&self, address: &ethereum_types::Address) -> vm::Result<Option<H256>> {
        todo!()
    }

    fn extcodesize(&self, address: &ethereum_types::Address) -> vm::Result<Option<usize>> {
        todo!()
    }

    fn log(&mut self, topics: Vec<H256>, data: &[u8]) -> vm::Result<()> {
        todo!()
    }

    fn ret(
        self,
        gas: &ethereum_types::U256,
        data: &evm::ReturnData,
        apply_state: bool,
    ) -> vm::Result<ethereum_types::U256> {
        todo!()
    }

    fn suicide(&mut self, refund_address: &ethereum_types::Address) -> vm::Result<()> {
        todo!()
    }

    fn schedule(&self) -> &evm::Schedule {
        todo!()
    }

    fn env_info(&self) -> &evm::EnvInfo {
        todo!()
    }

    fn chain_id(&self) -> u64 {
        todo!()
    }

    fn depth(&self) -> usize {
        todo!()
    }

    fn add_sstore_refund(&mut self, value: usize) {
        todo!()
    }

    fn sub_sstore_refund(&mut self, value: usize) {
        todo!()
    }

    fn is_static(&self) -> bool {
        todo!()
    }

    fn al_is_enabled(&self) -> bool {
        todo!()
    }

    fn al_contains_storage_key(&self, address: &ethereum_types::Address, key: &H256) -> bool {
        todo!()
    }

    fn al_insert_storage_key(&mut self, address: ethereum_types::Address, key: H256) {
        todo!()
    }

    fn al_contains_address(&self, address: &ethereum_types::Address) -> bool {
        todo!()
    }

    fn al_insert_address(&mut self, address: ethereum_types::Address) {
        todo!()
    }
}
