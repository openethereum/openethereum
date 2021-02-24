// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use lazy_static::lazy_static;
use structopt::StructOpt;

lazy_static! {
    /// gets a list of all known valid block actions short names that can be used
    /// as a CLI argument. Those strings are translated into concrete implementations.
    static ref BLOCK_ACTIONS_VALUES: Vec<String> =
        crate::action::BLOCK_ACTIONS
            .iter()
            .map(|a| a.short_name())
            .collect();

    static ref BLOCK_ACTIONS_VALUES_REF: Vec<&'static str> =
        BLOCK_ACTIONS_VALUES.iter().map(|v| v as &str).collect();

    /// gets a list of all known valid transaction actions short names.
    /// This is used to map string-based CLI parameters to concrete
    /// implementations of TX actions.
    static ref TRANSACTION_ACTIONS_VALUES: Vec<String> =
        crate::action::TRANSACTION_ACTIONS
            .iter()
            .map(|a| a.short_name())
            .collect();

    static ref TRANSACTION_ACTIONS_VALUES_REF: Vec<&'static str> =
        TRANSACTION_ACTIONS_VALUES.iter().map(|v| v as &str).collect();
}

#[derive(Debug, StructOpt)]
#[structopt(name = "EthRun", rename_all = "kebab-case")]
pub(crate) struct CliOptions {
    #[structopt(short, long)]
    pub input_path: String,

    #[structopt(
        short, long,
        possible_values = &BLOCK_ACTIONS_VALUES_REF)]
    pub block_action: String,

    #[structopt(
        short, long,
        possible_values = &TRANSACTION_ACTIONS_VALUES_REF)]
    pub tx_action: String,
}
