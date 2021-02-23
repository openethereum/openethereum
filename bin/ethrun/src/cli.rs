// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use structopt::StructOpt;
use lazy_static::lazy_static;

lazy_static! {
    /// gets a list of all known valid block actions short names that can be used
    /// as a CLI argument. Those strings are translated into concrete implementations.
    static ref BLOCK_ACTIONS_VALUES: Vec<&'static str> =
        crate::action::BLOCK_ACTIONS.iter().map(|a| a.short_name).collect();

    static ref TRANSACTION_ACTIONS_VALUES: Vec<&'static str> = 
        crate::action::TRANSACTION_ACTIONS.iter().map(|a| a.short_name).collect();
}

#[derive(Debug, StructOpt)]
#[structopt(name = "EthRun", rename_all = "kebab-case")]
pub(crate) struct CliOptions {
    #[structopt(short, long)]
    pub input_path: String,

    #[structopt(
        short, long, 
        possible_values = &BLOCK_ACTIONS_VALUES)]
    pub block_action: String,

    #[structopt(
        short, long,
        possible_values = &TRANSACTION_ACTIONS_VALUES)]
    pub tx_action: String,
}
