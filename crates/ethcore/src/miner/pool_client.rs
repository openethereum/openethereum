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

//! Blockchain access for transaction pool.

use std::fmt;

use ethcore_miner::{
    local_accounts::LocalAccounts, pool, pool::client::NonceClient,
    service_transaction_checker::ServiceTransactionChecker,
};
use ethereum_types::{Address, H256, U256};
use types::{
    header::Header,
    transaction::{self, SignedTransaction, UnverifiedTransaction},
};

use call_contract::CallContract;
use client::{Balance, BlockId, BlockInfo, Nonce, TransactionId};
use engines::EthEngine;
use ethcore_miner::pool::client::BalanceClient;
use miner::{
    self,
    cache::{Cache, CachedClient},
};
use transaction_ext::Transaction;

pub(crate) struct CachedNonceClient<'a, C: 'a> {
    cached_client: CachedClient<'a, C, Address, U256>,
}

impl<'a, C: 'a> CachedNonceClient<'a, C> {
    pub fn new(client: &'a C, cache: &'a Cache<Address, U256>) -> Self {
        Self {
            cached_client: CachedClient::new(client, cache),
        }
    }
}

impl<'a, C: 'a> Clone for CachedNonceClient<'a, C> {
    fn clone(&self) -> Self {
        Self {
            cached_client: self.cached_client.clone(),
        }
    }
}

impl<'a, C: 'a> fmt::Debug for CachedNonceClient<'a, C> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("CachedNonceClient")
            .field("cached_client", &self.cached_client)
            .finish()
    }
}

impl<'a, C: 'a> NonceClient for CachedNonceClient<'a, C>
where
    C: Nonce + Sync,
{
    fn account_nonce(&self, address: &Address) -> U256 {
        self.cached_client.cache().get_or_insert(*address, || {
            self.cached_client.client().latest_nonce(address)
        })
    }
}

pub(crate) struct CachedBalanceClient<'a, C: 'a> {
    cached_client: CachedClient<'a, C, Address, U256>,
}

impl<'a, C: 'a> CachedBalanceClient<'a, C> {
    pub fn new(client: &'a C, cache: &'a Cache<Address, U256>) -> Self {
        Self {
            cached_client: CachedClient::new(client, cache),
        }
    }
}

impl<'a, C: 'a> Clone for CachedBalanceClient<'a, C> {
    fn clone(&self) -> Self {
        Self {
            cached_client: self.cached_client.clone(),
        }
    }
}

impl<'a, C: 'a> fmt::Debug for CachedBalanceClient<'a, C> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("CachedBalanceClient")
            .field("cached_client", &self.cached_client)
            .finish()
    }
}

impl<'a, C: 'a> BalanceClient for CachedBalanceClient<'a, C>
where
    C: Balance + Sync,
{
    fn account_balance(&self, address: &ethereum_types::H160) -> U256 {
        self.cached_client.cache().get_or_insert(*address, || {
            self.cached_client.client().latest_balance(address)
        })
    }
}

/// Blockchain accesss for transaction pool.
pub struct PoolClient<'a, C: 'a> {
    chain: &'a C,
    cached_nonces: CachedNonceClient<'a, C>,
    cached_balances: CachedBalanceClient<'a, C>,
    engine: &'a dyn EthEngine,
    accounts: &'a dyn LocalAccounts,
    best_block_header: Header,
    service_transaction_checker: Option<&'a ServiceTransactionChecker>,
}

impl<'a, C: 'a> Clone for PoolClient<'a, C> {
    fn clone(&self) -> Self {
        PoolClient {
            chain: self.chain,
            cached_nonces: self.cached_nonces.clone(),
            cached_balances: self.cached_balances.clone(),
            engine: self.engine,
            accounts: self.accounts.clone(),
            best_block_header: self.best_block_header.clone(),
            service_transaction_checker: self.service_transaction_checker.clone(),
        }
    }
}

