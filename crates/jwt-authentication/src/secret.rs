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

use log::info;
use ring::rand::SecureRandom;
use std::{
    convert::{AsRef, From},
    fmt::Formatter,
    fs,
    str::FromStr,
};

/// An error returned when parsing a `Secret` using `from_str` fails.
#[derive(Debug)]
pub struct ParseSecretError(hex::FromHexError);

impl std::fmt::Display for ParseSecretError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "provided string was not valid hex-encoded 256 bit secret key: {}",
            self.0
        )
    }
}

impl std::error::Error for ParseSecretError {}

/// Wrapper for 256 bit secret key used in JWT authentication.
#[derive(Debug, PartialEq, Eq)]
pub struct Secret([u8; 32]);

impl From<[u8; 32]> for Secret {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl AsRef<[u8; 32]> for Secret {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl FromStr for Secret {
    type Err = ParseSecretError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut secret = [0u8; 32];
        let _ = hex::decode_to_slice(s.trim().strip_prefix("0x").unwrap_or(&s), &mut secret)
            .map_err(|err| ParseSecretError(err))?;
        Ok(Self(secret))
    }
}

impl Secret {
    /// Try to read a secret from specified file. If specified file does not exist,
    /// generate a new jwt secret and write it into the the file.
    pub fn new(file_path: String, random: &dyn SecureRandom) -> anyhow::Result<Self> {
        if let Ok(data) = fs::read_to_string(&file_path) {
            info!("Reading jwt secret from {}", file_path);
            Ok(Secret::from_str(&data)?)
        } else {
            // generate new secret and write it into the file
            info!("Generating jwt secret");
            let mut secret = [0u8; 32];
            let _ = random.fill(&mut secret)?;
            fs::write(&file_path, format!("0x{}\n", hex::encode(secret)))?;
            info!("Secret have been written to {}", file_path);
            Ok(Secret(secret))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Secret;
    use ring::rand::SystemRandom;
    use std::fs;
    use tempfile::tempdir;
    use uuid::Uuid;

    const RANDOM: SystemRandom = SystemRandom;

    #[test]
    fn should_read_secret_from_file() {
        // given
        let dir = tempdir().unwrap();
        let file_name = format!("{}", Uuid::new_v4());
        let file_path = dir.path().join(file_name).to_str().unwrap().to_string();
        let secret = [1u8; 32];
        let _ = fs::write(&file_path, format!("0x{}", hex::encode(secret.clone()))).unwrap();

        // when
        let result = Secret::new(file_path, &RANDOM);

        // then
        match result {
            Ok(s) => {
                let expected = Secret(secret);
                assert_eq!(expected, s, "Incorrect secret");
            }
            Err(_) => panic!("Secret initialization should not fail"),
        }
    }

    #[test]
    fn should_fail_to_read_invalid_secret_from_file() {
        // given
        let dir = tempdir().unwrap();
        let file_name = format!("{}", Uuid::new_v4());
        let file_path = dir.path().join(file_name).to_str().unwrap().to_string();
        let invalid_secret = [1u8; 5];
        let _ = fs::write(&file_path, format!("0x{}", hex::encode(invalid_secret))).unwrap();

        // when
        let result = Secret::new(file_path, &RANDOM);

        // then
        match result {
            Ok(_) => panic!("Secret initialization succeeded while should fail"),
            Err(_) => {}
        }
    }

    #[test]
    fn should_generate_secret_when_file_does_not_exist() {
        // given
        let dir = tempdir().unwrap();
        let file_name = format!("{}", Uuid::new_v4());
        let file_path = dir.path().join(file_name).to_str().unwrap().to_string();

        // when
        let result = Secret::new(file_path.clone(), &RANDOM);

        // then
        match result {
            Ok(s) => {
                let stored = Secret::new(file_path, &RANDOM)
                    .expect("Reading a generated secret should not fail");
                assert_eq!(s, stored, "Returned and stored secrets are different")
            }
            Err(_) => {
                panic!("Secret initialization should not fail")
            }
        }
    }

    #[test]
    fn should_fail_to_generate_secret_when_directory_does_not_exist() {
        // given
        let dir = tempdir().unwrap();
        let file_name = format!("{}", Uuid::new_v4());
        let invalid_file_path = dir
            .path()
            .join("invalid")
            .join(file_name)
            .to_str()
            .unwrap()
            .to_string();

        // when
        let result = Secret::new(invalid_file_path.clone(), &RANDOM);

        // then
        match result {
            Ok(_) => panic!("Secret initialization succeeded while should fail"),
            Err(_) => {}
        }
    }
}
