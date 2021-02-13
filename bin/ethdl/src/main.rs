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

use std::sync::Arc;
use std::error::Error;
use structopt::StructOpt;
use std::time::SystemTime;
use tokio_stream::{self as tstream};
use futures::stream::StreamExt;

#[derive(Debug, StructOpt)]
#[structopt(name="ethdl")]
struct DownloadOptions {
  
  #[structopt(short, long)]
  start_block: Option<u64>,
  
  #[structopt(short, long)]
  end_block: Option<u64>,

  #[structopt()]
  apikey: String,

  #[structopt(short= "n", long="network", default_value="kovan")]
  network: String,

  #[structopt(short= "o", long="output", default_value=".")]
  output_dir: String
}

fn request_url(options: &DownloadOptions) -> String {
  format!("https://{}.infura.io/v3/{}", options.network, options.apikey)
}

async fn get_blockchain_height(options: &DownloadOptions) 
  -> Result<u64, Box<dyn std::error::Error>> {
  let res_json: serde_json::Value = reqwest::Client::new()
    .post(&request_url(&options))
    .json(&serde_json::json!({
      "jsonrpc": "2.0",
      "id": SystemTime::now().elapsed()?.subsec_nanos(),
      "method": "eth_blockNumber",
      "params": []
    })).send().await?.json().await?;
  let valuehex = res_json.get("result").unwrap().as_str().unwrap();
  Ok(u64::from_str_radix(&valuehex[2..], 16)?)
}

async fn get_block_by_number(options: &DownloadOptions, number: u64) 
  -> Result<serde_json::Value, reqwest::Error> {
  reqwest::Client::new()
    .post(&request_url(&options))
    .json(&serde_json::json!({
      "jsonrpc": "2.0",
      "id": SystemTime::now().elapsed().unwrap().subsec_nanos(),
      "method": "eth_getBlockByNumber",
      "params": [format!("0x{:x}", number), true]
    })).send().await?.json::<serde_json::Value>().await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let mut opts = DownloadOptions::from_args();
  opts.start_block = Some(opts.start_block.unwrap_or(0));
  opts.end_block = match opts.end_block {
    None => Some(get_blockchain_height(&opts).await?),
    _ => opts.end_block
  };
  println!("startup options: {:?}", &opts);
  std::fs::create_dir_all(&opts.output_dir)?;
  let blocks_range = opts.start_block.unwrap()..opts.end_block.unwrap();
  let rangelen = blocks_range.end - blocks_range.start;
  println!("about to download {} blocks...", rangelen);
  
  let optstate = Arc::new(opts);
  let mut blocks_stream = tstream::iter(blocks_range)
    .map(|i| { get_block_by_number(&optstate, i) }) 
    .buffer_unordered(num_cpus::get() * 4);
    
  while let Ok(response) = blocks_stream.next().await.unwrap() {
    let blocknohex = response.pointer("/result/number").unwrap().as_str().unwrap();
    let blocknumber = u64::from_str_radix(&blocknohex[2..], 16)?;
    let filename = format!("{}/{}.json", optstate.output_dir, blocknumber); 
    serde_json::to_writer_pretty(
      &std::fs::File::create(&filename).unwrap(), 
      &response).unwrap();
    println!("saving block to: {}  [{}/{}]", 
      filename, blocknumber, rangelen);
  }

  println!("Download complete");

  Ok(())
}
