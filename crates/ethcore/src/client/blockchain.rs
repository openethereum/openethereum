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

use std::{cmp, collections::BTreeMap, sync::Arc};

use super::{
    client::Client,
    info::{BlockChain, BlockInfo},
    io::IoClient,
    traits::TransactionRequest,
    AccountData, BadBlocks, ChainInfo, Executed, ImportBlock, Mode, StateOrBlock, TraceFilter,
    TransactionInfo,
};

use crate::{
    engines::MAX_UNCLE_AGE, executed::CallError, trace::LocalizedTrace, verification::QueueInfo,
};
use blockchain::{BlockProvider, BlockReceipts, TreeRoute};
use bytes::Bytes;
use call_contract::{CallContract, RegistryInfo};
use client::info::ScheduleInfo;
use client::traits::Nonce;
use db::DBValue;
use ethcore_miner::pool::VerifiedTransaction;
use ethereum_types::{Address, H256, U256};
use hash::keccak;
use itertools::Itertools;
use miner::MinerService;
use state;
use trace;
use trace::Database;
use trie::Trie;
use types::{
    basic_account::BasicAccount,
    block_status::BlockStatus,
    call_analytics::CallAnalytics,
    encoded,
    ids::{BlockId, TraceId, TransactionId, UncleId},
    log_entry::LocalizedLogEntry,
    pruning_info::PruningInfo,
    receipt::LocalizedReceipt,
    transaction,
    transaction::{LocalizedTransaction, SignedTransaction, TypedTransaction},
    BlockNumber,
};
use vm::LastHashes;

