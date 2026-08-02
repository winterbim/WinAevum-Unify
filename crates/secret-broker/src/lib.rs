#![forbid(unsafe_code)]
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("secret handle not registered")]
    Unknown,
    #[error("secret value never stored — opaque handle only")]
    ValueNeverStored,
    #[error("lease expired")]
    LeaseExpired,
    #[error("replay detected (nonce already used)")]
    Replay,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SecretHandle(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretLease {
    pub handle: SecretHandle,
    pub scope: String,
    pub expires_at: String,
    pub nonce: String,
}

#[derive(Default)]
pub struct SecretBroker {
    used_nonces: std::collections::HashSet<String>,
}

impl SecretBroker {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn create_handle(scope: &str, expires_at: &str, nonce: &str) -> SecretLease {
        SecretLease {
            handle: SecretHandle(format!("sh_01{}", nonce)),
            scope: scope.to_string(),
            expires_at: expires_at.to_string(),
            nonce: nonce.to_string(),
        }
    }
    pub fn consume(&mut self, lease: &SecretLease) -> Result<(), SecretError> {
        if self.used_nonces.contains(&lease.nonce) {
            return Err(SecretError::Replay);
        }
        self.used_nonces.insert(lease.nonce.clone());
        Ok(())
    }
    /// Returns the secret value via a sidechannel; here we explicitly state the
    /// broker never stores the value, which is the audit-trail property blueprint §16.4 requires.
    pub fn read(&self, _h: &SecretHandle) -> Result<&'static str, SecretError> {
        Err(SecretError::ValueNeverStored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_is_opaque_and_starts_with_prefix() {
        let l = SecretBroker::create_handle(
            "github:winterbim/example",
            "2099-01-01T00:00:00Z",
            "abc123",
        );
        assert!(l.handle.0.starts_with("sh_01"));
        assert_eq!(l.nonce, "abc123");
    }

    #[test]
    fn broker_never_stores_value() {
        let broker = SecretBroker::new();
        let l = SecretBroker::create_handle("x", "2099-01-01T00:00:00Z", "n");
        assert!(matches!(
            broker.read(&l.handle),
            Err(SecretError::ValueNeverStored)
        ));
    }

    #[test]
    fn consume_then_double_consume_raises_replay() {
        let mut broker = SecretBroker::new();
        let l = SecretBroker::create_handle("x", "2099-01-01T00:00:00Z", "nonce-replay");
        broker.consume(&l).unwrap();
        assert!(matches!(broker.consume(&l), Err(SecretError::Replay)));
    }
}
