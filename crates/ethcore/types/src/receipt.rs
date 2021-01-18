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

//! Receipt

use super::transaction::TypedTxId;
use ethereum_types::{Address, Bloom, H160, H256, U256};
use heapsize::HeapSizeOf;
use rlp::{DecoderError, Rlp, RlpStream};
use std::{
    convert::{TryFrom, TryInto},
    ops::{Deref, DerefMut},
};

use serde_repr::*;

use log_entry::{LocalizedLogEntry, LogEntry};
use BlockNumber;

/// Transaction outcome store in the receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionOutcome {
    /// Status and state root are unknown under EIP-98 rules.
    Unknown,
    /// State root is known. Pre EIP-98 and EIP-658 rules.
    StateRoot(H256),
    /// Status code is known. EIP-658 rules.
    StatusCode(u8),
}

/// Information describing execution of a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyReceipt {
    /// The total gas used in the block following execution of the transaction.
    pub gas_used: U256,
    /// The OR-wide combination of all logs' blooms for this transaction.
    pub log_bloom: Bloom,
    /// The logs stemming from this transaction.
    pub logs: Vec<LogEntry>,
    /// Transaction outcome.
    pub outcome: TransactionOutcome,
}

impl LegacyReceipt {
    pub fn new(outcome: TransactionOutcome, gas_used: U256, logs: Vec<LogEntry>) -> Self {
        LegacyReceipt {
            gas_used,
            log_bloom: logs.iter().fold(Bloom::default(), |mut b, l| {
                b.accrue_bloom(&l.bloom());
                b
            }),
            logs,
            outcome,
        }
    }
    pub fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        match rlp.item_count()? {
            3 => Ok(LegacyReceipt {
                outcome: TransactionOutcome::Unknown,
                gas_used: rlp.val_at(0)?,
                log_bloom: rlp.val_at(1)?,
                logs: rlp.list_at(2)?,
            }),
            4 => Ok(LegacyReceipt {
                gas_used: rlp.val_at(1)?,
                log_bloom: rlp.val_at(2)?,
                logs: rlp.list_at(3)?,
                outcome: {
                    let first = rlp.at(0)?;
                    if first.is_data() && first.data()?.len() <= 1 {
                        TransactionOutcome::StatusCode(first.as_val()?)
                    } else {
                        TransactionOutcome::StateRoot(first.as_val()?)
                    }
                },
            }),
            _ => Err(DecoderError::RlpIncorrectListLen),
        }
    }

    pub fn rlp_append(&self, s: &mut RlpStream) {
        match self.outcome {
            TransactionOutcome::Unknown => {
                s.begin_list(3);
            }
            TransactionOutcome::StateRoot(ref root) => {
                s.begin_list(4);
                s.append(root);
            }
            TransactionOutcome::StatusCode(ref status_code) => {
                s.begin_list(4);
                s.append(status_code);
            }
        }
        s.append(&self.gas_used);
        s.append(&self.log_bloom);
        s.append_list(&self.logs);
    }
}

#[derive(Serialize_repr, Eq, Hash, Deserialize_repr, Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum TypedReceiptId {
    Legacy = 0x00,
}

impl Default for TypedReceiptId {
    fn default() -> TypedReceiptId {
        TypedReceiptId::Legacy
    }
}

impl TryFrom<u8> for TypedReceiptId {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if (x & 0x80) != 0x00 => Ok(TypedReceiptId::Legacy),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedReceipt {
    Legacy(LegacyReceipt),
}

impl TypedReceipt {
    /// Create a new receipt.
    pub fn new(type_id: TypedTxId, legacy_receipt: LegacyReceipt) -> Self {
        //curently we are using same receipt for both legacy and typed transaction
        match type_id {
            TypedTxId::AccessList => Self::Legacy(legacy_receipt), // access list uses legacy receipt
            TypedTxId::Legacy => Self::Legacy(legacy_receipt),
        }
    }

    pub fn receipt_type(&self) -> TypedReceiptId {
        match self {
            Self::Legacy(_) => TypedReceiptId::Legacy,
        }
    }

    pub fn receipt(&self) -> &LegacyReceipt {
        match self {
            Self::Legacy(receipt) => receipt,
        }
    }

    pub fn receipt_mut(&mut self) -> &mut LegacyReceipt {
        match self {
            Self::Legacy(receipt) => receipt,
        }
    }

    fn decode(receipt: &[u8]) -> Result<Self, DecoderError> {
        if receipt.is_empty() {
            // at least one byte needs to be present
            return Err(DecoderError::RlpIncorrectListLen);
        }
        let id = receipt[0].try_into();
        if id.is_err() {
            return Err(DecoderError::Custom("Unknown receipt id"));
        }
        // other transaction types
        match id.unwrap() {
            TypedReceiptId::Legacy => Ok(Self::Legacy(LegacyReceipt::decode(&Rlp::new(receipt))?)),
        }
    }

    pub fn decode_rlp(rlp: &Rlp) -> Result<Self, DecoderError> {
        if rlp.is_list() {
            //legacy transaction wrapped around RLP encoding
            Ok(Self::Legacy(LegacyReceipt::decode(rlp)?))
        } else {
            Self::decode(rlp.data()?)
        }
    }

    pub fn decode_rlp_list(rlp: &Rlp) -> Result<Vec<Self>, DecoderError> {
        if !rlp.is_list() {
            // at least one byte needs to be present
            return Err(DecoderError::RlpIncorrectListLen);
        }
        let mut output = Vec::with_capacity(rlp.item_count()?);
        for tx in rlp.iter() {
            output.push(Self::decode_rlp(&tx)?);
        }
        Ok(output)
    }

    pub fn rlp_append(&self, s: &mut RlpStream) {
        match self {
            Self::Legacy(receipt) => receipt.rlp_append(s),
        }
    }

