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

//! View onto transaction rlp

use std::cmp::min;

use crate::{
    bytes::Bytes,
    hash::keccak,
    transaction::{signature, TypedTxId},
    views::ViewRlp,
};

use ethereum_types::{H256, U256};
use rlp::Rlp;

/// View onto transaction rlp. Assumption is this is part of block.
/// Typed Transaction View. It handles raw bytes to search for particular field.
/// EIP1559 tx:
/// 2 | [chainId, nonce, maxPriorityFeePerGas, maxFeePerGas(gasPrice), gasLimit, to, value, data, access_list, senderV, senderR, senderS]
/// Access tx:
/// 1 | [chainId, nonce, gasPrice, gasLimit, to, value, data, access_list, senderV, senderR, senderS]
/// Legacy tx:
/// [nonce, gasPrice, gasLimit, to, value, data, senderV, senderR, senderS]
pub struct TypedTransactionView<'a> {
    rlp: ViewRlp<'a>,
    transaction_type: TypedTxId,
}
impl<'a> TypedTransactionView<'a> {
    /// Creates new view onto valid transaction rlp.
    /// Use the `view!` macro to create this view in order to capture debugging info.
    pub fn new(rlp: ViewRlp<'a>) -> TypedTransactionView<'a> {
        let transaction_type = Self::extract_transaction_type(&rlp.rlp);
        TypedTransactionView {
            rlp: rlp,
            transaction_type,
        }
    }

    /// Extract transaction type from rlp bytes.
    fn extract_transaction_type(rlp: &Rlp) -> TypedTxId {
        if rlp.is_list() {
            return TypedTxId::Legacy;
        }
        let tx = rlp.data().expect("unable to decode tx rlp");
        let id = TypedTxId::try_from_wire_byte(tx[0]).expect("unable to decode tx type");
        if id == TypedTxId::Legacy {
            panic!("Transaction RLP View should be valid. Legacy byte found");
        }
        id
    }

    /// Returns reference to transaction type.
    pub fn transaction_type(&self) -> &TypedTxId {
        &self.transaction_type
    }

    /// Returns transaction hash.
    pub fn hash(&self) -> H256 {
        match self.transaction_type {
            TypedTxId::Legacy => keccak(self.rlp.as_raw()),
            _ => keccak(self.rlp.rlp.data().unwrap()),
        }
    }

    /// Get chain Id field of the transaction.
    pub fn chain_id(&self) -> u64 {
        match self.transaction_type {
            TypedTxId::Legacy => {
                signature::extract_chain_id_from_legacy_v(self.rlp.val_at(6)).unwrap_or(0)
            }
            TypedTxId::AccessList => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(0),
            TypedTxId::EIP1559Transaction => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(0),
        }
    }

    /// Get the nonce field of the transaction.
    pub fn nonce(&self) -> U256 {
        match self.transaction_type {
            TypedTxId::Legacy => self.rlp.val_at(0),
            TypedTxId::AccessList => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(1),
            TypedTxId::EIP1559Transaction => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(1),
        }
    }

    /// Get the gas_price field of the transaction.
    pub fn gas_price(&self) -> U256 {
        match self.transaction_type {
            TypedTxId::Legacy => self.rlp.val_at(1),
            TypedTxId::AccessList => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(2),
            TypedTxId::EIP1559Transaction => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(3),
        }
    }

    /// Get the effective_gas_price field of the transaction.
    pub fn effective_gas_price(&self, block_base_fee: Option<U256>) -> U256 {
        match self.transaction_type {
            TypedTxId::Legacy => self.gas_price(),
            TypedTxId::AccessList => self.gas_price(),
            TypedTxId::EIP1559Transaction => {
                let max_priority_fee_per_gas: U256 =
                    view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                        .rlp
                        .val_at(2);

                min(
                    self.gas_price(),
                    max_priority_fee_per_gas + block_base_fee.unwrap_or_default(),
                )
            }
        }
    }

    /// Get the actual priority gas price paid to the miner
    pub fn effective_priority_gas_price(&self, block_base_fee: Option<U256>) -> U256 {
        match self.transaction_type {
            TypedTxId::Legacy => self
                .gas_price()
                .saturating_sub(block_base_fee.unwrap_or_default()),
            TypedTxId::AccessList => self
                .gas_price()
                .saturating_sub(block_base_fee.unwrap_or_default()),
            TypedTxId::EIP1559Transaction => {
                let max_priority_fee_per_gas: U256 =
                    view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                        .rlp
                        .val_at(2);
                min(
                    max_priority_fee_per_gas,
                    self.gas_price()
                        .saturating_sub(block_base_fee.unwrap_or_default()),
                )
            }
        }
    }

    /// Get the gas field of the transaction.
    pub fn gas(&self) -> U256 {
        match self.transaction_type {
            TypedTxId::Legacy => self.rlp.val_at(2),
            TypedTxId::AccessList => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(3),
            TypedTxId::EIP1559Transaction => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(4),
        }
    }

    /// Get the value field of the transaction.
    pub fn value(&self) -> U256 {
        match self.transaction_type {
            TypedTxId::Legacy => self.rlp.val_at(4),
            TypedTxId::AccessList => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(5),
            TypedTxId::EIP1559Transaction => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(6),
        }
    }

    /// Get the data field of the transaction.
    pub fn data(&self) -> Bytes {
        match self.transaction_type {
            TypedTxId::Legacy => self.rlp.val_at(5),
            TypedTxId::AccessList => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(6),
            TypedTxId::EIP1559Transaction => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(7),
        }
    }

    /// Get the v field of the transaction.
    pub fn legacy_v(&self) -> u8 {
        let r = match self.transaction_type {
            TypedTxId::Legacy => self.rlp.val_at(6),
            TypedTxId::AccessList => {
                let chain_id = match self.chain_id() {
                    0 => None,
                    n => Some(n),
                };
                signature::add_chain_replay_protection(
                    view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                        .rlp
                        .val_at(8),
                    chain_id,
                )
            }
            TypedTxId::EIP1559Transaction => {
                let chain_id = match self.chain_id() {
                    0 => None,
                    n => Some(n),
                };
                signature::add_chain_replay_protection(
                    view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                        .rlp
                        .val_at(9),
                    chain_id,
                )
            }
        };
        r as u8
    }

    pub fn standard_v(&self) -> u8 {
        match self.transaction_type {
            TypedTxId::Legacy => signature::extract_standard_v(self.rlp.val_at(6)),
            TypedTxId::AccessList => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(8),
            TypedTxId::EIP1559Transaction => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(9),
        }
    }

    /// Get the r field of the transaction.
    pub fn r(&self) -> U256 {
        match self.transaction_type {
            TypedTxId::Legacy => self.rlp.val_at(7),
            TypedTxId::AccessList => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(9),
            TypedTxId::EIP1559Transaction => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(10),
        }
    }

    /// Get the s field of the transaction.
    pub fn s(&self) -> U256 {
        match self.transaction_type {
            TypedTxId::Legacy => self.rlp.val_at(8),
            TypedTxId::AccessList => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(10),
            TypedTxId::EIP1559Transaction => view!(Self, &self.rlp.rlp.data().unwrap()[1..])
                .rlp
                .val_at(11),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TypedTransactionView;
    use rustc_hex::FromHex;

    #[test]
    fn test_transaction_view() {
        let rlp = "f87c80018261a894095e7baea6a6c7c4c2dfeb977efac326af552d870a9d00000000000000000000000000000000000000000000000000000000001ba048b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353a0efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c804".from_hex().unwrap();

        let view = view!(TypedTransactionView, &rlp);
        assert_eq!(view.nonce(), 0.into());
        assert_eq!(view.gas_price(), 1.into());
        assert_eq!(view.effective_gas_price(None), 1.into());
        assert_eq!(view.effective_priority_gas_price(None), 1.into());
        assert_eq!(view.gas(), 0x61a8.into());
        assert_eq!(view.value(), 0xa.into());
        assert_eq!(
            view.data(),
            "0000000000000000000000000000000000000000000000000000000000"
                .from_hex()
                .unwrap()
        );
        assert_eq!(
            view.r(),
            "48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353".into()
        );
        assert_eq!(
            view.s(),
            "efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c804".into()
        );
        assert_eq!(view.legacy_v(), 0x1b);
    }

    #[test]
    fn test_access_list_transaction_view() {
        let rlp = "b8c101f8be01010a8301e24194000000000000000000000000000000000000aaaa8080f85bf859940000000000000000000000000000000000000000f842a00000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000000080a082dc119130f280bd72e3fd4e10220e35b767031b84b8dd1f64085e0158f234dba072228551e678a8a6c6e9bae0ae786b8839c7fda0a994caddd23910f45f385cc0".from_hex().unwrap();
        let view = view!(TypedTransactionView, &rlp);
        assert_eq!(view.nonce(), 0x1.into());
        assert_eq!(view.gas_price(), 0xa.into());
        assert_eq!(view.effective_priority_gas_price(None), 0xa.into());
        assert_eq!(view.gas(), 0x1e241.into());
        assert_eq!(view.value(), 0x0.into());
        assert_eq!(view.data(), "".from_hex().unwrap());
        assert_eq!(
            view.r(),
            "82dc119130f280bd72e3fd4e10220e35b767031b84b8dd1f64085e0158f234db".into()
        );
        assert_eq!(
            view.s(),
            "72228551e678a8a6c6e9bae0ae786b8839c7fda0a994caddd23910f45f385cc0".into()
        );
        assert_eq!(view.standard_v(), 0x0);
    }

    #[test]
    fn test_eip1559_transaction_view() {
        let rlp = "b8c202f8bf0101010a8301e24194000000000000000000000000000000000000aaaa8080f85bf859940000000000000000000000000000000000000000f842a00000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000000080a082dc119130f280bd72e3fd4e10220e35b767031b84b8dd1f64085e0158f234dba072228551e678a8a6c6e9bae0ae786b8839c7fda0a994caddd23910f45f385cc0".from_hex().unwrap();
        let view = view!(TypedTransactionView, &rlp);
        assert_eq!(view.nonce(), 0x1.into());
        assert_eq!(view.gas_price(), 0xa.into());
        assert_eq!(view.effective_gas_price(Some(0x07.into())), 0x08.into());
        assert_eq!(
            view.effective_priority_gas_price(Some(0x07.into())),
            0x01.into()
        );
        assert_eq!(view.gas(), 0x1e241.into());
        assert_eq!(view.value(), 0x0.into());
        assert_eq!(view.data(), "".from_hex().unwrap());
        assert_eq!(
            view.r(),
            "82dc119130f280bd72e3fd4e10220e35b767031b84b8dd1f64085e0158f234db".into()
        );
        assert_eq!(
            view.s(),
            "72228551e678a8a6c6e9bae0ae786b8839c7fda0a994caddd23910f45f385cc0".into()
        );
        assert_eq!(view.standard_v(), 0x0);
    }
}
