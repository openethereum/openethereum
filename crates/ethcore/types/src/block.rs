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

//! Base data structure of this module is `Block`.
//!
//! Blocks can be produced by a local node or they may be received from the network.
//!
//! To create a block locally, we start with an `OpenBlock`. This block is mutable
//! and can be appended to with transactions and uncles.
//!
//! When ready, `OpenBlock` can be closed and turned into a `ClosedBlock`. A `ClosedBlock` can
//! be reopend again by a miner under certain circumstances. On block close, state commit is
//! performed.
//!
//! `LockedBlock` is a version of a `ClosedBlock` that cannot be reopened. It can be sealed
//! using an engine.
//!
//! `ExecutedBlock` is an underlaying data structure used by all structs above to store block
//! related info.

use crate::bytes::Bytes;

use crate::{
    header::Header,
    transaction::{TypedTransaction, UnverifiedTransaction},
    BlockNumber,
};
use rlp::{DecoderError, Rlp, RlpStream};

/// A block, encoded as it is on the block chain.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Block {
    /// The header of this block.
    pub header: Header,
    /// The transactions in this block.
    pub transactions: Vec<UnverifiedTransaction>,
    /// The uncles of this block.
    pub uncles: Vec<Header>,
}

impl Block {
    /// Get the RLP-encoding of the block with the seal.
    pub fn rlp_bytes(&self) -> Bytes {
        let mut block_rlp = RlpStream::new_list(3);
        block_rlp.append(&self.header);
        TypedTransaction::rlp_append_list(&mut block_rlp, &self.transactions);
        block_rlp.append_list(&self.uncles);
        block_rlp.out()
    }

    pub fn decode_rlp(rlp: &Rlp, eip1559_transition: BlockNumber) -> Result<Self, DecoderError> {
        if rlp.as_raw().len() != rlp.payload_info()?.total() {
            return Err(DecoderError::RlpIsTooBig);
        }
        if rlp.item_count()? != 3 {
            return Err(DecoderError::RlpIncorrectListLen);
        }
        Ok(Block {
            header: Header::decode_rlp(&rlp.at(0)?, eip1559_transition)?,
            transactions: TypedTransaction::decode_rlp_list(&rlp.at(1)?)?,
            uncles: Header::decode_rlp_list(&rlp.at(2)?, eip1559_transition)?,
        })
    }
}
