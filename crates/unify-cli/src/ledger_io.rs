//! Disk-backed signed ledger I/O — thin wrapper over `aevum_ledger::TrustLedger`.

use std::fs;
use std::path::Path;

use aevum_identity::Identity;
use aevum_ledger::{LedgerEntry, TrustLedger};

use crate::atomic;
use crate::authority;
use crate::{chrono_now_iso, load_metadata, CliError};

/// Append a signed `capability.effect` entry to ledger.jsonl + audit_trail.jsonl.
pub(crate) fn append_audit_trail(
    mission_dir: &str,
    actor: &str,
    capability: &str,
    argv: &str,
    attestation_id: &str,
) -> Result<(), CliError> {
    let key = authority::load_authority_secret(mission_dir)?;
    let meta = load_metadata(mission_dir)?;
    let signer = Identity {
        spiffe_id: meta.mission.authority_actor.clone(),
        key,
        audience: "aevum".into(),
    };
    let trail = Path::new(mission_dir).join("audit_trail.jsonl");
    let ledger = Path::new(mission_dir).join("ledger.jsonl");
    let existing = fs::read_to_string(&ledger).unwrap_or_default();

    let mut tl = if existing.trim().is_empty() {
        TrustLedger::new(signer)
    } else {
        TrustLedger::from_jsonl(signer, &existing)
            .map_err(|e| CliError::Verify(format!("refuse to append: ledger corrupt ({e})")))?
    };

    tl.append(LedgerEntry {
        sequence: 0,
        event_type: "capability.effect".into(),
        schema_version: "aevum.ledger/v1".into(),
        tenant_id: "ten_local".into(),
        mission_id: meta.mission.mission_id.clone(),
        correlation_id: attestation_id.into(),
        causation_id: None,
        actor_id: actor.into(),
        occurred_at: chrono_now_iso(),
        payload: serde_json::json!({
            "capability": capability,
            "argv": argv,
            "attestation_id": attestation_id,
        }),
        previous_digest: String::new(),
        signature: None,
    })
    .map_err(|e| CliError::Verify(format!("ledger append: {e}")))?;

    let tip_digest = tl.last_digest();
    let new_text = tl.to_jsonl();
    atomic::atomic_write(&ledger, new_text.as_bytes()).map_err(CliError::Io)?;
    atomic::atomic_write(&trail, new_text.as_bytes()).map_err(CliError::Io)?;
    authority::write_ledger_tip(mission_dir, &tip_digest, &tl.key.key)?;
    Ok(())
}

/// Verify signed ledger JSONL against a public key. Returns last entry digest
/// (or genesis if empty).
pub fn verify_signed_ledger_text(text: &str, public_hex: &str) -> Result<String, String> {
    let entries = aevum_ledger::parse_jsonl(text).map_err(|e| e.to_string())?;
    aevum_ledger::verify_chain_with_public(&entries, public_hex).map_err(|e| e.to_string())
}
