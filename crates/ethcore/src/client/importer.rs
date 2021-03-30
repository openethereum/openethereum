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

use super::{
    ancient_import::AncientVerifier,
    bad_blocks,
    blockchain::BlockChainClient,
    chain_notify::{ChainRoute, NewBlocks},
    Client, ClientConfig, ClientIoMessage,
};
use crate::{
    block::{Drain, LockedBlock},
    error,
    state_db::StateDB,
    verification::{self, queue::kind::blocks::Unverified, BlockQueue, PreverifiedBlock, Verifier},
};
use block::enact_verified;
use blockchain::BlockProvider;
use blockchain::{BlockChain, ExtrasInsert, ImportRoute};
use db::DBTransaction;
use db::KeyValueDB;
use engines::{epoch::Transition, EngineError, EthEngine};
use error::EthcoreResult;
use ethereum_types::{H256, U256};
use evm::EnvInfo;
use executive::{Executive, TransactOptions};
use io::IoChannel;
use miner::Miner;
use miner::MinerService;
use parking_lot::Mutex;
use rand::rngs::OsRng;
use rlp::Rlp;
use state::State;
use std::{collections::HashSet, sync::Arc, time::Instant};
use trace::{Database, ImportRequest};
use types::{
    ancestry_action::AncestryAction,
    encoded,
    engines::epoch::PendingTransition,
    engines::ForkChoice,
    header::{ExtendedHeader, Header},
    ids::BlockId,
    receipt::TypedReceipt,
};

pub struct Importer {
    /// Lock used during block import
    pub import_lock: Mutex<()>, // FIXME Maybe wrap the whole `Importer` instead?

    /// Used to verify blocks
    pub verifier: Box<dyn Verifier<Client>>,

    /// Queue containing pending blocks
    pub block_queue: BlockQueue,

    /// Handles block sealing
    pub miner: Arc<Miner>,

    /// Ancient block verifier: import an ancient sequence of blocks in order from a starting epoch
    pub ancient_verifier: AncientVerifier,

    /// Ethereum engine to be used during import
    pub engine: Arc<dyn EthEngine>,

    /// A lru cache of recently detected bad blocks
    pub bad_blocks: bad_blocks::BadBlocks,
}

impl Importer {
    pub fn new(
        config: &ClientConfig,
        engine: Arc<dyn EthEngine>,
        message_channel: IoChannel<ClientIoMessage>,
        miner: Arc<Miner>,
    ) -> Result<Importer, error::Error> {
        let block_queue = BlockQueue::new(
            config.queue.clone(),
            engine.clone(),
            message_channel.clone(),
            config.verifier_type.verifying_seal(),
        );

        Ok(Importer {
            import_lock: Mutex::new(()),
            verifier: verification::new(config.verifier_type.clone()),
            block_queue,
            miner,
            ancient_verifier: AncientVerifier::new(engine.clone()),
            engine,
            bad_blocks: Default::default(),
        })
    }