/// Blockchain database client. Owns and manages a blockchain and a block queue.
pub trait BlockChainClient:
    Sync
    + Send
    + AccountData
    + BlockChain
    + CallContract
    + RegistryInfo
    + ImportBlock
    + IoClient
    + BadBlocks
{
    /// Look up the block number for the given block ID.
    fn block_number(&self, id: BlockId) -> Option<BlockNumber>;

    /// Get raw block body data by block id.
    /// Block body is an RLP list of two items: uncles and transactions.
    fn block_body(&self, id: BlockId) -> Option<encoded::Body>;

    /// Get block status by block header hash.
    fn block_status(&self, id: BlockId) -> BlockStatus;

    /// Get block total difficulty.
    fn block_total_difficulty(&self, id: BlockId) -> Option<U256>;

    /// Attempt to get address storage root at given block.
    /// May not fail on BlockId::Latest.
    fn storage_root(&self, address: &Address, id: BlockId) -> Option<H256>;

    /// Get block hash.
    fn block_hash(&self, id: BlockId) -> Option<H256>;

    /// Get address code at given block's state.
    fn code(&self, address: &Address, state: StateOrBlock) -> Option<Option<Bytes>>;

    /// Get address code at the latest block's state.
    fn latest_code(&self, address: &Address) -> Option<Bytes> {
        self.code(address, BlockId::Latest.into())
            .expect("code will return Some if given BlockId::Latest; qed")
    }

    /// Returns true if the given block is known and in the canon chain.
    fn is_canon(&self, hash: &H256) -> bool;

    /// Get address code hash at given block's state.

    /// Get value of the storage at given position at the given block's state.
    ///
    /// May not return None if given BlockId::Latest.
    /// Returns None if and only if the block's root hash has been pruned from the DB.
    fn storage_at(&self, address: &Address, position: &H256, state: StateOrBlock) -> Option<H256>;

    /// Get value of the storage at given position at the latest block's state.
    fn latest_storage_at(&self, address: &Address, position: &H256) -> H256 {
        self.storage_at(address, position, BlockId::Latest.into())
			.expect("storage_at will return Some if given BlockId::Latest. storage_at was given BlockId::Latest. \
			Therefore storage_at has returned Some; qed")
    }

    /// Get a list of all accounts in the block `id`, if fat DB is in operation, otherwise `None`.
    /// If `after` is set the list starts with the following item.
    fn list_accounts(
        &self,
        id: BlockId,
        after: Option<&Address>,
        count: u64,
    ) -> Option<Vec<Address>>;

    /// Get a list of all storage keys in the block `id`, if fat DB is in operation, otherwise `None`.
    /// If `after` is set the list starts with the following item.
    fn list_storage(
        &self,
        id: BlockId,
        account: &Address,
        after: Option<&H256>,
        count: u64,
    ) -> Option<Vec<H256>>;

    /// Get transaction with given hash.
    fn transaction(&self, id: TransactionId) -> Option<LocalizedTransaction>;

    /// Get uncle with given id.
    fn uncle(&self, id: UncleId) -> Option<encoded::Header>;

    /// Get transaction receipt with given hash.
    fn transaction_receipt(&self, id: TransactionId) -> Option<LocalizedReceipt>;

    /// Get localized receipts for all transaction in given block.
    fn localized_block_receipts(&self, id: BlockId) -> Option<Vec<LocalizedReceipt>>;

    /// Get a tree route between `from` and `to`.
    /// See `BlockChain::tree_route`.
    fn tree_route(&self, from: &H256, to: &H256) -> Option<TreeRoute>;

    /// Get all possible uncle hashes for a block.
    fn find_uncles(&self, hash: &H256) -> Option<Vec<H256>>;

    /// Get block receipts data by block header hash.
    fn block_receipts(&self, hash: &H256) -> Option<BlockReceipts>;

    /// Get block queue information.
    fn queue_info(&self) -> QueueInfo;

    /// Returns true if block queue is empty.
    fn is_queue_empty(&self) -> bool {
        self.queue_info().is_empty()
    }

    /// Clear block queue and abort all import activity.
    fn clear_queue(&self);

    /// Get the registrar address, if it exists.
    fn additional_params(&self) -> BTreeMap<String, String>;

    /// Returns logs matching given filter. If one of the filtering block cannot be found, returns the block id that caused the error.
    fn logs(&self, filter: types::filter::Filter) -> Result<Vec<LocalizedLogEntry>, BlockId>;

    /// Replays a given transaction for inspection.
    fn replay(&self, t: TransactionId, analytics: CallAnalytics) -> Result<Executed, CallError>;

    /// Replays all the transactions in a given block for inspection.
    fn replay_block_transactions(
        &self,
        block: BlockId,
        analytics: CallAnalytics,
    ) -> Result<Box<dyn Iterator<Item = (H256, Executed)>>, CallError>;

    /// Returns traces matching given filter.
    fn filter_traces(&self, filter: TraceFilter) -> Option<Vec<LocalizedTrace>>;

    /// Returns trace with given id.
    fn trace(&self, trace: TraceId) -> Option<LocalizedTrace>;

    /// Returns traces created by transaction.
    fn transaction_traces(&self, trace: TransactionId) -> Option<Vec<LocalizedTrace>>;

    /// Returns traces created by transaction from block.
    fn block_traces(&self, trace: BlockId) -> Option<Vec<LocalizedTrace>>;

    /// Get last hashes starting from best block.
    fn last_hashes(&self) -> LastHashes;

    /// List all ready transactions that should be propagated to other peers.
    fn transactions_to_propagate(&self) -> Vec<Arc<VerifiedTransaction>>;

    /// Sorted list of transaction gas prices from at least last sample_size blocks.
    fn gas_price_corpus(&self, sample_size: usize) -> ::stats::Corpus<U256> {
        let mut h = self.chain_info().best_block_hash;
        let mut corpus = Vec::new();
        while corpus.is_empty() {
            for _ in 0..sample_size {
                let block = match self.block(BlockId::Hash(h)) {
                    Some(block) => block,
                    None => return corpus.into(),
                };

                if block.number() == 0 {
                    return corpus.into();
                }
                block
                    .transaction_views()
                    .iter()
                    .foreach(|t| corpus.push(t.gas_price()));
                h = block.parent_hash().clone();
            }
        }
        corpus.into()
    }

    /// Get the preferred chain ID to sign on
    fn signing_chain_id(&self) -> Option<u64>;

    /// Get the chain spec name.
    fn spec_name(&self) -> String;

    /// Set the chain via a spec name.
    fn set_spec_name(&self, spec_name: String) -> Result<(), ()>;

    /// Disable the client from importing blocks. This cannot be undone in this session and indicates
    /// that a subsystem has reason to believe this executable incapable of syncing the chain.
    fn disable(&self);

    /// Returns engine-related extra info for `BlockId`.
    fn block_extra_info(&self, id: BlockId) -> Option<BTreeMap<String, String>>;

    /// Returns engine-related extra info for `UncleId`.
    fn uncle_extra_info(&self, id: UncleId) -> Option<BTreeMap<String, String>>;

    /// Returns information about pruning/data availability.
    fn pruning_info(&self) -> PruningInfo;

    /// Returns a transaction signed with the key configured in the engine signer.
    fn create_transaction(
        &self,
        tx_request: TransactionRequest,
    ) -> Result<SignedTransaction, transaction::Error>;

    /// Schedule state-altering transaction to be executed on the next pending
    /// block with the given gas and nonce parameters.
    fn transact(&self, tx_request: TransactionRequest) -> Result<(), transaction::Error>;

    /// Get the address of the registry itself.
    fn registrar_address(&self) -> Option<Address>;

    /// Returns true, if underlying import queue is processing possible fork at the moment
    fn is_processing_fork(&self) -> bool;

    /// Get the mode.
    fn mode(&self) -> Mode;

    /// Set the mode.
    fn set_mode(&self, mode: Mode);
}