    pub fn rlp_append_list(s: &mut RlpStream, list: &[TypedReceipt]) {
        s.begin_list(list.len());
        for rec in list.iter() {
            rec.rlp_append(s)
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Legacy(receipt) => {
                let mut s = RlpStream::new();
                receipt.rlp_append(&mut s);
                s.drain()
            }
        }
    }
}

impl Deref for TypedReceipt {
    type Target = LegacyReceipt;

    fn deref(&self) -> &Self::Target {
        self.receipt()
    }
}

impl DerefMut for TypedReceipt {
    fn deref_mut(&mut self) -> &mut LegacyReceipt {
        self.receipt_mut()
    }
}

impl HeapSizeOf for TypedReceipt {
    fn heap_size_of_children(&self) -> usize {
        self.receipt().logs.heap_size_of_children()
    }
}

/// Receipt with additional info.
#[derive(Debug, Clone, PartialEq)]
pub struct RichReceipt {
    /// Transaction type
    pub receipt_type: TypedReceiptId,
    /// Transaction hash.
    pub transaction_hash: H256,
    /// Transaction index.
    pub transaction_index: usize,
    /// The total gas used in the block following execution of the transaction.
    pub cumulative_gas_used: U256,
    /// The gas used in the execution of the transaction. Note the difference of meaning to `Receipt::gas_used`.
    pub gas_used: U256,
    /// Contract address.
    /// NOTE: It is an Option because only `Action::Create` transactions has a contract address
    pub contract_address: Option<Address>,
    /// Logs
    pub logs: Vec<LogEntry>,
    /// Logs bloom
    pub log_bloom: Bloom,
    /// Transaction outcome.
    pub outcome: TransactionOutcome,
    /// Receiver address
    /// NOTE: It is an Option because only `Action::Call` transactions has a receiver address
    pub to: Option<H160>,
    /// Sender
    pub from: H160,
}

/// Receipt with additional info.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalizedReceipt {
    /// Transaction type
    pub receipt_type: TypedReceiptId,
    /// Transaction hash.
    pub transaction_hash: H256,
    /// Transaction index.
    pub transaction_index: usize,
    /// Block hash.
    pub block_hash: H256,
    /// Block number.
    pub block_number: BlockNumber,
    /// The total gas used in the block following execution of the transaction.
    pub cumulative_gas_used: U256,
    /// The gas used in the execution of the transaction. Note the difference of meaning to `Receipt::gas_used`.
    pub gas_used: U256,
    /// Contract address.
    /// NOTE: It is an Option because only `Action::Create` transactions has a contract address
    pub contract_address: Option<Address>,
    /// Logs
    pub logs: Vec<LocalizedLogEntry>,
    /// Logs bloom
    pub log_bloom: Bloom,
    /// Transaction outcome.
    pub outcome: TransactionOutcome,
    /// Receiver address
    /// NOTE: It is an Option because only `Action::Call` transactions has a receiver address
    pub to: Option<H160>,
    /// Sender
    pub from: H160,
}

#[cfg(test)]
mod tests {
    use super::{LegacyReceipt, TransactionOutcome, TypedReceipt, TypedTxId};
    use log_entry::LogEntry;

    #[test]
    fn test_no_state_root() {
        let expected = ::rustc_hex::FromHex::from_hex("f9014183040caeb9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000000f838f794dcf421d093428b096ca501a7cd1a740855a7976fc0a00000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let r = TypedReceipt::new(
            TypedTxId::Legacy,
            LegacyReceipt::new(
                TransactionOutcome::Unknown,
                0x40cae.into(),
                vec![LogEntry {
                    address: "dcf421d093428b096ca501a7cd1a740855a7976f".into(),
                    topics: vec![],
                    data: vec![0u8; 32],
                }],
            ),
        );
        assert_eq!(r.encode(), expected);
    }

    #[test]
    fn test_basic_legacy() {
        let expected = ::rustc_hex::FromHex::from_hex("f90162a02f697d671e9ae4ee24a43c4b0d7e15f1cb4ba6de1561120d43b9a4e8c4a8a6ee83040caeb9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000000f838f794dcf421d093428b096ca501a7cd1a740855a7976fc0a00000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let r = TypedReceipt::new(
            TypedTxId::Legacy,
            LegacyReceipt::new(
                TransactionOutcome::StateRoot(
                    "2f697d671e9ae4ee24a43c4b0d7e15f1cb4ba6de1561120d43b9a4e8c4a8a6ee".into(),
                ),
                0x40cae.into(),
                vec![LogEntry {
                    address: "dcf421d093428b096ca501a7cd1a740855a7976f".into(),
                    topics: vec![],
                    data: vec![0u8; 32],
                }],
            ),
        );
        let encoded = r.encode();
        assert_eq!(encoded, expected);
        let decoded = TypedReceipt::decode(&encoded).expect("decoding receipt failed");
        assert_eq!(decoded, r);
    }

    #[test]
    fn test_status_code() {
        let expected = ::rustc_hex::FromHex::from_hex("f901428083040caeb9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000000f838f794dcf421d093428b096ca501a7cd1a740855a7976fc0a00000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let r = TypedReceipt::new(
            TypedTxId::Legacy,
            LegacyReceipt::new(
                TransactionOutcome::StatusCode(0),
                0x40cae.into(),
                vec![LogEntry {
                    address: "dcf421d093428b096ca501a7cd1a740855a7976f".into(),
                    topics: vec![],
                    data: vec![0u8; 32],
                }],
            ),
        );
        let encoded = r.encode();
        assert_eq!(&encoded[..], &expected[..]);
        let decoded = TypedReceipt::decode(&encoded).expect("decoding receipt failed");
        assert_eq!(decoded, r);
    }
}
