use bytes::Bytes;
use client::ImportBlock;
use engines::beacon::{BEACON_DIFFICULTY, BEACON_MIX_HASH, BEACON_NONCE};
use error::{BlockError, Error, ErrorKind};
use ethereum_types::{H256, H64};
use ethjson::types::header::Header;
use spec::Spec;
use test_helpers::{self, TestBlockBuilder};
use unexpected::{Mismatch, OutOfBounds};
use verification::queue::kind::blocks::Unverified;

fn create_seal(mix_hash: &H256, nonce: &H64) -> Vec<Bytes> {
    vec![rlp::encode(mix_hash), rlp::encode(nonce)]
}

#[test]
fn test_header_verification() {
    let client = test_helpers::generate_dummy_client_with_spec(Spec::new_test_beacon);

    let block = TestBlockBuilder::new(client.clone())
        .set_seal(create_seal(&BEACON_MIX_HASH, &BEACON_NONCE))
        .build();

    if let Err(e) = client.import_block(
        Unverified::from_rlp(block, client.engine().params().eip1559_transition).unwrap(),
    ) {
        panic!("error importing block with valid header: {:?}", e);
    }
}

#[test]
fn test_header_verification_with_invalid_difficulty_fails() {
    let client = test_helpers::generate_dummy_client_with_spec(Spec::new_test_beacon);

    let invalid_difficulty = 1.into();
    let block = TestBlockBuilder::new(client.clone())
        .set_difficulty(invalid_difficulty)
        .set_seal(create_seal(&BEACON_MIX_HASH, &BEACON_NONCE))
        .build();

    let result = client.import_block(
        Unverified::from_rlp(block, client.engine().params().eip1559_transition).unwrap(),
    );
    match result {
        Err(Error(
            ErrorKind::Block(BlockError::InvalidDifficulty(Mismatch {
                expected: BEACON_DIFFICULTY,
                found: _,
            })),
            _,
        )) => {}
        Err(_) => {
            panic!(
                "should be invalid difficulty mismatch error (got {:?})",
                result
            )
        }
        _ => {
            panic!("should be error, got Ok");
        }
    }
}

#[test]
fn test_header_verification_with_invalid_mix_hash_fails() {
    let client = test_helpers::generate_dummy_client_with_spec(Spec::new_test_beacon);

    let invalid_mix_hash = H256::from_low_u64_be(1);
    let block = TestBlockBuilder::new(client.clone())
        .set_seal(create_seal(&invalid_mix_hash, &BEACON_NONCE))
        .build();

    let result = client.import_block(
        Unverified::from_rlp(block, client.engine().params().eip1559_transition).unwrap(),
    );
    match result {
        Err(Error(
            ErrorKind::Block(BlockError::MismatchedH256SealElement(Mismatch {
                expected,
                found: _,
            })),
            _,
        )) if expected == BEACON_MIX_HASH => {}
        Err(_) => {
            panic!("should be invalid seal mismatch error (got {:?})", result)
        }
        _ => {
            panic!("should be error, got Ok");
        }
    }
}

#[test]
fn test_header_verification_with_invalid_nonce_fails() {
    let client = test_helpers::generate_dummy_client_with_spec(Spec::new_test_beacon);

    let invalid_nonce = H64::from_low_u64_be(1);
    let block = TestBlockBuilder::new(client.clone())
        .set_seal(create_seal(&BEACON_MIX_HASH, &invalid_nonce))
        .build();

    let result = client.import_block(
        Unverified::from_rlp(block, client.engine().params().eip1559_transition).unwrap(),
    );
    match result {
        Err(Error(ErrorKind::Block(BlockError::InvalidSeal), _)) => {}
        Err(_) => {
            panic!("should be invalid seal error (got {:?})", result)
        }
        _ => {
            panic!("should be error, got Ok");
        }
    }
}

#[test]
fn test_header_verification_with_too_big_extra_data_fails() {
    let client = test_helpers::generate_dummy_client_with_spec(Spec::new_test_beacon);

    let too_large_extra_data = vec![1u8; 33];
    let block = TestBlockBuilder::new(client.clone())
        .set_seal(create_seal(&BEACON_MIX_HASH, &BEACON_NONCE))
        .set_extra_data(too_large_extra_data)
        .build();

    let result = client.import_block(
        Unverified::from_rlp(block, client.engine().params().eip1559_transition).unwrap(),
    );
    match result {
        Err(Error(
            ErrorKind::Block(BlockError::ExtraDataOutOfBounds(OutOfBounds {
                min: None,
                max: Some(32),
                found: _,
            })),
            _,
        )) => {}
        Err(_) => {
            panic!(
                "should be extra data out of bounds error (got {:?})",
                result
            )
        }
        _ => {
            panic!("should be error, got Ok");
        }
    }
}
