use rpc_server::{MetaIoHandler, Metadata, Middleware};
use std::sync::Arc;

use crate::{
    account_utils::{self, AccountProvider},
    miner::external::ExternalMiner,
    sync::SyncProvider,
};
use ethcore::{client::Client, miner::Miner, snapshot::SnapshotService};

/// RPC dependencies for a full node.
pub struct EthClientDependencies {
    pub client: Arc<Client>,
    pub snapshot: Arc<dyn SnapshotService>,
    pub sync: Arc<dyn SyncProvider>,
    pub accounts: Arc<AccountProvider>,
    pub miner: Arc<Miner>,
    pub external_miner: Arc<ExternalMiner>,
    pub experimental_rpcs: bool,
    pub gas_price_percentile: usize,
    pub allow_missing_blocks: bool,
    pub no_ancient_blocks: bool,
}

impl EthClientDependencies {
    pub fn extend_api<M, S>(&self, handler: &mut MetaIoHandler<M, S>)
    where
        S: Middleware<M>,
        M: Metadata,
    {
        use parity_rpc::v1::{Eth, EthClient, EthClientOptions};

        let accounts = account_utils::accounts_list(self.accounts.clone());
        let client = EthClient::new(
            &self.client,
            &self.snapshot,
            &self.sync,
            &accounts,
            &self.miner,
            &self.external_miner,
            EthClientOptions {
                gas_price_percentile: self.gas_price_percentile,
                allow_missing_blocks: self.allow_missing_blocks,
                allow_experimental_rpcs: self.experimental_rpcs,
                no_ancient_blocks: self.no_ancient_blocks,
            },
        );
        handler.extend_with(client.to_delegate());
    }
}