/// Extended client interface for providing proofs of the state.
pub trait ProvingBlockChainClient: BlockChainClient {
    /// Prove account storage at a specific block id.
    ///
    /// Both provided keys assume a secure trie.
    /// Returns a vector of raw trie nodes (in order from the root) proving the storage query.
    fn prove_storage(&self, key1: H256, key2: H256, id: BlockId) -> Option<(Vec<Bytes>, H256)>;

    /// Prove account existence at a specific block id.
    /// The key is the keccak hash of the account's address.
    /// Returns a vector of raw trie nodes (in order from the root) proving the query.
    fn prove_account(&self, key1: H256, id: BlockId) -> Option<(Vec<Bytes>, BasicAccount)>;

    /// Prove execution of a transaction at the given block.
    /// Returns the output of the call and a vector of database items necessary
    /// to reproduce it.
    fn prove_transaction(
        &self,
        transaction: SignedTransaction,
        id: BlockId,
    ) -> Option<(Bytes, Vec<DBValue>)>;

    /// Get an epoch change signal by block hash.
    fn epoch_signal(&self, hash: H256) -> Option<Vec<u8>>;
}

impl BlockChainClient for Client {
    fn replay(&self, id: TransactionId, analytics: CallAnalytics) -> Result<Executed, CallError> {
        let address = self
            .transaction_address(id)
            .ok_or(CallError::TransactionNotFound)?;
        let block = BlockId::Hash(address.block_hash);

        const PROOF: &'static str =
            "The transaction address contains a valid index within block; qed";
        Ok(self
            .replay_block_transactions(block, analytics)?
            .nth(address.index)
            .expect(PROOF)
            .1)
    }

    fn replay_block_transactions(
        &self,
        block: BlockId,
        analytics: CallAnalytics,
    ) -> Result<Box<dyn Iterator<Item = (H256, Executed)>>, CallError> {
        let mut env_info = self.env_info(block).ok_or(CallError::StatePruned)?;
        let body = self.block_body(block).ok_or(CallError::StatePruned)?;
        let mut state = self
            .state_at_beginning(block)
            .ok_or(CallError::StatePruned)?;
        let txs = body.transactions();
        let engine = self.engine();

        const PROOF: &'static str =
            "Transactions fetched from blockchain; blockchain transactions are valid; qed";
        const EXECUTE_PROOF: &'static str = "Transaction replayed; qed";

        Ok(Box::new(txs.into_iter().map(move |t| {
            let transaction_hash = t.hash();
            let t = SignedTransaction::new(t).expect(PROOF);
            let machine = engine.machine();
            let x = Self::do_virtual_call(machine, &env_info, &mut state, &t, analytics)
                .expect(EXECUTE_PROOF);
            env_info.gas_used = env_info.gas_used + x.gas_used;
            (transaction_hash, x)
        })))
    }

