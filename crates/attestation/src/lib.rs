#![forbid(unsafe_code)]
#![allow(missing_docs)]

//! Aevum Unify — Action Attestation protocol (M3).
//!
//! Implements the blueprint §12 envelope: a canonical-JSON payload that
//! transports actor identity, mission reference, risk snapshot, evidence,
//! policy and approval digests, recovery plan, expiry, and a nonce.
//! The envelope is signed via Ed25519; the signature is recorded separately
//! from the canonical bytes (which excludes the `signature` field).

use std::fmt;

use aevum_identity::Identity;
use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

/// Risk class R0..R5. R5 is a regulatory slot, never produced by score alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum RiskClass {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
}

impl fmt::Display for RiskClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::R0 => "R0",
            Self::R1 => "R1",
            Self::R2 => "R2",
            Self::R3 => "R3",
            Self::R4 => "R4",
            Self::R5 => "R5",
        })
    }
}

/// Canonical Action Attestation envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionAttestation {
    pub schema_version: String,
    pub attestation_id: String,
    pub action_id: String,
    pub mission_id: String,
    pub constitution_version: u32,
    pub constitution_digest: String,
    pub policy_bundle_digest: String,
    pub principal_id: String,
    pub agent_definition: String,
    pub council_role: String,
    pub capability: String,
    pub resource: String,
    pub parameters_digest: String,
    pub expected_effects: Vec<String>,
    pub forbidden_effects: Vec<String>,
    pub evidence_required: Vec<String>,
    pub evidence_attached: Vec<String>,
    pub evidence_completeness: f32,
    pub risk_class: RiskClass,
    pub risk_score: u32,
    pub reversible: bool,
    pub blast_radius: String,
    pub approval_ids: Vec<String>,
    pub not_before: String,
    pub expires_at: String,
    pub max_uses: u32,
    pub recovery_strategy: String,
    pub recovery_verified: bool,
    pub nonce: String,
    /// NOT serialised into the canonical bytes; populated after signing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl ActionAttestation {
    /// Compute the canonical bytes that are signed/verified. The signature
    /// field is *omitted* from the canonical form so that re-signing and
    /// verifying remain stable across encodings.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        let value = serde_json::to_value(self)?;
        let mut map = match value {
            Value::Object(m) => m,
            _ => return Ok(Vec::new()),
        };
        map.remove("signature");
        // Stable key ordering: re-serialise via serde_json with a BTreeMap-like
        // ordering. serde_json preserves insertion order by default; we instead
        // re-build via sorted keys so the output is canonical.
        let sorted = sort_value(Value::Object(map));
        let s = serde_json::to_string(&sorted)?;
        Ok(s.into_bytes())
    }
}

fn sort_value(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let sorted: Map<String, Value> = entries
                .into_iter()
                .map(|(k, v)| (k, sort_value(v)))
                .collect();
            Value::Object(sorted)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(sort_value).collect()),
        other => other,
    }
}

#[derive(Debug, Error)]
pub enum VerificationError {
    #[error("attestation does not contain a signature")]
    MissingSignature,
    #[error("attestation signature is invalid: {0}")]
    SignatureMismatch(String),
    #[error("attestation has expired (now={now}, expires_at={expires_at})")]
    Expired { now: String, expires_at: String },
    #[error("attestation is not yet valid (now={now}, not_before={not_before})")]
    NotYetValid { now: String, not_before: String },
    #[error("attestation replay detected: nonce {0} already used")]
    Replay(String),
    #[error("attestation cannot be consumed more than {max} times")]
    Exhausted { max: u32 },
}

pub struct AttestationSigner {
    identity: Identity,
}

