// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity Ethereum.

// Parity Ethereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Ethereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Ethereum.  If not, see <http://www.gnu.org/licenses/>.

//! PoA block chunker and rebuilder tests.

use std::{cell::RefCell, str::FromStr, sync::Arc};

use client::{BlockChainClient, ChainInfo, Client};
use ethkey::KeyPair;
use snapshot::tests::helpers as snapshot_helpers;
use spec::Spec;
use tempdir::TempDir;
use test_helpers::generate_dummy_client_with_spec;
use types::transaction::{Action, SignedTransaction, Transaction};

use ethereum_types::Address;
use test_helpers;

use_contract!(test_validator_set, "res/contracts/test_validator_set.json");

const TRANSITION_BLOCK_1: usize = 2; // block at which the contract becomes activated.
const TRANSITION_BLOCK_2: usize = 10; // block at which the second contract activates.

macro_rules! secret {
    ($e: expr) => {
        KeyPair::from_secret($crate::hash::keccak($e).into()).unwrap()
    };
}

lazy_static! {
    // contract addresses.
    static ref CONTRACT_ADDR_1: Address = Address::from_str("0000000000000000000000000000000000000005").unwrap();
    static ref CONTRACT_ADDR_2: Address = Address::from_str("0000000000000000000000000000000000000006").unwrap();
    // secret: `keccak(1)`, and initial validator.
    static ref RICH_SECRET: KeyPair = secret!("1");
}

/// Contract code used here: https://gist.github.com/anonymous/2a43783647e0f0dfcc359bd6fd81d6d9
/// Account with secrets keccak("1") is initially the validator.
/// Transitions to the contract at block 2, initially same validator set.
/// Create a new Spec with AuthorityRound which uses a contract at address 5 to determine the current validators using `getValidators`.
/// `test_validator_set::ValidatorSet` provides a native wrapper for the ABi.
fn spec_fixed_to_contract() -> Spec {
    let data = include_bytes!("test_validator_contract.json");
    let tempdir = TempDir::new("").unwrap();
    Spec::load(&tempdir.path(), &data[..]).unwrap()
}

// validator transition. block number and new validators. must be after `TRANSITION_BLOCK`.
// all addresses in the set must be in the account provider.
enum Transition {
    // manual transition via transaction
    Manual(usize, Vec<KeyPair>),
    // implicit transition via multi-set
    Implicit(usize, Vec<KeyPair>),
}

// create a chain with the given transitions and some blocks beyond that transition.
fn make_chain(blocks_beyond: usize, transitions: Vec<Transition>) -> Arc<Client> {
    let client = generate_dummy_client_with_spec(spec_fixed_to_contract);

    let mut cur_signers = vec![RICH_SECRET.clone()];
    {
        let engine = client.engine();
        engine.register_client(Arc::downgrade(&client) as _);
    }

    {
        // push a block with given number, signed by one of the signers, with given transactions.
        let push_block = |signers: &[ethkey::KeyPair], n, txs: Vec<SignedTransaction>| {
            use miner::{self, MinerService};

            let idx = n as usize % signers.len();
            trace!(target: "snapshot", "Pushing block #{}, {} txs, author={}",
				n, txs.len(), signers[idx]);

            client
                .miner()
                .set_author(miner::Author::Sealer(signers[idx].clone()));
            client
                .miner()
                .import_external_transactions(&*client, txs.into_iter().map(Into::into).collect());

            client.engine().step();

            assert_eq!(client.chain_info().best_block_number, n);
        };

        // execution callback for native contract: push transaction to be sealed.
        let nonce = RefCell::new(client.engine().account_start_nonce(0));

        // create useless transactions vector so we don't have to dig in
        // and force sealing.
        let make_useless_transactions = || {
            let mut nonce = nonce.borrow_mut();
            let transaction = Transaction {
                nonce: *nonce,
                gas_price: 1.into(),
                gas: 21_000.into(),
                action: Action::Call(Address::new()),
                value: 1.into(),
                data: Vec::new(),
            }
            .sign(RICH_SECRET.secret(), client.signing_chain_id());

            *nonce = *nonce + 1;
            vec![transaction]
        };

        // apply all transitions.
        for transition in transitions {
            let (num, manual, new_set) = match transition {
                Transition::Manual(num, new_set) => (num, true, new_set),
                Transition::Implicit(num, new_set) => (num, false, new_set),
            };

            if num < TRANSITION_BLOCK_1 {
                panic!("Bad test: issued epoch change before transition to contract.");
            }

            if (num as u64) < client.chain_info().best_block_number {
                panic!("Bad test: issued epoch change before previous transition finalized.");
            }

            for number in client.chain_info().best_block_number + 1..num as u64 {
                push_block(&cur_signers, number, make_useless_transactions());
            }

            let pending = if manual {
                trace!(target: "snapshot", "applying set transition at block #{}", num);
                let address = match num >= TRANSITION_BLOCK_2 {
                    true => &CONTRACT_ADDR_2 as &Address,
                    false => &CONTRACT_ADDR_1 as &Address,
                };

                let data = test_validator_set::functions::set_validators::encode_input(
                    new_set.iter().map(KeyPair::address).collect::<Vec<_>>(),
                );
                let mut nonce = nonce.borrow_mut();
                let transaction = Transaction {
                    nonce: *nonce,
                    gas_price: 0.into(),
                    gas: 1_000_000.into(),
                    action: Action::Call(*address),
                    value: 0.into(),
                    data,
                }
                .sign(RICH_SECRET.secret(), client.signing_chain_id());

                *nonce = *nonce + 1;
                vec![transaction]
            } else {
                make_useless_transactions()
            };

            // push transition block.
            push_block(&cur_signers, num as u64, pending);

            // push blocks to finalize transition
            for finalization_count in 1.. {
                if finalization_count * 2 > cur_signers.len() {
                    break;
                }
                push_block(
                    &cur_signers,
                    (num + finalization_count) as u64,
                    make_useless_transactions(),
                );
            }

            cur_signers = new_set;
        }

        // make blocks beyond.
        for number in (client.chain_info().best_block_number..).take(blocks_beyond) {
            push_block(&cur_signers, number + 1, make_useless_transactions());
        }
    }

    client
}