    fn disable(&self) {
        self.set_mode(Mode::Off);
        self.disable();
        self.clear_queue();
    }

    fn spec_name(&self) -> String {
        self.config.spec_name.clone()
    }

    fn is_canon(&self, hash: &H256) -> bool {
        self.chain.read().is_canon(hash)
    }

    /// Get the mode.
    fn mode(&self) -> Mode {
        Client::mode(&self)
    }

    /// Set the mode.
    fn set_mode(&self, mode: Mode) {
        Client::set_mode(&self, mode);
    }

    fn set_spec_name(&self, new_spec_name: String) -> Result<(), ()> {
        trace!(target: "mode", "Client::set_spec_name({:?})", new_spec_name);
        if !self.enabled() {
            return Err(());
        }
        if let Some(ref h) = *self.exit_handler.lock() {
            (*h)(new_spec_name);
            Ok(())
        } else {
            warn!("Not hypervised; cannot change chain.");
            Err(())
        }
    }

    fn block_number(&self, id: BlockId) -> Option<BlockNumber> {
        self.block_number_ref(&id)
    }

    fn block_body(&self, id: BlockId) -> Option<encoded::Body> {
        let chain = self.chain.read();

        Self::block_hash(&chain, id).and_then(|hash| chain.block_body(&hash))
    }

    fn block_status(&self, id: BlockId) -> BlockStatus {
        let chain = self.chain.read();
        match Self::block_hash(&chain, id) {
            Some(ref hash) if chain.is_known(hash) => BlockStatus::InChain,
            Some(hash) => self.importer.block_queue.status(&hash).into(),
            None => BlockStatus::Unknown,
        }
    }

    fn is_processing_fork(&self) -> bool {
        let chain = self.chain.read();
        self.importer
            .block_queue
            .is_processing_fork(&chain.best_block_hash(), &chain)
    }

    fn block_total_difficulty(&self, id: BlockId) -> Option<U256> {
        let chain = self.chain.read();

        Self::block_hash(&chain, id)
            .and_then(|hash| chain.block_details(&hash))
            .map(|d| d.total_difficulty)
    }

    fn storage_root(&self, address: &Address, id: BlockId) -> Option<H256> {
        self.state_at(id)
            .and_then(|s| s.storage_root(address).ok())
            .and_then(|x| x)
    }

    fn block_hash(&self, id: BlockId) -> Option<H256> {
        let chain = self.chain.read();
        Self::block_hash(&chain, id)
    }

    fn code(&self, address: &Address, state: StateOrBlock) -> Option<Option<Bytes>> {
        let result = match state {
            StateOrBlock::State(s) => s.code(address).ok(),
            StateOrBlock::Block(id) => self.state_at(id).and_then(|s| s.code(address).ok()),
        };

        // Converting from `Option<Option<Arc<Bytes>>>` to `Option<Option<Bytes>>`
        result.map(|c| c.map(|c| (&*c).clone()))
    }

    fn storage_at(&self, address: &Address, position: &H256, state: StateOrBlock) -> Option<H256> {
        match state {
            StateOrBlock::State(s) => s.storage_at(address, position).ok(),
            StateOrBlock::Block(id) => self
                .state_at(id)
                .and_then(|s| s.storage_at(address, position).ok()),
        }
    }