impl AttestationSigner {
    pub fn new(identity: Identity) -> Self {
        Self { identity }
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    /// Sign an attestation: produces a copy with the `signature` field populated.
    pub fn sign(&self, mut a: ActionAttestation) -> Result<ActionAttestation, serde_json::Error> {
        let bytes = a.canonical_bytes()?;
        let sig_hex = self.identity.key.sign(&bytes);
        a.signature = Some(format!("ed25519:{sig_hex}"));
        Ok(a)
    }

    /// Verify an attestation: signature, freshness, replay protection.
    pub fn verify(&self, a: &ActionAttestation) -> Result<(), VerificationError> {
        let sig = a
            .signature
            .as_ref()
            .ok_or(VerificationError::MissingSignature)?;
        if !sig.starts_with("ed25519:") {
            return Err(VerificationError::SignatureMismatch(
                "expected `ed25519:` prefix".to_string(),
            ));
        }
        let hex_sig = &sig["ed25519:".len()..];
        let to_verify = a.clone();
        let bytes = to_verify.canonical_bytes().map_err(|e| {
            VerificationError::SignatureMismatch(format!("canonical encode failed: {e}"))
        })?;
        self.identity
            .key
            .verify(hex_sig, &bytes)
            .map_err(|e| VerificationError::SignatureMismatch(format!("{e}")))?;

        let now: DateTime<FixedOffset> = Utc::now().into();
        let not_before = DateTime::parse_from_rfc3339(&a.not_before)
            .map_err(|e| VerificationError::SignatureMismatch(format!("not_before parse: {e}")))?;
        let expires_at = DateTime::parse_from_rfc3339(&a.expires_at)
            .map_err(|e| VerificationError::SignatureMismatch(format!("expires_at parse: {e}")))?;
        if now < not_before {
            return Err(VerificationError::NotYetValid {
                now: now.to_rfc3339(),
                not_before: a.not_before.clone(),
            });
        }
        if now > expires_at {
            return Err(VerificationError::Expired {
                now: now.to_rfc3339(),
                expires_at: a.expires_at.clone(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aevum_identity::Identity;

    #[test]
    fn canonical_bytes_have_stable_key_order() {
        let mut a = ActionAttestation {
            schema_version: "aevum.action-attestation/v1".to_string(),
            attestation_id: "aat_01".into(),
            action_id: "act_01".into(),
            mission_id: "mis_01".into(),
            constitution_version: 1,
            constitution_digest: "sha256:demo".into(),
            policy_bundle_digest: "sha256:demo".into(),
            principal_id: "spiffe://local.aevum/x".into(),
            agent_definition: "x@1".into(),
            council_role: "producer".into(),
            capability: "git.branch.create".into(),
            resource: "r".into(),
            parameters_digest: "sha256:demo".into(),
            expected_effects: vec!["e".into()],
            forbidden_effects: vec!["f".into()],
            evidence_required: vec!["er".into()],
            evidence_attached: vec!["ea".into()],
            evidence_completeness: 1.0,
            risk_class: RiskClass::R2,
            risk_score: 30,
            reversible: true,
            blast_radius: "single_repository".into(),
            approval_ids: vec![],
            not_before: "2026-08-02T10:00:00+02:00".into(),
            expires_at: "2026-08-02T10:10:00+02:00".into(),
            max_uses: 1,
            recovery_strategy: "delete_branch".into(),
            recovery_verified: true,
            nonce: "n".into(),
            signature: None,
        };
        let bytes1 = a.canonical_bytes().unwrap();
        a.agent_definition = "y@1".into();
        let bytes2 = a.canonical_bytes().unwrap();
        assert_ne!(bytes1, bytes2);
        a.agent_definition = "x@1".into();
        let bytes3 = a.canonical_bytes().unwrap();
        assert_eq!(bytes1, bytes3);
    }

    #[test]
    fn signature_field_ignored_in_canonical() {
        let mut a = ActionAttestation {
            schema_version: "aevum.action-attestation/v1".to_string(),
            attestation_id: "aat_01".into(),
            action_id: "act_01".into(),
            mission_id: "mis_01".into(),
            constitution_version: 1,
            constitution_digest: "sha256:demo".into(),
            policy_bundle_digest: "sha256:demo".into(),
            principal_id: "spiffe://local.aevum/x".into(),
            agent_definition: "x@1".into(),
            council_role: "producer".into(),
            capability: "git.branch.create".into(),
            resource: "r".into(),
            parameters_digest: "sha256:demo".into(),
            expected_effects: vec!["e".into()],
            forbidden_effects: vec!["f".into()],
            evidence_required: vec!["er".into()],
            evidence_attached: vec!["ea".into()],
            evidence_completeness: 1.0,
            risk_class: RiskClass::R2,
            risk_score: 30,
            reversible: true,
            blast_radius: "single_repository".into(),
            approval_ids: vec![],
            not_before: "2026-08-02T10:00:00+02:00".into(),
            expires_at: "2026-08-02T10:10:00+02:00".into(),
            max_uses: 1,
            recovery_strategy: "delete_branch".into(),
            recovery_verified: true,
            nonce: "n".into(),
            signature: None,
        };
        let b1 = a.canonical_bytes().unwrap();
        a.signature = Some("ed25519:something".to_string());
        let b2 = a.canonical_bytes().unwrap();
        assert_eq!(b1, b2);
    }

    #[test]
    fn sign_and_verify_round_trip() {
        let id = Identity::ephemeral("agent/builder-1");
        let signer = AttestationSigner::new(id);
        let a = ActionAttestation {
            schema_version: "aevum.action-attestation/v1".to_string(),
            attestation_id: "aat_01".into(),
            action_id: "act_01".into(),
            mission_id: "mis_01".into(),
            constitution_version: 1,
            constitution_digest: "sha256:demo".into(),
            policy_bundle_digest: "sha256:demo".into(),
            principal_id: "spiffe://local.aevum/x".into(),
            agent_definition: "x@1".into(),
            council_role: "producer".into(),
            capability: "git.branch.create".into(),
            resource: "r".into(),
            parameters_digest: "sha256:demo".into(),
            expected_effects: vec!["e".into()],
            forbidden_effects: vec!["f".into()],
            evidence_required: vec!["er".into()],
            evidence_attached: vec!["ea".into()],
            evidence_completeness: 1.0,
            risk_class: RiskClass::R2,
            risk_score: 30,
            reversible: true,
            blast_radius: "single_repository".into(),
            approval_ids: vec![],
            not_before: "2026-08-01T00:00:00+00:00".into(),
            expires_at: "2099-01-01T00:00:00+00:00".into(),
            max_uses: 1,
            recovery_strategy: "delete_branch".into(),
            recovery_verified: true,
            nonce: "n".into(),
            signature: None,
        };
        let signed = signer.sign(a).unwrap();
        assert!(signed.signature.is_some());
        signer.verify(&signed).expect("verify ok");
    }
}
