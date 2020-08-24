// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

use std::sync::Arc;

use dir::Directories;
use ethereum_types::Address;
use ethkey::Password;

use params::{AccountsConfig, SpecType};

#[cfg(not(feature = "accounts"))]
mod accounts {
    use super::*;

    /// Dummy AccountProvider
    pub struct AccountProvider;

    impl ::ethcore::miner::LocalAccounts for AccountProvider {
        fn is_local(&self, _address: &Address) -> bool {
            false
        }
    }

    pub fn prepare_account_provider(
        _spec: &SpecType,
        _dirs: &Directories,
        _data_dir: &str,
        _cfg: AccountsConfig,
        _passwords: &[Password],
    ) -> Result<AccountProvider, String> {
        warn!("Note: Your instance of Parity Ethereum is running without account support. Some CLI options are ignored.");
        Ok(AccountProvider)
    }

    pub fn miner_local_accounts(_: Arc<AccountProvider>) -> AccountProvider {
        AccountProvider
    }

    pub fn miner_author(
        _spec: &SpecType,
        _dirs: &Directories,
        _account_provider: &Arc<AccountProvider>,
        _engine_signer: Address,
        _passwords: &[Password],
    ) -> Result<Option<::ethcore::miner::Author>, String> {
        Ok(None)
    }

    pub fn accounts_list(
        _account_provider: Arc<AccountProvider>,
    ) -> Arc<dyn Fn() -> Vec<Address> + Send + Sync> {
        Arc::new(|| vec![])
    }
}

#[cfg(feature = "accounts")]
mod accounts {
    use super::*;

    pub use accounts::AccountProvider;

    /// Initialize account provider
    pub fn prepare_account_provider(
        spec: &SpecType,
        dirs: &Directories,
        data_dir: &str,
        cfg: AccountsConfig,
    ) -> Result<AccountProvider, String> {
        use ethstore::{accounts_dir::RootDiskDirectory, EthStore};

        let path = dirs.keys_path(data_dir);
        let dir = Box::new(
            RootDiskDirectory::create(&path)
                .map_err(|e| format!("Could not open keys directory: {}", e))?,
        );

        let ethstore = EthStore::open_with_iterations(dir, cfg.iterations)
            .map_err(|e| format!("Could not open keys directory: {}", e))?;
        let account_provider = AccountProvider::new(Box::new(ethstore));

        // Add development account if running dev chain:
        if let SpecType::Dev = *spec {
            insert_dev_account(&account_provider);
        }

        Ok(account_provider)
    }

    pub struct LocalAccounts(Arc<AccountProvider>);
    impl ::ethcore::miner::LocalAccounts for LocalAccounts {
        fn is_local(&self, address: &Address) -> bool {
            self.0.has_account(*address)
        }
    }

    pub fn miner_local_accounts(account_provider: Arc<AccountProvider>) -> LocalAccounts {
        LocalAccounts(account_provider)
    }

    pub fn accounts_list(
        account_provider: Arc<AccountProvider>,
    ) -> Arc<dyn Fn() -> Vec<Address> + Send + Sync> {
        Arc::new(move || account_provider.accounts().unwrap_or_default())
    }

    fn insert_dev_account(account_provider: &AccountProvider) {
        let secret: ethkey::Secret =
            "4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7".into();
        let dev_account = ethkey::KeyPair::from_secret(secret.clone())
            .expect("Valid secret produces valid key;qed");
        if !account_provider.has_account(dev_account.address()) {
            match account_provider.insert_account(secret, &Password::from(String::new())) {
                Err(e) => warn!("Unable to add development account: {}", e),
                Ok(address) => {
                    let _ = account_provider
                        .set_account_name(address.clone(), "Development Account".into());
                    let _ = account_provider.set_account_meta(
                        address,
                        ::serde_json::to_string(
                            &(vec![
                                (
                                    "description",
                                    "Never use this account outside of development chain!",
                                ),
                                ("passwordHint", "Password is empty string"),
                            ]
                            .into_iter()
                            .collect::<::std::collections::HashMap<_, _>>()),
                        )
                        .expect("Serialization of hashmap does not fail."),
                    );
                }
            }
        }
    }
}

pub use self::accounts::{
    accounts_list, miner_local_accounts, prepare_account_provider, AccountProvider,
};
