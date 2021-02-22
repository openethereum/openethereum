// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use chrono::{TimeZone, Utc};
use colored::Colorize;
use common_types::{
    encoded::Block,
    transaction::{TypedTransaction, UnverifiedTransaction},
};
use ethcore::{contract_address, CreateContractAddress};
use ethereum_types::{Address, Public};
use indicatif::HumanBytes;
use std::{cmp::min, error::Error, io::Write};
use tiny_keccak::Keccak;

pub trait Keccak256<T> {
    fn keccak256(&self) -> T
    where
        T: Sized;
}

impl Keccak256<[u8; 32]> for [u8] {
    fn keccak256(&self) -> [u8; 32] {
        let mut keccak = Keccak::new_keccak256();
        let mut result = [0u8; 32];
        keccak.update(self);
        keccak.finalize(&mut result);
        result
    }
}

fn public_to_address(public: &Public) -> Address {
    let hash = public.keccak256();
    let mut result = Address::default();
    result.copy_from_slice(&hash[12..]);
    result
}

pub fn format_block_row(block: &Block) -> String {
    let header = block.header_view();
    format!(
        "{:>8} | {} | {:>3} tx | {:.2} gas | {} | {:#?}",
        header.number().to_string().cyan().bold(),
        "wasm".red().bold(),
        block.transactions().len().to_string().yellow().bold(),
        header.gas_used().as_u64() as f64 / 1_000_000f64,
        Utc.timestamp(header.timestamp() as i64, 0),
        header.state_root()
    )
}

pub fn format_transaction(tx: &UnverifiedTransaction) -> Result<String, Box<dyn Error>> {
    let mut output = Vec::new();
    let sender = public_to_address(&tx.recover_public()?);
    writeln!(&mut output, " - tx {:?}", tx.hash())?;
    writeln!(&mut output, "    sender: {:?}", sender)?;
    if let TypedTransaction::Legacy(tx) = tx.as_unsigned() {
        let address = contract_address(
            CreateContractAddress::FromSenderAndNonce,
            &sender,
            &tx.nonce,
            &tx.data,
        );
        writeln!(&mut output, "    value: {:?}", tx.value)?;
        writeln!(&mut output, "    action: {:?}", tx.action)?;
        writeln!(&mut output, "    address: {:?}", address.0)?;
        writeln!(
            &mut output,
            "    code: 0x{}..{} [{}]",
            &hex::encode(&tx.data[0..min(8, tx.data.len())]),
            &hex::encode(&tx.data[tx.data.len() - 8..]),
            HumanBytes(tx.data.len() as u64)
        )?;
        writeln!(
            &mut output,
            "    codehash: 0x{}",
            &hex::encode(tx.data.keccak256())
        )?;
    }
    Ok(String::from_utf8(output)?)
}
