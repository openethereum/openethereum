use super::HookType;
use ethereum_types::U256;
use ethjson::{self, blockchain::Block};
use log::warn;
use rlp::RlpStream;
use std::path::Path;
use types::{
    transaction::{TypedTransaction, TypedTxId, UnverifiedTransaction},
    BlockNumber,
};
use verification::queue::kind::blocks::Unverified;

pub fn json_local_block_en_de_test<H: FnMut(&str, HookType)>(
    _test: &ethjson::test::LocalTests,
    path: &Path,
    json_data: &[u8],
    start_stop_hook: &mut H,
) -> Vec<String> {
    let tests = ethjson::local_tests::BlockEnDeTest::load(json_data).expect(&format!(
        "Could not parse JSON chain test data from {}",
        path.display()
    ));
    let mut failed = Vec::new();

    for (name, ref_block) in tests.into_iter() {
        start_stop_hook(&name, HookType::OnStart);

        let block = Unverified::from_rlp(ref_block.rlp(), BlockNumber::max_value());
        let block = match block {
            Ok(block) => block,
            Err(decoder_err) => {
                warn!(target: "json-tests", "Error decoding test block: {:?}", decoder_err);
                failed.push(name.clone());
                continue;
            }
        };
        if !is_same_block(&ref_block, &block) {
            println!("block failed {:?}", block);
            failed.push(name.clone())
        }

        start_stop_hook(&name, HookType::OnStop);
    }
    failed
}

fn rlp_append_block(block: &Unverified) -> Vec<u8> {
    let mut rlps = RlpStream::new();
    rlps.begin_list(3);
    rlps.append(&block.header);
    UnverifiedTransaction::rlp_append_list(&mut rlps, &block.transactions);
    rlps.append_list(&block.uncles);
    rlps.out()
}

