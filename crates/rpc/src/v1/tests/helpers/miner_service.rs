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

//! Test implementation of miner service.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use bytes::Bytes;
use ethcore::{
    block::{BlockInternals, SealedBlock},
    client::{
        test_client::TestState, traits::ForceUpdateSealing, EngineInfo,
        StateClient,
    },
    engines::{signer::EngineSigner, EthEngine},
    error::Error,
    miner::{
        self, AuthoringParams, MinerPendingBlock, MinerRPC,
        MinerService, MinerTxpool, PendingOrdering, TransactionFilter,
    },
};
use ethereum_types::{Address, H256, U256};
use miner::pool::{
    local_transactions::Status as LocalTransactionStatus, verifier, QueueStatus,
    VerifiedTransaction,
};
use parking_lot::{Mutex, RwLock};
use txpool;
use types::{
    header::Header,
    ids::BlockId,
    receipt::TypedReceipt,
    transaction::{self, PendingTransaction, SignedTransaction, UnverifiedTransaction},
    BlockNumber,
};


/// Test miner service.
pub struct TestMinerService {
    /// Imported transactions.
    pub imported_transactions: Mutex<Vec<SignedTransaction>>,
    /// Pre-existed local transactions
    pub local_transactions: Mutex<BTreeMap<H256, LocalTransactionStatus>>,
    /// Next nonces.
    pub next_nonces: RwLock<HashMap<Address, U256>>,
    /// Minimum gas price
    pub min_gas_price: RwLock<Option<U256>>,
    /// Signer (if any)
    pub signer: RwLock<Option<Box<dyn EngineSigner>>>,

    pub pending_block: Mutex<TestBlockInternalData>,

    authoring_params: RwLock<AuthoringParams>,
}

pub struct TestBlockInternalData {
    /// Pre-existed pending transactions
    pub transactions: Vec<SignedTransaction>,
    /// Pre-existed pending receipts
    pub receipts: Vec<TypedReceipt>,
}

impl Default for TestBlockInternalData {
    fn default() -> TestBlockInternalData {
        TestBlockInternalData {
            transactions: Default::default(),
            receipts: Default::default(),
        }
    }
}

impl BlockInternals for TestBlockInternalData {
    fn transactions(&self) -> &[SignedTransaction] {
        &self.transactions
    }

    fn header(&self) -> &Header {
        unimplemented!()
    }

    fn uncles(&self) -> &[Header] {
        unimplemented!()
    }

    fn receipts(&self) -> Option<&[TypedReceipt]> {
        Some(&self.receipts)
    }
}

impl Default for TestMinerService {
    fn default() -> TestMinerService {
        TestMinerService {
            imported_transactions: Default::default(),
            pending_block: Default::default(),
            local_transactions: Default::default(),
            next_nonces: Default::default(),
            min_gas_price: RwLock::new(Some(0.into())),
            authoring_params: RwLock::new(AuthoringParams {
                author: Address::zero(),
                gas_range_target: (12345.into(), 54321.into()),
                extra_data: vec![1, 2, 3, 4],
            }),
            signer: RwLock::new(None),
        }
    }
}

impl TestMinerService {
    /// Increments nonce for given address.
    pub fn increment_nonce(&self, address: &Address) {
        let mut next_nonces = self.next_nonces.write();
        let nonce = next_nonces.entry(*address).or_insert_with(|| 0.into());
        *nonce = *nonce + 1;
    }
}

impl StateClient for TestMinerService {
    // State will not be used by test client anyway, since all methods that accept state are mocked
    type State = TestState;

    fn latest_state_and_header(&self) -> (Self::State, Header) {
        (TestState, Header::default())
    }

    fn state_at(&self, _id: BlockId) -> Option<Self::State> {
        Some(TestState)
    }
}

impl EngineInfo for TestMinerService {
    fn engine(&self) -> &dyn EthEngine {
        unimplemented!()
    }
}

