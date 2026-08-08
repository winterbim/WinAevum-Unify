//! Aevum Unify — identity authority (M2 + M3).
//!
//! Provides `KeyMaterial` (Ed25519 secret/public pair) and `Identity` (SPIFFE-compatible
//! local authority). Blueprint §16.2: SPIFFE/SPIRE is the production-grade option; this
//! crate is the **local-first** authority with the same canonical identity model so that
//! Team/Enterprise can swap implementations behind the same contract.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use std::fmt;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey, SECRET_KEY_LENGTH};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroize;

/// Errors that this crate can produce.
#[derive(Debug, Error)]
pub enum KeyError {
    /// Signature did not verify against the supplied message.
    #[error("signature does not verify against the message")]
    SignatureInvalid,
    /// Hex-encoded secret had the wrong length.
    #[error("invalid secret hex length: expected 64 hex chars, got {0}")]
    InvalidHexLength(usize),
    /// Hex decode failure.
    #[error("invalid hex: {0}")]
    InvalidHex(#[from] hex::FromHexError),
}

/// Ed25519 key pair used to sign and verify Action Attestations.
pub struct KeyMaterial {
    inner: SigningKey,
    secret: [u8; SECRET_KEY_LENGTH],
}

impl fmt::Debug for KeyMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KeyMaterial {{ public: {:?} }}",
            self.inner.verifying_key().to_bytes()
        )
    }
}

impl Clone for KeyMaterial {
    fn clone(&self) -> Self {
        // Re-wrap the existing secret bytes into a new SigningKey. Avoids
        // exposing raw bytes to the caller while still allowing clones for
        // tests and short-lived copies.
        Self {
            inner: SigningKey::from_bytes(&self.secret),
            secret: self.secret,
        }
    }
}

impl Drop for KeyMaterial {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

impl KeyMaterial {
    pub fn generate() -> Self {
        let mut rng = OsRng;
        let mut bytes = [0u8; SECRET_KEY_LENGTH];
        rng.fill_bytes(&mut bytes);
        let inner = SigningKey::from_bytes(&bytes);
        Self {
            inner,
            secret: bytes,
        }
    }

    pub fn from_secret_hex(hex_str: &str) -> Result<Self, KeyError> {
        let n = hex_str.len();
        if n != SECRET_KEY_LENGTH * 2 {
            return Err(KeyError::InvalidHexLength(n));
        }
        let bytes: [u8; SECRET_KEY_LENGTH] = hex::decode(hex_str.trim())?
            .try_into()
            .map_err(|v: Vec<u8>| KeyError::InvalidHexLength(v.len()))?;
        let inner = SigningKey::from_bytes(&bytes);
        Ok(Self {
            inner,
            secret: bytes,
        })
    }

    pub fn secret_hex(&self) -> String {
        hex::encode(self.inner.to_bytes())
    }

    pub fn secret_bytes(&self) -> [u8; SECRET_KEY_LENGTH] {
        self.inner.to_bytes()
    }

    pub fn public_bytes(&self) -> [u8; 32] {
        self.inner.verifying_key().to_bytes()
    }

    pub fn sign(&self, message: &[u8]) -> String {
        let sig: Signature = self.inner.sign(message);
        hex::encode(sig.to_bytes())
    }

    pub fn verify(&self, signature_hex: &str, message: &[u8]) -> Result<(), KeyError> {
        verify_signature_hex(
            &hex::encode(self.public_bytes()),
            signature_hex,
            message,
        )
    }
}

/// Verify an Ed25519 signature against a public key (hex) — no secret required.
pub fn verify_signature_hex(
    public_hex: &str,
    signature_hex: &str,
    message: &[u8],
) -> Result<(), KeyError> {
    let pk_bytes: [u8; 32] = hex::decode(public_hex.trim())
        .map_err(KeyError::from)?
        .try_into()
        .map_err(|v: Vec<u8>| KeyError::InvalidHexLength(v.len()))?;
    let vk = VerifyingKey::from_bytes(&pk_bytes).map_err(|_| KeyError::SignatureInvalid)?;
    let sig_bytes: [u8; 64] = hex::decode(signature_hex.trim())
        .map_err(KeyError::from)?
        .try_into()
        .map_err(|v: Vec<u8>| KeyError::InvalidHexLength(v.len()))?;
    let sig = Signature::from_bytes(&sig_bytes);
    vk.verify(message, &sig)
        .map_err(|_| KeyError::SignatureInvalid)
}

#[derive(Clone, Debug)]
pub struct Identity {
    pub spiffe_id: String,
    pub key: KeyMaterial,
    pub audience: String,
}

impl Identity {
    pub fn ephemeral(role_path: &str) -> Self {
        let trimmed = role_path.trim_start_matches('/').trim_end_matches('/');
        Self {
            spiffe_id: format!("spiffe://local.aevum/{trimmed}"),
            key: KeyMaterial::generate(),
            audience: "aevum".to_string(),
        }
    }
}

impl Serialize for Identity {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Never serialize the secret bytes — only the public metadata.
        use serde::ser::SerializeStruct;
        let mut state = s.serialize_struct("Identity", 3)?;
        state.serialize_field("spiffe_id", &self.spiffe_id)?;
        state.serialize_field("public_bytes", &hex::encode(self.key.public_bytes()))?;
        state.serialize_field("audience", &self.audience)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Identity {
    fn deserialize<D: serde::Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        Err(D::Error::custom("Identity cannot be deserialized from raw bytes — reconstruct KeyMaterial from hex via identity.secrets.create() in the Sentinel Kernel"))
    }
}
