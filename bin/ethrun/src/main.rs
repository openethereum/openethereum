// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

mod action;
mod backend;
mod cli;
mod db;
mod debug;
mod exec;
mod machine;
mod wasm;

use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader, Cursor},
    path::Path,
    str::FromStr,
};

use crate::action::{block_action_by_name, tx_action_by_name, BlockActionResult};

use cli::CliOptions;
use common_types::encoded;
use ethereum_types::{Address, U256};
use evm::ActionParams;
use filesize::PathExt;
use indicatif::{ProgressBar, ProgressStyle};
use machine::SmallMachine;
use structopt::StructOpt;

fn evm_call(spec: &ethcore::spec::Spec, codehex: &str) {
    // instantiate a VM that executes EVM smart contracts
    let mut vm = exec::new_evm(&spec).unwrap();
    let mut params = ActionParams::default();
    params.address = Address::from_str("0f572e5295c57f15886f9b263e2f6d2d6c7b5ec6").unwrap();
    params.gas = U256::from(1000000000);
    params.call_type = evm::CallType::Call;
    params.code = Some(hex::decode(codehex).unwrap().into());
    println!("action params: {:#?}", params);
    let callres = vm.call(params).unwrap();
    println!("callres: {:#?}", callres);
}

fn main() -> Result<(), Box<dyn Error>> {
    let opts = CliOptions::from_args();
    let path = Path::new(&opts.input_path);
    println!("startup configuration: {:#?}", &opts);

    let file = File::open(&path)?;
    let spec_read = Cursor::new(include_bytes!("../res/kovan.spec.json"));
    let spec_json: ethjson::spec::Spec = serde_json::from_reader(spec_read.clone())?;
    let spec_core = ethcore::spec::Spec::load(&path, spec_read.clone())?;

    evm_call(&spec_core, "6001600081905550");

    // keep track of read position for progress reporting
    let progress = ProgressBar::new(path.size_on_disk()?);
    progress.set_style(ProgressStyle::default_bar().template(concat!(
        "{elapsed_precise} | {wide_bar} | {percent}% ",
        "| {bytes_per_sec} | eta {eta} | {msg}"
    )));

    // read block by block from ./openethereum export --format hex
    let mut blockno = 0;
    let mut lines_iter = BufReader::new(file).lines();
    let mut block_action = block_action_by_name(&opts.block_action)
        .unwrap()
        .lock()
        .unwrap();
    let mut tx_action = tx_action_by_name(&opts.tx_action).unwrap().lock().unwrap();

    // prints messages above the progress bar
    // for None optionals its a noop
    let optional_print = |msg| {
        if let Some(msg) = msg {
            progress.println(msg);
        }
    };

    // initialize the chain with the genesis block
    if let Some(Ok(genesis)) = lines_iter.next() {
        progress.inc(genesis.len() as u64);
        // create the initial value of the machine that
        // is going to run the entire chain.
        let mut machine = SmallMachine::new(spec_json, encoded::Block::new(hex::decode(genesis)?))?;

        // then for every block, include it in the chain
        while let Some(Ok(block)) = lines_iter.next() {
            // update UI
            blockno += 1;
            progress.inc(block.len() as u64);

            if blockno % 1000 == 0 {
                progress.set_message(&format!("{:08}", blockno));
            }

            // decode block from hex representation in exported file
            let generic_block = encoded::Block::new(hex::decode(block)?);

            // ingest the block by the eth machine and print wasm blocks
            if let Ok(consumed_block) = machine.consume_block(generic_block) {
                match block_action.invoke(&consumed_block) {
                    BlockActionResult::Include(msg) => {
                        optional_print(msg);
                        for tx in consumed_block.transactions() {
                            optional_print(tx_action.invoke(&tx, &consumed_block));
                        }
                    }
                    BlockActionResult::Skip(msg) => optional_print(msg),
                }
            }
        }
        progress.finish_and_clear();
    }
    Ok(())
}