    // t_nb 6.0 This is triggered by a message coming from a block queue when the block is ready for insertion
    pub fn import_verified_blocks(&self, client: &Client) -> usize {
        // Shortcut out if we know we're incapable of syncing the chain.
        trace!(target: "block_import", "fn import_verified_blocks");
        if !client.enabled() {
            self.block_queue.reset_verification_ready_signal();
            return 0;
        }

        let max_blocks_to_import = client.config.max_round_blocks_to_import;
        let (
            imported_blocks,
            import_results,
            invalid_blocks,
            imported,
            proposed_blocks,
            duration,
            has_more_blocks_to_import,
        ) = {
            let mut imported_blocks = Vec::with_capacity(max_blocks_to_import);
            let mut invalid_blocks = HashSet::new();
            let proposed_blocks = Vec::with_capacity(max_blocks_to_import);
            let mut import_results = Vec::with_capacity(max_blocks_to_import);

            let _import_lock = self.import_lock.lock();
            let blocks = self.block_queue.drain(max_blocks_to_import);
            if blocks.is_empty() {
                debug!(target: "block_import", "block_queue is empty");
                self.block_queue.resignal_verification();
                return 0;
            }
            trace_time!("import_verified_blocks");
            let start = Instant::now();

            for block in blocks {
                let header = block.header.clone();
                let bytes = block.bytes.clone();
                let hash = header.hash();

                let is_invalid = invalid_blocks.contains(header.parent_hash());
                if is_invalid {
                    debug!(
                        target: "block_import",
                        "Refusing block #{}({}) with invalid parent {}",
                        header.number(),
                        header.hash(),
                        header.parent_hash()
                    );
                    invalid_blocks.insert(hash);
                    continue;
                }
                // t_nb 7.0 check and lock block
                match self.check_and_lock_block(&bytes, block, client) {
                    Ok((closed_block, pending)) => {
                        imported_blocks.push(hash);
                        let transactions_len = closed_block.transactions.len();
                        trace!(target:"block_import","Block #{}({}) check pass",header.number(),header.hash());
                        // t_nb 8.0 commit block to db
                        let route = self.commit_block(
                            closed_block,
                            &header,
                            encoded::Block::new(bytes),
                            pending,
                            client,
                        );
                        trace!(target:"block_import","Block #{}({}) commited",header.number(),header.hash());
                        import_results.push(route);
                        client
                            .report
                            .write()
                            .accrue_block(&header, transactions_len);
                    }
                    Err(err) => {
                        self.bad_blocks.report(bytes, format!("{:?}", err));
                        invalid_blocks.insert(hash);
                    }
                }
            }

            let imported = imported_blocks.len();
            let invalid_blocks = invalid_blocks.into_iter().collect::<Vec<H256>>();

            if !invalid_blocks.is_empty() {
                self.block_queue.mark_as_bad(&invalid_blocks);
            }
            let has_more_blocks_to_import = !self.block_queue.mark_as_good(&imported_blocks);
            (
                imported_blocks,
                import_results,
                invalid_blocks,
                imported,
                proposed_blocks,
                start.elapsed(),
                has_more_blocks_to_import,
            )
        };

        {
            if !imported_blocks.is_empty() {
                trace!(target:"block_import","Imported block, notify rest of system");
                let route = ChainRoute::from(import_results.as_ref());

                // t_nb 10 Notify miner about new included block.
                if !has_more_blocks_to_import {
                    self.miner.chain_new_blocks(
                        client,
                        &imported_blocks,
                        &invalid_blocks,
                        route.enacted(),
                        route.retracted(),
                        false,
                    );
                }

                // t_nb 11 notify rest of system about new block inclusion
                client.notify(|notify| {
                    notify.new_blocks(NewBlocks::new(
                        imported_blocks.clone(),
                        invalid_blocks.clone(),
                        route.clone(),
                        Vec::new(),
                        proposed_blocks.clone(),
                        duration,
                        has_more_blocks_to_import,
                    ));
                });
            }
        }
        trace!(target:"block_import","Flush block to db");
        let db = client.db.read();
        db.key_value().flush().expect("DB flush failed.");

        self.block_queue.resignal_verification();
        trace!(target:"block_import","Resignal verifier");
        imported
    }

