//! Engine rpc interface.

use jsonrpc_core::Result;
use jsonrpc_derive::rpc;

use crate::v1::types::{
    ExecutionPayload, ForkchoiceResponse, ForkchoiceState, PayloadAttributes, PayloadId,
    PayloadStatus, TransitionConfiguration,
};

/// Engine rpc interface.
#[rpc(server)]
pub trait Engine {
    #[rpc(name = "engine_newPayloadV1")]
    fn new_payload(&self, payload: ExecutionPayload) -> Result<PayloadStatus>;

    #[rpc(name = "engine_forkchoiceUpdatedV1")]
    fn forkchoice_updated(
        &self,
        state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> Result<ForkchoiceResponse>;

    #[rpc(name = "engine_getPayloadV1")]
    fn get_payload(&self, payload_id: PayloadId) -> Result<ExecutionPayload>;

    #[rpc(name = "engine_exchangeTransitionConfigurationV1")]
    fn exchange_transition_configuration(
        &self,
        configuration: TransitionConfiguration,
    ) -> Result<TransitionConfiguration>;
}
