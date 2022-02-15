//! Engine rpc implementation.

use crate::v1::traits::Engine;
use crate::v1::types::{
    ExecutionPayload, ForkchoiceResponse, ForkchoiceState, PayloadAttributes, PayloadId,
    PayloadStatus, TransitionConfiguration,
};

use jsonrpc_core::Result;

/// Engine rpc implementation.
pub struct EngineClient {}

impl Engine for EngineClient {
    fn new_payload(&self, _payload: ExecutionPayload) -> Result<PayloadStatus> {
        todo!()
    }

    fn forkchoice_updated(
        &self,
        _state: ForkchoiceState,
        _payload_attributes: Option<PayloadAttributes>,
    ) -> Result<ForkchoiceResponse> {
        todo!()
    }

    fn get_payload(&self, _payload_id: PayloadId) -> Result<ExecutionPayload> {
        todo!()
    }

    fn exchange_transition_configuration(
        &self,
        _configuration: TransitionConfiguration,
    ) -> Result<TransitionConfiguration> {
        todo!()
    }
}
