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
            None => "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
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
        let mut prev =
            "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string();
        for (idx, entry) in self.entries.iter().enumerate() {
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
            if self.key.key.verify(&sig.value, d.as_bytes()).is_err() {
                return Err(LedgerError::BadSignature { seq });
            }
            prev = d;
        }
        Ok(self.entries.len() as u64)
    }
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
}