impl<'a, C: 'a> PoolClient<'a, C>
where
    C: BlockInfo + CallContract,
{
    /// Creates new client given chain, nonce cache, accounts and service transaction verifier.
    pub fn new(
        chain: &'a C,
        cached_nonces: &'a Cache<Address, U256>,
        cached_balances: &'a Cache<Address, U256>,
        engine: &'a dyn EthEngine,
        accounts: &'a dyn LocalAccounts,
        service_transaction_checker: Option<&'a ServiceTransactionChecker>,
    ) -> Self {
        let best_block_header = chain.best_block_header();
        PoolClient {
            chain,
            cached_nonces: CachedNonceClient::new(chain, cached_nonces),
            cached_balances: CachedBalanceClient::new(chain, cached_balances),
            engine,
            accounts,
            best_block_header,
            service_transaction_checker,
        }
    }

    /// Verifies transaction against its block (before its import into this block)
    /// Also Verifies if signed transaction is executable.
    ///
    /// This should perform any verifications that rely on chain status.
    pub fn verify_for_pending_block(
        &self,
        tx: &SignedTransaction,
        header: &Header,
    ) -> Result<(), transaction::Error> {
        self.engine.machine().verify_transaction_basic(tx, header)?;
        self.engine
            .machine()
            .verify_transaction(tx, &self.best_block_header, self.chain)
    }
}

impl<'a, C: 'a> fmt::Debug for PoolClient<'a, C> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "PoolClient")
    }
}

impl<'a, C: 'a> pool::client::Client for PoolClient<'a, C>
where
    C: miner::TransactionVerifierClient + Sync,
{
    fn transaction_already_included(&self, hash: &H256) -> bool {
        self.chain
            .transaction_block(TransactionId::Hash(*hash))
            .is_some()
    }

    fn verify_transaction_basic(
        &self,
        tx: &UnverifiedTransaction,
    ) -> Result<(), transaction::Error> {
        self.engine
            .verify_transaction_basic(tx, &self.best_block_header)?;
        Ok(())
    }

    fn verify_transaction(
        &self,
        tx: UnverifiedTransaction,
    ) -> Result<SignedTransaction, transaction::Error> {
        self.engine
            .verify_transaction_basic(&tx, &self.best_block_header)?;

        let tx = SignedTransaction::new(tx)?;

        self.engine
            .machine()
            .verify_transaction(&tx, &self.best_block_header, self.chain)?;
        Ok(tx)
    }

    fn account_details(&self, address: &Address) -> pool::client::AccountDetails {
        pool::client::AccountDetails {
            nonce: self.cached_nonces.account_nonce(address),
            balance: self.cached_balances.account_balance(address),
            code_hash: self.chain.code_hash(address, BlockId::Latest),
            is_local: self.accounts.is_local(address),
        }
    }

    fn required_gas(&self, tx: &transaction::Transaction) -> U256 {
        tx.gas_required(&self.chain.latest_schedule()).into()
    }

    fn transaction_type(&self, tx: &SignedTransaction) -> pool::client::TransactionType {
        match self.service_transaction_checker {
            None => pool::client::TransactionType::Regular,
            Some(ref checker) => match checker.check(self.chain, &tx) {
                Ok(true) => pool::client::TransactionType::Service,
                Ok(false) => pool::client::TransactionType::Regular,
                Err(e) => {
                    debug!(target: "txqueue", "Unable to verify service transaction: {:?}", e);
                    pool::client::TransactionType::Regular
                }
            },
        }
    }

    fn decode_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<UnverifiedTransaction, transaction::Error> {
        let number = self.chain.best_block_header().number();
        self.engine.decode_transaction(transaction, number)
    }
}

impl<'a, C: 'a> NonceClient for PoolClient<'a, C>
where
    C: Nonce + Sync,
{
    fn account_nonce(&self, address: &Address) -> U256 {
        self.cached_nonces.account_nonce(address)
    }
}

impl<'a, C: 'a> BalanceClient for PoolClient<'a, C>
where
    C: Balance + Sync,
{
    fn account_balance(&self, address: &ethereum_types::H160) -> U256 {
        self.cached_balances.account_balance(address)
    }
}
