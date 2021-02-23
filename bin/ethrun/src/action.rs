// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use common_types::{
    encoded::Block,
    transaction::{Action, TypedTransaction, UnverifiedTransaction},
};

use lazy_static::lazy_static;

use crate::debug;

pub(crate) enum BlockActionResult {
    Include(Option<String>),
    Skip(Option<String>),
}

pub(crate) type BlockActionImpl = &'static dyn Fn(&Block) -> BlockActionResult;

/// This type represents a functor that will be invoked on every block in the
/// blockchain. If the functor returns true, then transaction actions (if any)
/// will be executed against transactions in that block, otherwise the txs
/// will be omitted. So one of the usecases of this functor is to act as
// a sort of filter/predicate/
pub(crate) struct BlockAction {
    pub short_name: &'static str,
    pub display_name: &'static str,
    pub action: BlockActionImpl,
}

unsafe impl Send for BlockAction {}
unsafe impl Sync for BlockAction {}

impl BlockAction {
    pub fn from_name(short_name: &str) -> Option<BlockActionImpl> {
        BLOCK_ACTIONS
            .iter()
            .find(|b| short_name == b.short_name)
            .map(|b| b.action)
    }
}

pub(crate) type TransactionActionImpl =
    &'static dyn Fn(&UnverifiedTransaction, &Block) -> Option<String>;

pub(crate) struct TransactionAction {
    pub short_name: &'static str,
    pub display_name: &'static str,
    pub action: TransactionActionImpl,
}

unsafe impl Send for TransactionAction {}
unsafe impl Sync for TransactionAction {}

impl TransactionAction {
    pub fn from_name(short_name: &str) -> Option<TransactionActionImpl> {
        TRANSACTION_ACTIONS
            .iter()
            .find(|b| short_name == b.short_name)
            .map(|b| b.action)
    }
}

lazy_static! {
    pub(crate) static ref BLOCK_ACTIONS: [BlockAction; 1] = [BlockAction {
        short_name: "filter-create-wasm",
        display_name: "Filter pWASM contract creation transactions",
        action: &|block: &Block| -> BlockActionResult {
            let has_wasm_txs = |tx: &UnverifiedTransaction| match tx.as_unsigned() {
                TypedTransaction::Legacy(tx) => match (&tx.action, tx.data.starts_with(b"\0asm")) {
                    (Action::Create, true) => true,
                    _ => false,
                },
                TypedTransaction::AccessList(_) => false,
            };
            match block.transactions().iter().any(has_wasm_txs) {
                true => BlockActionResult::Include(Some(debug::format_block_row(&block))),
                false => BlockActionResult::Skip(None),
            }
        }
    }];
    pub(crate) static ref TRANSACTION_ACTIONS: [TransactionAction; 1] = [TransactionAction {
        short_name: "print-wasm",
        display_name: "Print all pWASM transactions details",
        action: &|utx, &_| match utx.as_unsigned() {
            TypedTransaction::Legacy(tx) => match (&tx.action, tx.data.starts_with(b"\0asm")) {
                (Action::Create, true) => Some(debug::format_transaction(&utx).unwrap()),
                _ => None,
            },
            TypedTransaction::AccessList(_) => None,
        }
    }];
}
