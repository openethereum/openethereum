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

use std::cmp::min;
use types::transaction::{
    AccessListTx, Action, EIP1559TransactionTx, SignedTransaction, Transaction, TypedTransaction,
    TypedTxId,
};

use ethereum_types::U256;
use jsonrpc_core::{Error, ErrorCode};
use v1::helpers::CallRequest;

pub fn sign_call(request: CallRequest) -> Result<SignedTransaction, Error> {
    let max_gas = U256::from(500_000_000);
    let gas = min(request.gas.unwrap_or(max_gas), max_gas);
    let from = request.from.unwrap_or_default();
    let mut tx_legacy = Transaction {
        nonce: request.nonce.unwrap_or_default(),
        action: request.to.map_or(Action::Create, Action::Call),
        gas,
        gas_price: request.gas_price.unwrap_or_default(),
        value: request.value.unwrap_or_default(),
        data: request.data.unwrap_or_default(),
    };
    let tx_typed = match TypedTxId::from_U64_option_id(request.transaction_type) {
        Some(TypedTxId::Legacy) => TypedTransaction::Legacy(tx_legacy),
        Some(TypedTxId::AccessList) => {
            if request.access_list.is_none() {
                return Err(Error::new(ErrorCode::InvalidParams));
            }
            TypedTransaction::AccessList(AccessListTx::new(
                tx_legacy,
                request
                    .access_list
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            ))
        }
        Some(TypedTxId::EIP1559Transaction) => {
            tx_legacy.gas_price = request.max_fee_per_gas.unwrap_or_default();

            let transaction = AccessListTx::new(
                tx_legacy,
                request
                    .access_list
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            );

            TypedTransaction::EIP1559Transaction(EIP1559TransactionTx {
                transaction,
                max_priority_fee_per_gas: request.max_priority_fee_per_gas.unwrap_or_default(),
            })
        }
        _ => return Err(Error::new(ErrorCode::InvalidParams)),
    };
    Ok(tx_typed.fake_sign(from))
}
