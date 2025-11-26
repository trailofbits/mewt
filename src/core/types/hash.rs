use sha2::{Digest, Sha256};
use std::convert::TryFrom;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Hash([u8; 32]);

impl Hash {
    pub fn digest(input: String) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        let mut array = [0u8; 32];
        array.copy_from_slice(&result);
        Hash(array)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl TryFrom<String> for Hash {
    type Error = HashError;

    fn try_from(value: String) -> Result<Self, HashError> {
        let bytes = hex::decode(&value).map_err(|_| HashError::InvalidHex(value))?;
        if bytes.len() != 32 {
            return Err(HashError::InvalidLength(bytes.len().try_into().unwrap()));
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(Hash(array))
    }
}

#[derive(Debug, Error)]
pub enum HashError {
    #[error("Given string is not valid hexidecimal: {0}")]
    InvalidHex(String),
    #[error("Expected 32 bytes, got {0}")]
    InvalidLength(u32),
}
