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

//! A signer used by Engines which need to sign messages.

use crypto::publickey::{self, ecies, Error, Public, Signature};
use ethereum_types::{Address, H256};
//TODO dr

/// Everything that an Engine needs to sign messages.
pub trait EngineSigner: Send + Sync {
    /// Sign a consensus message hash.
    fn sign(&self, hash: H256) -> Result<Signature, publickey::Error>;

    /// Signing address
    fn address(&self) -> Address;

    /// Decrypt a message that was encrypted to this signer's key.
    fn decrypt(&self, auth_data: &[u8], cipher: &[u8]) -> Result<Vec<u8>, Error>;

    /// The signer's public key, if available.
    fn public(&self) -> Option<Public>;
}

/// Creates a new `EngineSigner` from given key pair.
pub fn from_keypair(keypair: publickey::KeyPair) -> Box<dyn EngineSigner> {
    Box::new(Signer(keypair))
}

struct Signer(publickey::KeyPair);

impl EngineSigner for Signer {
    fn sign(&self, hash: H256) -> Result<Signature, publickey::Error> {
        publickey::sign(self.0.secret(), &hash)
    }

    fn address(&self) -> Address {
        self.0.address()
    }

    fn decrypt(&self, auth_data: &[u8], cipher: &[u8]) -> Result<Vec<u8>, Error> {
        ecies::decrypt(self.0.secret(), auth_data, cipher).map_err(From::from)
    }

    fn public(&self) -> Option<Public> {
        Some(*self.0.public())
    }
}

#[cfg(test)]
mod test_signer {

    extern crate ethkey;

    use std::sync::Arc;

    use self::ethkey::Password;
    use accounts::{self, AccountProvider, SignError};

    use super::*;

    impl EngineSigner for (Arc<AccountProvider>, Address, Password) {
        fn sign(&self, hash: H256) -> Result<Signature, crypto::publickey::Error> {
            match self.0.sign(self.1, Some(self.2.clone()), hash) {
                Err(SignError::NotUnlocked) => unreachable!(),
                Err(SignError::NotFound) => Err(crypto::publickey::Error::InvalidAddress),
                Err(SignError::SStore(accounts::Error::EthCryptoPublicKey(err))) => Err(err),
                Err(SignError::SStore(accounts::Error::EthCrypto(err))) => {
                    warn!("Low level crypto error: {:?}", err);
                    Err(crypto::publickey::Error::InvalidSecretKey)
                }
                Err(SignError::SStore(err)) => {
                    warn!("Error signing for engine: {:?}", err);
                    Err(crypto::publickey::Error::InvalidSignature)
                }
                Ok(ok) => Ok(ok),
            }
        }

        fn address(&self) -> Address {
            self.1
        }

        fn decrypt(&self, auth_data: &[u8], cipher: &[u8]) -> Result<Vec<u8>, Error> {
            self.0
                .decrypt(self.1, None, auth_data, cipher)
                .map_err(|e| {
                    warn!("Unable to decrypt message: {:?}", e);
                    Error::InvalidMessage
                })
        }

        fn public(&self) -> Option<Public> {
            self.0.account_public(self.1, &self.2).ok()
        }
    }
}
