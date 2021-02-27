// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use ethcore::trace::{self, FlatTrace, RewardType, Tracer};
use ethereum_types::{Address, H256, U256};
use evm::ActionParams;
use std::{collections::HashMap, io};

#[derive(Default, Copy, Clone)]
pub struct Config {
    omit_storage_output: bool,
    omit_memory_output: bool,
}

pub struct Informant {
    code: Vec<u8>,
    instruction: u8,
    depth: usize,
    stack: Vec<U256>,
    storage: HashMap<H256, H256>,
    subinfos: Vec<Informant>,
    subdepth: usize,
    trace_sink: std::io::Stderr,
    out_sink: std::io::Stdout,
    config: Config,
}

impl Config {
    pub fn new(omit_storage_output: bool, omit_memory_output: bool) -> Config {
        Config {
            omit_storage_output,
            omit_memory_output,
        }
    }

    pub fn omit_storage_output(&self) -> bool {
        self.omit_storage_output
    }

    pub fn omit_memory_output(&self) -> bool {
        self.omit_memory_output
    }
}

impl Tracer for Informant {
    type Output = FlatTrace;

    fn prepare_trace_call(&mut self, _: &ActionParams, _: usize, _: bool) {}
    fn prepare_trace_create(&mut self, _: &ActionParams) {}
    fn done_trace_call(&mut self, _: U256, _: &[u8]) {}
    fn done_trace_create(&mut self, _: U256, _: &[u8], _: Address) {}
    fn done_trace_failed(&mut self, _: &vm::Error) {}
    fn trace_suicide(&mut self, _: Address, _: U256, _: Address) {}
    fn trace_reward(&mut self, _: Address, _: U256, _: RewardType) {}
    fn drain(self) -> Vec<FlatTrace> {
        vec![]
    }
}

pub struct NoopTracer;
impl trace::VMTracer for NoopTracer {
    type Output = ();

    fn prepare_subtrace(&mut self, _code: &[u8]) {
        Default::default()
    }
    fn done_subtrace(&mut self) {}
    fn drain(self) -> Option<()> {
        None
    }

    fn trace_next_instruction(&mut self, _pc: usize, _instruction: u8, _current_gas: U256) -> bool {
        true
    }

    fn trace_prepare_execute(
        &mut self,
        _pc: usize,
        _instruction: u8,
        _gas_cost: U256,
        _mem_written: Option<(usize, usize)>,
        _store_written: Option<(U256, U256)>,
    ) {
        println!(
            "vm trace: pc: {}, gas: {}, mem: ({:?}), store: ({:?})",
            _pc, _gas_cost, _mem_written, _store_written
        )
    }

    fn trace_failed(&mut self) {}

    fn trace_executed(&mut self, _gas_used: U256, _stack_push: &[U256], _mem: &[u8]) {}
}

impl Default for Informant {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

pub trait Writer: io::Write + Send + Sized {
    fn clone(&self) -> Self;
    fn default() -> Self;
}

impl Writer for io::Stdout {
    fn clone(&self) -> Self {
        io::stdout()
    }

    fn default() -> Self {
        io::stdout()
    }
}

impl Writer for io::Stderr {
    fn clone(&self) -> Self {
        io::stderr()
    }

    fn default() -> Self {
        io::stderr()
    }
}

impl Informant {
    pub fn new_default(config: Config) -> Self {
        let mut informant = Self::default();
        informant.config = config;
        informant
    }
}

impl Informant {
    pub fn new(config: Config) -> Self {
        Informant {
            code: Default::default(),
            instruction: Default::default(),
            depth: Default::default(),
            stack: Default::default(),
            storage: Default::default(),
            subinfos: Default::default(),
            subdepth: 0,
            trace_sink: std::io::stderr(),
            out_sink: std::io::stdout(),
            config,
        }
    }
}
