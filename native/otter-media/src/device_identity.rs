//! Bounded in-memory mainline device identities/protected format; no implicit pairing, file writes, or existing-identity replacement.
use crate::device_key_protection;
use base64::{engine::general_purpose::STANDARD, Engine};
use ring::{
    rand::{SecureRandom, SystemRandom},
    signature::{Ed25519KeyPair, KeyPair},
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

pub const MAX_DOCUMENT: usize = 64 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Document,
    Schema,
    Identity,
    Key,
    Protection,
    Random,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    device_id: Option<String>,
    private_key: String,
    public_key: String,
}

// No Debug/Clone/Serialize; expose only public keys and explicitly protected storage documents.
pub struct Identity {
    seed: Zeroizing<[u8; 32]>,
    device: Option<String>,
    legacy: bool,
}
impl Identity {
    pub fn generate() -> Result<Self, Error> {
        let mut seed = Zeroizing::new([0; 32]);
        SystemRandom::new()
            .fill(seed.as_mut())
            .map_err(|_| Error::Random)?;
        Ok(Self {
            seed,
            device: None,
            legacy: false,
        })
    }
    /// Callers own raw JSON buffers and cleanup of legacy plaintext input.
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_DOCUMENT {
            return Err(Error::Limit);
        }
        let mut stored: Stored = serde_json::from_slice(bytes).map_err(|_| Error::Document)?;
        let encoded = Zeroizing::new(std::mem::take(&mut stored.private_key));
        if stored
            .device_id
            .as_deref()
            .is_some_and(|id| !valid_field(id, 160))
        {
            return Err(Error::Identity);
        }
        let seed = match stored.schema_version {
            1 => {
                let mut seed = Zeroizing::new([0u8; 32]);
                if STANDARD
                    .decode_slice(encoded.trim(), seed.as_mut())
                    .map_err(|_| Error::Key)?
                    != 32
                {
                    return Err(Error::Key);
                }
                seed
            }
            2 => device_key_protection::unprotect(&encoded).map_err(|_| Error::Protection)?,
            _ => return Err(Error::Schema),
        };
        let mut public = [0u8; 32];
        if STANDARD
            .decode_slice(&stored.public_key, &mut public)
            .map_err(|_| Error::Key)?
            != 32
        {
            return Err(Error::Key);
        }
        Ed25519KeyPair::from_seed_and_public_key(seed.as_ref(), &public).map_err(|_| Error::Key)?;
        Ok(Self {
            seed,
            device: stored.device_id,
            legacy: stored.schema_version == 1,
        })
    }
    pub fn device_id(&self) -> Option<&str> {
        self.device.as_deref()
    }
    pub fn needs_migration(&self) -> bool {
        self.legacy
    }
    /// Bind only after outer-layer pairing-response confirmation; this operation alone is not server approval.
    pub fn bind(&mut self, device: &str) -> Result<(), Error> {
        if !valid_field(device, 160) {
            return Err(Error::Identity);
        }
        self.device = Some(device.to_owned());
        Ok(())
    }
    fn key(&self) -> Result<Ed25519KeyPair, Error> {
        Ed25519KeyPair::from_seed_unchecked(self.seed.as_ref()).map_err(|_| Error::Key)
    }
    pub fn public_key(&self) -> Result<String, Error> {
        Ok(STANDARD.encode(self.key()?.public_key().as_ref()))
    }
    pub fn protected_document(&self) -> Result<Vec<u8>, Error> {
        let stored = Stored {
            schema_version: 2,
            device_id: self.device.clone(),
            private_key: device_key_protection::protect(&self.seed)
                .map_err(|_| Error::Protection)?,
            public_key: self.public_key()?,
        };
        let bytes = serde_json::to_vec(&stored).map_err(|_| Error::Document)?;
        if bytes.len() > MAX_DOCUMENT {
            return Err(Error::Limit);
        }
        Ok(bytes)
    }
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Error> {
        Ok(self.key()?.sign(message).as_ref().to_vec())
    }
}
pub fn valid_field(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && value.bytes().all(|b| b.is_ascii_graphic())
}

#[cfg(test)]
#[path = "device_identity_tests.rs"]
mod tests;
