use crate::v1::types::{PayloadId, PayloadStatus};
use serde::Serialize;

/// Contains response for `engine_forkchoiceUpdatedV1` call.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkchoiceResponse {
    /// Status of the forkchoice update call.
    pub payload_status: PayloadStatus,
    /// Identifier of the payload build process.
    pub payload_id: Option<PayloadId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v1::types::Status;
    use std::str::FromStr;

    #[test]
    fn test_forkchoice_response_without_payload_id_serialize() {
        let s = r#"{"payloadStatus":{"status":"VALID","latestValidHash":null,"validationError":null},"payloadId":null}"#;
        let payload_status = ForkchoiceResponse {
            payload_status: PayloadStatus {
                status: Status::Valid,
                latest_valid_hash: None,
                validation_error: None,
            },
            payload_id: None,
        };

        let serialized = serde_json::to_string(&payload_status).unwrap();
        assert_eq!(serialized, s);
    }

    #[test]
    fn test_forkchoice_response_with_payload_id_serialize() {
        let s = r#"{"payloadStatus":{"status":"VALID","latestValidHash":null,"validationError":null},"payloadId":"0xa247243752eb10b4"}"#;
        let payload_status = ForkchoiceResponse {
            payload_status: PayloadStatus {
                status: Status::Valid,
                latest_valid_hash: None,
                validation_error: None,
            },
            payload_id: Some(PayloadId::from_str("a247243752eb10b4").unwrap()),
        };

        let serialized = serde_json::to_string(&payload_status).unwrap();
        assert_eq!(serialized, s);
    }
}
