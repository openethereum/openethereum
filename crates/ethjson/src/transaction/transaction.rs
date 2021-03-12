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

//! Transaction test transaction deserialization.

use crate::{bytes::Bytes, hash::Address, maybe::MaybeEmpty, uint::Uint};
use common_types::transaction::{
    signature, Action, SignatureComponents, Transaction as CoreTransaction, TypedTransaction,
    UnverifiedTransaction,
};
use ethereum_types::H256;

/// Transaction test transaction deserialization.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Transaction data.
    pub data: Bytes,
    /// Gas limit.
    pub gas_limit: Uint,
    /// Gas price.
    pub gas_price: Uint,
    /// Nonce.
    pub nonce: Uint,
    /// To.
    pub to: MaybeEmpty<Address>,
    /// Value.
    pub value: Uint,
    /// R.
    pub r: Uint,
    /// S.
    pub s: Uint,
    /// V.
    pub v: Uint,
}

impl From<Transaction> for UnverifiedTransaction {
    fn from(t: Transaction) -> Self {
        let to: Option<Address> = t.to.into();
        UnverifiedTransaction {
            unsigned: TypedTransaction::Legacy(CoreTransaction {
                nonce: t.nonce.into(),
                gas_price: t.gas_price.into(),
                gas: t.gas_limit.into(),
                action: match to {
                    Some(to) => Action::Call(to.into()),
                    None => Action::Create,
                },
                value: t.value.into(),
                data: t.data.into(),
            }),
            chain_id: signature::extract_chain_id_from_legacy_v(t.v.into()),
            signature: SignatureComponents {
                r: t.r.into(),
                s: t.s.into(),
                standard_v: signature::extract_standard_v(t.v.into()),
            },
            hash: H256::zero(),
        }
        .compute_hash()
    }
}

#[cfg(test)]
mod tests {
    use super::Transaction;
    use serde_json;

    #[test]
    fn transaction_deserialization() {
        let s = r#"{
			"data" : "0x",
			"gasLimit" : "0xf388",
			"gasPrice" : "0x09184e72a000",
			"nonce" : "0x00",
			"r" : "0x2c",
			"s" : "0x04",
			"to" : "",
			"v" : "0x1b",
			"value" : "0x00"
		}"#;
        let _deserialized: Transaction = serde_json::from_str(s).unwrap();
        // TODO: validate all fields
    }
}
