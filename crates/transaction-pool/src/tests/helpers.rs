// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

use std::{cmp, collections::HashMap};

use super::Transaction;
use crate::{pool, scoring, Readiness, Ready, ReplaceTransaction, Scoring, ShouldReplace};
use ethereum_types::{H160 as Sender, U256};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum DummyScoringEvent {
    /// Penalize transactions
    Penalize,
    /// Update scores to the gas price
    UpdateScores,
}
#[derive(Debug, Default)]
pub struct DummyScoring {
    always_insert: bool,
}

impl DummyScoring {
    pub fn always_insert() -> Self {
        DummyScoring {
            always_insert: true,
        }
    }
}

impl Scoring<Transaction> for DummyScoring {
    type Score = U256;
    type Event = DummyScoringEvent;

    fn compare(&self, old: &Transaction, new: &Transaction) -> cmp::Ordering {
        old.nonce.cmp(&new.nonce)
    }

    fn choose(&self, old: &Transaction, new: &Transaction) -> scoring::Choice {
        if old.nonce == new.nonce {
            if new.gas_price > old.gas_price {
                scoring::Choice::ReplaceOld
            } else {
                scoring::Choice::RejectNew
            }
        } else {
            scoring::Choice::InsertNew
        }
    }

    fn update_scores(
        &self,
        txs: &[pool::Transaction<Transaction>],
        scores: &mut [Self::Score],
        change: scoring::Change<DummyScoringEvent>,
    ) {
        match change {
            scoring::Change::Event(event) => {
                match event {
                    DummyScoringEvent::Penalize => {
                        println!("entered");
                        // In case of penalize reset all scores to 0
                        for i in 0..txs.len() {
                            scores[i] = 0.into();
                        }
                    }
                    DummyScoringEvent::UpdateScores => {
                        // Set to a gas price otherwise
                        for i in 0..txs.len() {
                            scores[i] = txs[i].gas_price;
                        }
                    }
                }
            }
            scoring::Change::InsertedAt(index) | scoring::Change::ReplacedAt(index) => {
                scores[index] = txs[index].gas_price;
            }
            scoring::Change::RemovedAt(_) => {}
            scoring::Change::Culled(_) => {}
        }
    }

    fn should_ignore_sender_limit(&self, _new: &Transaction) -> bool {
        self.always_insert
    }
}

impl ShouldReplace<Transaction> for DummyScoring {
    fn should_replace(
        &self,
        old: &ReplaceTransaction<Transaction>,
        new: &ReplaceTransaction<Transaction>,
    ) -> scoring::Choice {
        if self.always_insert {
            scoring::Choice::InsertNew
        } else if new.gas_price > old.gas_price {
            scoring::Choice::ReplaceOld
        } else {
            scoring::Choice::RejectNew
        }
    }
}

#[derive(Default)]
pub struct NonceReady(HashMap<Sender, U256>, U256);

impl NonceReady {
    pub fn new<T: Into<U256>>(min: T) -> Self {
        let mut n = NonceReady::default();
        n.1 = min.into();
        n
    }
}

impl Ready<Transaction> for NonceReady {
    fn is_ready(&mut self, tx: &Transaction) -> Readiness {
        let min = self.1;
        let nonce = self.0.entry(tx.sender).or_insert_with(|| min);
        match tx.nonce.cmp(nonce) {
            cmp::Ordering::Greater => Readiness::Future,
            cmp::Ordering::Equal => {
                *nonce += 1.into();
                Readiness::Ready
            }
            cmp::Ordering::Less => Readiness::Stale,
        }
    }
}