    fn list_accounts(
        &self,
        id: BlockId,
        after: Option<&Address>,
        count: u64,
    ) -> Option<Vec<Address>> {
        if !self.factories.trie.is_fat() {
            trace!(target: "fatdb", "list_accounts: Not a fat DB");
            return None;
        }

        let state = match self.state_at(id) {
            Some(state) => state,
            _ => return None,
        };

        let (root, db) = state.drop();
        let db = &db.as_hash_db();
        let trie = match self.factories.trie.readonly(db, &root) {
            Ok(trie) => trie,
            _ => {
                trace!(target: "fatdb", "list_accounts: Couldn't open the DB");
                return None;
            }
        };

        let mut iter = match trie.iter() {
            Ok(iter) => iter,
            _ => return None,
        };

        if let Some(after) = after {
            if let Err(e) = iter.seek(after.as_bytes()) {
                trace!(target: "fatdb", "list_accounts: Couldn't seek the DB: {:?}", e);
            } else {
                // Position the iterator after the `after` element
                iter.next();
            }
        }

        let accounts = iter
            .filter_map(|item| item.ok().map(|(addr, _)| Address::from_slice(&addr)))
            .take(count as usize)
            .collect();

        Some(accounts)
    }

    fn list_storage(
        &self,
        id: BlockId,
        account: &Address,
        after: Option<&H256>,
        count: u64,
    ) -> Option<Vec<H256>> {
        if !self.factories.trie.is_fat() {
            trace!(target: "fatdb", "list_storage: Not a fat DB");
            return None;
        }

        let state = match self.state_at(id) {
            Some(state) => state,
            _ => return None,
        };

        let root = match state.storage_root(account) {
            Ok(Some(root)) => root,
            _ => return None,
        };

        let (_, db) = state.drop();
        let account_db = &self
            .factories
            .accountdb
            .readonly(db.as_hash_db(), keccak(account));
        let account_db = &account_db.as_hash_db();
        let trie = match self.factories.trie.readonly(account_db, &root) {
            Ok(trie) => trie,
            _ => {
                trace!(target: "fatdb", "list_storage: Couldn't open the DB");
                return None;
            }
        };

        let mut iter = match trie.iter() {
            Ok(iter) => iter,
            _ => return None,
        };

        if let Some(after) = after {
            if let Err(e) = iter.seek(after.as_bytes()) {
                trace!(target: "fatdb", "list_storage: Couldn't seek the DB: {:?}", e);
            } else {
                // Position the iterator after the `after` element
                iter.next();
            }
        }

        let keys = iter
            .filter_map(|item| item.ok().map(|(key, _)| H256::from_slice(&key)))
            .take(count as usize)
            .collect();

        Some(keys)
    }

    fn transaction(&self, id: TransactionId) -> Option<LocalizedTransaction> {
        self.transaction_address(id)
            .and_then(|address| self.chain.read().transaction(&address))
    }

    fn uncle(&self, id: UncleId) -> Option<encoded::Header> {
        let index = id.position;
        self.block_body(id.block)
            .and_then(|body| body.view().uncle_rlp_at(index))
            .map(encoded::Header::new)
    }

    fn transaction_receipt(&self, id: TransactionId) -> Option<LocalizedReceipt> {
        // NOTE Don't use block_receipts here for performance reasons
        let address = self.transaction_address(id)?;
        let hash = address.block_hash;
        let chain = self.chain.read();
        let number = chain.block_number(&hash)?;
        let body = chain.block_body(&hash)?;
        let mut receipts = chain.block_receipts(&hash)?.receipts;
        receipts.truncate(address.index + 1);

        let transaction = body
            .view()
            .localized_transaction_at(&hash, number, address.index)?;
        let receipt = receipts.pop()?;
        let gas_used = receipts.last().map_or_else(|| 0.into(), |r| r.gas_used);
        let no_of_logs = receipts
            .into_iter()
            .map(|receipt| receipt.logs.len())
            .sum::<usize>();

        let receipt = super::transaction_receipt(
            self.engine().machine(),
            transaction,
            receipt,
            gas_used,
            no_of_logs,
        );
        Some(receipt)
    }

