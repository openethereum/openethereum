// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

mod db;
mod machine;

use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use chrono::TimeZone;
use colored::Colorize;
use common_types::encoded;
use ethcore::spec::SpecParams;
use ethjson::spec::Spec;
use filesize::PathExt;
use indicatif::{ProgressBar, ProgressStyle};
use machine::SmallMachine;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "EthState", rename_all = "kebab-case")]
struct ImportOptions {
    #[structopt(short, long)]
    input_path: String,
}

async fn process_blockchain(path: &Path) -> Result<(), Box<dyn Error>> {
    println!("reading blocks from: {:?}", path);

    let file = File::open(path)?;
    let spec: Spec = serde_json::from_slice(include_bytes!("../res/kovan.spec.json"))?;
    let core_spec = ethcore::spec::Spec::load(
        SpecParams::from_path(path),
        &include_bytes!("../res/kovan.spec.json")[..],
    )?;

    println!("spec genesis state: {:#?}", core_spec.genesis_state());
    println!(
        "spec genesis state root: {:#?}",
        core_spec.genesis_header().state_root()
    );

    // keep track of read position for progress reporting
    let progress = ProgressBar::new(path.size_on_disk()?);
    progress.set_style(ProgressStyle::default_bar().template(concat!(
        "{elapsed_precise} | {wide_bar:green/gray} | ",
        "{percent}% | {bytes_per_sec} | eta {eta} | block {msg}k"
    )));

    // read block by block from ./openethereum export --format hex
    let mut blockno = 0;
    let mut lines_iter = BufReader::new(file).lines();

    // initialize the chain with the genesis block
    if let Some(Ok(genesis)) = lines_iter.next() {
        progress.inc(genesis.len() as u64);
        // create the initial value of the machine that
        // is going to run the entire chain.
        let mut machine = SmallMachine::new(spec, encoded::Block::new(hex::decode(genesis)?));

        // then for every block, include it in the chain
        while let Some(Ok(block)) = lines_iter.next() {
            // update UI
            blockno += 1;
            progress.inc(block.len() as u64);

            if blockno % 1000 == 0 {
                progress.set_message(&(blockno / 1000).to_string());
            }

            // ingest the block by the eth machine
            let generic_block =encoded::Block::new(hex::decode(block)?);
            if let Some(wasmblock) = machine.consume_block(generic_block)? {
                let header = wasmblock.header_view();
                progress.println(format!(
                    "{:>8} | {} | {:>3} tx | {:.2} gas | {} | {:#?}",
                    header.number().to_string().cyan().bold(),
                    "wasm".red().bold(),
                    wasmblock.transactions().len().to_string().yellow().bold(),
                    header.gas_used().as_u64() as f64 / 1_000_000f64,
                    chrono::Utc.timestamp(header.timestamp() as i64, 0),
                    header.state_root()
                ));
            }
        }
        progress.finish_and_clear();
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opts = ImportOptions::from_args();
    println! {"options: {:#?}", &opts};
    process_blockchain(&Path::new(&opts.input_path)).await?;
    Ok(())
}
