#![forbid(unsafe_code)]
#![allow(missing_docs)]

use std::fmt;

use aevum_identity::Identity;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("chain integrity broken at sequence {seq}: prev_digest mismatch (expected={expected}, got={actual})")]
    ChainBroken {
        seq: u64,
        expected: String,
        actual: String,
    },
    #[error("signature did not verify at sequence {seq}")]
    BadSignature { seq: u64 },
    #[error("digest mismatch at sequence {seq}: expected {expected}, got {actual}")]
    BadDigest {
        seq: u64,
        expected: String,
        actual: String,
    },
    #[error("schema version must be set")]
    BadSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub sequence: u64,
    pub event_type: String,
    pub schema_version: String,
    pub tenant_id: String,
    pub mission_id: String,
    pub correlation_id: String,
    pub causation_id: Option<String>,
    pub actor_id: String,
    pub occurred_at: String,
    pub payload: serde_json::Value,
    pub previous_digest: String,
    pub signature: Option<Signature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub alg: String,
    pub value: String,
    pub key_id: String,
    pub public_bytes: String,
}

/// Genesis previous-digest for an empty chain.
pub const GENESIS_DIGEST: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

pub struct TrustLedger {
    entries: Vec<LedgerEntry>,
    pub key: Identity,
}

impl TrustLedger {
    pub fn new(signer: Identity) -> Self {
        Self {
            entries: Vec::new(),
            key: signer,
        }
    }

    pub fn signer(&self) -> &Identity {
        &self.key
    }
    pub fn entries(&self) -> &[LedgerEntry] {
        &self.entries
    }
    pub fn length(&self) -> u64 {
        self.entries.len() as u64
    }
    pub fn last_digest(&self) -> String {
        match self.entries.last() {
            Some(e) => digest_entry(e),
            None => GENESIS_DIGEST.to_string(),
        }
    }

    pub fn append(&mut self, mut entry: LedgerEntry) -> Result<u64, LedgerError> {
        if entry.schema_version.is_empty() {
            return Err(LedgerError::BadSchema);
        }
        let seq = self.entries.len() as u64 + 1;
        entry.sequence = seq;
        entry.previous_digest = self.last_digest();
        let d = digest_entry(&entry);
        let sig_hex = self.key.key.sign(d.as_bytes());
        entry.signature = Some(Signature {
            alg: "ed25519".into(),
            value: sig_hex,
            key_id: self.key.spiffe_id.clone(),
            public_bytes: hex::encode(self.key.key.public_bytes()),
        });
        self.entries.push(entry);
        Ok(seq)
    }

    pub fn verify(&self) -> Result<u64, LedgerError> {
        verify_chain_with_public(&self.entries, &hex::encode(self.key.key.public_bytes()))?;
        Ok(self.entries.len() as u64)
    }

    /// Load a JSONL ledger that already verifies against `signer`'s public key.
    pub fn from_jsonl(signer: Identity, text: &str) -> Result<Self, LedgerError> {
        let entries = parse_jsonl(text)?;
        let pub_hex = hex::encode(signer.key.public_bytes());
        verify_chain_with_public(&entries, &pub_hex)?;
        Ok(Self {
            entries,
            key: signer,
        })
    }

    /// Serialize entries as JSONL (one LedgerEntry per line, trailing newline).
    pub fn to_jsonl(&self) -> String {
        entries_to_jsonl(&self.entries)
    }
}

pub fn parse_jsonl(text: &str) -> Result<Vec<LedgerEntry>, LedgerError> {
    let mut out = Vec::new();
    for (i, line) in text.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        let entry: LedgerEntry =
            serde_json::from_str(line).map_err(|_| LedgerError::BadDigest {
                seq: (i + 1) as u64,
                expected: "valid LedgerEntry json".into(),
                actual: format!("line {}", i + 1),
            })?;
        out.push(entry);
    }
    Ok(out)
}

pub fn entries_to_jsonl(entries: &[LedgerEntry]) -> String {
    let mut s = String::new();
    for e in entries {
        s.push_str(&serde_json::to_string(e).unwrap_or_default());
        s.push('\n');
    }
    s
}

