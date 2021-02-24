// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

#![feature(fn_traits, unboxed_closures)]

use common_types::{
    encoded::Block,
    transaction::{Action, TypedTransaction, UnverifiedTransaction},
};

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