    fn localized_block_receipts(&self, id: BlockId) -> Option<Vec<LocalizedReceipt>> {
        let hash = self.block_hash(id)?;

        let chain = self.chain.read();
        let receipts = chain.block_receipts(&hash)?;
        let number = chain.block_number(&hash)?;
        let body = chain.block_body(&hash)?;
        let engine = self.engine();

        let mut gas_used = 0.into();
        let mut no_of_logs = 0;

        Some(
            body.view()
                .localized_transactions(&hash, number)
                .into_iter()
                .zip(receipts.receipts)
                .map(move |(transaction, receipt)| {
                    let result = super::transaction_receipt(
                        engine.machine(),
                        transaction,
                        receipt,
                        gas_used,
                        no_of_logs,
                    );
                    gas_used = result.cumulative_gas_used;
                    no_of_logs += result.logs.len();
                    result
                })
                .collect(),
        )
    }

    fn tree_route(&self, from: &H256, to: &H256) -> Option<TreeRoute> {
        let chain = self.chain.read();
        match chain.is_known(from) && chain.is_known(to) {
            true => chain.tree_route(from.clone(), to.clone()),
            false => None,
        }
    }

    fn find_uncles(&self, hash: &H256) -> Option<Vec<H256>> {
        self.chain.read().find_uncle_hashes(hash, MAX_UNCLE_AGE)
    }

    fn block_receipts(&self, hash: &H256) -> Option<BlockReceipts> {
        self.chain.read().block_receipts(hash)
    }

    fn queue_info(&self) -> QueueInfo {
        self.importer.block_queue.queue_info()
    }

    fn is_queue_empty(&self) -> bool {
        self.importer.block_queue.is_empty()
    }

    fn clear_queue(&self) {
        self.importer.block_queue.clear();
    }

    fn additional_params(&self) -> BTreeMap<String, String> {
        self.engine().additional_params().into_iter().collect()
    }

    fn logs(&self, filter: types::filter::Filter) -> Result<Vec<LocalizedLogEntry>, BlockId> {
        let chain = self.chain.read();

        // First, check whether `filter.from_block` and `filter.to_block` is on the canon chain. If so, we can use the
        // optimized version.
        let is_canon = |id| {
            match id {
                // If it is referred by number, then it is always on the canon chain.
                &BlockId::Earliest | &BlockId::Latest | &BlockId::Number(_) => true,
                // If it is referred by hash, we see whether a hash -> number -> hash conversion gives us the same
                // result.
                &BlockId::Hash(ref hash) => chain.is_canon(hash),
            }
        };

        let blocks = if is_canon(&filter.from_block) && is_canon(&filter.to_block) {
            // If we are on the canon chain, use bloom filter to fetch required hashes.
            //
            // If we are sure the block does not exist (where val > best_block_number), then return error. Note that we
            // don't need to care about pending blocks here because RPC query sets pending back to latest (or handled
            // pending logs themselves).
            let from = match self.block_number_ref(&filter.from_block) {
                Some(val) if val <= chain.best_block_number() => val,
                _ => return Err(filter.from_block.clone()),
            };
            let to = match self.block_number_ref(&filter.to_block) {
                Some(val) if val <= chain.best_block_number() => val,
                _ => return Err(filter.to_block.clone()),
            };

            // If from is greater than to, then the current bloom filter behavior is to just return empty
            // result. There's no point to continue here.
            if from > to {
                return Err(filter.to_block.clone());
            }

            chain
                .blocks_with_bloom(&filter.bloom_possibilities(), from, to)
                .into_iter()
                .filter_map(|n| chain.block_hash(n))
                .collect::<Vec<H256>>()
        } else {
            // Otherwise, we use a slower version that finds a link between from_block and to_block.
            let from_hash = Self::block_hash(&chain, filter.from_block)
                .ok_or_else(|| filter.from_block.clone())?;
            let from_number = chain
                .block_number(&from_hash)
                .ok_or_else(|| BlockId::Hash(from_hash))?;
            let to_hash =
                Self::block_hash(&chain, filter.to_block).ok_or_else(|| filter.to_block.clone())?;

            let blooms = filter.bloom_possibilities();
            let bloom_match = |header: &encoded::Header| {
                blooms
                    .iter()
                    .any(|bloom| header.log_bloom().contains_bloom(bloom))
            };

            let (blocks, last_hash) = {
                let mut blocks = Vec::new();
                let mut current_hash = to_hash;

                loop {
                    let header = chain
                        .block_header_data(&current_hash)
                        .ok_or_else(|| BlockId::Hash(current_hash))?;
                    if bloom_match(&header) {
                        blocks.push(current_hash);
                    }

                    // Stop if `from` block is reached.
                    if header.number() <= from_number {
                        break;
                    }
                    current_hash = header.parent_hash();
                }

                blocks.reverse();
                (blocks, current_hash)
            };

            // Check if we've actually reached the expected `from` block.
            if last_hash != from_hash || blocks.is_empty() {
                // In this case, from_hash is the cause (for not matching last_hash).
                return Err(BlockId::Hash(from_hash));
            }

            blocks
        };

        Ok(chain.logs(blocks, |entry| filter.matches(entry), filter.limit))
    }