/// Verify a chain using only a public key (no secret required).
/// Returns the last entry digest (or [`GENESIS_DIGEST`] if empty).
pub fn verify_chain_with_public(
    entries: &[LedgerEntry],
    public_hex: &str,
) -> Result<String, LedgerError> {
    let mut prev = GENESIS_DIGEST.to_string();
    let mut last = prev.clone();
    for (idx, entry) in entries.iter().enumerate() {
        let seq = (idx + 1) as u64;
        if entry.sequence != seq {
            return Err(LedgerError::BadDigest {
                seq,
                expected: seq.to_string(),
                actual: entry.sequence.to_string(),
            });
        }
        if entry.previous_digest != prev {
            return Err(LedgerError::ChainBroken {
                seq,
                expected: prev,
                actual: entry.previous_digest.clone(),
            });
        }
        let sig = entry
            .signature
            .as_ref()
            .ok_or(LedgerError::BadSignature { seq })?;
        let d = digest_entry(entry);
        aevum_identity::verify_signature_hex(public_hex, &sig.value, d.as_bytes())
            .map_err(|_| LedgerError::BadSignature { seq })?;
        prev = d.clone();
        last = d;
    }
    Ok(last)
}

pub fn digest_entry(e: &LedgerEntry) -> String {
    let mut entry_no_sig = e.clone();
    entry_no_sig.signature = None;
    let json = serde_json::to_string(&entry_no_sig).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(json.as_bytes());
    format!("sha256:{}", hex::encode(h.finalize()))
}

impl fmt::Display for LedgerEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:06}] {} ts={} actor={} dig={}",
            self.sequence,
            self.event_type,
            self.occurred_at,
            self.actor_id,
            digest_entry(self)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aevum_identity::Identity;
    use serde_json::json;

    fn evt(t: &str, payload: serde_json::Value) -> LedgerEntry {
        LedgerEntry {
            sequence: 0,
            event_type: t.to_string(),
            schema_version: "aevum.ledger/v1".to_string(),
            tenant_id: "ten_local".to_string(),
            mission_id: "mis_01".to_string(),
            correlation_id: "cor_01".to_string(),
            causation_id: None,
            actor_id: "agt_x".to_string(),
            occurred_at: "2026-08-02T10:00:00+00:00".to_string(),
            payload,
            previous_digest: String::new(),
            signature: None,
        }
    }

    #[test]
    fn append_assigns_sequence_and_chain() {
        let mut ledger = TrustLedger::new(Identity::ephemeral("ledger-authority"));
        let seq1 = ledger
            .append(evt("mission.created", json!({"a":1})))
            .unwrap();
        let seq2 = ledger
            .append(evt("mission.constitution.accepted", json!({"v":1})))
            .unwrap();
        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);
        assert_ne!(ledger.entries()[0].previous_digest, "sha256:0000");
        assert_eq!(
            ledger.entries()[1].previous_digest,
            digest_entry(&ledger.entries()[0])
        );
        // Verification should pass
        ledger.verify().expect("chain must verify");
    }

    #[test]
    fn tampering_break_chain_is_detected() {
        let mut ledger = TrustLedger::new(Identity::ephemeral("ledger-authority"));
        ledger
            .append(evt("mission.created", json!({"a":1})))
            .unwrap();
        ledger
            .append(evt("mission.constitution.accepted", json!({"v":1})))
            .unwrap();
        // Mutate an entry's payload without re-signing
        ledger.entries[0].payload = json!({"a": 999});
        let err = ledger.verify().unwrap_err();
        assert!(
            matches!(err, LedgerError::BadSignature { .. })
                || matches!(err, LedgerError::BadDigest { .. })
        );
    }

    #[test]
    fn empty_ledger_verifies() {
        let ledger = TrustLedger::new(Identity::ephemeral("ledger-authority"));
        ledger.verify().expect("empty ledger must verify");
    }

    #[test]
    fn digest_is_stable_for_identical_entries() {
        let e = evt("mission.created", json!({"a":1}));
        let d1 = digest_entry(&e);
        let d2 = digest_entry(&e);
        assert_eq!(d1, d2);
    }

    #[test]
    fn jsonl_round_trip_verifies() {
        let mut ledger = TrustLedger::new(Identity::ephemeral("ledger-authority"));
        ledger
            .append(evt("mission.created", json!({"a": 1})))
            .unwrap();
        ledger
            .append(evt("capability.effect", json!({"cap": "x"})))
            .unwrap();
        let text = ledger.to_jsonl();
        let pub_hex = hex::encode(ledger.key.key.public_bytes());
        let tip = verify_chain_with_public(&parse_jsonl(&text).unwrap(), &pub_hex).unwrap();
        assert_eq!(tip, ledger.last_digest());
        let reloaded = TrustLedger::from_jsonl(ledger.key.clone(), &text).unwrap();
        assert_eq!(reloaded.length(), 2);
        reloaded.verify().unwrap();
    }
}
