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
#[derive(Debug)]
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
