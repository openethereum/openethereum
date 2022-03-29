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
        if *header.difficulty() != U256::zero() {
            return Err(Error::from(BlockError::InvalidDifficulty(Mismatch {
                expected: U256::zero(),
                found: *header.difficulty(),
            })));
        }

        // Both nonce and mixHash fields of the seal should be zero
        let seal = Seal::parse_seal(header.seal())?;
        if seal.mix_hash != H256::zero() {
            return Err(Error::from(BlockError::MismatchedH256SealElement(
                Mismatch {
                    expected: H256::zero(),
                    found: seal.mix_hash,
                },
            )));
        }
        if seal.nonce != H64::zero() {
            return Err(Error::from(BlockError::InvalidSeal));
        }

        Ok(())
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
        unimplemented!()
    }

    fn executive_author(&self, header: &Header) -> Result<Address, Error> {
        Ok(*header.author())
    }
}