impl miner::MinerPendingState for TestMinerService {
    type State = TestState;

    fn pending_state(&self, _latest_block_number: BlockNumber) -> Option<Self::State> {
        None
    }
}

impl MinerPendingBlock for TestMinerService {
    /// Get pending block information.
    fn pending<Fn, T>(&self, _latest_block: BlockNumber, f: Fn) -> Option<T>
    where
        Fn: FnOnce(&dyn BlockInternals) -> T,
    {
        let block = self.pending_block.lock();
        Some(f(&*block))
    }
}

impl MinerRPC for TestMinerService {
    fn is_currently_sealing(&self) -> bool {
        false
    }

    fn work_package(
        &self
    ) -> Option<(H256, BlockNumber, u64, U256)> {
        
        unimplemented!();
        /* TODO [dr] chain ref in testlet
        let params = self.authoring_params();
        let open_block = self.chain
            .(params.author, params.gas_range_target, params.extra_data)
            .unwrap();
        let closed = open_block.close().unwrap();
        let header = &closed.header;

        Some((
            header.hash(),
            header.number(),
            header.timestamp(),
            *header.difficulty(),
        ))*/
    }

    /// Submit `seal` as a valid solution for the header of `pow_hash`.
    /// Will check the seal, but not actually insert the block into the chain.
    fn submit_seal(&self, _pow_hash: H256, _seal: Vec<Bytes>) -> Result<SealedBlock, Error> {
        unimplemented!();
    }

    fn set_extra_data(&self, extra_data: Bytes) {
        self.authoring_params.write().extra_data = extra_data;
    }

    fn set_gas_range_target(&self, target: (U256, U256)) {
        self.authoring_params.write().gas_range_target = target;
    }

    fn set_author<T: Into<Option<miner::Author>>>(&self, author: T) {
        let author_opt = author.into();
        self.authoring_params.write().author = author_opt
            .as_ref()
            .map(miner::Author::address)
            .unwrap_or_default();
        match author_opt {
            Some(miner::Author::Sealer(signer)) => *self.signer.write() = Some(signer),
            Some(miner::Author::External(_addr)) => (),
            None => *self.signer.write() = None,
        }
    }

    fn queue_status(&self) -> QueueStatus {
        QueueStatus {
            options: verifier::Options {
                minimal_gas_price: 0x1312d00.into(),
                block_gas_limit: 5_000_000.into(),
                tx_gas_limit: 5_000_000.into(),
                no_early_reject: false,
            },
            status: txpool::LightStatus {
                mem_usage: 1_000,
                transaction_count: 52,
                senders: 1,
            },
            limits: txpool::Options {
                max_count: 1_024,
                max_per_sender: 16,
                max_mem_usage: 5_000,
            },
        }
    }
}

impl MinerTxpool for TestMinerService {
    fn ready_transactions_filtered(
        &self,
        _max_len: usize,
        filter: Option<TransactionFilter>,
        _ordering: miner::PendingOrdering,
    ) -> Vec<Arc<VerifiedTransaction>> {
        match filter {
            Some(f) => self
                .queued_transactions()
                .into_iter()
                .filter(|tx| f.matches(tx))
                .collect(),
            None => self.queued_transactions(),
        }
    }

    fn queued_transactions(&self) -> Vec<Arc<VerifiedTransaction>> {
        self.pending_block
            .lock().transactions
            .iter()
            .cloned()
            .map(|tx| Arc::new(VerifiedTransaction::from_pending_block_transaction(tx)))
            .collect()
    }

    fn queued_transaction_hashes(&self) -> Vec<H256> {
        self.pending_block
            .lock().transactions
            .iter()
            .map(|tx| tx.hash.clone())
            .collect()
    }

    fn next_nonce(&self, address: &Address) -> U256 {
        self.next_nonces
            .read()
            .get(address)
            .cloned()
            .unwrap_or_default()
    }

