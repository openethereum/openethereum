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

use api::TransactionStats;
use ethereum_types::{H256, H512};
use fastmap::H256FastMap;
use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasher,
};
use types::BlockNumber;

type NodeId = H512;

#[derive(Debug, PartialEq, Clone)]
pub struct Stats {
    first_seen: BlockNumber,
    propagated_to: HashMap<NodeId, usize>,
}

impl Stats {
    pub fn new(number: BlockNumber) -> Self {
        Stats {
            first_seen: number,
            propagated_to: Default::default(),
        }
    }
}

impl<'a> From<&'a Stats> for TransactionStats {
    fn from(other: &'a Stats) -> Self {
        TransactionStats {
            first_seen: other.first_seen,
            propagated_to: other
                .propagated_to
                .iter()
                .map(|(hash, size)| (*hash, *size))
                .collect(),
        }
    }
}

#[derive(Debug, Default)]
pub struct TransactionsStats {
    pending_transactions: H256FastMap<Stats>,
    new_transactions: H256FastMap<Stats>,
}

impl TransactionsStats {
    /// Increases number of propagations to given `enodeid`.
    pub fn propagated(
        &mut self,
        hash: &H256,
        is_new: bool,
        enode_id: Option<NodeId>,
        current_block_num: BlockNumber,
    ) {
        let enode_id = enode_id.unwrap_or_default();
        let stats = if is_new {
            self.new_transactions
                .entry(*hash)
                .or_insert_with(|| Stats::new(current_block_num))
        } else {
            self.pending_transactions
                .entry(*hash)
                .or_insert_with(|| Stats::new(current_block_num))
        };
        let count = stats.propagated_to.entry(enode_id).or_insert(0);
        *count = count.saturating_add(1);
    }

    /// Returns propagation stats for given hash or `None` if hash is not known or
    /// does not correspond to pending transaction.
    #[cfg(test)]
    pub fn get_pending(&self, hash: &H256) -> Option<&Stats> {
        self.pending_transactions.get(hash)
    }

    /// Returns propagation stats for given hash or `None` if hash is not known or
    /// does not correspond to new transaction.
    #[cfg(test)]
    pub fn get_new(&self, hash: &H256) -> Option<&Stats> {
        self.new_transactions.get(hash)
    }

    /// Stats for pending transactions.
    pub fn pending_transactions_stats(&self) -> &H256FastMap<Stats> {
        &self.pending_transactions
    }

    /// Stats for new transactions.
    pub fn new_transactions_stats(&self) -> &H256FastMap<Stats> {
        &self.new_transactions
    }

    /// Retains only pending transactions present in given `HashSet`.
    pub fn retain_pending<S: BuildHasher>(&mut self, hashes: &HashSet<H256, S>) {
        let to_remove = self
            .pending_transactions
            .keys()
            .filter(|hash| !hashes.contains(hash))
            .cloned()
            .collect::<Vec<_>>();

        for hash in to_remove {
            self.pending_transactions.remove(&hash);
        }
    }

    pub fn retain_new(
        &mut self,
        current_block_num: BlockNumber,
        new_transactions_stats_period: BlockNumber,
    ) {
        let to_remove = self
            .new_transactions
            .iter()
            .filter(|(_, stats)| {
                current_block_num.saturating_sub(stats.first_seen) > new_transactions_stats_period
            })
            .map(|(hash, _)| hash.clone())
            .collect::<Vec<_>>();

        for hash in to_remove {
            self.new_transactions.remove(&hash);
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{Stats, TransactionsStats};
    use ethereum_types::{H256, H512};
    use std::collections::{HashMap, HashSet};

    #[test]
    fn should_keep_track_of_propagations() {
        // given
        let hash = H256::from_low_u64_be(5);
        let enodeid1 = H512::from_low_u64_be(2);
        let enodeid2 = H512::from_low_u64_be(5);

        {
            // given
            let mut stats = TransactionsStats::default();

            // when
            stats.propagated(&hash, false, Some(enodeid1), 5);
            stats.propagated(&hash, false, Some(enodeid1), 10);
            stats.propagated(&hash, false, Some(enodeid2), 15);

            // then
            let pending_stats = stats.get_pending(&hash);
            assert_eq!(
                pending_stats,
                Some(&Stats {
                    first_seen: 5,
                    propagated_to: hash_map![
                        enodeid1 => 2,
                        enodeid2 => 1
                    ],
                }),
                "Pending transactions propagation should update pending_transactions stats"
            );

            let new_stats = stats.get_new(&hash);
            assert_eq!(
                new_stats, None,
                "Pending transactions propagation should not update new_transactions stats"
            );
        }

        {
            // given
            let mut stats = TransactionsStats::default();

            // when
            stats.propagated(&hash, true, Some(enodeid1), 5);
            stats.propagated(&hash, true, Some(enodeid1), 10);
            stats.propagated(&hash, true, Some(enodeid2), 15);

            // then
            let pending_stats = stats.get_pending(&hash);
            assert_eq!(
                pending_stats, None,
                "New transactions propagation should not update pending_transactions stats"
            );

            let new_stats = stats.get_new(&hash);
            assert_eq!(
                new_stats,
                Some(&Stats {
                    first_seen: 5,
                    propagated_to: hash_map![
                        enodeid1 => 2,
                        enodeid2 => 1
                    ],
                }),
                "New transactions propagation should update new_transactions stats"
            );
        }
    }

    #[test]
    fn should_remove_pending_hash_from_tracking() {
        // given
        let mut stats = TransactionsStats::default();
        let hash = H256::from_low_u64_be(5);
        let enodeid1 = H512::from_low_u64_be(5);
        stats.propagated(&hash, false, Some(enodeid1), 10);

        // when
        stats.retain_pending(&HashSet::new());

        // then
        let stats = stats.get_pending(&hash);
        assert_eq!(stats, None);
    }

    #[test]
    fn should_remove_expired_new_hashes_from_tracking() {
        //given
        let mut stats = TransactionsStats::default();

        let hash1 = H256::from_low_u64_be(5);
        let hash2 = H256::from_low_u64_be(6);
        let hash3 = H256::from_low_u64_be(7);

        let enodeid1 = H512::from_low_u64_be(5);
        let enodeid2 = H512::from_low_u64_be(6);
        let enodeid3 = H512::from_low_u64_be(7);

        stats.propagated(&hash1, true, Some(enodeid1), 5);
        stats.propagated(&hash2, true, Some(enodeid2), 6);
        stats.propagated(&hash3, true, Some(enodeid3), 7);

        // when
        stats.retain_new(10, 3);

        // then
        assert_eq!(stats.get_new(&hash1), None);
        assert_eq!(stats.get_new(&hash2), None);
        assert_eq!(
            stats.get_new(&hash3),
            Some(&Stats {
                first_seen: 7,
                propagated_to: hash_map![
                    enodeid3 => 1
                ],
            }),
        )
    }
}
