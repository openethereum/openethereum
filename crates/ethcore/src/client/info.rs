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

use std::sync::Arc;

use super::Client;
use crate::engines::EthEngine;

use blockchain::BlockProvider;
use ethereum_types::{Address, H256};
use evm::Schedule;
use types::{
    blockchain_info::BlockChainInfo,
    encoded,
    header::Header,
    ids::{BlockId, TransactionId},
};

/// Provides various blockchain information, like block header, chain state etc.
pub trait BlockChain: ChainInfo + BlockInfo + TransactionInfo {}
impl BlockChain for Client {}

/// Provides `chain_info` method
pub trait ChainInfo {
    /// Get blockchain information.
    fn chain_info(&self) -> BlockChainInfo;
}

impl ChainInfo for Client {
    fn chain_info(&self) -> BlockChainInfo {
        let mut chain_info = self.chain.read().chain_info();
        chain_info.pending_total_difficulty =
            chain_info.total_difficulty + self.importer.block_queue.total_difficulty();
        chain_info
    }
}

/// Provides various information on a block by it's ID
pub trait BlockInfo {
    /// Get raw block header data by block id.
    fn block_header(&self, id: BlockId) -> Option<encoded::Header>;

    /// Get the best block header.
    fn best_block_header(&self) -> Header;

    /// Get raw block data by block header hash.
    fn block(&self, id: BlockId) -> Option<encoded::Block>;

    /// Get address code hash at given block's state.
    fn code_hash(&self, address: &Address, id: BlockId) -> Option<H256>;
}

impl BlockInfo for Client {
    fn block_header(&self, id: BlockId) -> Option<encoded::Header> {
        let chain = self.chain.read();

        Self::block_hash(&chain, id).and_then(|hash| chain.block_header_data(&hash))
    }

    fn best_block_header(&self) -> Header {
        self.chain.read().best_block_header()
    }

    fn block(&self, id: BlockId) -> Option<encoded::Block> {
        let chain = self.chain.read();

        Self::block_hash(&chain, id).and_then(|hash| chain.block(&hash))
    }

    fn code_hash(&self, address: &Address, id: BlockId) -> Option<H256> {
        self.state_at(id)
            .and_then(|s| s.code_hash(address).unwrap_or(None))
    }
}

/// Provides various information on a transaction by it's ID
pub trait TransactionInfo {
    /// Get the hash of block that contains the transaction, if any.
    fn transaction_block(&self, id: TransactionId) -> Option<H256>;
}

impl TransactionInfo for Client {
    fn transaction_block(&self, id: TransactionId) -> Option<H256> {
        self.transaction_address(id).map(|addr| addr.block_hash)
    }
}

/// Provides `engine` method
pub trait EngineInfo {
    /// Get underlying engine object
    fn engine(&self) -> Arc<dyn EthEngine>;
}

impl EngineInfo for Client {
    fn engine(&self) -> Arc<dyn EthEngine> {
        self.engine()
    }
}

/// Provides `latest_schedule` method
pub trait ScheduleInfo {
    /// Returns latest schedule.
    fn latest_schedule(&self) -> Schedule;
}

impl ScheduleInfo for Client {
    fn latest_schedule(&self) -> Schedule {
        self.engine().schedule(self.latest_env_info().number)
    }
}
