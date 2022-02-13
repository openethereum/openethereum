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

//! Blockchain test transaction deserialization.

use crate::{bytes::Bytes, uint::Uint};
use ethereum_types::{H160, H256};

/// Blockchain test transaction deserialization.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    #[serde(rename = "type")]
    pub transaction_type: Option<Uint>,
    pub data: Bytes,
    pub gas_limit: Uint,
    pub gas_price: Option<Uint>,
    pub nonce: Uint,
    pub r: Uint,
    pub s: Uint,
    pub v: Uint,
    pub value: Uint,
    pub chain_id: Option<Uint>,
    pub access_list: Option<AccessList>,
    pub max_fee_per_gas: Option<Uint>,
    pub max_priority_fee_per_gas: Option<Uint>,
    pub hash: Option<H256>,
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccessListItem {
    pub address: H160,
    pub storage_keys: Vec<H256>,
}

pub type AccessList = Vec<AccessListItem>;
