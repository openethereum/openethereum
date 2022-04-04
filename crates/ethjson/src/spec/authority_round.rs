// Copyright 2015-2020 Parity Technologies (UK) Ltd.
// This file is part of OpenEthereum.

// OpenEthereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// OpenEthereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with OpenEthereum.  If not, see <http://www.gnu.org/licenses/>.

//! Authority Round parameter deserialization.
//!
//! Here is an example of input parameters where the step duration is constant at 5 seconds, the set
//! of validators is decided by the contract at address `0x10..01` starting from block 0, and where
//! the address of the contract that computes block rewards is set to `0x20..02` for blocks 0
//! through 41 and to `0x30.03` for all blocks starting from block 42.
//!
//! ```ignore
//! "params": {
//!     "stepDuration": "5",
//!     "validators": {
//!         "multi": {
//!             "0": {
//!                 "contract": "0x1000000000000000000000000000000000000001"
//!             }
//!         }
//!     },
//!     "blockRewardContractTransitions": {
//!         "0": "0x2000000000000000000000000000000000000002",
//!         "42": "0x3000000000000000000000000000000000000003"
//!     }
//! }
//! ```

use super::{BlockReward, StepDuration, ValidatorSet};
use crate::{bytes::Bytes, hash::Address, uint::Uint};
use std::collections::BTreeMap;

/// Authority params deserialization.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AuthorityRoundParams {
    /// Block duration, in seconds.
    pub step_duration: StepDuration,
    /// Valid authorities
    pub validators: ValidatorSet,
    /// Starting step. Determined automatically if not specified.
    /// To be used for testing only.
    pub start_step: Option<Uint>,
    /// Block at which score validation should start.
    pub validate_score_transition: Option<Uint>,
    /// Block from which monotonic steps start.
    pub validate_step_transition: Option<Uint>,
    /// Whether transitions should be immediate.
    pub immediate_transitions: Option<bool>,
    /// Reward per block in wei.
    pub block_reward: Option<BlockReward>,
    /// Block at which the block reward contract should start being used. This option allows one to
    /// add a single block reward contract transition and is compatible with the multiple address
    /// option `block_reward_contract_transitions` below.
    pub block_reward_contract_transition: Option<Uint>,
    /// Block reward contract address which overrides the `block_reward` setting. This option allows
    /// one to add a single block reward contract address and is compatible with the multiple
    /// address option `block_reward_contract_transitions` below.
    pub block_reward_contract_address: Option<Address>,
    /// Block reward contract addresses with their associated starting block numbers.
    ///
    /// Setting the block reward contract overrides `block_reward`. If the single block reward
    /// contract address is also present then it is added into the map at the block number stored in
    /// `block_reward_contract_transition` or 0 if that block number is not provided. Therefore both
    /// a single block reward contract transition and a map of reward contract transitions can be
    /// used simulataneously in the same configuration. In such a case the code requires that the
    /// block number of the single transition is strictly less than any of the block numbers in the
    /// map.
    pub block_reward_contract_transitions: Option<BTreeMap<Uint, Address>>,
    /// Block reward code. This overrides the block reward contract address.
    pub block_reward_contract_code: Option<Bytes>,
    /// Block at which maximum uncle count should be considered.
    pub maximum_uncle_count_transition: Option<Uint>,
    /// Maximum number of accepted uncles.
    pub maximum_uncle_count: Option<Uint>,
    /// Block at which empty step messages should start.
    pub empty_steps_transition: Option<Uint>,
    /// Maximum number of accepted empty steps.
    pub maximum_empty_steps: Option<Uint>,
    /// Strict validation of empty steps transition block.
    pub strict_empty_steps_transition: Option<Uint>,
    /// First block for which a 2/3 quorum (instead of 1/2) is required.
    pub two_thirds_majority_transition: Option<Uint>,
    /// The random number contract's address, or a map of contract transitions.
    pub randomness_contract_address: Option<BTreeMap<Uint, Address>>,
    /// The addresses of contracts that determine the block gas limit starting from the block number
    /// associated with each of those contracts.
    pub block_gas_limit_contract_transitions: Option<BTreeMap<Uint, Address>>,
    /// The block number at which the consensus engine switches from AuRa to AuRa with POSDAO
    /// modifications.
    pub posdao_transition: Option<Uint>,
    /// The block numbers at which the bytecodes should be rewritten for
    /// the specified contracts (can be more than one per block)
    #[serde(rename = "rewriteBytecode")]
    pub rewrite_bytecode_transitions: Option<BTreeMap<Uint, BTreeMap<Address, Bytes>>>,
}

