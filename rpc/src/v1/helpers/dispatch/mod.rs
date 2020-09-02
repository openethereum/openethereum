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

//! Utilities and helpers for transaction dispatch.

mod full;

pub use self::full::FullDispatcher;
pub use v1::helpers::nonce::Reservations;

use bytes::Bytes;
use ethcore::{client::BlockChainClient, miner::MinerService};
use ethereum_types::{Address, H256, U256};
use hash::keccak;
use types::transaction::PendingTransaction;

use jsonrpc_core::{BoxFuture, Result};
use v1::helpers::{FilledTransactionRequest, TransactionRequest};

/// Has the capability to dispatch, sign, and decrypt.
///
/// Requires a clone implementation, with the implication that it be cheap;
/// usually just bumping a reference count or two.
pub trait Dispatcher: Send + Sync + Clone {
    // TODO: when ATC exist, use zero-cost
    // type Out<T>: IntoFuture<T, Error>

    /// Fill optional fields of a transaction request, fetching gas price but not nonce.
    fn fill_optional_fields(
        &self,
        request: TransactionRequest,
        default_sender: Address,
        force_nonce: bool,
    ) -> BoxFuture<FilledTransactionRequest>;

    /// "Dispatch" a local transaction.
    fn dispatch_transaction(&self, signed_transaction: PendingTransaction) -> Result<H256>;
}

/// Returns a eth_sign-compatible hash of data to sign.
/// The data is prepended with special message to prevent
/// malicious DApps from using the function to sign forged transactions.
pub fn eth_data_hash(mut data: Bytes) -> H256 {
    let mut message_data = format!("\x19Ethereum Signed Message:\n{}", data.len()).into_bytes();
    message_data.append(&mut data);
    keccak(message_data)
}

/// Extract the default gas price from a client and miner.
pub fn default_gas_price<C, M>(client: &C, miner: &M, percentile: usize) -> U256
where
    C: BlockChainClient,
    M: MinerService,
{
    client
        .gas_price_corpus(100)
        .percentile(percentile)
        .cloned()
        .unwrap_or_else(|| miner.sensible_gas_price())
}
