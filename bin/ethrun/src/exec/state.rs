// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use std::error::Error;

use ethereum_types::{Address, H256, U256};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Specifies whether the new key-value pair creates
/// a new key in the map or replaces a value on an
/// existing key. This distinction matters when calculating
/// gas usage, as new keys incur 20000 gas and updates 5000.
pub enum StoreResult {
    /// Nothing changed in the storage.
    /// The key-value pair already exists in the
    /// storage tree and their value are identical
    /// to the proposed new values.
    ///
    /// When this is returned from Storage::store()
    /// then the hash of that store remains unchanged.
    NoOp,

    /// New key was created in the storage tree and
    /// a value was assigned to it.
    CreatedNew,

    /// No new keys were created in the storage, but
    /// an existing one changed its value.
    ReplacedExisting,
}

pub trait Storage {
    /// The merkle tree Keccak256 hash of a state tree.
    /// This filed correspond to the "storate_root"
    /// value in the raw blockchain representation.
    ///
    /// This value might change only as a result of calling
    /// self.store(..).
    fn hash(&self) -> H256;

    /// This function reads a value stored under the given key
    /// within contract's private storage.
    fn read(&self, key: U256) -> Option<U256>;

    /// Creates or updates a key-value pair within contract's
    /// private storage.
    fn store(&self, key: U256, value: U256) -> StoreResult;
}

/// Represents the raw bytecode of a contract.
///
/// This bytecode gets executed by the EVM when called
/// through transactions. Once a contract is created, its
/// code never changes, so the values here are immutable.
pub trait Code<'s> {
    /// The Keccak256 hash of the bytecode of the contract.
    fn hash(&self) -> H256;

    /// The EVM code that the contract executes.
    ///
    /// Note: in some testnets there are additional VM engines
    /// that run different types of code (such as WASM), but
    /// they are in the process of being depricated and moving
    /// forward only EVM will be supported.
    fn bytecode(&self) -> Option<&'s [u8]>;
}

/// An account controlled by a private key with non-zero balance or nonce.
///
/// Note: at the byte-level all account types have the same structure
/// and fields, however EO accounts always have zeroed storage_root
/// and code_hash values, thus they are not represented in this
/// logical account representation.
pub trait ExternallyOwned<'s> {
    /// A global counter of transactions sent by this account.
    fn nonce(&self) -> U256;

    /// The amount of Wei owned by this account.
    fn balance(&self) -> U256;
}

/// Represents a contract account that has its own storage and code.
pub trait Contract<'s> {
    /// The number of contracts created by this contract account.
    fn nonce(&self) -> U256;

    /// The amount of Wei owned by this contract.
    fn balance(&self) -> U256;

    /// The contract private storage tree.
    /// See [Storage] trait for more details.
    fn storage(&self) -> Option<&'s dyn Storage>;

    /// The code of this private contract.
    /// See [Code] trait for more details.
    fn code(&self) -> Option<&'s dyn Code>;
}

/// Represents an account on the blockchain.
pub enum Account<'s> {
    /// A smart contract account that has its own storage
    /// tree and code. It can be invoked by other contracts
    // or externally owned accounts.
    Contract(&'s dyn Contract<'s>),

    /// A private-key controlled account that has no storage
    /// or code. It only has nonce and balance associated with
    /// it.
    ExternallyOwned(&'s dyn ExternallyOwned<'s>),
}

/// Represents the entire world state for an Ethereum Blockchain,
/// it holds all known accounts in the system.
///
/// Implementations of this trait follow the ethereum-style
/// merkle patricia tree behaviour.
pub trait WorldState<'s> {
    /// The storage root hash of the world state database.
    ///
    /// This hash descends only to the account level and doesn't
    /// go into individual account storage trees.
    ///
    /// The individual account substorage integrity is guarded by
    /// the storage_root hash for each account.
    fn hash(&self) -> H256;

    /// Gets a reference to an account stored at a given address.
    /// For accoutns that have no balance or nonce this will return None.
    fn access(&self, address: &Address) -> Result<Option<&'s Account<'s>>>;
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    
    use super::*;
    use ethereum_types::{Address, H256, U256};
    use common_types::transaction::{
        Action, Transaction, 
        TypedTransaction
    };

    struct TestWorldState;
    struct TestEOAccount;
    struct TestCode;
    struct TestStorage;
    struct TestContractAccount;
    
    impl<'s> WorldState<'s> for TestWorldState {
        fn hash(&self) -> H256 { todo!() }
        fn access(&self, address: &Address) -> Result<Option<&'s Account<'s>>> { todo!() }
    }

    impl<'s> ExternallyOwned<'s> for TestEOAccount {
        fn nonce(&self) -> U256 { todo!() }
        fn balance(&self) -> U256 { todo!() }
    }

    impl<'s> Contract<'s> for TestContractAccount {
        fn nonce(&self) -> U256 { todo!() }
        fn balance(&self) -> U256 { todo!() }
        fn storage(&self) -> Option<&'s dyn Storage> { todo!() }
        fn code(&self) -> Option<&'s dyn Code> { todo!() }
    }

    impl<'s> Code<'s> for TestCode {
        fn hash(&self) -> H256 { todo!() }
        fn bytecode(&self) -> Option<&'s [u8]> { todo!() }
    }

    impl<'s> Storage for TestStorage {
        fn hash(&self) -> H256 { todo!() }
        fn read(&self, key: U256) -> Option<U256> { todo!() }
        fn store(&self, key: U256, value: U256) -> StoreResult { todo!() }
    }

    #[test]
    fn test_ea_transaction_code_interaction() -> Result<()> {
        let state = TestWorldState{};
        let tx = TypedTransaction::Legacy(Transaction {
            action: Action::Create,
            nonce: U256::from(42),
            gas_price: U256::from(3000),
            gas: U256::from(50_000),
            value: U256::from(1),
            data: b"Hello!".to_vec(),
        }).fake_sign(Address::from(0x69));
        Ok(())
    }
}