    fn filter_traces(&self, filter: TraceFilter) -> Option<Vec<LocalizedTrace>> {
        if !self.tracedb.read().tracing_enabled() {
            return None;
        }

        let start = self.block_number(filter.range.start)?;
        let end = self.block_number(filter.range.end)?;

        let db_filter = trace::Filter {
            range: start as usize..end as usize,
            from_address: filter.from_address.into(),
            to_address: filter.to_address.into(),
        };

        let traces = self
            .tracedb
            .read()
            .filter(&db_filter)
            .into_iter()
            .skip(filter.after.unwrap_or(0))
            .take(filter.count.unwrap_or(usize::max_value()))
            .collect();
        Some(traces)
    }

    fn trace(&self, trace: TraceId) -> Option<LocalizedTrace> {
        if !self.tracedb.read().tracing_enabled() {
            return None;
        }

        let trace_address = trace.address;
        self.transaction_address(trace.transaction)
            .and_then(|tx_address| {
                self.block_number(BlockId::Hash(tx_address.block_hash))
                    .and_then(|number| {
                        self.tracedb
                            .read()
                            .trace(number, tx_address.index, trace_address)
                    })
            })
    }

    fn transaction_traces(&self, transaction: TransactionId) -> Option<Vec<LocalizedTrace>> {
        if !self.tracedb.read().tracing_enabled() {
            return None;
        }

        self.transaction_address(transaction)
            .and_then(|tx_address| {
                self.block_number(BlockId::Hash(tx_address.block_hash))
                    .and_then(|number| {
                        self.tracedb
                            .read()
                            .transaction_traces(number, tx_address.index)
                    })
            })
    }

    fn block_traces(&self, block: BlockId) -> Option<Vec<LocalizedTrace>> {
        if !self.tracedb.read().tracing_enabled() {
            return None;
        }

        self.block_number(block)
            .and_then(|number| self.tracedb.read().block_traces(number))
    }

    fn last_hashes(&self) -> LastHashes {
        (*self.build_last_hashes(&self.chain.read().best_block_hash())).clone()
    }

    fn transactions_to_propagate(&self) -> Vec<Arc<VerifiedTransaction>> {
        const PROPAGATE_FOR_BLOCKS: u32 = 4;
        const MIN_TX_TO_PROPAGATE: usize = 256;

        let block_gas_limit = *self.best_block_header().gas_limit();
        let min_tx_gas: U256 = self.latest_schedule().tx_gas.into();

        let max_len = if min_tx_gas.is_zero() {
            usize::max_value()
        } else {
            cmp::max(
                MIN_TX_TO_PROPAGATE,
                cmp::min(
                    (block_gas_limit / min_tx_gas) * PROPAGATE_FOR_BLOCKS,
                    // never more than usize
                    usize::max_value().into(),
                )
                .as_u64() as usize,
            )
        };
        self.importer
            .miner
            .ready_transactions(self, max_len, ::miner::PendingOrdering::Priority)
    }

    fn signing_chain_id(&self) -> Option<u64> {
        self.engine().signing_chain_id(&self.latest_env_info())
    }