#[test]
fn fixed_to_contract_only() {
    let signers = vec![
        RICH_SECRET.clone(),
        secret!("foo"),
        secret!("bar"),
        secret!("test"),
        secret!("signer"),
        secret!("crypto"),
        secret!("wizard"),
        secret!("dog42"),
    ];

    let client = make_chain(
        3,
        vec![
            Transition::Manual(
                3,
                vec![
                    signers[2].clone(),
                    signers[3].clone(),
                    signers[5].clone(),
                    signers[7].clone(),
                ],
            ),
            Transition::Manual(
                6,
                vec![
                    signers[0].clone(),
                    signers[1].clone(),
                    signers[4].clone(),
                    signers[6].clone(),
                ],
            ),
        ],
    );

    // 6, 7, 8 prove finality for transition at 6.
    // 3 beyond gets us to 11.
    assert_eq!(client.chain_info().best_block_number, 11);
    let (reader, _tempdir) = snapshot_helpers::snap(&*client);

    let new_db = test_helpers::new_db();
    let spec = spec_fixed_to_contract();

    // ensure fresh engine's step matches.
    for _ in 0..11 {
        spec.engine.step()
    }
    snapshot_helpers::restore(new_db, &*spec.engine, &*reader, &spec.genesis_block()).unwrap();
}

#[test]
fn fixed_to_contract_to_contract() {
    let signers = vec![
        RICH_SECRET.clone(),
        secret!("foo"),
        secret!("bar"),
        secret!("test"),
        secret!("signer"),
        secret!("crypto"),
        secret!("wizard"),
        secret!("dog42"),
    ];

    let client = make_chain(
        3,
        vec![
            Transition::Manual(
                3,
                vec![
                    signers[2].clone(),
                    signers[3].clone(),
                    signers[5].clone(),
                    signers[7].clone(),
                ],
            ),
            Transition::Manual(
                6,
                vec![
                    signers[0].clone(),
                    signers[1].clone(),
                    signers[4].clone(),
                    signers[6].clone(),
                ],
            ),
            Transition::Implicit(10, vec![signers[0].clone()]),
            Transition::Manual(
                13,
                vec![
                    signers[2].clone(),
                    signers[4].clone(),
                    signers[6].clone(),
                    signers[7].clone(),
                ],
            ),
        ],
    );

    assert_eq!(client.chain_info().best_block_number, 16);
    let (reader, _tempdir) = snapshot_helpers::snap(&*client);
    let new_db = test_helpers::new_db();
    let spec = spec_fixed_to_contract();

    for _ in 0..16 {
        spec.engine.step()
    }
    snapshot_helpers::restore(new_db, &*spec.engine, &*reader, &spec.genesis_block()).unwrap();
}
