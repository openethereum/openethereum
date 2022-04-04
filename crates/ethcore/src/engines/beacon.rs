use engines::{Engine, ForkChoice};
use error::{BlockError, Error};
use ethereum::ethash::Seal;
use ethereum_types::{H64, U256};
use ethjson::{
    crypto::publickey::Address,
    types::{
        hash::H256,
        header::{ExtendedHeader, Header},
        BlockNumber,
    },
};
use machine::EthereumMachine;
use std::collections::BTreeMap;
use unexpected::Mismatch;

pub struct Beacon {
    machine: EthereumMachine,
}

pub const BEACON_NONCE: H64 = H64::zero();
pub const BEACON_MIX_HASH: H256 = H256::zero();
pub const BEACON_DIFFICULTY: U256 = U256::zero();

impl Beacon {
    pub fn new(machine: EthereumMachine) -> Self {
        Self { machine }
    }
}

impl Engine<EthereumMachine> for Beacon {
    fn name(&self) -> &str {
        "BeaconChain"
    }

    fn machine(&self) -> &EthereumMachine {
        &self.machine
    }

    fn seal_fields(&self, _header: &Header) -> usize {
        // For now we use EIP-3565 specification with two fields - nonce and mix.
        2
    }

    fn extra_info(&self, header: &Header) -> BTreeMap<String, String> {
        match Seal::parse_seal(header.seal()) {
            Ok(seal) => map![
                "nonce".to_owned() => format!("0x{:x}", seal.nonce),
                "mixHash".to_owned() => format!("0x{:x}", seal.mix_hash)
            ],
            _ => BTreeMap::default(),
        }
    }

    fn maximum_uncle_count(&self, _block: BlockNumber) -> usize {
        // Uncles field should be empty after the merge
        0
    }

    fn verify_local_seal(&self, _header: &Header) -> Result<(), Error> {
        Ok(())
    }

    fn verify_block_basic(&self, header: &Header) -> Result<(), Error> {
        // difficulty field should always be 0 after the merge
        if *header.difficulty() != BEACON_DIFFICULTY {
            return Err(Error::from(BlockError::InvalidDifficulty(Mismatch {
                expected: BEACON_DIFFICULTY,
                found: *header.difficulty(),
            })));
        }

        // Both nonce and mixHash fields of the seal should be zero
        let seal = Seal::parse_seal(header.seal())?;
        if seal.mix_hash != BEACON_MIX_HASH {
            return Err(Error::from(BlockError::MismatchedH256SealElement(
                Mismatch {
                    expected: BEACON_MIX_HASH,
                    found: seal.mix_hash,
                },
            )));
        }
        if seal.nonce != BEACON_NONCE {
            return Err(Error::from(BlockError::InvalidSeal));
        }

        Ok(())
    }

    fn populate_from_parent(&self, header: &mut Header, _parent: &Header) {
        header.set_difficulty(BEACON_DIFFICULTY);
    }

    // TODO: is any implementation actually required?
    // fn epoch_verifier<'a>(
    //     &self,
    //     header: &Header,
    //     proof: &'a [u8],
    // ) -> ConstructedVerifier<'a, EthereumMachine> {
    //     todo!()
    // }

    fn fork_choice(&self, _new: &ExtendedHeader, _best: &ExtendedHeader) -> ForkChoice {
        ForkChoice::Old
    }

    fn executive_author(&self, header: &Header) -> Result<Address, Error> {
        Ok(*header.author())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use error::ErrorKind;
    use spec::Spec;

    #[test]
    fn has_valid_metadata() {
        let engine = Spec::new_test_beacon().engine;
        assert_eq!("BeaconChain", engine.name())
    }

    #[test]
    fn can_do_seal_verification_fail() {
        let engine = Spec::new_test_beacon().engine;
        let header: Header = Header::default();

        let verify_result = engine.verify_block_basic(&header);

        match verify_result {
            Err(Error(
                ErrorKind::Block(BlockError::InvalidSealArity(Mismatch {
                    expected: 2,
                    found: _,
                })),
                _,
            )) => {}
            Err(_) => {
                panic!(
                    "should be block seal-arity mismatch error (got {:?})",
                    verify_result
                );
            }
            _ => {
                panic!("Should be error, got Ok");
            }
        }
    }

    #[test]
    fn can_do_seal_verification_fail_with_invalid_mix_hash() {
        let engine = Spec::new_test_beacon().engine;

        let mut header: Header = Header::default();
        let invalid_mix_hash = H256::from_low_u64_be(1);
        header.set_seal(vec![
            rlp::encode(&invalid_mix_hash),
            rlp::encode(&BEACON_NONCE),
        ]);

        let verify_result = engine.verify_block_basic(&header);

        match verify_result {
            Err(Error(
                ErrorKind::Block(BlockError::MismatchedH256SealElement(Mismatch {
                    expected,
                    found: _,
                })),
                _,
            )) if expected == BEACON_MIX_HASH => {}
            Err(_) => {
                panic!(
                    "should be block invalid seal mismatch error (got {:?})",
                    verify_result
                );
            }
            _ => {
                panic!("Should be error, got Ok");
            }
        }
    }

    #[test]
    fn can_do_seal_verification_fail_with_invalid_nonce() {
        let engine = Spec::new_test_beacon().engine;

        let mut header: Header = Header::default();
        let invalid_nonce = H64::from_low_u64_be(1);
        header.set_seal(vec![
            rlp::encode(&BEACON_MIX_HASH),
            rlp::encode(&invalid_nonce),
        ]);

        let verify_result = engine.verify_block_basic(&header);

        match verify_result {
            Err(Error(ErrorKind::Block(BlockError::InvalidSeal), _)) => {}
            Err(_) => {
                panic!(
                    "should be block invalid seal error (got {:?})",
                    verify_result
                );
            }
            _ => {
                panic!("Should be error, got Ok");
            }
        }
    }

    #[test]
    fn can_do_difficulty_verification_fail() {
        let engine = Spec::new_test_beacon().engine;

        let mut header = Header::default();
        let invalid_difficulty = U256::from(1);
        header.set_difficulty(invalid_difficulty);

        let verify_result = engine.verify_block_basic(&header);

        match verify_result {
            Err(Error(
                ErrorKind::Block(BlockError::InvalidDifficulty(Mismatch {
                    expected: BEACON_DIFFICULTY,
                    found: _,
                })),
                _,
            )) => {}
            Err(_) => {
                panic!("should be block difficulty error (got {:?})", verify_result);
            }
            _ => {
                panic!("Should be error, got Ok");
            }
        }
    }
}
