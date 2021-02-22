// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use url::Url;
use std::error::Error;

use futures::{
  TryStreamExt, 
  stream::Stream
};

use futures_util::{
  SinkExt, StreamExt,
};

use tokio_tungstenite::{
  connect_async,
  tungstenite::protocol::Message
};

type LocalResult<T> = Result<T, Box<dyn Error>>;
type WsResult<T> = Result<T, tokio_tungstenite::tungstenite::Error>;

pub struct BlocksQuery {
  target_server: Url,
  first_block: Option<u64>,
  last_block: Option<u64>
}

impl BlocksQuery {
  pub fn new(target: Url, from: Option<u64>, to: Option<u64>) -> Self {
    BlocksQuery {
      target_server: target,
      first_block: Some(from.unwrap_or(0)),
      last_block: to
    }
  }
}

pub async fn stream_blocks(query: &BlocksQuery) 
  -> LocalResult<impl Stream<Item=WsResult<Message>>> {
  let (wsstream, _) = connect_async(&query.target_server).await?;
  let (mut write, read) = wsstream.split();

  //common_types::block::Block
  for i in 1..10 {
    write.send(Message::text(serde_json::json!({
      "jsonrpc": "2.0",
      "id": i.to_string(),
      "method": "eth_blockNumber",
      "params": []
    }).to_string())).await?;
  }

  Ok(read.into_stream()) //.map_ok(|blockjson| { }
}