/// Authority engine deserialization.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityRound {
    /// Authority Round parameters.
    pub params: AuthorityRoundParams,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::BlockReward;
    use crate::{
        bytes::Bytes,
        hash::Address,
        spec::{
            authority_round::AuthorityRound, step_duration::StepDuration,
            validator_set::ValidatorSet,
        },
        uint::Uint,
    };
    use ethereum_types::{H160, U256};
    use serde_json;
    use std::str::FromStr;

    #[test]
    fn authority_round_deserialization() {
        let s = r#"{
			"params": {
				"stepDuration": "0x02",
				"validators": {
					"list" : ["0xc6d9d2cd449a754c494264e1809c50e34d64562b"]
				},
				"startStep" : 24,
				"validateStepTransition": 150,
				"blockReward": 5000000,
				"maximumUncleCountTransition": 10000000,
				"maximumUncleCount": 5,
				"randomnessContractAddress": {
					"10": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
					"20": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                },
				"blockGasLimitContractTransitions": {
					"10": "0x1000000000000000000000000000000000000001",
                    "20": "0x2000000000000000000000000000000000000002"
                }
			}
		}"#;

        let deserialized: AuthorityRound = serde_json::from_str(s).unwrap();
        assert_eq!(
            deserialized.params.step_duration,
            StepDuration::Single(Uint(U256::from(2)))
        );
        assert_eq!(
            deserialized.params.validators,
            ValidatorSet::List(vec![Address(
                H160::from_str("c6d9d2cd449a754c494264e1809c50e34d64562b").unwrap()
            )])
        );
        assert_eq!(deserialized.params.start_step, Some(Uint(U256::from(24))));
        assert_eq!(deserialized.params.immediate_transitions, None);
        assert_eq!(
            deserialized.params.maximum_uncle_count_transition,
            Some(Uint(10_000_000.into()))
        );
        assert_eq!(
            deserialized.params.maximum_uncle_count,
            Some(Uint(5.into()))
        );
        assert_eq!(
            deserialized.params.randomness_contract_address.unwrap(),
            vec![
                (
                    Uint(10.into()),
                    Address(H160::from_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap())
                ),
                (
                    Uint(20.into()),
                    Address(H160::from_str("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap())
                ),
            ]
            .into_iter()
            .collect()
        );
        let expected_bglc = [
            (
                Uint(10.into()),
                Address(H160::from_str("1000000000000000000000000000000000000001").unwrap()),
            ),
            (
                Uint(20.into()),
                Address(H160::from_str("2000000000000000000000000000000000000002").unwrap()),
            ),
        ];
        assert_eq!(
            deserialized.params.block_gas_limit_contract_transitions,
            Some(expected_bglc.to_vec().into_iter().collect())
        );
    }

    #[test]
    fn authority_round_deserialization_multi_block() {
        let s = r#"{
			"params": {
				"stepDuration": "0x02",
				"validators": {
					"contract" : "0xc6d9d2cd449a754c494264e1809c50e34d64562b"
				},
				"blockReward": {
                    "0": 5000000,
                    "100": 150
                }
			}
		}"#;

        let deserialized: AuthorityRound = serde_json::from_str(s).unwrap();
        assert_eq!(
            deserialized.params.step_duration,
            StepDuration::Single(Uint(U256::from(0x02)))
        );
        assert_eq!(
            deserialized.params.validators,
            ValidatorSet::Contract(Address(
                H160::from_str("c6d9d2cd449a754c494264e1809c50e34d64562b").unwrap()
            ))
        );
        let mut rewards: BTreeMap<Uint, Uint> = BTreeMap::new();
        rewards.insert(Uint(U256::from(0)), Uint(U256::from(5000000)));
        rewards.insert(Uint(U256::from(100)), Uint(U256::from(150)));
        assert_eq!(
            deserialized.params.block_reward,
            Some(BlockReward::Multi(rewards))
        );
    }

    #[test]
    fn authority_round_deserialization_rewrite_bytecode_transitions() {
        let s = r#"{
			"params": {
				"stepDuration": "0x02",
				"validators": {
					"contract" : "0xc6d9d2cd449a754c494264e1809c50e34d64562b"
				},
				"blockReward": {
                    "0": 5000000,
                    "100": 150
                },
                "rewriteBytecode": {
                  "21300000": {
                    "0x1234000000000000000000000000000000000001": "0x111",
                    "0x1234000000000000000000000000000000000002": "0x222"
                  },
                  "21300001": {
                    "0x1234000000000000000000000000000000000003": "0x333",
                    "0x1234000000000000000000000000000000000004": "0x444"
                  }
                }
			}
		}"#;
        let deserialized: AuthorityRound = serde_json::from_str(s).unwrap();
        assert_eq!(
            deserialized.params.step_duration,
            StepDuration::Single(Uint(U256::from(0x02)))
        );
        assert_eq!(
            deserialized.params.validators,
            ValidatorSet::Contract(Address(
                H160::from_str("c6d9d2cd449a754c494264e1809c50e34d64562b").unwrap()
            ))
        );

        let mut rewrite_bytecode1: BTreeMap<Address, Bytes> = BTreeMap::new();
        rewrite_bytecode1.insert(
            Address(H160::from_str("1234000000000000000000000000000000000001").unwrap()),
            Bytes::from_str("0x111").unwrap(),
        );
        rewrite_bytecode1.insert(
            Address(H160::from_str("1234000000000000000000000000000000000002").unwrap()),
            Bytes::from_str("0x222").unwrap(),
        );
        let mut rewrite_bytecode2: BTreeMap<Address, Bytes> = BTreeMap::new();
        rewrite_bytecode2.insert(
            Address(H160::from_str("1234000000000000000000000000000000000003").unwrap()),
            Bytes::from_str("0x333").unwrap(),
        );
        rewrite_bytecode2.insert(
            Address(H160::from_str("1234000000000000000000000000000000000004").unwrap()),
            Bytes::from_str("0x444").unwrap(),
        );

        let mut rewrite_bytecode_transitions: BTreeMap<Uint, BTreeMap<Address, Bytes>> =
            BTreeMap::new();
        rewrite_bytecode_transitions.insert(Uint(21300000.into()), rewrite_bytecode1);
        rewrite_bytecode_transitions.insert(Uint(21300001.into()), rewrite_bytecode2);
        assert_eq!(
            deserialized.params.rewrite_bytecode_transitions,
            Some(rewrite_bytecode_transitions)
        );
    }
}
