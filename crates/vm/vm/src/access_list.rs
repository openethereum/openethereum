use ethereum_types::{Address, H256};
use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::{Hash, Hasher},
};

use std::{cell::RefCell, rc::Rc};

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

#[derive(Debug)]
struct Journal {
    enabled: bool,
    last_id: usize,
    addresses: HashMap<Address, usize>,
    storage_keys: HashMap<(Address, H256), usize>,
}
#[derive(Debug)]
pub struct AccessList {
    id: usize,
    journal: Rc<RefCell<Journal>>,
}

impl Clone for AccessList {
    fn clone(&self) -> Self {
        let mut journal = self.journal.as_ref().borrow_mut();
        let id = journal.last_id + 1;
        journal.last_id = id;
        Self {
            id: id,
            journal: self.journal.clone(),
        }
    }
}

impl Default for AccessList {
    fn default() -> Self {
        AccessList::new(false)
    }
}

impl std::fmt::Display for AccessList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let journal = self.journal.as_ref().borrow();
        for (addr, id) in journal.addresses.iter() {
            write!(f, "| ADDR {} -> {}\n", addr, id)?;
        }
        for ((addr, slot), id) in journal.storage_keys.iter() {
            write!(f, "| SLOT {}:{} -> {}\n", addr, slot, id)?;
        }
        Ok(())
    }
}

impl AccessList {
    /// Returns if the list is enabled
    pub fn new(enabled: bool) -> Self {
        let journal = Journal {
            enabled,
            last_id: 0,
            addresses: HashMap::new(),
            storage_keys: HashMap::new(),
        };
        Self {
            id: 0,
            journal: Rc::new(RefCell::new(journal)),
        }
    }

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
            journal
                .storage_keys
                .contains_key(&(address, key) as &dyn KeyPair<Address, H256>)
        } else {
            false
        }
    }

    /// Inserts a storage key
    pub fn insert_storage_key(&mut self, address: Address, key: H256) {
        let mut journal = self.journal.as_ref().borrow_mut();
        if journal.enabled
            && !journal
                .storage_keys
                .contains_key(&(address, key) as &dyn KeyPair<Address, H256>)
        {
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
        let mut journal = self.journal.as_ref().borrow_mut();
        if journal.enabled && !journal.addresses.contains_key(&address) {
            journal.addresses.insert(address, self.id);
        }
    }
    /// Removes all changes in journal
    pub fn rollback(&self) {
        let mut journal = self.journal.as_ref().borrow_mut();
        // `id < self.id` instead `id != self.if` is to take care about recursive calls
        journal.addresses.retain(|_, id| *id < self.id);
        journal.storage_keys.retain(|_, id| *id < self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_accesslist_is_disabled() {
        let access_list = AccessList::default();
        assert_eq!(false, access_list.is_enabled());
    }

    #[test]
    fn default_disabled_accesslist_does_nothing() {
        let mut access_list = AccessList::default();
        access_list.insert_address(Address::from_low_u64_be(1));
        access_list.insert_storage_key(Address::from_low_u64_be(2), H256::from_low_u64_be(3));
        assert_eq!(
            false,
            access_list.contains_address(&Address::from_low_u64_be(1))
        );
        assert_eq!(
            false,
            access_list
                .contains_storage_key(&Address::from_low_u64_be(2), &H256::from_low_u64_be(3))
        );
    }

    #[test]
    fn default_enabled_accesslist_registers() {
        let mut access_list = AccessList::default();
        access_list.enable();
        assert_eq!(true, access_list.is_enabled());
        access_list.insert_address(Address::from_low_u64_be(1));
        access_list.insert_storage_key(Address::from_low_u64_be(2), H256::from_low_u64_be(3));
        assert_eq!(
            true,
            access_list.contains_address(&Address::from_low_u64_be(1))
        );
        assert_eq!(
            true,
            access_list
                .contains_storage_key(&Address::from_low_u64_be(2), &H256::from_low_u64_be(3))
        );
    }

    #[test]
    fn cloned_accesslist_registers_in_parent() {
        let mut access_list = AccessList::default();
        access_list.enable();
        assert_eq!(true, access_list.is_enabled());
        access_list.insert_address(Address::from_low_u64_be(1));
        access_list.insert_storage_key(Address::from_low_u64_be(2), H256::from_low_u64_be(3));

        let access_list_call = access_list.clone();
        assert_eq!(
            true,
            access_list_call.contains_address(&Address::from_low_u64_be(1))
        );
        assert_eq!(
            true,
            access_list_call
                .contains_storage_key(&Address::from_low_u64_be(2), &H256::from_low_u64_be(3))
        );
        access_list.insert_address(Address::from_low_u64_be(4));
        assert_eq!(
            true,
            access_list_call.contains_address(&Address::from_low_u64_be(4))
        );

        assert_eq!(
            true,
            access_list.contains_address(&Address::from_low_u64_be(4))
        );
    }
    #[test]
    fn cloned_accesslist_rollbacks_in_parent() {
        let mut access_list = AccessList::default();
        access_list.enable();
        assert_eq!(true, access_list.is_enabled());
        access_list.insert_address(Address::from_low_u64_be(1));
        access_list.insert_storage_key(Address::from_low_u64_be(2), H256::from_low_u64_be(3));

        let mut access_list_call = access_list.clone();
        access_list_call.insert_address(Address::from_low_u64_be(1));
        access_list_call.insert_storage_key(Address::from_low_u64_be(2), H256::from_low_u64_be(3));
        access_list_call.insert_address(Address::from_low_u64_be(4));

        let mut access_list_call_call = access_list.clone();
        access_list_call_call.insert_address(Address::from_low_u64_be(1));
        access_list_call_call
            .insert_storage_key(Address::from_low_u64_be(2), H256::from_low_u64_be(3));
        access_list_call_call.insert_address(Address::from_low_u64_be(5));
        access_list_call_call
            .insert_storage_key(Address::from_low_u64_be(6), H256::from_low_u64_be(7));

        access_list_call.rollback();

        assert_eq!(
            true,
            access_list.contains_address(&Address::from_low_u64_be(1))
        );
        assert_eq!(
            false,
            access_list.contains_address(&Address::from_low_u64_be(4))
        );
        assert_eq!(
            false,
            access_list.contains_address(&Address::from_low_u64_be(5))
        );
        assert_eq!(
            true,
            access_list
                .contains_storage_key(&Address::from_low_u64_be(2), &H256::from_low_u64_be(3))
        );
        assert_eq!(
            false,
            access_list
                .contains_storage_key(&Address::from_low_u64_be(6), &H256::from_low_u64_be(7))
        );
    }
}
