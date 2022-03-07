//! Engine rpc implementation.

use crate::v1::{
    traits::Engine,
    types::{
        ExecutionPayload, ForkchoiceResponse, ForkchoiceState, PayloadAttributes, PayloadId,
        PayloadStatus, Status, TransitionConfiguration,
    },
};

use jsonrpc_core::Result;

/// Engine rpc implementation.
pub struct EngineClient {}

impl EngineClient {
    pub fn new() -> Self {
        Self {}
    }
}

impl Engine for EngineClient {
    fn new_payload(&self, _payload: ExecutionPayload) -> Result<PayloadStatus> {
        Ok(PayloadStatus {
            status: Status::Valid,
            latest_valid_hash: None,
            validation_error: None,
        })
    }

    fn forkchoice_updated(
        &self,
        _state: ForkchoiceState,
        _payload_attributes: Option<PayloadAttributes>,
    ) -> Result<ForkchoiceResponse> {
        Ok(ForkchoiceResponse {
            payload_status: PayloadStatus {
                status: Status::Valid,
                latest_valid_hash: None,
                validation_error: None,
            },
            payload_id: None,
        })
    }

    fn get_payload(&self, _payload_id: PayloadId) -> Result<ExecutionPayload> {
        Ok(ExecutionPayload {
            parent_hash: Default::default(),
            fee_recipient: Default::default(),
            state_root: Default::default(),
            receipts_root: Default::default(),
            logs_bloom: Default::default(),
            random: Default::default(),
            block_number: Default::default(),
            gas_limit: Default::default(),
            gas_used: Default::default(),
            timestamp: Default::default(),
            extra_data: Default::default(),
            base_fee_per_gas: Default::default(),
            block_hash: Default::default(),
            transactions: Default::default(),
        })
    }

    fn exchange_transition_configuration(
        &self,
        _configuration: TransitionConfiguration,
    ) -> Result<TransitionConfiguration> {
        Ok(TransitionConfiguration {
            terminal_total_difficulty: Default::default(),
            terminal_block_hash: Default::default(),
            terminal_block_number: Default::default(),
        })
    }
}
