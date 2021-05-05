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

use accounts::AccountProvider;
use bytes::Bytes;
use crypto::{publickey::Signature, DEFAULT_MAC};
use ethereum_types::{Address, H256, U256};
use jsonrpc_core::{Error, ErrorCode};
use types::transaction::{
    AccessListTx, Action, EIP1559TransactionTx, SignedTransaction, Transaction, TypedTransaction,
    TypedTxId,
};

use jsonrpc_core::Result;
use v1::helpers::{errors, FilledTransactionRequest};

use super::{eth_data_hash, SignMessage, SignWith, WithToken};

/// Account-aware signer
pub struct Signer {
    accounts: Arc<AccountProvider>,
}

impl Signer {
    /// Create new instance of signer
    pub fn new(accounts: Arc<AccountProvider>) -> Self {
        Signer { accounts }
    }
}

impl super::Accounts for Signer {
    fn sign_transaction(
        &self,
        filled: FilledTransactionRequest,
        chain_id: Option<u64>,
        nonce: U256,
        password: SignWith,
    ) -> Result<WithToken<SignedTransaction>> {
        let mut legacy_tx = Transaction {
            nonce,
            action: filled.to.map_or(Action::Create, Action::Call),
            gas: filled.gas,
            gas_price: filled.gas_price.unwrap_or_default(),
            value: filled.value,
            data: filled.data,
        };
        let t = match TypedTxId::from_U64_option_id(filled.transaction_type) {
            Some(TypedTxId::Legacy) => TypedTransaction::Legacy(legacy_tx),
            Some(TypedTxId::AccessList) => {
                if filled.access_list.is_none() {
                    return Err(Error::new(ErrorCode::InvalidParams));
                }
                TypedTransaction::AccessList(AccessListTx::new(
                    legacy_tx,
                    filled
                        .access_list
                        .unwrap_or_default()
                        .into_iter()
                        .map(Into::into)
                        .collect(),
                ))
            }
            Some(TypedTxId::EIP1559Transaction) => {
                if let Some(max_fee_per_gas) = filled.max_fee_per_gas {
                    legacy_tx.gas_price = max_fee_per_gas;
                } else {
                    return Err(Error::new(ErrorCode::InvalidParams));
                }

                if let Some(max_priority_fee_per_gas) = filled.max_priority_fee_per_gas {
                    let transaction = AccessListTx::new(
                        legacy_tx,
                        filled
                            .access_list
                            .unwrap_or_default()
                            .into_iter()
                            .map(Into::into)
                            .collect(),
                    );
                    TypedTransaction::EIP1559Transaction(EIP1559TransactionTx {
                        transaction,
                        max_priority_fee_per_gas,
                    })
                } else {
                    return Err(Error::new(ErrorCode::InvalidParams));
                }
            }
            None => return Err(Error::new(ErrorCode::InvalidParams)),
        };

        let hash = t.signature_hash(chain_id);
        let signature = signature(&*self.accounts, filled.from, hash, password)?;

        Ok(signature.map(|sig| {
            SignedTransaction::new(t.with_signature(sig, chain_id))
				.expect("Transaction was signed by AccountsProvider; it never produces invalid signatures; qed")
        }))
    }

    fn sign_message(
        &self,
        address: Address,
        password: SignWith,
        hash: SignMessage,
    ) -> Result<WithToken<Signature>> {
        match hash {
            SignMessage::Data(data) => {
                let hash = eth_data_hash(data);
                signature(&self.accounts, address, hash, password)
            }
            SignMessage::Hash(hash) => signature(&self.accounts, address, hash, password),
        }
    }

    fn decrypt(
        &self,
        address: Address,
        password: SignWith,
        data: Bytes,
    ) -> Result<WithToken<Bytes>> {
        match password.clone() {
            SignWith::Nothing => self
                .accounts
                .decrypt(address, None, &DEFAULT_MAC, &data)
                .map(WithToken::No),
            SignWith::Password(pass) => self
                .accounts
                .decrypt(address, Some(pass), &DEFAULT_MAC, &data)
                .map(WithToken::No),
            SignWith::Token(token) => self
                .accounts
                .decrypt_with_token(address, token, &DEFAULT_MAC, &data)
                .map(Into::into),
        }
        .map_err(|e| match password {
            SignWith::Nothing => errors::signing(e),
            _ => errors::password(e),
        })
    }

    fn supports_prospective_signing(&self, address: &Address, password: &SignWith) -> bool {
        // If the account is permanently unlocked we can try to sign
        // using prospective nonce. This should speed up sending
        // multiple subsequent transactions in multi-threaded RPC environment.
        let is_unlocked_permanently = self.accounts.is_unlocked_permanently(address);
        let has_password = password.is_password();

        is_unlocked_permanently || has_password
    }

    fn default_account(&self) -> Address {
        self.accounts.default_account().ok().unwrap_or_default()
    }

    fn is_unlocked(&self, address: &Address) -> bool {
        self.accounts.is_unlocked(address)
    }
}

fn signature(
    accounts: &AccountProvider,
    address: Address,
    hash: H256,
    password: SignWith,
) -> Result<WithToken<Signature>> {
    match password.clone() {
        SignWith::Nothing => accounts.sign(address, None, hash).map(WithToken::No),
        SignWith::Password(pass) => accounts.sign(address, Some(pass), hash).map(WithToken::No),
        SignWith::Token(token) => accounts
            .sign_with_token(address, token, hash)
            .map(Into::into),
    }
    .map_err(|e| match password {
        SignWith::Nothing => errors::signing(e),
        _ => errors::password(e),
    })
}
