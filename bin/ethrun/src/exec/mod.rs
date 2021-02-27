// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

mod trace;

use std::{io, sync::Arc};

use ethcore::{
    client, error, executive,
    factory::{self, Factories},
    pod_state,
    spec::{self, Spec},
    state, state_db,
};
use ethereum_types::H256;
use ethtrie::trie;
use evm::{ActionParams, FinalizationResult, VMType};
use kvdb::KeyValueDB;
use patricia_trie_ethereum as ethtrie;

use self::trace::{Informant, NoopTracer};

/// EVM test Error.
#[derive(Debug)]
pub enum EvmTestError {
    /// Trie integrity error.
    Trie(Box<ethtrie::TrieError>),
    /// EVM error.
    Evm(vm::Error),
    /// Initialization error.
    ClientError(error::Error),
    /// Post-condition failure,
    PostCondition(String),
}

pub struct EvmTestClient<'a> {
    state: ethcore::state::State<state_db::StateDB>,
    spec: &'a spec::Spec,
    dump_state: fn(&ethcore::state::State<state_db::StateDB>) -> Option<pod_state::PodState>,
    informant: Informant,
    tracer: NoopTracer
}

pub fn new_evm<'a>(spec: &'a Spec) -> Result<EvmTestClient<'a>, EvmTestError> {
    EvmTestClient::new(spec)
}

impl<'a> EvmTestClient<'a> {
    fn state_from_spec(
        spec: &'a spec::Spec,
        factories: &Factories,
    ) -> Result<state::State<state_db::StateDB>, EvmTestError> {
        let db = Arc::new(kvdb_memorydb::create(7));
        let journal_db = journaldb::new(db.clone(), journaldb::Algorithm::EarlyMerge, Some(0));
        let mut state_db = state_db::StateDB::new(journal_db, 5 * 1024 * 1024);
        state_db = spec.ensure_db_good(state_db, factories).unwrap();

        let genesis = spec.genesis_header();
        // Write DB
        {
            let mut batch = kvdb::DBTransaction::new();
            state_db
                .journal_under(&mut batch, 0, &genesis.hash())
                .unwrap();
            db.write(batch).unwrap();
        }

        state::State::from_existing(
            state_db,
            *genesis.state_root(),
            spec.engine.account_start_nonce(0),
            factories.clone(),
        )
        .map_err(EvmTestError::Trie)
    }

    pub fn new_with_trie(
        spec: &'a spec::Spec,
        trie_spec: trie::TrieSpec,
    ) -> Result<Self, EvmTestError> {
        let factories = Self::factories(trie_spec);
        let state = Self::state_from_spec(spec, &factories)?;

        Ok(EvmTestClient {
            state,
            spec,
            dump_state: |s: &state::State<state_db::StateDB>| {
                None // TODO, continue investigating here.
            },
            informant: Informant::default(),
            tracer: NoopTracer
        })
    }

    pub fn call(&mut self, params: ActionParams) -> Result<FinalizationResult, EvmTestError> {
        let genesis = self.spec.genesis_header();
        let info = client::EnvInfo {
            number: genesis.number(),
            author: *genesis.author(),
            timestamp: genesis.timestamp(),
            difficulty: *genesis.difficulty(),
            last_hashes: Arc::new([H256::default(); 256].to_vec()),
            gas_used: 0.into(),
            gas_limit: *genesis.gas_limit(),
        };

        let mut tracer = NoopTracer;
        let mut informant = Informant::default();

        self.call_envinfo(params, &mut informant, &mut tracer, info)
    }

    /// Execute the VM given envinfo, ActionParams and tracer.
    /// Returns amount of gas left and the output.
    pub fn call_envinfo<T: ethcore::trace::Tracer, V: ethcore::trace::VMTracer>(
        &mut self,
        params: ActionParams,
        tracer: &mut T,
        vm_tracer: &mut V,
        info: client::EnvInfo,
    ) -> Result<FinalizationResult, EvmTestError> {
        let mut substate = state::Substate::new();
        let machine = self.spec.engine.machine();
        let schedule = machine.schedule(info.number);
        let mut executive = executive::Executive::new(&mut self.state, &info, &machine, &schedule);
        executive
            .call(params, &mut substate, tracer, vm_tracer)
            .map_err(EvmTestError::Evm)
    }

    /// Creates new EVM test client with an in-memory DB initialized with genesis of given chain Spec.
    pub fn new(spec: &'a spec::Spec) -> Result<Self, EvmTestError> {
        Self::new_with_trie(spec, trie::TrieSpec::Secure)
    }

    fn factories(trie_spec: trie::TrieSpec) -> Factories {
        Factories {
            vm: factory::VmFactory::new(VMType::Interpreter, 5 * 1024),
            trie: trie::TrieFactory::new(trie_spec),
            accountdb: Default::default(),
        }
    }
}

// #[cfg(test)]
// mod tests {

//     use ethcore::{client::{Executive, test_client::get_temp_state_db}, test_helpers::get_temp_state_with_factory};
//     use evm::{ActionParams, EnvInfo, Factory, Schedule, VMType};

//     #[test]
//     fn test_vm_creation() {
//         let action = ActionParams::default();
//         let schedule = Schedule::new_berlin();
//         let vm = Factory::new(VMType::Interpreter, 1024 * 32);
//         let journaldb = get_temp_state_db();
//         let env = EnvInfo::default();
//         let mut exec = Executive::new(&mut state, env, machine, &schedule);
//         //let exec = vm.create(action, &schedule, 1024);
//     }
// }
