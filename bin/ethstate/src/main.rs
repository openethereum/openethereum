// Copyright 2015-2020 Parity Technologies (UK) Ltd.
// This file is part of OpenEthereum.

// OpenEthereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// OpenEthereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with OpenEthereum.  If not, see <http://www.gnu.org/licenses/>.

use std::{
    fs::File, 
    path::Path,
    error::Error, 
    io::{BufReader, BufRead}, 
};

use common_types::{
    block::Block
};

use colored::*;
use chrono::TimeZone;
use ethereum_types::H256;
use journaldb::new_memory_db;
use keccak_hasher::KeccakHasher;
use kvdb::DBValue;
use memory_db::MemoryDB;
use structopt::StructOpt;
use rlp::{Rlp, Decodable};
use trie_db::TrieMut;
use patricia_trie_ethereum::TrieDBMut;
use pad::PadStr;

#[derive(Debug, StructOpt)]
#[structopt(name="EthState", rename_all="kebab-case")]
struct ImportOptions {
    #[structopt(short, long)]
    input_path: String
}

struct GlobalState {
    pub root: H256,
    pub state: MemoryDB<KeccakHasher, DBValue>
}

impl GlobalState {
    fn new() -> Self {
        GlobalState {
            root: H256::new(),
            state: new_memory_db()
        }
    }
}

fn process_block(buffer : &[u8], index: usize, state: &mut GlobalState) -> Result<(), Box<dyn Error>> {
    let size = size::Size::Bytes(buffer.len());
    let block = Block::decode(&Rlp::new(buffer))?;
    if block.transactions.len() != 0 {
        TrieDBMut::new(&mut state.state, &mut state.root).insert(
            block.header.author(), block.header.transactions_root())?;
        println!("processing #{} | {:>3} txs | sz: {} | ts: {} | sr: {:#?}", 
            index, block.transactions.len().to_string().yellow().bold(), 
            format!("{}", size).pad_to_width(10), 
            chrono::Utc.timestamp(block.header.timestamp() as i64, 0),
            state.root)
    }

    Ok(())
}

async fn process_blockchain(path: &Path) -> Result<(), Box<dyn Error>> {
    println!("reading blocks from: {:?}", path);
    let file = File::open(path)?;
    let mut state = GlobalState::new();
    for block in BufReader::new(file).lines().zip(0..) {
        process_block(&hex::decode(block.0?)?, block.1, &mut state)?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opts = ImportOptions::from_args();
    println!{"options: {:#?}", &opts};
    process_blockchain(&Path::new(&opts.input_path)).await?;
    Ok(())
}
