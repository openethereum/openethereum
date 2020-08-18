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

#![warn(missing_docs)]

//! Account management.

mod account_data;
mod error;
mod stores;

use self::stores::AddressBook;

use std::collections::HashMap;

use ethkey::{Address, Generator, Message, Password, Public, Random, Secret};
use ethstore::{accounts_dir::MemoryDirectory, EthStore, SecretStore, SecretVaultRef};
use log::*;
use parking_lot::RwLock;

pub use ethkey::Signature;
pub use ethstore::{Derivation, Error, IndexDerivation, KeyFile};

pub use self::{account_data::AccountMeta, error::SignError};

/// Account management.
pub struct AccountProvider {
    /// Address book.
    address_book: RwLock<AddressBook>,
    /// Accounts on disk
    sstore: Box<dyn SecretStore>,
}

impl AccountProvider {
    /// Creates new account provider.
    pub fn new(sstore: Box<dyn SecretStore>) -> Self {
        AccountProvider {
            address_book: RwLock::new(AddressBook::new(&sstore.local_path())),
            sstore: sstore,
        }
    }

    /// Creates not disk backed provider.
    pub fn transient_provider() -> Self {
        AccountProvider {
            address_book: RwLock::new(AddressBook::transient()),
            sstore: Box::new(
                EthStore::open(Box::new(MemoryDirectory::default()))
                    .expect("MemoryDirectory load always succeeds; qed"),
            ),
        }
    }

    /// Creates new random account.
    pub fn new_account(&self, password: &Password) -> Result<Address, Error> {
        self.new_account_and_public(password).map(|d| d.0)
    }

    /// Creates new random account and returns address and public key
    pub fn new_account_and_public(&self, password: &Password) -> Result<(Address, Public), Error> {
        let acc = Random
            .generate()
            .expect("secp context has generation capabilities; qed");
        let public = acc.public().clone();
        let secret = acc.secret().clone();
        let account = self
            .sstore
            .insert_account(SecretVaultRef::Root, secret, password)?;
        Ok((account.address, public))
    }

    /// Inserts new account into underlying store.
    pub fn insert_account(&self, secret: Secret, password: &Password) -> Result<Address, Error> {
        let account = self
            .sstore
            .insert_account(SecretVaultRef::Root, secret, password)?;
        Ok(account.address)
    }

    /// Generates new derived account based on the existing one
    /// New account will be created with the same password (if save: true)
    pub fn derive_account(
        &self,
        address: &Address,
        password: Password,
        derivation: Derivation,
        save: bool,
    ) -> Result<Address, SignError> {
        let account = self.sstore.account_ref(&address)?;
        Ok(if save {
            self.sstore
                .insert_derived(SecretVaultRef::Root, &account, &password, derivation)?
                .address
        } else {
            self.sstore
                .generate_derived(&account, &password, derivation)?
        })
    }

    /// Import a new wallet.
    pub fn import_wallet(
        &self,
        json: &[u8],
        password: &Password,
        gen_id: bool,
    ) -> Result<Address, Error> {
        let account = self
            .sstore
            .import_wallet(SecretVaultRef::Root, json, password, gen_id)?;
        Ok(Address::from(account.address).into())
    }

    /// Checks whether an account with a given address is present.
    pub fn has_account(&self, address: Address) -> bool {
        self.sstore.account_ref(&address).is_ok()
    }

    /// Returns addresses of all accounts.
    pub fn accounts(&self) -> Result<Vec<Address>, Error> {
        let accounts = self.sstore.accounts()?;
        Ok(accounts.into_iter().map(|a| a.address).collect())
    }

    /// Returns the address of default account.
    pub fn default_account(&self) -> Result<Address, Error> {
        Ok(self.accounts()?.first().cloned().unwrap_or_default())
    }

    /// Returns each address along with metadata.
    pub fn addresses_info(&self) -> HashMap<Address, AccountMeta> {
        self.address_book.read().get()
    }

    /// Returns each address along with metadata.
    pub fn set_address_name(&self, account: Address, name: String) {
        self.address_book.write().set_name(account, name)
    }

    /// Returns each address along with metadata.
    pub fn set_address_meta(&self, account: Address, meta: String) {
        self.address_book.write().set_meta(account, meta)
    }

