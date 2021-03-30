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

//! Traits implemented by client.

use std::{collections::BTreeMap, sync::Arc};

use blockchain::{BlockReceipts, TreeRoute};
use bytes::Bytes;
use call_contract::{CallContract, RegistryInfo};
use ethcore_miner::pool::VerifiedTransaction;
use ethereum_types::{Address, H256, U256};
use evm::Schedule;
use itertools::Itertools;
use kvdb::DBValue;
use types::{
    basic_account::BasicAccount,
    block_status::BlockStatus,
    blockchain_info::BlockChainInfo,
    call_analytics::CallAnalytics,
    data_format::DataFormat,
    encoded,
    filter::Filter,
    header::Header,
    ids::*,
    log_entry::LocalizedLogEntry,
    pruning_info::PruningInfo,
    receipt::LocalizedReceipt,
    trace_filter::Filter as TraceFilter,
    transaction::{self, Action, LocalizedTransaction, SignedTransaction},
    BlockNumber,
};
use vm::LastHashes;

use block::{ClosedBlock, OpenBlock, SealedBlock};
use client::Mode;
use engines::EthEngine;
use error::{Error, EthcoreResult};
use executed::CallError;
use executive::Executed;
use state::StateInfo;
use trace::LocalizedTrace;
use verification::queue::{kind::blocks::Unverified, QueueInfo as BlockQueueInfo};

use super::{blockchain::BlockChainClient, ChainInfo};

/// State information to be used during client query
pub enum StateOrBlock {
    /// State to be used, may be pending
    State(Box<dyn StateInfo>),

    /// Id of an existing block from a chain to get state from
    Block(BlockId),
}

impl<S: StateInfo + 'static> From<S> for StateOrBlock {
    fn from(info: S) -> StateOrBlock {
        StateOrBlock::State(Box::new(info) as Box<_>)
    }
}

impl From<Box<dyn StateInfo>> for StateOrBlock {
    fn from(info: Box<dyn StateInfo>) -> StateOrBlock {
        StateOrBlock::State(info)
    }
}

impl From<BlockId> for StateOrBlock {
    fn from(id: BlockId) -> StateOrBlock {
        StateOrBlock::Block(id)
    }
}

/// Provides `nonce` and `latest_nonce` methods
pub trait Nonce {
    /// Attempt to get address nonce at given block.
    /// May not fail on BlockId::Latest.
    fn nonce(&self, address: &Address, id: BlockId) -> Option<U256>;

    /// Get address nonce at the latest block's state.
    fn latest_nonce(&self, address: &Address) -> U256 {
        self.nonce(address, BlockId::Latest).expect(
            "nonce will return Some when given BlockId::Latest. nonce was given BlockId::Latest. \
			Therefore nonce has returned Some; qed",
        )
    }
}

/// Provides `balance` and `latest_balance` methods
pub trait Balance {
    /// Get address balance at the given block's state.
    ///
    /// May not return None if given BlockId::Latest.
    /// Returns None if and only if the block's root hash has been pruned from the DB.
    fn balance(&self, address: &Address, state: StateOrBlock) -> Option<U256>;

    /// Get address balance at the latest block's state.
    fn latest_balance(&self, address: &Address) -> U256 {
        self.balance(address, BlockId::Latest.into()).expect(
            "balance will return Some if given BlockId::Latest. balance was given BlockId::Latest \
			Therefore balance has returned Some; qed",
        )
    }
}

/// Provides methods to access account info
pub trait AccountData: Nonce + Balance {}

/// Provides methods to access chain state
pub trait StateClient {
    /// Type representing chain state
    type State: StateInfo;

    /// Get a copy of the best block's state and header.
    fn latest_state_and_header(&self) -> (Self::State, Header);

    /// Attempt to get a copy of a specific block's final state.
    ///
    /// This will not fail if given BlockId::Latest.
    /// Otherwise, this can fail (but may not) if the DB prunes state or the block
    /// is unknown.
    fn state_at(&self, id: BlockId) -> Option<Self::State>;
}

// FIXME Why these methods belong to BlockChainClient and not MiningBlockChainClient?
/// Provides methods to import block into blockchain
pub trait ImportBlock {
    /// Import a block into the blockchain.
    fn import_block(&self, block: Unverified) -> EthcoreResult<H256>;
}

/// Provides `call` and `call_many` methods
pub trait Call {
    /// Type representing chain state
    type State: StateInfo;

    /// Makes a non-persistent transaction call.
    fn call(
        &self,
        tx: &SignedTransaction,
        analytics: CallAnalytics,
        state: &mut Self::State,
        header: &Header,
    ) -> Result<Executed, CallError>;

    /// Makes multiple non-persistent but dependent transaction calls.
    /// Returns a vector of successes or a failure if any of the transaction fails.
    fn call_many(
        &self,
        txs: &[(SignedTransaction, CallAnalytics)],
        state: &mut Self::State,
        header: &Header,
    ) -> Result<Vec<Executed>, CallError>;

    /// Estimates how much gas will be necessary for a call.
    fn estimate_gas(
        &self,
        t: &SignedTransaction,
        state: &Self::State,
        header: &Header,
    ) -> Result<U256, CallError>;
}

