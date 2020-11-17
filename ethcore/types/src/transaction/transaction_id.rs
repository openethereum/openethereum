// Copyright 2020-2020 Parity Technologies (UK) Ltd.
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

//! Transaction Id.

use serde_repr::*;
use std::convert::TryFrom;

#[derive(Serialize_repr, Eq, Hash, Deserialize_repr, Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum TypedTxId {
    AccessList = 0x01,
    Legacy = 0x80, // With 0x80 we are sure that all other types will not overlap
}

impl Default for TypedTxId {
    fn default() -> TypedTxId {
        TypedTxId::Legacy
    }
}

impl TryFrom<u8> for TypedTxId {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == TypedTxId::AccessList as u8 => Ok(TypedTxId::AccessList),
            x if (x & 0x80) != 0x0 => Ok(TypedTxId::Legacy),
            _ => Err(()),
        }
    }
}
