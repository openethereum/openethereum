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

#![warn(missing_docs)]

//! Miner module
//! Keeps track of transactions and currently sealed pending block.

mod miner;

pub mod pool_client;
#[cfg(feature = "stratum")]
pub mod stratum;

use crate::{block::{BlockInternals}};

pub use self::miner::{Author, AuthoringParams, Miner, MinerOptions, Penalization, PendingSet};
pub use ethcore_miner::{
    local_accounts::LocalAccounts,
    pool::{transaction_filter::TransactionFilter, PendingOrdering},
};

use std::{collections::{BTreeMap}, sync::Arc};

use bytes::Bytes;
use ethcore_miner::pool::{QueueStatus, VerifiedTransaction, local_transactions};
use ethereum_types::{Address, H256, U256};
use types::{
    transaction::{self, PendingTransaction, UnverifiedTransaction},
    BlockNumber,
};

use block::SealedBlock;
use call_contract::{CallContract, RegistryInfo};
use client::{
    traits::ForceUpdateSealing, AccountData, BlockChain, BlockProducer, ChainInfo, Nonce,
    ScheduleInfo, SealedBlockImporter,
};
use error::Error;
use state::StateInfo;

/// Provides methods to verify incoming external transactions
pub trait TransactionVerifierClient: Send + Sync
	// Required for ServiceTransactionChecker
	+ CallContract + RegistryInfo
	// Required for verifiying transactions
	+ BlockChain + ScheduleInfo + AccountData
{}

/// Extended client interface used for mining
pub trait BlockChainClient:
    TransactionVerifierClient + BlockProducer + SealedBlockImporter
{
}

/// Used by external actors. JSONRPC or Stratus.
pub trait MinerRPC: Send + Sync {
    /// Is it currently sealing?. used only by RPC
    fn is_currently_sealing(&self) -> bool;

    /// Get the sealing work package preparing it if doesn't exist yet.
    ///
    /// Returns `None` if engine seals internally.
    /// Used by stratus and RPC
    fn work_package(&self) -> Option<(H256, BlockNumber, u64, U256)>;

    /// Submit `seal` as a valid solution for the header of `pow_hash`.
    /// Will check the seal, but not actually insert the block into the chain.
    /// used by stratus and RPC.
    fn submit_seal(&self, pow_hash: H256, seal: Vec<Bytes>) -> Result<SealedBlock, Error>;

    /// Set the lower and upper bound of gas limit we wish to target when sealing a new block.
    /// used in bin/oe and in parity_set RPC
    fn set_gas_range_target(&self, gas_range_target: (U256, U256));

    /// Set the extra_data that we will seal blocks with.
    /// used in bin/oe and in parity_set RPC
    fn set_extra_data(&self, extra_data: Bytes);

    /// Set info necessary to sign consensus messages and block authoring.
    /// On chains where sealing is done externally (e.g. PoW) we provide only reward beneficiary.
    /// used in bin.oe and in parity_set RPC and tests
    fn set_author<T: Into<Option<Author>>>(&self, author: T);

    /// Get current queue status.
    ///
    /// Status includes verification thresholds and current pool utilization and limits.
    /// used in parity RPC to get transactions_limit and min_gas_price
    fn queue_status(&self) -> QueueStatus;
}

/// Miner client API
pub trait MinerService:
    Send + Sync + MinerPendingBlock + MinerTxpool + MinerRPC + MinerPendingState
{
    // Sealing

    /// Update current pending block. Used By AuRa, implemented in Clients super::traits::EngineClient
    fn update_sealing(&self, force: ForceUpdateSealing);

    /// Called when blocks are imported to chain, updates transactions queue.
    /// `is_internal_import` indicates that the block has just been created in miner and internally sealed by the engine,
    /// so we shouldn't attempt creating new block again.
    /// Additionally it is used by tests in few places.
    fn chain_new_blocks(
        &self,
        imported: &[H256],
        invalid: &[H256],
        enacted: &[H256],
        retracted: &[H256],
        is_internal_import: bool,
    );

    // Block authoring

    /// Get current authoring parameters. Used by AuRa and RPC
    fn authoring_params(&self) -> AuthoringParams;

    // Misc

    /// Suggested gas price. Used by AuRa in transact_contract and Rpc.
    fn sensible_gas_price(&self) -> U256;

    /// Suggested gas limit. Used by AuRa in transact_contract and RPC
    fn sensible_gas_limit(&self) -> U256;

    /// Set a new minimum gas limit.
    /// Will not work if dynamic gas calibration is set.
    fn set_minimal_gas_price(&self, gas_price: U256) -> Result<bool, &str>;
}

