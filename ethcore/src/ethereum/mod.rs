// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity Ethereum.

// Parity Ethereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Ethereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Ethereum.  If not, see <http://www.gnu.org/licenses/>.

//! Ethereum protocol module.
//!
//! Contains all Ethereum network specific stuff, such as denominations and
//! consensus specifications.

/// Export the denominations module.
pub mod denominations;
/// Export the ethash module.
pub mod ethash;

pub use self::{denominations::*, ethash::Ethash};

use super::spec::*;
use machine::EthereumMachine;

/// Load chain spec from `SpecParams` and JSON.
pub fn load<'a, T: Into<Option<SpecParams<'a>>>>(params: T, b: &[u8]) -> Spec {
    match params.into() {
        Some(params) => Spec::load(params, b),
        None => Spec::load(&::std::env::temp_dir(), b),
    }
    .expect("chain spec is invalid")
}

fn load_machine(b: &[u8]) -> EthereumMachine {
    Spec::load_machine(b).expect("chain spec is invalid")
}

/// Create a new Foundation mainnet chain spec.
pub fn new_foundation<'a, T: Into<SpecParams<'a>>>(params: T) -> Spec {
    load(
        params.into(),
        include_bytes!("../../res/ethereum/foundation.json"),
    )
}

/// Create a new POA Network mainnet chain spec.
pub fn new_poanet<'a, T: Into<SpecParams<'a>>>(params: T) -> Spec {
    load(
        params.into(),
        include_bytes!("../../res/ethereum/poacore.json"),
    )
}

/// Create a new xDai chain spec.
pub fn new_xdai<'a, T: Into<SpecParams<'a>>>(params: T) -> Spec {
    load(
        params.into(),
        include_bytes!("../../res/ethereum/xdai.json"),
    )
}

/// Create a new Volta mainnet chain spec.
pub fn new_volta<'a, T: Into<SpecParams<'a>>>(params: T) -> Spec {
    load(
        params.into(),
        include_bytes!("../../res/ethereum/volta.json"),
    )
}

/// Create a new EWC mainnet chain spec.
pub fn new_ewc<'a, T: Into<SpecParams<'a>>>(params: T) -> Spec {
    load(params.into(), include_bytes!("../../res/ethereum/ewc.json"))
}

/// Create a new Ellaism mainnet chain spec.
pub fn new_ellaism<'a, T: Into<SpecParams<'a>>>(params: T) -> Spec {
    load(
        params.into(),
        include_bytes!("../../res/ethereum/ellaism.json"),
    )
}
/// Create a new Ropsten testnet chain spec.
pub fn new_ropsten<'a, T: Into<SpecParams<'a>>>(params: T) -> Spec {
    load(
        params.into(),
        include_bytes!("../../res/ethereum/ropsten.json"),
    )
}

/// Create a new Rinkeby testnet chain spec.
pub fn new_rinkeby<'a, T: Into<SpecParams<'a>>>(params: T) -> Spec {
    load(
        params.into(),
        include_bytes!("../../res/ethereum/rinkeby.json"),
    )
}

/// Create a new Görli testnet chain spec.
pub fn new_goerli<'a, T: Into<SpecParams<'a>>>(params: T) -> Spec {
    load(
        params.into(),
        include_bytes!("../../res/ethereum/goerli.json"),
    )
}

/// Create a new POA Sokol testnet chain spec.
pub fn new_sokol<'a, T: Into<SpecParams<'a>>>(params: T) -> Spec {
    load(
        params.into(),
        include_bytes!("../../res/ethereum/poasokol.json"),
    )
}

// For tests

/// Create a new Foundation Frontier-era chain spec as though it never changes to Homestead.
pub fn new_frontier_test() -> Spec {
    load(
        None,
        include_bytes!("../../res/ethereum/frontier_test.json"),
    )
}

/// Create a new Ropsten chain spec.
pub fn new_ropsten_test() -> Spec {
    load(None, include_bytes!("../../res/ethereum/ropsten.json"))
}

/// Create a new Foundation Homestead-era chain spec as though it never changed from Frontier.
pub fn new_homestead_test() -> Spec {
    load(
        None,
        include_bytes!("../../res/ethereum/homestead_test.json"),
    )
}

/// Create a new Foundation Homestead-EIP150-era chain spec as though it never changed from Homestead/Frontier.
pub fn new_eip150_test() -> Spec {
    load(None, include_bytes!("../../res/ethereum/eip150_test.json"))
}

/// Create a new Foundation Homestead-EIP161-era chain spec as though it never changed from Homestead/Frontier.
pub fn new_eip161_test() -> Spec {
    load(None, include_bytes!("../../res/ethereum/eip161_test.json"))
}

/// Create a new Foundation Frontier/Homestead/DAO chain spec with transition points at #5 and #8.
pub fn new_transition_test() -> Spec {
    load(
        None,
        include_bytes!("../../res/ethereum/transition_test.json"),
    )
}

/// Create a new Foundation Mainnet chain spec without genesis accounts.
pub fn new_mainnet_like() -> Spec {
    load(
        None,
        include_bytes!("../../res/ethereum/frontier_like_test.json"),
    )
}