    /// Removes and address from the address book
    pub fn remove_address(&self, addr: Address) {
        self.address_book.write().remove(addr)
    }

    /// Returns each account along with name and meta.
    pub fn accounts_info(&self) -> Result<HashMap<Address, AccountMeta>, Error> {
        let r = self
            .sstore
            .accounts()?
            .into_iter()
            .map(|a| {
                (
                    a.address.clone(),
                    self.account_meta(a.address).ok().unwrap_or_default(),
                )
            })
            .collect();
        Ok(r)
    }

    /// Returns each account along with name and meta.
    pub fn account_meta(&self, address: Address) -> Result<AccountMeta, Error> {
        let account = self.sstore.account_ref(&address)?;
        Ok(AccountMeta {
            name: self.sstore.name(&account)?,
            meta: self.sstore.meta(&account)?,
            uuid: self.sstore.uuid(&account).ok().map(Into::into), // allowed to not have a Uuid
        })
    }

    /// Returns account public key.
    pub fn account_public(&self, address: Address, password: &Password) -> Result<Public, Error> {
        self.sstore
            .public(&self.sstore.account_ref(&address)?, password)
    }

    /// Returns each account along with name and meta.
    pub fn set_account_name(&self, address: Address, name: String) -> Result<(), Error> {
        self.sstore
            .set_name(&self.sstore.account_ref(&address)?, name)?;
        Ok(())
    }

    /// Returns each account along with name and meta.
    pub fn set_account_meta(&self, address: Address, meta: String) -> Result<(), Error> {
        self.sstore
            .set_meta(&self.sstore.account_ref(&address)?, meta)?;
        Ok(())
    }

    /// Returns `true` if the password for `account` is `password`. `false` if not.
    pub fn test_password(&self, address: &Address, password: &Password) -> Result<bool, Error> {
        self.sstore
            .test_password(&self.sstore.account_ref(&address)?, password)
            .map_err(Into::into)
    }

    /// Permanently removes an account.
    pub fn kill_account(&self, address: &Address, password: &Password) -> Result<(), Error> {
        self.sstore
            .remove_account(&self.sstore.account_ref(&address)?, &password)?;
        Ok(())
    }

    /// Changes the password of `account` from `password` to `new_password`. Fails if incorrect `password` given.
    pub fn change_password(
        &self,
        address: &Address,
        password: Password,
        new_password: Password,
    ) -> Result<(), Error> {
        self.sstore
            .change_password(&self.sstore.account_ref(address)?, &password, &new_password)
    }

    /// Exports an account for given address.
    pub fn export_account(&self, address: &Address, password: Password) -> Result<KeyFile, Error> {
        self.sstore
            .export_account(&self.sstore.account_ref(address)?, &password)
    }

    /// Get account secret.
    pub fn get_secret(&self, address: Address, password: Password) -> Result<Secret, Error> {
        let account = self.sstore.account_ref(&address)?;
        let secret = self.sstore.raw_secret(&account, &password)?;
        Ok(secret)
    }

    /// Signs the message. If password is not provided the account must be unlocked.
    pub fn sign(
        &self,
        address: Address,
        password: Password,
        message: Message,
    ) -> Result<Signature, SignError> {
        let account = self.sstore.account_ref(&address)?;
        Ok(self.sstore.sign(&account, &password, &message)?)
    }

    /// Signs message using the derived secret. If password is not provided the account must be unlocked.
    pub fn sign_derived(
        &self,
        address: &Address,
        password: Password,
        derivation: Derivation,
        message: Message,
    ) -> Result<Signature, SignError> {
        let account = self.sstore.account_ref(address)?;
        Ok(self
            .sstore
            .sign_derived(&account, &password, derivation, &message)?)
    }

    /// Decrypts a message. If password is not provided the account must be unlocked.
    pub fn decrypt(
        &self,
        address: Address,
        password: Password,
        shared_mac: &[u8],
        message: &[u8],
    ) -> Result<Vec<u8>, SignError> {
        let account = self.sstore.account_ref(&address)?;
        Ok(self
            .sstore
            .decrypt(&account, &password, shared_mac, message)?)
    }