    // t_nb 6.0.1 check and lock block,
    fn check_and_lock_block(
        &self,
        bytes: &[u8],
        block: PreverifiedBlock,
        client: &Client,
    ) -> EthcoreResult<(LockedBlock, Option<PendingTransition>)> {
        let engine = &*self.engine;
        let header = block.header.clone();

        // Check the block isn't so old we won't be able to enact it.
        // t_nb 7.1 check if block is older then last pruned block
        let best_block_number = client.chain.read().best_block_number();
        if client.pruning_info().earliest_state > header.number() {
            warn!(target: "client", "Block import failed for #{} ({})\nBlock is ancient (current best block: #{}).", header.number(), header.hash(), best_block_number);
            bail!("Block is ancient");
        }

        // t_nb 7.2 Check if parent is in chain
        let parent = match client.block_header_decoded(BlockId::Hash(*header.parent_hash())) {
            Some(h) => h,
            None => {
                warn!(target: "client", "Block import failed for #{} ({}): Parent not found ({}) ", header.number(), header.hash(), header.parent_hash());
                bail!("Parent not found");
            }
        };

        let chain = client.chain.read();
        // t_nb 7.3 verify block family
        let verify_family_result = self.verifier.verify_block_family(
            &header,
            &parent,
            engine,
            Some(verification::FullFamilyParams {
                block: &block,
                block_provider: &**chain,
                client,
            }),
        );

        if let Err(e) = verify_family_result {
            warn!(target: "client", "Stage 3 block verification failed for #{} ({})\nError: {:?}", header.number(), header.hash(), e);
            bail!(e);
        };

        // t_nb 7.4 verify block external
        let verify_external_result = self.verifier.verify_block_external(&header, engine);
        if let Err(e) = verify_external_result {
            warn!(target: "client", "Stage 4 block verification failed for #{} ({})\nError: {:?}", header.number(), header.hash(), e);
            bail!(e);
        };

        // Enact Verified Block
        // t_nb 7.5 Get build last hashes. Get parent state db. Get epoch_transition
        let last_hashes = client.build_last_hashes(header.parent_hash());

        let db = client
            .state_db
            .read()
            .boxed_clone_canon(header.parent_hash());

        let is_epoch_begin = chain
            .epoch_transition(parent.number(), *header.parent_hash())
            .is_some();

        // t_nb 8.0 Block enacting. Execution of transactions.
        let enact_result = enact_verified(
            block,
            engine,
            client.tracedb.read().tracing_enabled(),
            db,
            &parent,
            last_hashes,
            client.factories.clone(),
            is_epoch_begin,
            &mut chain.ancestry_with_metadata_iter(*header.parent_hash()),
        );

        let mut locked_block = match enact_result {
            Ok(b) => b,
            Err(e) => {
                warn!(target: "client", "Block import failed for #{} ({})\nError: {:?}", header.number(), header.hash(), e);
                bail!(e);
            }
        };

        // t_nb 7.6 Strip receipts for blocks before validate_receipts_transition,
        // if the expected receipts root header does not match.
        // (i.e. allow inconsistency in receipts outcome before the transition block)
        if header.number() < engine.params().validate_receipts_transition
            && header.receipts_root() != locked_block.header.receipts_root()
        {
            locked_block.strip_receipts_outcomes();
        }

        // t_nb 7.7 Final Verification. See if block that we created (executed) matches exactly with block that we received.
        if let Err(e) = self
            .verifier
            .verify_block_final(&header, &locked_block.header)
        {
            warn!(target: "client", "Stage 5 block verification failed for #{} ({})\nError: {:?}", header.number(), header.hash(), e);
            bail!(e);
        }

        let pending = self.check_epoch_end_signal(
            &header,
            bytes,
            &locked_block.receipts,
            locked_block.state.db(),
            client,
        )?;

        Ok((locked_block, pending))
    }

    /// Import a block with transaction receipts.
    ///
    /// The block is guaranteed to be the next best blocks in the
    /// first block sequence. Does no sealing or transaction validation.
    pub fn import_old_block(
        &self,
        unverified: Unverified,
        receipts_bytes: &[u8],
        db: &dyn KeyValueDB,
        chain: &BlockChain,
    ) -> EthcoreResult<()> {
        let receipts = TypedReceipt::decode_rlp_list(&Rlp::new(receipts_bytes))
            .unwrap_or_else(|e| panic!("Receipt bytes should be valid: {:?}", e));
        let _import_lock = self.import_lock.lock();

        if unverified.header.number() >= chain.best_block_header().number() {
            panic!("Ancient block number is higher then best block number");
        }

        {
            trace_time!("import_old_block");
            // verify the block, passing the chain for updating the epoch verifier.
            let mut rng = OsRng;
            self.ancient_verifier
                .verify(&mut rng, &unverified.header, &chain)?;

            // Commit results
            let mut batch = DBTransaction::new();
            chain.insert_unordered_block(
                &mut batch,
                encoded::Block::new(unverified.bytes),
                receipts,
                None,
                false,
                true,
            );
            // Final commit to the DB
            db.write_buffered(batch);
            chain.commit();
        }
        db.flush().expect("DB flush failed.");
        Ok(())
    }

