use ethereum_types::{H256, U256, U64};
use serde::{Deserialize, Serialize};

/// Configurable settings of the transition process.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct TransitionConfiguration {
    /// Maps on the `TERMINAL_TOTAL_DIFFICULTY` parameter of EIP-3675.
    pub terminal_total_difficulty: U256,
    /// Maps on `TERMINAL_BLOCK_HASH` parameter of EIP-3675.
    pub terminal_block_hash: H256,
    /// Maps on `TERMINAL_BLOCK_NUMBER` parameter of EIP-3675.
    pub terminal_block_number: U64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_transition_configuration_serialize_and_deserialize() {
        let s = r#"{"terminalTotalDifficulty":"0xff","terminalBlockHash":"0x0d98f14f2a081328c81806658c0eae43c155568a895b11141bbbda07d0a30993","terminalBlockNumber":"0x10"}"#;
        let transition_configuration = TransitionConfiguration {
            terminal_total_difficulty: 255.into(),
            terminal_block_hash: H256::from_str(
                "0d98f14f2a081328c81806658c0eae43c155568a895b11141bbbda07d0a30993",
            )
            .unwrap(),
            terminal_block_number: 16.into(),
        };

        let serialized = serde_json::to_string(&transition_configuration).unwrap();
        assert_eq!(serialized, s, "Invalid serialization");

        let deserialized: TransitionConfiguration = serde_json::from_str(s).unwrap();
        assert_eq!(deserialized, transition_configuration);
    }
}