    /// Agree on shared key.
    pub fn agree(
        &self,
        address: Address,
        password: Password,
        other_public: &Public,
    ) -> Result<Secret, SignError> {
        let account = self.sstore.account_ref(&address)?;
        Ok(self.sstore.agree(&account, &password, other_public)?)
    }

    /// Create new vault.
    pub fn create_vault(&self, name: &str, password: &Password) -> Result<(), Error> {
        self.sstore.create_vault(name, password).map_err(Into::into)
    }

    /// Open existing vault.
    pub fn open_vault(&self, name: &str, password: &Password) -> Result<(), Error> {
        self.sstore.open_vault(name, password).map_err(Into::into)
    }

    /// Close previously opened vault.
    pub fn close_vault(&self, name: &str) -> Result<(), Error> {
        self.sstore.close_vault(name).map_err(Into::into)
    }

    /// List all vaults
    pub fn list_vaults(&self) -> Result<Vec<String>, Error> {
        self.sstore.list_vaults().map_err(Into::into)
    }

    /// List all currently opened vaults
    pub fn list_opened_vaults(&self) -> Result<Vec<String>, Error> {
        self.sstore.list_opened_vaults().map_err(Into::into)
    }

    /// Change vault password.
    pub fn change_vault_password(&self, name: &str, new_password: &Password) -> Result<(), Error> {
        self.sstore
            .change_vault_password(name, new_password)
            .map_err(Into::into)
    }

    /// Change vault of the given address.
    pub fn change_vault(&self, address: Address, new_vault: &str) -> Result<(), Error> {
        let new_vault_ref = if new_vault.is_empty() {
            SecretVaultRef::Root
        } else {
            SecretVaultRef::Vault(new_vault.to_owned())
        };
        let old_account_ref = self.sstore.account_ref(&address)?;
        self.sstore
            .change_account_vault(new_vault_ref, old_account_ref)
            .map_err(Into::into)
            .map(|_| ())
    }

    /// Get vault metadata string.
    pub fn get_vault_meta(&self, name: &str) -> Result<String, Error> {
        self.sstore.get_vault_meta(name).map_err(Into::into)
    }

    /// Set vault metadata string.
    pub fn set_vault_meta(&self, name: &str, meta: &str) -> Result<(), Error> {
        self.sstore.set_vault_meta(name, meta).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::AccountProvider;
    use ethereum_types::H256;
    use ethkey::{Generator, Random};
    use ethstore::Derivation;

    #[test]
    fn derived_account_nosave() {
        let kp = Random.generate().unwrap();
        let ap = AccountProvider::transient_provider();
        assert!(ap
            .insert_account(kp.secret().clone(), &"base".into())
            .is_ok());

        ap.derive_account(
            &kp.address(),
            "base".into(),
            Derivation::SoftHash(H256::from(999)),
            false,
        )
        .expect("Derivation should not fail");
    }

    #[test]
    fn derived_account_save() {
        let kp = Random.generate().unwrap();
        let ap = AccountProvider::transient_provider();
        assert!(ap
            .insert_account(kp.secret().clone(), &"base".into())
            .is_ok());

        ap.derive_account(
            &kp.address(),
            "base".into(),
            Derivation::SoftHash(H256::from(999)),
            true,
        )
        .expect("Derivation should not fail");
    }

    #[test]
    fn derived_account_sign() {
        let kp = Random.generate().unwrap();
        let ap = AccountProvider::transient_provider();
        assert!(ap
            .insert_account(kp.secret().clone(), &"base".into())
            .is_ok());

        let derived_addr = ap
            .derive_account(
                &kp.address(),
                "base".into(),
                Derivation::SoftHash(H256::from(1999)),
                true,
            )
            .expect("Derivation should not fail");

        let msg = Default::default();
        let signed_msg1 = ap
            .sign(derived_addr, "base".into(), msg)
            .expect("Signing with existing unlocked account should not fail");
        let signed_msg2 = ap
            .sign_derived(
                &kp.address(),
                "base".into(),
                Derivation::SoftHash(H256::from(1999)),
                msg,
            )
            .expect("Derived signing with existing unlocked account should not fail");

        assert_eq!(signed_msg1, signed_msg2, "Signed messages should match");
    }
}
