use ethereum_types::H256;
use serde::Deserialize;

/// Encapsulates the fork choice state.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(PartialEq, Eq))]
struct ForkchoiceState {
    /// Block hash of the head of the canonical chain.
    head_block_hash: H256,
    /// The "safe" block hash of the canonical chain under
    /// certain synchrony and honesty assumptions.
    safe_block_hash: H256,
    /// Block hash of the most recent finalized block.
    finalized_block_hash: H256,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_forkchoice_state_deserialize() {
        let s = r#"{"headBlockHash":"0x3559e851470f6e7bbed1db474980683e8c315bfce99b2a6ef47c057c04de7858","safeBlockHash":"0x3559e851470f6e7bbed1db474980683e8c315bfce99b2a6ef47c057c04de7858","finalizedBlockHash":"0x3b8fb240d288781d4aac94d3fd16809ee413bc99294a085798a589dae51ddd4a"}"#;

        let forkchoice_state = ForkchoiceState {
            head_block_hash: H256::from_str(
                "0x3559e851470f6e7bbed1db474980683e8c315bfce99b2a6ef47c057c04de7858",
            )
            .unwrap(),
            safe_block_hash: H256::from_str(
                "0x3559e851470f6e7bbed1db474980683e8c315bfce99b2a6ef47c057c04de7858",
            )
            .unwrap(),
            finalized_block_hash: H256::from_str(
                "0x3b8fb240d288781d4aac94d3fd16809ee413bc99294a085798a589dae51ddd4a",
            )
            .unwrap(),
        };
        let deserialized: ForkchoiceState = serde_json::from_str(s).unwrap();

        assert_eq!(deserialized, forkchoice_state);
    }
}
