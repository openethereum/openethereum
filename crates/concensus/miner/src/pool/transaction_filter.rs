// Copyright 2015-2021 Parity Technologies (UK) Ltd.
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

//! Filter options available in the parity_pendingTransaction endpoint of the JSONRPC API.

#![allow(missing_docs)]

use ethereum_types::{Address, U256};

use pool::VerifiedTransaction;
use types::transaction::Action;

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize, Serialize)]
#[serde()]
pub enum SenderArgument {
    eq(Address),
    None,
}

impl Default for SenderArgument {
    fn default() -> Self {
        Self::None
    }
}

impl SenderArgument {
    fn matches(&self, value: &Address) -> bool {
        match self {
            Self::eq(expected) => value == expected,
            Self::None => true,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize, Serialize)]
#[serde()]
pub enum ActionArgument {
    eq(Address),
    action(String),
    None,
}

impl Default for ActionArgument {
    fn default() -> Self {
        Self::None
    }
}

impl ActionArgument {
    fn matches(&self, value: &Action) -> bool {
        match self {
            Self::eq(expected) => *value == Action::Call(*expected),
            Self::action(name) => *value == Action::Create && name == "contract_creation",
            Self::None => true,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize, Serialize)]
#[serde()]
pub enum ValueFilterArgument {
    eq(U256),
    lt(U256),
    gt(U256),
    None,
}

impl Default for ValueFilterArgument {
    fn default() -> Self {
        Self::None
    }
}

impl ValueFilterArgument {
    fn matches(&self, value: &U256) -> bool {
        match self {
            ValueFilterArgument::eq(expected) => value == expected,
            ValueFilterArgument::lt(threshold) => value < threshold,
            ValueFilterArgument::gt(threshold) => value > threshold,
            ValueFilterArgument::None => true,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TransactionFilter {
    from: SenderArgument,
    to: ActionArgument,
    gas: ValueFilterArgument,
    gas_price: ValueFilterArgument,
    value: ValueFilterArgument,
    nonce: ValueFilterArgument,
}

impl TransactionFilter {
    pub fn matches(&self, transaction: &VerifiedTransaction) -> bool {
        let tx = transaction.signed().tx();
        self.from.matches(&transaction.sender)
            && self.to.matches(&tx.action)
            && self.gas.matches(&tx.gas)
            && self.gas_price.matches(&tx.gas_price)
            && self.nonce.matches(&tx.nonce)
            && self.value.matches(&tx.value)
    }
}

pub fn match_filter(filter: &Option<TransactionFilter>, transaction: &VerifiedTransaction) -> bool {
    match filter {
        Some(f) => f.matches(transaction),
        None => true,
    }
}