/// Step to unification of miner and client
pub trait MinerPoolClient: Nonce + Sync + ChainInfo + BlockChainClient
{}

/// Functions related to transactions
pub trait MinerTxpool: Send + Sync {

    /// Removes transaction from the pool.
    ///
    /// Attempts to "cancel" a transaction. If it was not propagated yet (or not accepted by other peers)
    /// there is a good chance that the tr  ansaction will actually be removed.
    /// NOTE: The transaction is not removed from pending block if there is one.
    fn remove_transaction(&self, hash: &H256) -> Option<Arc<VerifiedTransaction>>;

    /// Query transaction from the pool given it's hash.
    fn transaction(&self, hash: &H256) -> Option<Arc<VerifiedTransaction>>;

    /// Get a list of all transactions in the pool (some of them might not be ready for inclusion yet).
    fn queued_transactions(&self) -> Vec<Arc<VerifiedTransaction>>;

    /// Get a list of all transaction hashes in the pool (some of them might not be ready for inclusion yet).
    fn queued_transaction_hashes(&self) -> Vec<H256>;

    /// Get a list of local transactions with statuses.
    fn local_transactions(&self) -> BTreeMap<H256, local_transactions::Status>;

    /// Returns next valid nonce for given address.
    ///
    /// This includes nonces of all transactions from this address in the pending queue
    /// if they are consecutive.
    /// NOTE: pool may contain some future transactions that will become pending after
    /// transaction with nonce returned from this function is signed on.
    fn next_nonce(&self, address: &Address) -> U256;
    /// Get a list of all ready transactions either ordered by priority or unordered (cheaper),
    /// and optionally filtered by sender, recipient, gas, gas price, value and/or nonce.
    ///
    /// Depending on the settings may look in transaction pool or only in pending block.
    /// If you don't need a full set of transactions, you can add `max_len` and create only a limited set of
    /// transactions.
    fn ready_transactions_filtered(
        &self,
        max_len: usize,
        filter: Option<TransactionFilter>,
        ordering: PendingOrdering,
    ) -> Vec<Arc<VerifiedTransaction>>;

    /// Get an unfiltered list of all ready transactions.
    fn ready_transactions(
        &self,
        max_len: usize,
        ordering: PendingOrdering,
    ) -> Vec<Arc<VerifiedTransaction>>
    {
        self.ready_transactions_filtered(max_len, None, ordering)
    }

    // Transaction Pool

    /// Imports transactions to transaction queue.
    fn import_external_transactions(
        &self,
        transactions: Vec<UnverifiedTransaction>,
    ) -> Vec<Result<(), transaction::Error>>;

    /// Imports own (node owner) transaction to queue.
    fn import_own_transaction(
        &self,
        transaction: PendingTransaction,
    ) -> Result<(), transaction::Error>;

    /// Imports transactions from potentially external sources, with behaviour determined
    /// by the config flag `tx_queue_allow_unfamiliar_locals`
    fn import_claimed_local_transaction(
        &self,
        transaction: PendingTransaction,
        trusted: bool,
    ) -> Result<(), transaction::Error>;
}

/// It is saparated because it is used for testing and it is needed to specify type of State.
pub trait MinerPendingState: Send + Sync {
    /// State Trait it can be for tests
    type State: StateInfo + 'static;

    /// Get `Some` `clone()` of the current pending block's state or `None` if we're not sealing.
    fn pending_state(&self, latest_block_number: BlockNumber) -> Option<Self::State>;
}

/// Functions related to pending block
/// Make it into one function. pending(block, block_number, fn-> T) -> T
pub trait MinerPendingBlock: Send + Sync {
    /// Get pending block information.
    fn pending<F, T>(&self, latest_block: BlockNumber, f: F) -> Option<T>
    where
        F: FnOnce(&dyn BlockInternals) -> T;
}
