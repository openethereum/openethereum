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

//! Definition of valid items for the verification queue.

use engines::EthEngine;
use error::Error;

use ethereum_types::{H256, U256};
use parity_util_mem::MallocSizeOf;

pub use self::{blocks::Blocks, headers::Headers};

/// Something which can produce a hash and a parent hash.
pub trait BlockLike {
    /// Get the hash of this item - i.e. the header hash.
    fn hash(&self) -> H256;

    /// Get a raw hash of this item - i.e. the hash of the RLP representation.
    fn raw_hash(&self) -> H256;

    /// Get the hash of this item's parent.
    fn parent_hash(&self) -> H256;

    /// Get the difficulty of this item.
    fn difficulty(&self) -> U256;
}

/// Defines transitions between stages of verification.
///
/// It starts with a fallible transformation from an "input" into the unverified item.
/// This consists of quick, simply done checks as well as extracting particular data.
///
/// Then, there is a `verify` function which performs more expensive checks and
/// produces the verified output.
///
/// For correctness, the hashes produced by each stage of the pipeline should be
/// consistent.
pub trait Kind: 'static + Sized + Send + Sync {
    /// The first stage: completely unverified.
    type Input: Sized + Send + BlockLike + MallocSizeOf;

    /// The second stage: partially verified.
    type Unverified: Sized + Send + BlockLike + MallocSizeOf;

    /// The third stage: completely verified.
    type Verified: Sized + Send + BlockLike + MallocSizeOf;

    /// Attempt to create the `Unverified` item from the input.
    fn create(
        input: Self::Input,
        engine: &dyn EthEngine,
        check_seal: bool,
    ) -> Result<Self::Unverified, (Self::Input, Error)>;

    /// Attempt to verify the `Unverified` item using the given engine.
    fn verify(
        unverified: Self::Unverified,
        engine: &dyn EthEngine,
        check_seal: bool,
    ) -> Result<Self::Verified, Error>;
}

/// The blocks verification module.
pub mod blocks {
    use super::{BlockLike, Kind};

    use engines::EthEngine;
    use error::{BlockError, Error, ErrorKind};
    use types::{
        header::Header,
        transaction::{TypedTransaction, UnverifiedTransaction},
        BlockNumber,
    };
    use verification::{verify_block_basic, verify_block_unordered, PreverifiedBlock};

    use bytes::Bytes;
    use ethereum_types::{H256, U256};
    use parity_util_mem::MallocSizeOf;

    /// A mode for verifying blocks.
    pub struct Blocks;

    impl Kind for Blocks {
        type Input = Unverified;
        type Unverified = Unverified;
        type Verified = PreverifiedBlock;

        // t_nb 4.0 verify_block_basic
        fn create(
            input: Self::Input,
            engine: &dyn EthEngine,
            check_seal: bool,
        ) -> Result<Self::Unverified, (Self::Input, Error)> {
            match verify_block_basic(&input, engine, check_seal) {
                Ok(()) => Ok(input),
                Err(Error(ErrorKind::Block(BlockError::TemporarilyInvalid(oob)), _)) => {
                    debug!(target: "client", "Block received too early {}: {:?}", input.hash(), oob);
                    Err((input, BlockError::TemporarilyInvalid(oob).into()))
                }
                Err(e) => {
                    warn!(target: "client", "Stage 1 block verification failed for {}: {:?}", input.hash(), e);
                    Err((input, e))
                }
            }
        }

        // t_nb 5.0 verify standalone block
        fn verify(
            un: Self::Unverified,
            engine: &dyn EthEngine,
            check_seal: bool,
        ) -> Result<Self::Verified, Error> {
            let hash = un.hash();
            match verify_block_unordered(un, engine, check_seal) {
                Ok(verified) => Ok(verified),
                Err(e) => {
                    warn!(target: "client", "Stage 2 block verification failed for {}: {:?}", hash, e);
                    Err(e)
                }
            }
        }
    }

    /// An unverified block.
    #[derive(PartialEq, Debug, MallocSizeOf)]
    pub struct Unverified {
        /// Unverified block header.
        pub header: Header,
        /// Unverified block transactions.
        pub transactions: Vec<UnverifiedTransaction>,
        /// Unverified block uncles.
        pub uncles: Vec<Header>,
        /// Raw block bytes.
        pub bytes: Bytes,
    }

    impl Unverified {
        /// Create an `Unverified` from raw bytes.
        pub fn from_rlp(
            bytes: Bytes,
            eip1559_transition: BlockNumber,
        ) -> Result<Self, ::rlp::DecoderError> {
            use rlp::Rlp;
            let (header, transactions, uncles) = {
                let rlp = Rlp::new(&bytes);
                let header = Header::decode_rlp(&rlp.at(0)?, eip1559_transition)?;
                let transactions = TypedTransaction::decode_rlp_list(&rlp.at(1)?)?;
                let uncles = Header::decode_rlp_list(&rlp.at(2)?, eip1559_transition)?;
                (header, transactions, uncles)
            };

            Ok(Unverified {
                header,
                transactions,
                uncles,
                bytes,
            })
        }
    }

    impl BlockLike for Unverified {
        fn hash(&self) -> H256 {
            self.header.hash()
        }

        fn raw_hash(&self) -> H256 {
            hash::keccak(&self.bytes)
        }

        fn parent_hash(&self) -> H256 {
            self.header.parent_hash().clone()
        }

        fn difficulty(&self) -> U256 {
            self.header.difficulty().clone()
        }
    }

    impl BlockLike for PreverifiedBlock {
        fn hash(&self) -> H256 {
            self.header.hash()
        }

        fn raw_hash(&self) -> H256 {
            hash::keccak(&self.bytes)
        }

        fn parent_hash(&self) -> H256 {
            self.header.parent_hash().clone()
        }

        fn difficulty(&self) -> U256 {
            self.header.difficulty().clone()
        }
    }
}

/// Verification for headers.
pub mod headers {
    use super::{BlockLike, Kind};

    use engines::EthEngine;
    use error::Error;
    use types::header::Header;
    use verification::verify_header_params;

    use ethereum_types::{H256, U256};

    impl BlockLike for Header {
        fn hash(&self) -> H256 {
            self.hash()
        }
        fn raw_hash(&self) -> H256 {
            self.hash()
        }
        fn parent_hash(&self) -> H256 {
            self.parent_hash().clone()
        }
        fn difficulty(&self) -> U256 {
            self.difficulty().clone()
        }
    }

    /// A mode for verifying headers.
    pub struct Headers;

    impl Kind for Headers {
        type Input = Header;
        type Unverified = Header;
        type Verified = Header;

        fn create(
            input: Self::Input,
            engine: &dyn EthEngine,
            check_seal: bool,
        ) -> Result<Self::Unverified, (Self::Input, Error)> {
            match verify_header_params(&input, engine, true, check_seal) {
                Ok(_) => Ok(input),
                Err(err) => Err((input, err)),
            }
        }

        fn verify(
            unverified: Self::Unverified,
            engine: &dyn EthEngine,
            check_seal: bool,
        ) -> Result<Self::Verified, Error> {
            match check_seal {
                true => engine
                    .verify_block_unordered(&unverified)
                    .map(|_| unverified),
                false => Ok(unverified),
            }
        }
    }
}