    // NOTE: the header of the block passed here is not necessarily sealed, as
    // it is for reconstructing the state transition.
    //
    // The header passed is from the original block data and is sealed.
    // TODO: should return an error if ImportRoute is none, issue #9910
    pub fn commit_block<B>(
        &self,
        block: B,
        header: &Header,
        block_data: encoded::Block,
        pending: Option<PendingTransition>,
        client: &Client,
    ) -> ImportRoute
    where
        B: Drain,
    {
        let hash = &header.hash();
        let number = header.number();
        let parent = header.parent_hash();
        let chain = client.chain.read();
        let mut is_finalized = false;

        // Commit results
        let block = block.drain();
        debug_assert_eq!(header.hash(), block_data.header_view().hash());

        let mut batch = DBTransaction::new();

        // t_nb 9.1 Gather all ancestry actions. (Used only by AuRa)
        let ancestry_actions = self
            .engine
            .ancestry_actions(&header, &mut chain.ancestry_with_metadata_iter(*parent));

        let receipts = block.receipts;
        let traces = block.traces.drain();
        let best_hash = chain.best_block_hash();

        let new = ExtendedHeader {
            header: header.clone(),
            is_finalized,
            parent_total_difficulty: chain
                .block_details(&parent)
                .expect("Parent block is in the database; qed")
                .total_difficulty,
        };

        let best = {
            let hash = best_hash;
            let header = chain
                .block_header_data(&hash)
                .expect("Best block is in the database; qed")
                .decode()
                .expect("Stored block header is valid RLP; qed");
            let details = chain
                .block_details(&hash)
                .expect("Best block is in the database; qed");

            ExtendedHeader {
                parent_total_difficulty: details.total_difficulty - *header.difficulty(),
                is_finalized: details.is_finalized,
                header: header,
            }
        };

        // t_nb 9.2 calcuate route between current and latest block.
        let route = chain.tree_route(best_hash, *parent).expect("forks are only kept when it has common ancestors; tree route from best to prospective's parent always exists; qed");

        // t_nb 9.3 Check block total difficulty
        let fork_choice = if route.is_from_route_finalized {
            ForkChoice::Old
        } else {
            self.engine.fork_choice(&new, &best)
        };

        // t_nb 9.4 CHECK! I *think* this is fine, even if the state_root is equal to another
        // already-imported block of the same number.
        // TODO: Prove it with a test.
        let mut state = block.state.drop().1;

        // t_nb 9.5 check epoch end signal, potentially generating a proof on the current
        // state. Write transition into db.
        if let Some(pending) = pending {
            chain.insert_pending_transition(&mut batch, header.hash(), pending);
        }

        // t_nb 9.6 push state to database Transaction. (It calls journal_under from JournalDB)
        state
            .journal_under(&mut batch, number, hash)
            .expect("DB commit failed");

        let finalized: Vec<_> = ancestry_actions
            .into_iter()
            .map(|ancestry_action| {
                let AncestryAction::MarkFinalized(a) = ancestry_action;

                if a != header.hash() {
                    // t_nb 9.7 if there are finalized ancester, mark that chainge in block in db. (Used by AuRa)
                    chain
                        .mark_finalized(&mut batch, a)
                        .expect("Engine's ancestry action must be known blocks; qed");
                } else {
                    // we're finalizing the current block
                    is_finalized = true;
                }

                a
            })
            .collect();

        // t_nb 9.8 insert block
        let route = chain.insert_block(
            &mut batch,
            block_data,
            receipts.clone(),
            ExtrasInsert {
                fork_choice: fork_choice,
                is_finalized,
            },
        );

        // t_nb 9.9 insert traces (if they are enabled)
        client.tracedb.read().import(
            &mut batch,
            ImportRequest {
                traces: traces.into(),
                block_hash: hash.clone(),
                block_number: number,
                enacted: route.enacted.clone(),
                retracted: route.retracted.len(),
            },
        );

        let is_canon = route.enacted.last().map_or(false, |h| h == hash);

        // t_nb 9.10 sync cache
        state.sync_cache(&route.enacted, &route.retracted, is_canon);
        // Final commit to the DB
        // t_nb 9.11 Write Transaction to database (cached)
        client.db.read().key_value().write_buffered(batch);
        // t_nb 9.12 commit changed to become current greatest by applying pending insertion updates (Sync point)
        chain.commit();

        // t_nb 9.13 check epoch end. Related only to AuRa and it seems light engine
        self.check_epoch_end(&header, &finalized, &chain, client);

        // t_nb 9.14 update last hashes. They are build in step 7.5
        client.update_last_hashes(&parent, hash);

        // t_nb 9.15 prune ancient states
        if let Err(e) = client.prune_ancient(state, &chain) {
            warn!("Failed to prune ancient state data: {}", e);
        }

        route
    }

