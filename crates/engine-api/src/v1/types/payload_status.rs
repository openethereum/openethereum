use ethereum_types::H256;
use serde::Serialize;

/// Status of payload processing.
#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    /// Validation succeeded.
    Valid,
    /// Validation failed.
    Invalid,
    /// The payload extends the canonical chain and
    /// requisite data for its validation is missing.
    Syncing,
    /// Following conditions were met:
    /// - the `blockHash` of the payload is valid;
    /// - the payload doesn't extend the canonical chain;
    /// - the payload hasn't been fully validated.
    Accepted,
    /// The `blockHash` validation failed.
    InvalidBlockHash,
    /// Terminal block conditions were not satisfied.
    InvalidTerminalBlock,
}

/// The result of processing a payload.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadStatus {
    /// Status of payload processing.
    pub status: Status,
    /// The hash of the most recent valid block
    /// in the branch defined by payload and its ancestors.
    pub latest_valid_hash: Option<H256>,
    /// A message providing additional details on the validation
    /// error if the payload is deemed `INVALID`.
    pub validation_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_payload_status_deserialize() {
        let s = r#"{"status":"VALID","latestValidHash":"0x3559e851470f6e7bbed1db474980683e8c315bfce99b2a6ef47c057c04de7858","validationError":null}"#;
        let payload_status = PayloadStatus {
            status: Status::Valid,
            latest_valid_hash: Some(
                H256::from_str("3559e851470f6e7bbed1db474980683e8c315bfce99b2a6ef47c057c04de7858")
                    .unwrap(),
            ),
            validation_error: None,
        };

        let serialized = serde_json::to_string(&payload_status).unwrap();
        assert_eq!(serialized, s);
    }

    #[test]
    fn test_payload_status_without_latest_valid_hash_deserialize() {
        let s =
            r#"{"status":"INVALID_TERMINAL_BLOCK","latestValidHash":null,"validationError":null}"#;
        let payload_status = PayloadStatus {
            status: Status::InvalidTerminalBlock,
            latest_valid_hash: None,
            validation_error: None,
        };

        let serialized = serde_json::to_string(&payload_status).unwrap();
        assert_eq!(serialized, s);
    }
}