    /// Imports transactions to transaction queue.
    fn import_external_transactions(
        &self,
        transactions: Vec<UnverifiedTransaction>,
    ) -> Vec<Result<(), transaction::Error>> {
        // lets assume that all txs are valid
        let transactions: Vec<_> = transactions
            .into_iter()
            .map(|tx| SignedTransaction::new(tx).unwrap())
            .collect();
        self.imported_transactions
            .lock()
            .extend_from_slice(&transactions);

        for sender in transactions.iter().map(|tx| tx.sender()) {
            let nonce = self.next_nonce(&sender);
            self.next_nonces.write().insert(sender, nonce);
        }

        transactions.iter().map(|_| Ok(())).collect()
    }

    /// Imports transactions to transaction queue.
    fn import_own_transaction(
        &self,
        _pending: PendingTransaction,
    ) -> Result<(), transaction::Error> {
        // this function is no longer called directly from RPC
        unimplemented!();
    }

    /// Imports transactions to queue - treats as local based on trusted flag, config, and tx source
    fn import_claimed_local_transaction(
        &self,
        pending: PendingTransaction,
        _trusted: bool,
    ) -> Result<(), transaction::Error> {
        // keep the pending nonces up to date
        let sender = pending.transaction.sender();
        let nonce = self.next_nonce(&sender);
        self.next_nonces.write().insert(sender, nonce);

        // lets assume that all txs are valid
        self.imported_transactions.lock().push(pending.transaction);

        Ok(())
    }

    fn transaction(&self, hash: &H256) -> Option<Arc<VerifiedTransaction>> {
        self.pending_block
            .lock().transactions
            .iter()
            .find(|tx| tx.hash == *hash)
            .cloned()
            .map(|tx| Arc::new(VerifiedTransaction::from_pending_block_transaction(tx)))
    }

    fn remove_transaction(&self, hash: &H256) -> Option<Arc<VerifiedTransaction>> {
        let txs = &mut self.pending_block
            .lock().transactions;
        let pos = txs.iter().position(|tx| tx.hash == *hash)?;
        let tx = txs.remove(pos);
        Some(Arc::new(VerifiedTransaction::from_pending_block_transaction(tx)))
    }

    fn local_transactions(&self) -> BTreeMap<H256, LocalTransactionStatus> {
        self.local_transactions
            .lock()
            .iter()
            .map(|(hash, stats)| (*hash, stats.clone()))
            .collect()
    }

    /// Get an unfiltered list of all ready transactions.
    fn ready_transactions(
        &self,
        max_len: usize,
        ordering: PendingOrdering,
    ) -> Vec<Arc<VerifiedTransaction>>
    {
        self.ready_transactions_filtered(max_len, None, ordering)
    }
}

impl MinerService for TestMinerService {
    fn authoring_params(&self) -> AuthoringParams {
        self.authoring_params.read().clone()
    }

    /// Called when blocks are imported to chain, updates transactions queue.
    fn chain_new_blocks(
        &self,
        _imported: &[H256],
        _invalid: &[H256],
        _enacted: &[H256],
        _retracted: &[H256],
        _is_internal: bool,
    ) {
        unimplemented!();
    }

    /// New chain head event. Restart mining operation.
    fn update_sealing(&self, _force: ForceUpdateSealing) {
        unimplemented!();
    }

    fn sensible_gas_price(&self) -> U256 {
        20_000_000_000u64.into()
    }

    fn sensible_gas_limit(&self) -> U256 {
        0x5208.into()
    }

    fn set_minimal_gas_price(&self, gas_price: U256) -> Result<bool, &str> {
        let mut new_price = self.min_gas_price.write();
        match *new_price {
            Some(ref mut v) => {
                *v = gas_price;
                Ok(true)
            }
            None => {
                let error_msg =
                    "Can't update fixed gas price while automatic gas calibration is enabled.";
                Err(error_msg)
            }
        }
    }
}
