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

use account_db::Factory as AccountFactory;
use ethtrie::RlpCodec;
use evm::Factory as VmFactory;
use keccak_hasher::KeccakHasher;
use trie::TrieFactory;

/// Collection of factories.
#[derive(Default, Clone)]
pub struct Factories {
    /// factory for evm.
    pub vm: VmFactory,
    /// factory for tries.
    pub trie: TrieFactory<KeccakHasher, RlpCodec>,
    /// factory for account databases.
    pub accountdb: AccountFactory,
}
