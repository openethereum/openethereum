// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use std::sync::Arc;

use ethcore::{error, ethereum, factory::{self, Factories}, pod_state, spec::{self, Spec}, state, state_db};
use ethjson::spec::ForkSpec;
use ethtrie::trie;
use evm::VMType;
use kvdb::KeyValueDB;
use patricia_trie_ethereum as ethtrie;

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
}

pub fn new<'a>(spec: &'a Spec) -> Result<EvmTestClient<'a>, EvmTestError> {
    EvmTestClient::new(spec)
}

impl<'a> EvmTestClient<'a> {
    /// Converts a json spec definition into spec.
    pub fn spec_from_json(spec: &ForkSpec) -> Option<spec::Spec> {
        match *spec {
            ForkSpec::Frontier => Some(ethereum::new_frontier_test()),
            ForkSpec::Homestead => Some(ethereum::new_homestead_test()),
            ForkSpec::EIP150 => Some(ethereum::new_eip150_test()),
            ForkSpec::EIP158 => Some(ethereum::new_eip161_test()),
            ForkSpec::Byzantium => Some(ethereum::new_byzantium_test()),
            ForkSpec::Constantinople => Some(ethereum::new_constantinople_test()),
            ForkSpec::ConstantinopleFix => Some(ethereum::new_constantinople_fix_test()),
            ForkSpec::Istanbul => Some(ethereum::new_istanbul_test()),
            ForkSpec::EIP158ToByzantiumAt5 => Some(ethereum::new_transition_test()),
            ForkSpec::ByzantiumToConstantinopleFixAt5 => {
                Some(ethereum::new_byzantium_to_constantinoplefixat5_test())
            }
            ForkSpec::Berlin => Some(ethereum::new_berlin_test()),
            ForkSpec::FrontierToHomesteadAt5
            | ForkSpec::HomesteadToDaoAt5
            | ForkSpec::HomesteadToEIP150At5
            | ForkSpec::ByzantiumToConstantinopleAt5 => None,
        }
    }

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
            state_db.journal_under(&mut batch, 0, &genesis.hash()).unwrap();
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
        })
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
