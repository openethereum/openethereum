// use ethcore_sync::SyncConfig;
// use trie_db as trie;
// use std::path::*;
// use std::{convert::TryFrom, env::temp_dir, error::Error};
// use ethereum_types::H256;
// use trie::{Trie, TrieMut};
// use ethtire::{TrieDB, TrieDBMut};
// use journaldb;
// use patricia_trie_ethereum as ethtire;
// use futures_util::StreamExt;
// use ethcore::{ethereum::new_kovan, spec::Genesis};

// async fn trie_db_tests() -> Result<(), Box<dyn Error>> {
//   let mut root = H256::new();
//   let mut memdb = journaldb::new_memory_db();

//   {
//       println!("state trie root clean state: {:#?}", &root);
//   }
  
//   {
//       let mut triemut = TrieDBMut::new(&mut memdb, &mut root);
//       triemut.insert(b"foo", b"bar")?;
//   }

//   {
//       let trie = TrieDB::new(&memdb, &root)?;
//       println!("state trie root after insert 1: {:#?}", trie.root());
//   }

//   {
//       let mut triemut = TrieDBMut::new(&mut memdb, &mut root);
//       for i in 0..100 {
//           for j in 0..180 {
//               let key = format!("key-{:02}-{:003}", i, j);
//               let value = format!("value-{}", std::char::from_u32('a' as u32 + (j % 26)).unwrap());
//               triemut.insert(&key.as_bytes(), &value.as_bytes())?;
//               println!("state root updated after insertion: {:#?}", triemut.root());
//           }
//       }
//   }

//   {
//       let trie = TrieDB::new(&memdb, &root)?;
//       for kv in trie.iter()? {
//           let pair = &kv?;
//           println!("{}: {}", 
//               std::str::from_utf8(&pair.0)?, 
//               std::str::from_utf8(&pair.1)?);
//       }
//   }

//   Ok(())
// }

// async fn eth_ws_test() -> Result<(), Box<dyn Error>> {
//   let peer = url::Url::parse("ws://localhost:8546")?;
//   let query = sink::BlocksQuery::new(peer, None, None);
//   let mut bstream = sink::stream_blocks(&query).await?;

//   while let Some(msg) = bstream.next().await {
//       println!("ws message: {:#?}", msg);
//   }
//   Ok(())
// }

// fn eth_sync_test() -> Result<(), Box<dyn Error>> {    
//   let dirs = dir::Directories::default();
//   let spec_params = ethcore::spec::SpecParams::from_path(
//       std::path::Path::new(&dirs.cache));

//   println!("dirs: {:#?}", &dirs);
//   println!("spec params: {:#?}", &spec_params);
  
//   let spec = ethcore::spec::Spec::load(&dirs.cache, 
//       include_bytes!("../../../crates/ethcore/res/chainspec/kovan.json") as &[u8])?;
//   let genesis_hash = spec.genesis_header().hash();
//   let db_dirs = dir::DatabaseDirectories {
//       path: dirs.db.clone(),
//       legacy_path: dirs.base.clone(),
//       genesis_hash,
//       fork_name: None,
//       spec_name: spec.data_dir.clone()
//   };
//   let client_path = db_dirs.client_path(
//       journaldb::Algorithm::Archive);

//   println!("spec name: {}", &spec.name);
//   println!("spec data-dir: {}", &spec.data_dir);
//   println!("spec root: {}", &spec.state_root());
//   println!("spec genesis hash: {}", genesis_hash);
//   println!("spec genesis: {:?}", &spec.genesis_header());
//   println!("db dirs: {:#?}", db_dirs);
//   println!("defaults dir: {:?}", db_dirs.user_defaults_path());
//   println!("client path: {:?}", &client_path);
//   println!("snapshot path: {:?}", db_dirs.snapshot_path());
//   println!("fork block: {:?}", spec.fork_block());
  
//   let mut sync_config = SyncConfig::default();
//   sync_config.network_id = 42;
//   sync_config.subprotocol_name.clone_from_slice(
//       spec.subprotocol_name().as_bytes());
//   sync_config.fork_block = spec.fork_block();
//   sync_config.download_old_blocks = true;

//   let blooms_path = client_path.join("blooms");
//   let trace_blooms_path = client_path.join("trace_blooms");

//   std::fs::create_dir_all(&blooms_path)?;
//   std::fs::create_dir_all(&trace_blooms_path)?;

//   // database columns
//   /// Column for State
//   const COL_STATE: Option<u32> = Some(0);
//   /// Column for Block headers
//   const COL_HEADERS: Option<u32> = Some(1);
//   /// Column for Block bodies
//   const COL_BODIES: Option<u32> = Some(2);
//   /// Column for Extras
//   const COL_EXTRA: Option<u32> = Some(3);
//   /// Column for Traces
//   const COL_TRACE: Option<u32> = Some(4);
//   /// Column for the accounts existence bloom filter.
//   #[deprecated(since = "3.0.0", note = "Accounts bloom column is deprecated")]
//   const COL_ACCOUNT_BLOOM: Option<u32> = Some(5);
//   /// Column for general information from the local node which can persist.
//   const COL_NODE_INFO: Option<u32> = Some(6);
//   /// Number of columns in DB
//   const NUM_COLUMNS: Option<u32> = Some(7);

//   let mut db_config = kvdb_rocksdb::DatabaseConfig::with_columns(NUM_COLUMNS);
//   db_config.memory_budget = Some(8); // in megabytes
//   db_config.compaction = kvdb_rocksdb::CompactionProfile::ssd();
//   let db = kvdb_rocksdb::Database::open(&db_config, 
//       client_path.as_path().to_str().unwrap()).unwrap();
//   println!("opened a database with {} columns", db.num_columns());

//   Ok(())
// }