// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use common_types::{encoded::Block, transaction::UnverifiedTransaction};

use lazy_static::lazy_static;

use crate::{
    debug,
    wasm::{has_wasm_create_txs, is_wasm_create_tx},
};

/// Decides if the transactions within a block should be included and transaction actions
/// executed for each contained transaction, or txs within a block should be skipped.
pub(crate) enum BlockActionResult {
    Include(Option<String>),
    Skip(Option<String>),
}

/// This type represents a functor that will be invoked on every block in the
/// blockchain. If the functor returns true, then transaction actions (if any)
/// will be executed against transactions in that block, otherwise the txs
/// will be omitted. So one of the usecases of this functor is to act as
// a sort of filter/predicate/
pub(crate) trait BlockAction: Send + Sync {
    fn short_name(&self) -> String;
    fn display_name(&self) -> String;
    fn invoke(&self, b: &Block) -> BlockActionResult;
}

pub(crate) fn block_action_by_name(short_name: &str) -> Option<&Box<dyn BlockAction>> {
    BLOCK_ACTIONS.iter().find(|b| short_name == b.short_name())
}

pub(crate) struct StatelessBlockAction {
    short_name: &'static str,
    display_name: &'static str,
    action: &'static dyn Fn(&Block) -> BlockActionResult,
}

impl StatelessBlockAction {
    pub fn new(
        name: &'static str,
        action: &'static dyn Fn(&Block) -> BlockActionResult,
    ) -> Box<Self> {
        Box::new(StatelessBlockAction {
            short_name: name,
            display_name: name,
            action: action,
        })
    }
}

impl BlockAction for StatelessBlockAction {
    fn short_name(&self) -> String {
        String::from(self.short_name)
    }

    fn display_name(&self) -> String {
        String::from(self.display_name)
    }

    fn invoke(&self, b: &Block) -> BlockActionResult {
        (self.action)(b)
    }
}

unsafe impl Send for StatelessBlockAction {}
unsafe impl Sync for StatelessBlockAction {}

/// Describes a transaction implementation and its metadata.
/// This information is needed to address a secific type of
/// an action from the command line interface.
pub(crate) trait TransactionAction: Send + Sync {
    fn short_name(&self) -> String;
    fn display_name(&self) -> String;
    fn invoke(&self, t: &UnverifiedTransaction, b: &Block) -> Option<String>;
}

pub(crate) fn tx_action_by_name(short_name: &str) -> Option<&Box<dyn TransactionAction>> {
    TRANSACTION_ACTIONS
        .iter()
        .find(|t| short_name == t.short_name())
}

pub(crate) struct StatelessTransactionAction {
    short_name: &'static str,
    display_name: &'static str,
    action: &'static dyn Fn(&UnverifiedTransaction, &Block) -> Option<String>,
}

impl StatelessTransactionAction {
    pub fn new(
        name: &'static str,
        action: &'static dyn Fn(&UnverifiedTransaction, &Block) -> Option<String>,
    ) -> Box<Self> {
        Box::new(StatelessTransactionAction {
            short_name: name,
            display_name: name,
            action: action,
        })
    }
}

impl TransactionAction for StatelessTransactionAction {
    fn short_name(&self) -> String {
        String::from(self.short_name)
    }

    fn display_name(&self) -> String {
        String::from(self.display_name)
    }

    fn invoke(&self, t: &UnverifiedTransaction, b: &Block) -> Option<String> {
        (self.action)(t, b)
    }
}

unsafe impl Send for StatelessTransactionAction {}
unsafe impl Sync for StatelessTransactionAction {}

lazy_static! {
    /// A list of all actions that could be executed during a blockchain run on the
    /// block-level. Everytime an action is executed on a block, it returns a BlockActionResult
    /// that specifies if the transaction actions should be invoked on the transactions included
    /// within that block or skip the current block and procede to the next one.
    /// In either case, include or skip, actions have the option to include a debug/output
    /// message that could be printed to stdout.
    pub(crate) static ref BLOCK_ACTIONS: [Box<dyn BlockAction>; 2] =
    [
        // will include only blocks that create new WASM contracts
        StatelessBlockAction::new("filter-create-wasm",
            &|block| match has_wasm_create_txs(&block) {
                true => BlockActionResult::Include(Some(debug::format_block_row(&block))),
                false => BlockActionResult::Skip(None),
            }),

        // will include all blocks in the blockchain
        StatelessBlockAction::new("include-all", &|_| BlockActionResult::Include(None))
    ];

    /// The list of actions that run per transaction in a block
    pub(crate) static ref TRANSACTION_ACTIONS: [Box<dyn TransactionAction>; 1] =
    [
        StatelessTransactionAction::new("print-wasm-create",
            &|utx, &_| match is_wasm_create_tx(&utx) {
                true => Some(debug::format_transaction(&utx).unwrap()),
                false => None,
            })
    ];
}
