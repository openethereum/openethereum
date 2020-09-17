use ethereum_types::{Address, H256};
use std::{
    borrow::Borrow,
    collections::HashSet,
    hash::{Hash, Hasher},
};

// Implementation of a hasheable borrowed pair
trait KeyPair<A, B> {
    fn a(&self) -> &A;
    fn b(&self) -> &B;
}

impl<'a, A, B> Borrow<dyn KeyPair<A, B> + 'a> for (A, B)
where
    A: Eq + Hash + 'a,
    B: Eq + Hash + 'a,
{
    fn borrow(&self) -> &(dyn KeyPair<A, B> + 'a) {
        self
    }
}

impl<A: Hash, B: Hash> Hash for (dyn KeyPair<A, B> + '_) {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.a().hash(state);
        self.b().hash(state);
    }
}

impl<A: Eq, B: Eq> PartialEq for (dyn KeyPair<A, B> + '_) {
    fn eq(&self, other: &Self) -> bool {
        self.a() == other.a() && self.b() == other.b()
    }
}

impl<A: Eq, B: Eq> Eq for (dyn KeyPair<A, B> + '_) {}

impl<A, B> KeyPair<A, B> for (A, B) {
    fn a(&self) -> &A {
        &self.0
    }
    fn b(&self) -> &B {
        &self.1
    }
}
impl<A, B> KeyPair<A, B> for (&A, &B) {
    fn a(&self) -> &A {
        self.0
    }
    fn b(&self) -> &B {
        self.1
    }
}

/// List of accessed accounts and storage keys
#[derive(Debug, Default)]
pub struct AccessList {
    enabled: bool,
    addresses: HashSet<Address>,
    storage_keys: HashSet<(Address, H256)>,
}

impl AccessList {
    /// Returns if the list is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    /// Checks if contains an storage key
    pub fn contains_storage_key(&self, address: &Address, key: &H256) -> bool {
        if self.enabled {
            self.storage_keys
                .contains(&(address, key) as &dyn KeyPair<Address, H256>)
        } else {
            false
        }
    }
    /// Inserts a storage key
    pub fn insert_storage_key(&mut self, address: Address, key: H256) {
        if self.enabled {
            self.storage_keys.insert((address, key));
        }
    }
    /// Checks if contains an address
    pub fn contains_address(&self, address: &Address) -> bool {
        if self.enabled {
            self.addresses.contains(&address)
        } else {
            false
        }
    }
    /// Inserts an address
    pub fn insert_address(&mut self, address: Address) {
        if self.enabled {
            self.addresses.insert(address);
        }
    }
    /// Merge secondary substate access list into self, accruing each element correspondingly.
    pub fn accrue(&mut self, access_list: &AccessList) {
        if self.enabled {
            self.addresses.extend(access_list.addresses.iter());
            self.storage_keys.extend(access_list.storage_keys.iter());
        }
    }
}
