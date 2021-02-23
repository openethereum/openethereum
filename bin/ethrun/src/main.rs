// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

mod backend;
mod db;
mod debug;
mod machine;

use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use filesize::PathExt;
use indicatif::{ProgressBar, ProgressStyle};
use structopt::StructOpt;

use common_types::encoded;
use ethjson::spec::Spec;
use machine::{is_wasm_creation_transaction, SmallMachine};

#[derive(Debug, StructOpt)]
#[structopt(name = "EthState", rename_all = "kebab-case")]
struct ImportOptions {
    #[structopt(short, long)]
    input_path: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opts = ImportOptions::from_args();
    let path = Path::new(&opts.input_path);
    println!("startup configuration: {:#?}", &opts);

    let file = File::open(&path)?;
    let spec: Spec = serde_json::from_slice(include_bytes!("../res/kovan.spec.json"))?;

    // keep track of read position for progress reporting
    let progress = ProgressBar::new(path.size_on_disk()?);
    progress.set_style(ProgressStyle::default_bar().template(concat!(
        "{elapsed_precise} | {wide_bar} | {percent}% ",
        "| {bytes_per_sec} | eta {eta} | {msg}"
    )));

    // read block by block from ./openethereum export --format hex
    let mut blockno = 0;
    let mut lines_iter = BufReader::new(file).lines();

    // initialize the chain with the genesis block
    if let Some(Ok(genesis)) = lines_iter.next() {
        progress.inc(genesis.len() as u64);
        // create the initial value of the machine that
        // is going to run the entire chain.
        let mut machine = SmallMachine::new(spec, encoded::Block::new(hex::decode(genesis)?))?;

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
            if let Some(wasmblock) = machine.consume_block(generic_block)? {
                progress.println(debug::format_block_row(&wasmblock));
                for tx in wasmblock.transactions() {
                    if is_wasm_creation_transaction(&&tx) {
                        progress.println(debug::format_transaction(&tx)?);
                    }
                }
            }
        }
        progress.finish_and_clear();
    }
    Ok(())
}
