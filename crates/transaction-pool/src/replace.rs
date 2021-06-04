// Copyright 2015-2019 Parity Technologies (UK) Ltd.
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

//! When queue limits are reached, decide whether to replace an existing transaction from the pool

use crate::{pool::Transaction, scoring::Choice};

/// Encapsulates a transaction to be compared, along with pooled transactions from the same sender
pub struct ReplaceTransaction<'a, T> {
    /// The transaction to be compared for replacement
    pub transaction: &'a Transaction<T>,
    /// Other transactions currently in the pool for the same sender
    pub pooled_by_sender: Option<&'a [Transaction<T>]>,
}

impl<'a, T> ReplaceTransaction<'a, T> {
    /// Creates a new `ReplaceTransaction`
    pub fn new(
        transaction: &'a Transaction<T>,
        pooled_by_sender: Option<&'a [Transaction<T>]>,
    ) -> Self {
        ReplaceTransaction {
            transaction,
            pooled_by_sender,
        }
    }
}

impl<'a, T> ::std::ops::Deref for ReplaceTransaction<'a, T> {
    type Target = Transaction<T>;
    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

/// Chooses whether a new transaction should replace an existing transaction if the pool is full.
pub trait ShouldReplace<T> {
    /// Decides if `new` should push out `old` transaction from the pool.
    ///
    /// NOTE returning `InsertNew` here can lead to some transactions being accepted above pool limits.
    fn should_replace(&self, old: &ReplaceTransaction<T>, new: &ReplaceTransaction<T>) -> Choice;
}
