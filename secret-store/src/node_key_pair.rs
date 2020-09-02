// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity Ethereum.

// Parity Ethereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Ethereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Ethereum.  If not, see <http://www.gnu.org/licenses/>.

use ethereum_types::{Address, H256};
use ethkey::{
    crypto::ecdh::agree, public_to_address, sign, Error as EthKeyError, KeyPair, Public, Signature,
};
use std::ops::Deref;

pub struct NodeKeyPair {
    key_pair: KeyPair,
}

impl Deref for NodeKeyPair {
    type Target = KeyPair;

    fn deref(&self) -> &Self::Target {
        &self.key_pair
    }
}

impl NodeKeyPair {
    pub fn new(key_pair: KeyPair) -> Self {
        NodeKeyPair { key_pair: key_pair }
    }

    pub fn public(&self) -> &Public {
        self.key_pair.public()
    }

    pub fn address(&self) -> Address {
        public_to_address(self.key_pair.public())
    }

    pub fn sign(&self, data: &H256) -> Result<Signature, EthKeyError> {
        sign(self.key_pair.secret(), data)
    }

    pub fn compute_shared_key(&self, peer_public: &Public) -> Result<KeyPair, EthKeyError> {
        agree(self.key_pair.secret(), peer_public)
            .map_err(|e| EthKeyError::Custom(e.to_string()))
            .and_then(KeyPair::from_secret)
    }
}