/// Create a new Foundation Byzantium era spec.
pub fn new_byzantium_test() -> Spec {
    load(
        None,
        include_bytes!("../../res/ethereum/byzantium_test.json"),
    )
}

/// Create a new Foundation Constantinople era spec.
pub fn new_constantinople_test() -> Spec {
    load(
        None,
        include_bytes!("../../res/ethereum/constantinople_test.json"),
    )
}

/// Create a new Foundation St. Peter's (Contantinople Fix) era spec.
pub fn new_constantinople_fix_test() -> Spec {
    load(
        None,
        include_bytes!("../../res/ethereum/st_peters_test.json"),
    )
}

/// Create a new Foundation Istanbul era spec.
pub fn new_istanbul_test() -> Spec {
    load(
        None,
        include_bytes!("../../res/ethereum/istanbul_test.json"),
    )
}

/// Create a new BizantiumToConstaninopleFixAt5 era spec.
pub fn new_byzantium_to_constantinoplefixat5_test() -> Spec {
    load(
        None,
        include_bytes!("../../res/ethereum/byzantium_to_constantinoplefixat5_test.json"),
    )
}

/// Create a new Foundation Berlin era spec.
pub fn new_berlin_test() -> Spec {
    load(None, include_bytes!("../../res/ethereum/berlin_test.json"))
}

// For tests

/// Create a new Foundation Frontier-era chain spec as though it never changes to Homestead.
pub fn new_frontier_test_machine() -> EthereumMachine {
    load_machine(include_bytes!("../../res/ethereum/frontier_test.json"))
}

/// Create a new Foundation Homestead-era chain spec as though it never changed from Frontier.
pub fn new_homestead_test_machine() -> EthereumMachine {
    load_machine(include_bytes!("../../res/ethereum/homestead_test.json"))
}

/// Create a new Foundation Byzantium era spec.
pub fn new_byzantium_test_machine() -> EthereumMachine {
    load_machine(include_bytes!("../../res/ethereum/byzantium_test.json"))
}

/// Create a new Foundation Constantinople era spec.
pub fn new_constantinople_test_machine() -> EthereumMachine {
    load_machine(include_bytes!(
        "../../res/ethereum/constantinople_test.json"
    ))
}

/// Create a new Foundation St. Peter's (Contantinople Fix) era spec.
pub fn new_constantinople_fix_test_machine() -> EthereumMachine {
    load_machine(include_bytes!("../../res/ethereum/st_peters_test.json"))
}

/// Create a new Foundation Istanbul era spec.
pub fn new_istanbul_test_machine() -> EthereumMachine {
    load_machine(include_bytes!("../../res/ethereum/istanbul_test.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use state::*;
    use test_helpers::get_temp_state_db;
    use types::{view, views::BlockView};

    #[test]
    fn ensure_db_good() {
        let spec = new_ropsten(&::std::env::temp_dir());
        let genesis_header = spec.genesis_header();
        let db = spec
            .ensure_db_good(get_temp_state_db(), &Default::default())
            .unwrap();
        let s = State::from_existing(db, genesis_header.state_root().clone(), Default::default())
            .unwrap();
        assert_eq!(
            s.balance(&"0000000000000000000000000000000000000001".into())
                .unwrap(),
            1u64.into()
        );
        assert_eq!(
            s.balance(&"0000000000000000000000000000000000000002".into())
                .unwrap(),
            1u64.into()
        );
        assert_eq!(
            s.balance(&"0000000000000000000000000000000000000003".into())
                .unwrap(),
            1u64.into()
        );
        assert_eq!(
            s.balance(&"0000000000000000000000000000000000000004".into())
                .unwrap(),
            1u64.into()
        );
        assert_eq!(
            s.balance(&"874b54a8bd152966d63f706bae1ffeb0411921e5".into())
                .unwrap(),
            "c9f2c9cd04674edea40000000".parse().unwrap()
        );
        assert_eq!(
            s.balance(&"0000000000000000000000000000000000000000".into())
                .unwrap(),
            1u64.into()
        );
    }

    #[test]
    fn ropsten() {
        let ropsten = new_ropsten(&::std::env::temp_dir());

        assert_eq!(
            ropsten.state_root(),
            "217b0bbcfb72e2d57e28f33cb361b9983513177755dc3f33ce3e7022ed62b77b"
                .parse()
                .unwrap()
        );
        let genesis = ropsten.genesis_block();
        assert_eq!(
            view!(BlockView, &genesis).header_view().hash(),
            "41941023680923e0fe4d74a34bdac8141f2540e3ae90623718e47d66d1ca4a2d"
                .parse()
                .unwrap()
        );

        let _ = ropsten.engine;
    }

    #[test]
    fn frontier() {
        let frontier = new_foundation(&::std::env::temp_dir());

        assert_eq!(
            frontier.state_root(),
            "d7f8974fb5ac78d9ac099b9ad5018bedc2ce0a72dad1827a1709da30580f0544".into()
        );
        let genesis = frontier.genesis_block();
        assert_eq!(
            view!(BlockView, &genesis).header_view().hash(),
            "d4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3".into()
        );

        let _ = frontier.engine;
    }
}
