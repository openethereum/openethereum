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

use super::{Address, Transaction, H256, U256};
use ethereum_types::BigEndianHash;

#[derive(Debug, Default, Clone)]
pub struct TransactionBuilder {
    nonce: U256,
    gas_price: U256,
    gas: U256,
    sender: Address,
    mem_usage: usize,
}

impl TransactionBuilder {
    pub fn tx(&self) -> Self {
        self.clone()
    }

    pub fn nonce(mut self, nonce: usize) -> Self {
        self.nonce = U256::from(nonce);
        self
    }

    pub fn gas_price(mut self, gas_price: usize) -> Self {
        self.gas_price = U256::from(gas_price);
        self
    }

    pub fn sender(mut self, sender: u64) -> Self {
        self.sender = Address::from_low_u64_be(sender);
        self
    }

    pub fn mem_usage(mut self, mem_usage: usize) -> Self {
        self.mem_usage = mem_usage;
        self
    }

    pub fn new(self) -> Transaction {
        let hash: U256 = self.nonce
            ^ (U256::from(100) * self.gas_price)
            ^ (U256::from(100_000) * U256::from(self.sender.to_low_u64_be()));
        Transaction {
            hash: H256::from_uint(&hash),
            nonce: self.nonce,
            gas_price: self.gas_price,
            gas: 21_000.into(),
            sender: self.sender,
            mem_usage: self.mem_usage,
        }
    }
}