/// Provides recently seen bad blocks.
pub trait BadBlocks {
    /// Returns a list of blocks that were recently not imported because they were invalid.
    fn bad_blocks(&self) -> Vec<(Unverified, String)>;
}

/// The data required for a `Client` to create a transaction.
///
/// Gas limit, gas price, or nonce can be set explicitly, e.g. to create service
/// transactions with zero gas price, or sequences of transactions with consecutive nonces.
/// Added for AuRa needs.
pub struct TransactionRequest {
    /// Transaction action
    pub action: Action,
    /// Transaction data
    pub data: Bytes,
    /// Transaction gas usage
    pub gas: Option<U256>,
    /// Transaction gas price
    pub gas_price: Option<U256>,
    /// Transaction nonce
    pub nonce: Option<U256>,
}

impl TransactionRequest {
    /// Creates a request to call a contract at `address` with the specified call data.
    pub fn call(address: Address, data: Bytes) -> TransactionRequest {
        TransactionRequest {
            action: Action::Call(address),
            data,
            gas: None,
            gas_price: None,
            nonce: None,
        }
    }

    /// Sets a gas limit. If this is not specified, a sensible default is used.
    pub fn gas(mut self, gas: U256) -> TransactionRequest {
        self.gas = Some(gas);
        self
    }

    /// Sets a gas price. If this is not specified or `None`, a sensible default is used.
    pub fn gas_price<T: Into<Option<U256>>>(mut self, gas_price: T) -> TransactionRequest {
        self.gas_price = gas_price.into();
        self
    }

    /// Sets a nonce. If this is not specified, the appropriate latest nonce for the author is used.
    pub fn nonce(mut self, nonce: U256) -> TransactionRequest {
        self.nonce = Some(nonce);
        self
    }
}

/// Provides `reopen_block` method
pub trait ReopenBlock {
    /// Reopens an OpenBlock and updates uncles.
    fn reopen_block(&self, block: ClosedBlock) -> OpenBlock;
}

/// Provides `prepare_open_block` method
pub trait PrepareOpenBlock {
    /// Returns OpenBlock prepared for closing.
    fn prepare_open_block(
        &self,
        author: Address,
        gas_range_target: (U256, U256),
        extra_data: Bytes,
    ) -> Result<OpenBlock, Error>;
}

/// Provides methods used for sealing new state
pub trait BlockProducer: PrepareOpenBlock + ReopenBlock {}

///Provides `import_sealed_block` method
pub trait ImportSealedBlock {
    /// Import sealed block. Skips all verifications.
    fn import_sealed_block(&self, block: SealedBlock) -> EthcoreResult<H256>;
}

/// Provides `broadcast_proposal_block` method
pub trait BroadcastProposalBlock {
    /// Broadcast a block proposal.
    fn broadcast_proposal_block(&self, block: SealedBlock);
}

/// Provides methods to import sealed block and broadcast a block proposal
pub trait SealedBlockImporter: ImportSealedBlock + BroadcastProposalBlock {}

/// Do we want to force update sealing?
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ForceUpdateSealing {
    /// Ideally you want to use `No` at all times as `Yes` skips `reseal_required` checks.
    Yes,
    /// Don't skip `reseal_required` checks
    No,
}

/// Client facilities used by internally sealing Engines.
pub trait EngineClient: Sync + Send + ChainInfo {
    /// Make a new block and seal it.
    fn update_sealing(&self, force: ForceUpdateSealing);

    /// Submit a seal for a block in the mining queue.
    fn submit_seal(&self, block_hash: H256, seal: Vec<Bytes>);

    /// Broadcast a consensus message to the network.
    fn broadcast_consensus_message(&self, message: Bytes);

    /// Get the transition to the epoch the given parent hash is part of
    /// or transitions to.
    /// This will give the epoch that any children of this parent belong to.
    ///
    /// The block corresponding the the parent hash must be stored already.
    fn epoch_transition_for(&self, parent_hash: H256) -> Option<::engines::EpochTransition>;

    /// Attempt to cast the engine client to a full client.
    fn as_full_client(&self) -> Option<&dyn BlockChainClient>;

    /// Get a block number by ID.
    fn block_number(&self, id: BlockId) -> Option<BlockNumber>;

    /// Get raw block header data by block id.
    fn block_header(&self, id: BlockId) -> Option<encoded::Header>;
}

/// Provides a method for importing/exporting blocks
pub trait ImportExportBlocks {
    /// Export blocks to destination, with the given from, to and format argument.
    /// destination could be a file or stdout.
    /// If the format is hex, each block is written on a new line.
    /// For binary exports, all block data is written to the same line.
    fn export_blocks<'a>(
        &self,
        destination: Box<dyn std::io::Write + 'a>,
        from: BlockId,
        to: BlockId,
        format: Option<DataFormat>,
    ) -> Result<(), String>;

    /// Import blocks from destination, with the given format argument
    /// Source could be a file or stdout.
    /// For hex format imports, it attempts to read the blocks on a line by line basis.
    /// For binary format imports, reads the 8 byte RLP header in order to decode the block
    /// length to be read.
    fn import_blocks<'a>(
        &self,
        source: Box<dyn std::io::Read + 'a>,
        format: Option<DataFormat>,
    ) -> Result<(), String>;
}
