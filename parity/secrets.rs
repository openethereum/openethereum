use ethkey::KeyPair;

/// Secrets that can be used by the client.
#[derive(Clone, Debug)]
pub struct Secrets {
    /// Signing key which is used for sealing blocks.
    pub engine_signer: Option<KeyPair>,
}

impl Secrets {
    /// Extract secrets from the environment.
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            engine_signer: std::env::var("ENGINE_SIGNER")
                .ok()
                .map(
                    |key| -> Result<KeyPair, Box<dyn std::error::Error + Send + Sync>> {
                        Ok(KeyPair::from_secret(key.parse()?)?)
                    },
                )
                .transpose()?,
        })
    }
}
