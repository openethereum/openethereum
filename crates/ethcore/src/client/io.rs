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

use std::sync::{Arc, atomic::AtomicI64};

use super::Client;
use crate::{
    error::{BlockError, ErrorKind, EthcoreResult, ImportErrorKind, QueueErrorKind},
    types::transaction::UnverifiedTransaction,
    verification::queue::kind::blocks::Unverified,
};
use blockchain::BlockProvider;
use bytes::Bytes;
use client::info::BlockInfo;
use ethereum_types::H256;
use miner::MinerService;
use verification::queue::kind::BlockLike;

const ANCIENT_BLOCKS_QUEUE_SIZE: usize = 4096;

/// IO operations that should off-load heavy work to another thread.
pub trait IoClient: Sync + Send {
    /// Queue transactions for importing.
    fn queue_transactions(&self, transactions: Vec<Bytes>, peer_id: usize);

    /// Queue block import with transaction receipts. Does no sealing and transaction validation.
    fn queue_ancient_block(
        &self,
        block_bytes: Unverified,
        receipts_bytes: Bytes,
    ) -> EthcoreResult<H256>;

    /// Return percentage of how full is queue that handles ancient blocks. 0 if empty, 1 if full.
    fn ancient_block_queue_fullness(&self) -> f32;

    /// Queue conensus engine message.
    fn queue_consensus_message(&self, message: Bytes);
}

impl IoClient for Client {
    fn queue_transactions(&self, transactions: Vec<Bytes>, peer_id: usize) {
        trace_time!("queue_transactions");
        let len = transactions.len();
        self.queue_transactions
            .queue(&self.io_channel.read(), len, move |client| {
                trace_time!("import_queued_transactions");
                let best_block_number = client.best_block_header().number();
                let txs: Vec<UnverifiedTransaction> = transactions
                    .iter()
                    .filter_map(|bytes| {
                        client
                            .engine()
                            .decode_transaction(bytes, best_block_number)
                            .ok()
                    })
                    .collect();

                client.notify(|notify| {
                    notify.transactions_received(&txs, peer_id);
                });

                client
                    .importer
                    .miner
                    .import_external_transactions(client, txs);
            })
            .unwrap_or_else(|e| {
                debug!(target: "client", "Ignoring {} transactions: {}", len, e);
            });
    }

    fn queue_ancient_block(
        &self,
        unverified: Unverified,
        receipts_bytes: Bytes,
    ) -> EthcoreResult<H256> {
        trace_time!("queue_ancient_block");

        let hash = unverified.hash();
        {
            // check block order
            if self.chain.read().is_known(&hash) {
                bail!(ErrorKind::Import(ImportErrorKind::AlreadyInChain));
            }
            let parent_hash = unverified.parent_hash();
            // NOTE To prevent race condition with import, make sure to check queued blocks first
            // (and attempt to acquire lock)
            let is_parent_pending = self.queued_ancient_blocks.read().contains(&parent_hash);
            if !is_parent_pending && !self.chain.read().is_known(&parent_hash) {
                bail!(ErrorKind::Block(BlockError::UnknownParent(parent_hash)));
            }
        }

        // we queue blocks here and trigger an Executer.
        {
            let mut queued = self.queued_ancient_blocks.write();
            queued.insert(hash);
        }

        // see content of executer in Client::new()
        match self.queued_ancient_blocks_executer.lock().as_ref() {
            Some(queue) => {
                if !queue.enqueue((unverified, receipts_bytes)) {
                    bail!(ErrorKind::Queue(QueueErrorKind::Full(
                        ANCIENT_BLOCKS_QUEUE_SIZE
                    )));
                }
            }
            None => (),
        }
        Ok(hash)
    }

    fn ancient_block_queue_fullness(&self) -> f32 {
        match self.queued_ancient_blocks_executer.lock().as_ref() {
            Some(queue) => queue.len() as f32 / ANCIENT_BLOCKS_QUEUE_SIZE as f32,
            None => 1.0, //return 1.0 if queue is not set
        }
    }

    fn queue_consensus_message(&self, message: Bytes) {
        match self
            .queue_consensus_message
            .queue(&self.io_channel.read(), 1, move |client| {
                if let Err(e) = client.engine().handle_message(&message) {
                    debug!(target: "poa", "Invalid message received: {}", e);
                }
            }) {
            Ok(_) => (),
            Err(e) => {
                debug!(target: "poa", "Ignoring the message, error queueing: {}", e);
            }
        }
    }
}
