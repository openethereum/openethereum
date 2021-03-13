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

//! Log entry type definition.

use crate::{bytes::Bytes, BlockNumber};
use ethereum_types::{Address, Bloom, BloomInput, H256};
use parity_util_mem::MallocSizeOf;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::Deref;

/// A record of execution for a `LOG` operation.
#[serde(rename_all = "camelCase")]
#[derive(
    Default,
    Debug,
    Clone,
    PartialEq,
    Eq,
    RlpEncodable,
    RlpDecodable,
    MallocSizeOf,
    Deserialize,
    Serialize,
)]
pub struct LogEntry {
    /// The address of the contract executing at the point of the `LOG` operation.
    pub address: Address,
    /// The topics associated with the `LOG` operation.
    pub topics: Vec<H256>,
    /// The data associated with the `LOG` operation.
    #[serde(
        serialize_with = "serialize_bytes",
        deserialize_with = "deserialize_bytes"
    )]
    pub data: Bytes,
}

fn deserialize_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let hexstr = String::deserialize(deserializer)?;
    Ok(hex::decode(&hexstr[2..]).unwrap())
}

pub fn serialize_bytes<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    format!("0x{}", hex::encode(bytes)).serialize(serializer)
}

impl LogEntry {
    /// Calculates the bloom of this log entry.
    pub fn bloom(&self) -> Bloom {
        self.topics.iter().fold(
            Bloom::from(BloomInput::Raw(self.address.as_bytes())),
            |mut b, t| {
                b.accrue(BloomInput::Raw(t.as_bytes()));
                b
            },
        )
    }
}

/// Log localized in a blockchain.
#[derive(Default, Debug, PartialEq, Clone)]
pub struct LocalizedLogEntry {
    /// Plain log entry.
    pub entry: LogEntry,
    /// Block in which this log was created.
    pub block_hash: H256,
    /// Block number.
    pub block_number: BlockNumber,
    /// Hash of transaction in which this log was created.
    pub transaction_hash: H256,
    /// Index of transaction within block.
    pub transaction_index: usize,
    /// Log position in the block.
    pub log_index: usize,
    /// Log position in the transaction.
    pub transaction_log_index: usize,
}

impl Deref for LocalizedLogEntry {
    type Target = LogEntry;

    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

#[cfg(test)]
mod tests {
    use super::LogEntry;
    use ethereum_types::{Address, Bloom};

    #[test]
    fn test_empty_log_bloom() {
        let bloom = "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".parse::<Bloom>().unwrap();
        let address = "0f572e5295c57f15886f9b263e2f6d2d6c7b5ec6"
            .parse::<Address>()
            .unwrap();
        let log = LogEntry {
            address: address,
            topics: vec![],
            data: vec![],
        };
        assert_eq!(log.bloom(), bloom);
    }

    #[test]
    fn test_data_hex_serialize() {
        let entry = LogEntry {
            address: Address::zero(),
            topics: vec![],
            data: vec![0, 0, 0, 0, 0, 1, 0],
        };
        let serialized = serde_json::to_string(&entry).unwrap();
        assert_eq!(
            serialized,
            r#"{"address":"0x0000000000000000000000000000000000000000","topics":[],"data":"0x00000000000100"}"#
        );
    }

    #[test]
    fn test_data_hex_deserialize() {
        let serialized = r#"{"address":"0x0000000000000000000000000000000000000000","topics":[],"data":"0x00000000000100"}"#;
        let deserialized: LogEntry = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.data, vec![0, 0, 0, 0, 0, 1, 0]);
    }
}
