use bytes::Bytes;
use ethereum_types::{Address, Bloom, H256, U256, U64};
use serde::{Deserialize, Serialize};

/// Execution block representation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct ExecutionPayload {
    /// Hash of the parent.
    pub parent_hash: H256,
    /// Recipient of priority fees (`beneficiary` in the yellow paper).
    pub fee_recipient: Address,
    /// State root hash.
    pub state_root: H256,
    /// Transactions receipts root hash.
    pub receipts_root: H256,
    /// Logs bloom.
    pub logs_bloom: Bloom,
    /// Randomness of the block (`difficulty` in the yellow paper).
    pub prev_randao: H256,
    /// Block number.
    pub block_number: U64,
    /// Gas limit.
    pub gas_limit: U64,
    /// Gas Used.
    pub gas_used: U64,
    /// Timestamp.
    pub timestamp: U64,
    /// Extra data.
    #[serde(with = "hex_bytes")]
    pub extra_data: Bytes,
    /// Base fee.
    pub base_fee_per_gas: U256,
    /// Hash of the block.
    pub block_hash: H256,
    /// Transactions.
    #[serde(with = "hex_bytes")]
    pub transactions: Bytes,
}

mod hex_bytes {
    use serde::{Deserializer, Serializer};

    use super::*;
    use serde::de::Error;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Bytes, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        Ok(hex::decode(s.strip_prefix("0x").unwrap_or(&s))
            .map_err(D::Error::custom)?
            .into())
    }

    pub fn serialize<S>(b: &Bytes, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{}", hex::encode(b)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_execution_payload_serialize_and_deserialize() {
        let s = r#"{"parentHash":"0x3b8fb240d288781d4aac94d3fd16809ee413bc99294a085798a589dae51ddd4a","feeRecipient":"0xa94f5374fce5edbc8e2a8697c15331677e6ebf0b","stateRoot":"0xca3149fa9e37db08d1cd49c9061db1002ef1cd58db2210f2115c8c989b2bdf45","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","prevRandao":"0x0000000000000000000000000000000000000000000000000000000000000001","blockNumber":"0x1","gasLimit":"0x1c95111","gasUsed":"0x0","timestamp":"0x5","extraData":"0x","baseFeePerGas":"0x7","blockHash":"0x6359b8381a370e2f54072a5784ddd78b6ed024991558c511d4452eb4f6ac898c","transactions":"0x"}"#;
        let execution_payload = ExecutionPayload {
            parent_hash: H256::from_str("3b8fb240d288781d4aac94d3fd16809ee413bc99294a085798a589dae51ddd4a").unwrap(),
            fee_recipient: Address::from_str("a94f5374fce5edbc8e2a8697c15331677e6ebf0b").unwrap(),
            state_root: H256::from_str("ca3149fa9e37db08d1cd49c9061db1002ef1cd58db2210f2115c8c989b2bdf45").unwrap(),
            receipts_root: H256::from_str("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421").unwrap(),
            logs_bloom: Bloom::from_str("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            prev_randao: H256::from_str("0000000000000000000000000000000000000000000000000000000000000001").unwrap(),
            block_number: 1.into(),
            gas_limit: 29970705.into(),
            gas_used: 0.into(),
            timestamp: 5.into(),
            extra_data: Bytes::new(),
            base_fee_per_gas: 7.into(),
            block_hash: H256::from_str("6359b8381a370e2f54072a5784ddd78b6ed024991558c511d4452eb4f6ac898c").unwrap(),
            transactions: Bytes::new()
        };

        let serialized = serde_json::to_string(&execution_payload).unwrap();
        assert_eq!(serialized, s, "Invalid serialization");

        let deserialized: ExecutionPayload = serde_json::from_str(s).unwrap();
        assert_eq!(deserialized, execution_payload, "Invalid deserialization");
    }
}
