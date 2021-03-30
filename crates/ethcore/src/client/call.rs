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

use crate::{
    client::{Call, Client},
    executed::CallError,
    state::State,
};
use error::ExecutionError;
use ethereum_types::U256;
use evm::EnvInfo;
use executive::{Executed, Executive, TransactOptions};
use transaction_ext::Transaction;
use types::{call_analytics::CallAnalytics, header::Header, transaction::SignedTransaction};

impl Call for Client {
    type State = State<::state_db::StateDB>;

    fn call(
        &self,
        transaction: &SignedTransaction,
        analytics: CallAnalytics,
        state: &mut Self::State,
        header: &Header,
    ) -> Result<Executed, CallError> {
        let env_info = EnvInfo {
            number: header.number(),
            author: header.author().clone(),
            timestamp: header.timestamp(),
            difficulty: header.difficulty().clone(),
            last_hashes: self.build_last_hashes(header.parent_hash()),
            gas_used: U256::default(),
            gas_limit: U256::max_value(),
        };
        let engine = self.engine();
        let machine = engine.machine();

        Self::do_virtual_call(&machine, &env_info, state, transaction, analytics)
    }

    fn call_many(
        &self,
        transactions: &[(SignedTransaction, CallAnalytics)],
        state: &mut Self::State,
        header: &Header,
    ) -> Result<Vec<Executed>, CallError> {
        let mut env_info = EnvInfo {
            number: header.number(),
            author: header.author().clone(),
            timestamp: header.timestamp(),
            difficulty: header.difficulty().clone(),
            last_hashes: self.build_last_hashes(header.parent_hash()),
            gas_used: U256::default(),
            gas_limit: U256::max_value(),
        };

        let mut results = Vec::with_capacity(transactions.len());
        let engine = self.engine();
        let machine = engine.machine();

        for &(ref t, analytics) in transactions {
            let ret = Self::do_virtual_call(machine, &env_info, state, t, analytics)?;
            env_info.gas_used = ret.cumulative_gas_used;
            results.push(ret);
        }

        Ok(results)
    }

    fn estimate_gas(
        &self,
        t: &SignedTransaction,
        state: &Self::State,
        header: &Header,
    ) -> Result<U256, CallError> {
        let (mut upper, max_upper, env_info) = {
            let init = *header.gas_limit();
            let max = init * U256::from(10);

            let env_info = EnvInfo {
                number: header.number(),
                author: header.author().clone(),
                timestamp: header.timestamp(),
                difficulty: header.difficulty().clone(),
                last_hashes: self.build_last_hashes(header.parent_hash()),
                gas_used: U256::default(),
                gas_limit: max,
            };

            (init, max, env_info)
        };

        let sender = t.sender();
        let options = || TransactOptions::with_tracing().dont_check_nonce();

        let exec = |gas| {
            let mut tx = t.as_unsigned().clone();
            tx.tx_mut().gas = gas;
            let tx = tx.fake_sign(sender);

            let mut clone = state.clone();
            let engine = self.engine();
            let machine = engine.machine();
            let schedule = machine.schedule(env_info.number);
            Executive::new(&mut clone, &env_info, &machine, &schedule)
                .transact_virtual(&tx, options())
        };

        let cond = |gas| exec(gas).ok().map_or(false, |r| r.exception.is_none());

        if !cond(upper) {
            upper = max_upper;
            match exec(upper) {
                Ok(v) => {
                    if let Some(exception) = v.exception {
                        return Err(CallError::Exceptional(exception));
                    }
                }
                Err(_e) => {
                    trace!(target: "estimate_gas", "estimate_gas failed with {}", upper);
                    let err = ExecutionError::Internal(format!(
                        "Requires higher than upper limit of {}",
                        upper
                    ));
                    return Err(err.into());
                }
            }
        }
        let lower = t
            .tx()
            .gas_required(&self.engine().schedule(env_info.number))
            .into();
        if cond(lower) {
            trace!(target: "estimate_gas", "estimate_gas succeeded with {}", lower);
            return Ok(lower);
        }

        /// Find transition point between `lower` and `upper` where `cond` changes from `false` to `true`.
        /// Returns the lowest value between `lower` and `upper` for which `cond` returns true.
        /// We assert: `cond(lower) = false`, `cond(upper) = true`
        fn binary_chop<F, E>(mut lower: U256, mut upper: U256, mut cond: F) -> Result<U256, E>
        where
            F: FnMut(U256) -> bool,
        {
            while upper - lower > 1.into() {
                let mid = (lower + upper) / 2;
                trace!(target: "estimate_gas", "{} .. {} .. {}", lower, mid, upper);
                let c = cond(mid);
                match c {
                    true => upper = mid,
                    false => lower = mid,
                };
                trace!(target: "estimate_gas", "{} => {} .. {}", c, lower, upper);
            }
            Ok(upper)
        }

        // binary chop to non-excepting call with gas somewhere between 21000 and block gas limit
        trace!(target: "estimate_gas", "estimate_gas chopping {} .. {}", lower, upper);
        binary_chop(lower, upper, cond)
    }
}
