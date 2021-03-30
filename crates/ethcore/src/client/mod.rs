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

//! Blockchain database client.

mod ancient_import;
mod bad_blocks;
mod client;
mod config;
#[cfg(any(test, feature = "test-helpers"))]
mod evm_test_client;
mod io_message;
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_client;
mod trace;

// internal client components
mod call;
mod importer;
mod io;
mod prometheus;

/// types like block, tx, chain info.
pub mod info;

/// types describing a blockchain client
pub mod blockchain;

#[cfg(any(test, feature = "test-helpers"))]
pub use self::evm_test_client::{EvmTestClient, EvmTestError, TransactErr, TransactSuccess};
#[cfg(any(test, feature = "test-helpers"))]
pub use self::test_client::{EachBlockWith, TestBlockChainClient};
pub use self::{
    blockchain::{BlockChainClient, BlockChainReset},
    chain_notify::{ChainMessageType, ChainNotify, ChainRoute, ChainRouteType, NewBlocks},
    client::*,
    config::{BlockChainConfig, ClientConfig, DatabaseCompactionProfile, VMType},
    info::{BlockChain, BlockInfo, ChainInfo, EngineInfo, ScheduleInfo, TransactionInfo},
    io::IoClient,
    io_message::ClientIoMessage,
    traits::{
        AccountData, BadBlocks, Balance, BlockProducer, BroadcastProposalBlock, Call, EngineClient,
        ImportBlock, ImportExportBlocks, ImportSealedBlock, Nonce, PrepareOpenBlock, ReopenBlock,
        SealedBlockImporter, StateClient, StateOrBlock,
    },
};
pub use state::StateInfo;

pub use types::{
    call_analytics::CallAnalytics, ids::*, pruning_info::PruningInfo,
    trace_filter::Filter as TraceFilter,
};

pub use executive::{Executed, Executive, TransactOptions};
pub use vm::{EnvInfo, LastHashes};

pub use error::TransactionImportError;
pub use verification::VerifierType;

pub mod traits;

mod chain_notify;