    // check for epoch end signal and write pending transition if it occurs.
    // state for the given block must be available.
    pub fn check_epoch_end_signal(
        &self,
        header: &Header,
        block_bytes: &[u8],
        receipts: &[TypedReceipt],
        state_db: &StateDB,
        client: &Client,
    ) -> EthcoreResult<Option<PendingTransition>> {
        use engines::EpochChange;

        let hash = header.hash();
        let auxiliary = ::machine::AuxiliaryData {
            bytes: Some(block_bytes),
            receipts: Some(&receipts),
        };

        match self.engine.signals_epoch_end(header, auxiliary) {
            EpochChange::Yes(proof) => {
                use engines::Proof;

                let proof = match proof {
                    Proof::Known(proof) => proof,
                    Proof::WithState(with_state) => {
                        let env_info = EnvInfo {
                            number: header.number(),
                            author: header.author().clone(),
                            timestamp: header.timestamp(),
                            difficulty: header.difficulty().clone(),
                            last_hashes: client.build_last_hashes(header.parent_hash()),
                            gas_used: U256::default(),
                            gas_limit: u64::max_value().into(),
                        };

                        let call = move |addr, data| {
                            let mut state_db = state_db.boxed_clone();
                            let backend = ::state::backend::Proving::new(state_db.as_hash_db_mut());

                            let transaction = client.contract_call_tx(
                                BlockId::Hash(*header.parent_hash()),
                                addr,
                                data,
                            );

                            let mut state = State::from_existing(
                                backend,
                                header.state_root().clone(),
                                self.engine.account_start_nonce(header.number()),
                                client.factories.clone(),
                            )
                            .expect("state known to be available for just-imported block; qed");

                            let options = TransactOptions::with_no_tracing().dont_check_nonce();
                            let machine = self.engine.machine();
                            let schedule = machine.schedule(env_info.number);
                            let res = Executive::new(&mut state, &env_info, &machine, &schedule)
                                .transact(&transaction, options);

                            let res = match res {
                                Err(e) => {
                                    trace!(target: "client", "Proved call failed: {}", e);
                                    Err(e.to_string())
                                }
                                Ok(res) => Ok((res.output, state.drop().1.extract_proof())),
                            };

                            res.map(|(output, proof)| {
                                (output, proof.into_iter().map(|x| x.into_vec()).collect())
                            })
                        };

                        match with_state.generate_proof(&call) {
                            Ok(proof) => proof,
                            Err(e) => {
                                warn!(target: "client", "Failed to generate transition proof for block {}: {}", hash, e);
                                warn!(target: "client", "Snapshots produced by this client may be incomplete");
                                return Err(EngineError::FailedSystemCall(e).into());
                            }
                        }
                    }
                };

                debug!(target: "client", "Block {} signals epoch end.", hash);

                Ok(Some(PendingTransition { proof: proof }))
            }
            EpochChange::No => Ok(None),
            EpochChange::Unsure(_) => {
                warn!(target: "client", "Detected invalid engine implementation.");
                warn!(target: "client", "Engine claims to require more block data, but everything provided.");
                Err(EngineError::InvalidEngine.into())
            }
        }
    }

    // check for ending of epoch and write transition if it occurs.
    fn check_epoch_end<'a>(
        &self,
        header: &'a Header,
        finalized: &'a [H256],
        chain: &BlockChain,
        client: &Client,
    ) {
        let is_epoch_end = self.engine.is_epoch_end(
            header,
            finalized,
            &(|hash| client.block_header_decoded(BlockId::Hash(hash))),
            &(|hash| chain.get_pending_transition(hash)), // TODO: limit to current epoch.
        );

        if let Some(proof) = is_epoch_end {
            debug!(target: "client", "Epoch transition at block {}", header.hash());

            let mut batch = DBTransaction::new();
            chain.insert_epoch_transition(
                &mut batch,
                header.number(),
                Transition {
                    block_hash: header.hash(),
                    block_number: header.number(),
                    proof: proof,
                },
            );

            // always write the batch directly since epoch transition proofs are
            // fetched from a DB iterator and DB iterators are only available on
            // flushed data.
            client
                .db
                .read()
                .key_value()
                .write(batch)
                .expect("DB flush failed");
        }
    }
}
