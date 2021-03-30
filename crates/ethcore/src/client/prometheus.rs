// Copyright 2015-2020 Parity Technologies (UK) Ltd.
// This file is part of OpenEthereum.

// OpenEthereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// OpenEthereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with OpenEthereum.  If not, see <http://www.gnu.org/licenses/>.

use super::Client;
use client::{blockchain::BlockChainClient, info::ChainInfo};
use ethereum_types::U256;
use stats::{PrometheusMetrics, PrometheusRegistry};

impl PrometheusMetrics for Client {
    fn prometheus_metrics(&self, r: &mut PrometheusRegistry) {
        // gas, tx & blocks
        let report = self.report();

        for (key, value) in report.item_sizes.iter() {
            r.register_gauge(
                &key,
                format!("Total item number of {}", key).as_str(),
                *value as i64,
            );
        }

        r.register_counter(
            "import_gas",
            "Gas processed",
            report.gas_processed.as_u64() as i64,
        );
        r.register_counter(
            "import_blocks",
            "Blocks imported",
            report.blocks_imported as i64,
        );
        r.register_counter(
            "import_txs",
            "Transactions applied",
            report.transactions_applied as i64,
        );

        let state_db = self.state_db.read();
        r.register_gauge(
            "statedb_cache_size",
            "State DB cache size",
            state_db.cache_size() as i64,
        );

        // blockchain cache
        let blockchain_cache_info = self.blockchain_cache_info();
        r.register_gauge(
            "blockchaincache_block_details",
            "BlockDetails cache size",
            blockchain_cache_info.block_details as i64,
        );
        r.register_gauge(
            "blockchaincache_block_recipts",
            "Block receipts size",
            blockchain_cache_info.block_receipts as i64,
        );
        r.register_gauge(
            "blockchaincache_blocks",
            "Blocks cache size",
            blockchain_cache_info.blocks as i64,
        );
        r.register_gauge(
            "blockchaincache_txaddrs",
            "Transaction addresses cache size",
            blockchain_cache_info.transaction_addresses as i64,
        );
        r.register_gauge(
            "blockchaincache_size",
            "Total blockchain cache size",
            blockchain_cache_info.total() as i64,
        );

        // chain info
        let chain = self.chain_info();

        let gap = chain
            .ancient_block_number
            .map(|x| U256::from(x + 1))
            .and_then(|first| {
                chain
                    .first_block_number
                    .map(|last| (first, U256::from(last)))
            });
        if let Some((first, last)) = gap {
            r.register_gauge(
                "chain_warpsync_gap_first",
                "Warp sync gap, first block",
                first.as_u64() as i64,
            );
            r.register_gauge(
                "chain_warpsync_gap_last",
                "Warp sync gap, last block",
                last.as_u64() as i64,
            );
        }

        r.register_gauge(
            "chain_block",
            "Best block number",
            chain.best_block_number as i64,
        );

        // prunning info
        let prunning = self.pruning_info();
        r.register_gauge(
            "prunning_earliest_chain",
            "The first block which everything can be served after",
            prunning.earliest_chain as i64,
        );
        r.register_gauge(
            "prunning_earliest_state",
            "The first block where state requests may be served",
            prunning.earliest_state as i64,
        );

        // queue info
        let queue = self.queue_info();
        r.register_gauge(
            "queue_mem_used",
            "Queue heap memory used in bytes",
            queue.mem_used as i64,
        );
        r.register_gauge(
            "queue_size_total",
            "The total size of the queues",
            queue.total_queue_size() as i64,
        );
        r.register_gauge(
            "queue_size_unverified",
            "Number of queued items pending verification",
            queue.unverified_queue_size as i64,
        );
        r.register_gauge(
            "queue_size_verified",
            "Number of verified queued items pending import",
            queue.verified_queue_size as i64,
        );
        r.register_gauge(
            "queue_size_verifying",
            "Number of items being verified",
            queue.verifying_queue_size as i64,
        );

        // database info
        self.db.read().key_value().prometheus_metrics(r);
    }
}
