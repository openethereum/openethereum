use ethereum_types::{Address, H256};
use std::{
    borrow::Borrow,
    collections::HashSet,
    hash::{Hash, Hasher},
};
/*
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
#[derive(Clone, Debug, Default)]
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
        // [adria0] eprintln!("insert_address({:?}) [{}]",address,self.addresses.contains(&address));
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
*/

use std::collections::{HashMap};
use std::sync::Arc;
use std::cell::RefCell;

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

const DEBUG : bool = false;

#[derive(Debug)]
struct Journal {
    enabled: bool,
    last_id :usize,
    addresses: HashMap<Address, usize>,
    storage_keys: HashMap<(Address, H256), usize>,
}
#[derive(Debug)]
pub struct AccessList {
    id : usize,
    journal : Arc<RefCell<Journal>>,
}

impl Clone for AccessList {
    fn clone(&self) -> Self {
        let mut journal = self.journal.as_ref().borrow_mut(); 
        let id = journal.last_id + 1;
        journal.last_id = id;
        Self {
            id : id,
            journal : self.journal.clone()
        }
    }
}

impl Default for AccessList {
    fn default() -> Self {
        let journal = Journal {
            enabled: false,
            last_id : 0,
            addresses : HashMap::new(),
            storage_keys: HashMap::new()
        };
        Self {
            id : 0,
            journal : Arc::new(RefCell::new(journal))
        }
    }
}

impl std::fmt::Display for AccessList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let journal = self.journal.as_ref().borrow();
        for (addr,id) in journal.addresses.iter() {
            write!(f, "| ADDR {} -> {}\n", addr, id)?;
        }
        for ((addr,slot),id) in journal.storage_keys.iter() {
            write!(f, "| SLOT {}:{} -> {}\n", addr, slot, id)?;
        }
        Ok(())
    }    
}

impl AccessList {
    /// Returns if the list is enabled
    pub fn is_enabled(&self) -> bool {
        let journal = self.journal.as_ref().borrow(); 
        journal.enabled
    }
    
    /// Enable the access list control
    pub fn enable(&mut self) {
        let mut journal = self.journal.as_ref().borrow_mut(); 
        journal.enabled = true;
    }
    /// Checks if contains an storage key
    pub fn contains_storage_key(&self, address: &Address, key: &H256) -> bool {
        let journal = self.journal.as_ref().borrow();
        if journal.enabled {
            journal.storage_keys
                .contains_key(&(address, key) as &dyn KeyPair<Address, H256>)
        } else {
            false
        }
    }
    /// Inserts a storage key
    pub fn insert_storage_key(&mut self, address: Address, key: H256) {
        if DEBUG { eprintln!("insert_storage_key({:?}) [{}]\n{}",address,self.journal.as_ref().borrow().storage_keys.contains_key(&(address, key) as &dyn KeyPair<Address, H256>),self); }
        let mut journal = self.journal.as_ref().borrow_mut();
        if journal.enabled && !journal.storage_keys.contains_key(&(address, key) as &dyn KeyPair<Address, H256>) {
            journal.storage_keys.insert((address, key), self.id);
        }
    }

    /// Checks if contains an address
    pub fn contains_address(&self, address: &Address) -> bool {
        let journal = self.journal.as_ref().borrow();
        if journal.enabled {
            journal.addresses.contains_key(&address)
        } else {
            false
        }
    }
    /// Inserts an address
    pub fn insert_address(&mut self, address: Address) {
        if DEBUG {  eprintln!("insert_address({:?},{})\n{}",address,self.id,self); }
        let mut journal = self.journal.as_ref().borrow_mut();
        if journal.enabled && !journal.addresses.contains_key(&address) {
            journal.addresses.insert(address, self.id);
        }
    }
    pub fn rollback(&self) {
        if DEBUG { eprintln!("ROLLBACK_BRGIN()\n{}",self); }
        {
            let mut journal = self.journal.as_ref().borrow_mut();
            journal.addresses.retain(|_,id| *id < self.id);
            journal.storage_keys.retain(|_,id| *id < self.id);
        }
        if DEBUG { eprintln!("ROLLBACK_END()\n{}",self); }
    }
    pub fn accrue(&mut self, _another: &AccessList) {
    }
}