    fn block_extra_info(&self, id: BlockId) -> Option<BTreeMap<String, String>> {
        self.block_header_decoded(id)
            .map(|header| self.engine().extra_info(&header))
    }

    fn uncle_extra_info(&self, id: UncleId) -> Option<BTreeMap<String, String>> {
        self.uncle(id)
            .and_then(|h| h.decode().map(|dh| self.engine().extra_info(&dh)).ok())
    }

    fn pruning_info(&self) -> PruningInfo {
        PruningInfo {
            earliest_chain: self.chain.read().first_block_number().unwrap_or(1),
            earliest_state: self
                .state_db
                .read()
                .journal_db()
                .earliest_era()
                .unwrap_or(0),
        }
    }

    fn create_transaction(
        &self,
        TransactionRequest {
            action,
            data,
            gas,
            gas_price,
            nonce,
        }: TransactionRequest,
    ) -> Result<SignedTransaction, transaction::Error> {
        let authoring_params = self.importer.miner.authoring_params();
        let service_transaction_checker = self.importer.miner.service_transaction_checker();
        let gas_price = if let Some(checker) = service_transaction_checker {
            match checker.check_address(self, authoring_params.author) {
                Ok(true) => U256::zero(),
                _ => gas_price.unwrap_or_else(|| self.importer.miner.sensible_gas_price()),
            }
        } else {
            self.importer.miner.sensible_gas_price()
        };
        let transaction = TypedTransaction::Legacy(transaction::Transaction {
            nonce: nonce.unwrap_or_else(|| self.latest_nonce(&authoring_params.author)),
            action,
            gas: gas.unwrap_or_else(|| self.importer.miner.sensible_gas_limit()),
            gas_price,
            value: U256::zero(),
            data,
        });
        let chain_id = self.engine().signing_chain_id(&self.latest_env_info());
        let signature = self
            .engine()
            .sign(transaction.signature_hash(chain_id))
            .map_err(|e| transaction::Error::InvalidSignature(e.to_string()))?;
        Ok(SignedTransaction::new(
            transaction.with_signature(signature, chain_id),
        )?)
    }

    fn transact(&self, tx_request: TransactionRequest) -> Result<(), transaction::Error> {
        let signed = self.create_transaction(tx_request)?;
        self.importer
            .miner
            .import_own_transaction(self, signed.into())
    }

    fn registrar_address(&self) -> Option<Address> {
        self.registrar_address.clone()
    }
}

impl ProvingBlockChainClient for Client {
    fn prove_storage(&self, key1: H256, key2: H256, id: BlockId) -> Option<(Vec<Bytes>, H256)> {
        self.state_at(id)
            .and_then(move |state| state.prove_storage(key1, key2).ok())
    }

    fn prove_account(
        &self,
        key1: H256,
        id: BlockId,
    ) -> Option<(Vec<Bytes>, ::types::basic_account::BasicAccount)> {
        self.state_at(id)
            .and_then(move |state| state.prove_account(key1).ok())
    }

    fn prove_transaction(
        &self,
        transaction: SignedTransaction,
        id: BlockId,
    ) -> Option<(Bytes, Vec<DBValue>)> {
        let (header, mut env_info) = match (self.block_header(id), self.env_info(id)) {
            (Some(s), Some(e)) => (s, e),
            _ => return None,
        };

        env_info.gas_limit = transaction.tx().gas.clone();
        let mut jdb = self.state_db.read().journal_db().boxed_clone();

        state::prove_transaction_virtual(
            jdb.as_hash_db_mut(),
            header.state_root().clone(),
            &transaction,
            self.engine().machine(),
            &env_info,
            self.factories.clone(),
        )
    }

    fn epoch_signal(&self, hash: H256) -> Option<Vec<u8>> {
        // pending transitions are never deleted, and do not contain
        // finality proofs by definition.
        self.chain
            .read()
            .get_pending_transition(hash)
            .map(|pending| pending.proof)
    }
}

/// resets the blockchain
pub trait BlockChainReset {
    /// reset to best_block - n
    fn reset(&self, num: u32) -> Result<(), String>;
}