pub fn is_same_block(ref_block: &Block, block: &Unverified) -> bool {
    let test_exp = |exp: bool, err: &str| -> bool {
        if !exp {
            println!("Test mismatch on:{}", err);
        }
        exp
    };

    let header_ok = if let Some(ref header) = ref_block.header {
        test_exp(*block.header.gas_used() == header.gas_used.0, "Gas used")
            && test_exp(
                *block.header.uncles_hash() == header.uncles_hash.0,
                "Uncles hash",
            )
            && test_exp(
                *block.header.transactions_root() == header.transactions_root.0,
                "Transaction Root",
            )
            && test_exp(
                block.header.timestamp() == header.timestamp.0.as_u64(),
                "Timestamp",
            )
            && test_exp(
                *block.header.state_root() == header.state_root.0,
                "StateRoot",
            )
            && test_exp(
                *block.header.receipts_root() == header.receipts_root.0,
                "ReceiptsRoot",
            )
            && test_exp(
                *block.header.parent_hash() == header.parent_hash.0,
                "ParentHash",
            )
            && test_exp(
                block.header.number() == header.number.0.as_u64(),
                "Blocn Number",
            )
            && test_exp(block.header.hash() == header.hash.0, "Header hash")
            && test_exp(*block.header.gas_limit() == header.gas_limit.0, "GasLimit")
            && test_exp(*block.header.gas_used() == header.gas_used.0, "GasUsed")
            && test_exp(
                *block.header.extra_data() == header.extra_data.0,
                "ExtraData",
            )
            && test_exp(
                *block.header.difficulty() == header.difficulty.0,
                "Difficulty",
            )
            && test_exp(*block.header.author() == header.author.0, "Author")
            && test_exp(*block.header.log_bloom() == header.bloom.0, "Bloom")
    } else {
        true
    };

    // check transactions
    let transaction_ok = if let Some(ref txs) = ref_block.transactions {
        let mut is_all_ok = true;
        for (ref_tx, tx) in txs.iter().zip(block.transactions.iter()) {
            // check signatures
            let mut is_ok = test_exp(U256::from(tx.signature().r()) == ref_tx.r.0, "Sig R")
                && test_exp(U256::from(tx.signature().s()) == ref_tx.s.0, "Sig S");
            is_ok = is_ok
                && if let Some(chain_id) = ref_tx.chain_id {
                    test_exp(tx.chain_id() == Some(chain_id.0.as_u64()), "Chain Id")
                } else {
                    true
                };
            // check type
            let ttype = if let Some(ttype) = ref_tx.transaction_type {
                let ttype = ttype.0.byte(0);
                if let Some(id) = TypedTxId::from_u8_id(ttype) {
                    id
                } else {
                    println!("Unknown transaction {}", ttype);
                    continue;
                }
            } else {
                TypedTxId::Legacy
            };
            is_ok = is_ok
                && {
                    match ref_tx.gas_price {
                        Some(gas_price) => {
                            test_exp(tx.tx().gas_price == gas_price.0, "Tx gas price")
                        }
                        None => {
                            test_exp(
                                tx.tx().gas_price == ref_tx.max_fee_per_gas.unwrap_or_default().0,
                                "Tx max fee per gas",
                            ) && test_exp(
                                tx.max_priority_fee_per_gas()
                                    == ref_tx.max_priority_fee_per_gas.unwrap_or_default().0,
                                "Tx max priority fee per gas",
                            )
                        }
                    }
                }
                && test_exp(tx.tx().nonce == ref_tx.nonce.0, "Tx nonce")
                && test_exp(tx.tx().gas == ref_tx.gas_limit.0, "Tx gas")
                && test_exp(tx.tx().value == ref_tx.value.0, "Tx value")
                && test_exp(tx.tx().data == ref_tx.data.0, "Tx data")
                && test_exp(ref_tx.hash.is_some(), "tx hash is none");

            if let Some(hash) = ref_tx.hash {
                is_ok = is_ok && test_exp(tx.hash() == hash, "Hash mismatch");
            }

            // check specific tx data
            is_ok = is_ok
                && match ttype {
                    TypedTxId::Legacy => {
                        test_exp(tx.legacy_v() == ref_tx.v.0.as_u64(), "Original Sig V")
                    }
                    TypedTxId::AccessList | TypedTxId::EIP1559Transaction => {
                        test_exp(tx.standard_v() as u64 == ref_tx.v.0.as_u64(), "Sig V");
                        let al = match tx.as_unsigned() {
                            TypedTransaction::AccessList(tx) => &tx.access_list,
                            TypedTransaction::EIP1559Transaction(tx) => &tx.transaction.access_list,
                            _ => {
                                println!("Wrong data in tx type");
                                continue;
                            }
                        };
                        if let Some(ref ref_al) = ref_tx.access_list {
                            if ref_al.len() != al.len() {
                                println!("Access list mismatch");
                                continue;
                            }
                            let mut is_ok = true;
                            for (al, ref_al) in al.iter().zip(ref_al.iter()) {
                                is_ok = is_ok && test_exp(al.0 == ref_al.address, "AL address");
                                if al.1.len() != ref_al.storage_keys.len() {
                                    println!("Access list mismatch");
                                    continue;
                                }
                                for (key, ref_key) in al.1.iter().zip(ref_al.storage_keys.iter()) {
                                    is_ok = is_ok && test_exp(key == ref_key, "Key mismatch");
                                }
                            }
                            is_ok
                        } else {
                            false
                        }
                    }
                };

            if !is_ok {
                println!(
                    "Transaction not valid got: {:?} \n expected:{:?}\n",
                    tx, ref_tx
                );
            }
            is_all_ok = is_ok && is_all_ok;
        }
        is_all_ok
    } else {
        true
    };

    let encript_ok = {
        let rlp = rlp_append_block(&block);
        rlp == ref_block.rlp()
    };

    header_ok && transaction_ok && encript_ok
}
