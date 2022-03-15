use ethereum_types::{Address, H256, U64};
use serde::Deserialize;

/// The attributes required to initiate a payload build process.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct PayloadAttributes {
    /// Value for the `timestamp` field of the new payload.
    pub timestamp: U64,
    /// Value for the `random` field of the new payload.
    pub prev_randao: H256,
    /// Suggested value for the `feeRecipient` field of the new payload.
    pub suggested_fee_recipient: Address,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_payload_attributes_deserialize() {
        let s = r#"{"timestamp":"0x5","prevRandao":"0x0d98f14f2a081328c81806658c0eae43c155568a895b11141bbbda07d0a30993","suggestedFeeRecipient":"0xa94f5374fce5edbc8e2a8697c15331677e6ebf0b"}"#;

        let payload_attributes = PayloadAttributes {
            timestamp: 5.into(),
            prev_randao: H256::from_str(
                "0d98f14f2a081328c81806658c0eae43c155568a895b11141bbbda07d0a30993",
            )
            .unwrap(),
            suggested_fee_recipient: Address::from_str("a94f5374fce5edbc8e2a8697c15331677e6ebf0b")
                .unwrap(),
        };
        let deserialized: PayloadAttributes = serde_json::from_str(s).unwrap();

        assert_eq!(deserialized, payload_attributes);
    }
}
