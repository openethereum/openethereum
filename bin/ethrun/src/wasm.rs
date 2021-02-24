// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use crate::{action::TransactionAction, debug::Keccak256};
use colored::Colorize;
use common_types::{
    encoded::Block,
    transaction::{Action, TypedTransaction, UnverifiedTransaction},
};
use ethcore::{contract_address, CreateContractAddress};
use ethereum_types::{Address, Public};
use std::collections::BTreeMap;

/// check for all transactions within a block, if any of them
/// matches the criteria, the entire block is concluded as having wasm contracts
/// creation transactions.
pub(crate) fn has_wasm_create_txs(block: &Block) -> bool {
    block.transactions().iter().any(is_wasm_create_tx)
}

/// Checks if the given transaction is a CREATE/CREATE2 call and that the
/// supplied contract code begins with the pWASM magic signature bytes.
pub(crate) fn is_wasm_create_tx(tx: &UnverifiedTransaction) -> bool {
    match tx.as_unsigned() {
        TypedTransaction::Legacy(tx) => match (&tx.action, tx.data.starts_with(b"\0asm")) {
            (Action::Create, true) => true,
            _ => false,
        },
        TypedTransaction::AccessList(_) => false,
    }
}

fn public_to_address(public: &Public) -> Address {
    let hash = public.keccak256();
    let mut result = Address::default();
    result.copy_from_slice(&hash[12..]);
    result
}

pub(crate) struct WasmContractsWithTxsDump {
    wasm_txs: BTreeMap<Address, Vec<UnverifiedTransaction>>,
}

impl WasmContractsWithTxsDump {
    pub fn new() -> Self {
        WasmContractsWithTxsDump {
            wasm_txs: BTreeMap::new(),
        }
    }
}

impl TransactionAction for WasmContractsWithTxsDump {
    fn short_name(&self) -> String {
        String::from("wasm-map")
    }

    fn display_name(&self) -> String {
        self.short_name()
    }

    fn invoke(&mut self, t: &UnverifiedTransaction, b: &Block) -> Option<String> {
        match t.as_unsigned() {
            TypedTransaction::Legacy(tx) => match (&tx.action, tx.data.starts_with(b"\0asm")) {
                (Action::Create, true) => {
                    let (addr, _) = contract_address(
                        CreateContractAddress::FromSenderAndNonce,
                        &public_to_address(&t.recover_public().unwrap()),
                        &tx.nonce,
                        &tx.data,
                    );

                    self.wasm_txs.insert(addr, Vec::new());
                    Some(format!(
                        "#{} - {} => {:?} @ {:?}",
                        b.number(),
                        "wasm create".red().bold(),
                        addr,
                        t.hash()
                    ))
                } // wasm create
                (Action::Create, false) => None, // evm create
                (Action::Call(addr), _) => match self.wasm_txs.get_mut(addr) {
                    None => None,
                    Some(callsvec) => {
                        callsvec.push(t.clone());
                        let sender = public_to_address(&t.recover_public().unwrap());
                        Some(format!(
                            "#{} - {} => {:?} by {} @ {:?}",
                            b.number(),
                            "wasm call  ".green().bold(),
                            addr,
                            &format!("{:?}", sender).dimmed(),
                            t.hash()
                        ))
                    }
                }, // contract call or simple transfer
            },
            _ => None,
        }
    }
}
