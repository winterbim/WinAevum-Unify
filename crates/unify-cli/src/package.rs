//! Evidence package v2 — Ed25519-signed JSON + trust pubkey sidecar (P0-2).

use std::fs;
use std::path::Path;

use crate::atomic;
use crate::authority;
use crate::graph_cmd;
use crate::ledger_io::verify_signed_ledger_text;
use crate::{load_metadata, optional_value, require_value, sha256_hex, CliError};

/// Extract v2 package_signature; content sha256 is informational only (not trust root).
pub fn package_ids(body: &str) -> (Option<String>, String) {
    let sig = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|j| {
            j.get("package_signature")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string())
        });
    (sig, sha256_hex(body))
}

pub fn cmd_package(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let out = require_value(args, "--out")?;
    let meta = load_metadata(&mission)?;
    // P0-1: refuse if metadata still carries a secret field.
    {
        let raw = fs::read_to_string(Path::new(&mission).join("metadata.json"))
            .map_err(|e| CliError::Io(format!("metadata: {e}")))?;
        if raw.contains("authority_secret_key_hex") || raw.contains("secret_key_hex") {
            return Err(CliError::Verify(
                "refuse to package: metadata.json still contains authority secret material — \
                 migrate with any unify run/exec or delete authority_secret_key_hex"
                    .into(),
            ));
        }
    }
    let key = authority::load_authority_secret(&mission)?;
    let pub_hex = authority::load_authority_public_hex(&mission)?;
    let ledger_path = Path::new(&mission).join("ledger.jsonl");
    let audit_path = Path::new(&mission).join("audit_trail.jsonl");
    let slop_path = Path::new(&mission).join("slop-report.json");

    let audit_raw = if audit_path.exists() {
        fs::read_to_string(&audit_path).unwrap_or_default()
    } else {
        String::new()
    };
    let ledger_entries = if ledger_path.exists() {
        fs::read_to_string(&ledger_path).unwrap_or_default()
    } else {
        String::new()
    };

    if !audit_raw.trim().is_empty() && ledger_entries.trim().is_empty() {
        return Err(CliError::Verify(
            "refuse to package: audit_trail has effects but ledger is empty".into(),
        ));
    }
    let last = verify_signed_ledger_text(&ledger_entries, &pub_hex)
        .map_err(|e| CliError::Verify(format!("refuse to package broken ledger: {e}")))?;
    if !ledger_entries.trim().is_empty() {
        authority::verify_ledger_tip(&mission, &last, &pub_hex)?;
    }

    let audit_digest = if audit_raw.trim().is_empty() {
        "sha256:none".into()
    } else {
        sha256_hex(&audit_raw)
    };
    let slop_digest = if slop_path.exists() {
        let s = fs::read_to_string(&slop_path).unwrap_or_default();
        sha256_hex(&s)
    } else {
        "sha256:none".into()
    };

    let mut pkg = serde_json::Map::new();
    pkg.insert(
        "package_version".into(),
        serde_json::Value::String("aevum.evidence-package/v2".into()),
    );
    pkg.insert(
        "mission".into(),
        serde_json::to_value(&meta.mission).unwrap(),
    );
    pkg.insert(
        "policy_bundle_digest".into(),
        serde_json::Value::String(meta.policy_bundle_digest.clone()),
    );
    pkg.insert(
        "authority_public_key".into(),
        serde_json::Value::String(meta.authority_public_key.clone()),
    );
    pkg.insert(
        "kernel_manifest_digest".into(),
        serde_json::Value::String(meta.kernel_manifest_digest.clone()),
    );
    pkg.insert(
        "ledger_entries".into(),
        serde_json::Value::String(ledger_entries.clone()),
    );
    pkg.insert(
        "audit_trail_digest".into(),
        serde_json::Value::String(audit_digest),
    );
    pkg.insert(
        "slop_report_digest".into(),
        serde_json::Value::String(slop_digest),
    );
    let graph_digest = if graph_cmd::graph_path(&mission).exists() {
        let graw = fs::read_to_string(graph_cmd::graph_path(&mission)).unwrap_or_default();
        sha256_hex(&graw)
    } else {
        "sha256:none".into()
    };
    pkg.insert(
        "temporal_graph_digest".into(),
        serde_json::Value::String(graph_digest),
    );
    let body = serde_json::Value::Object(pkg);
    let text = serde_json::to_string_pretty(&body).unwrap();
    let sig = key.sign(text.as_bytes());
    let mut signed = body.as_object().unwrap().clone();
    signed.insert(
        "package_signature".into(),
        serde_json::Value::String(format!("ed25519:{sig}")),
    );
    let final_value = serde_json::Value::Object(signed);
    let out_text = serde_json::to_string_pretty(&final_value).unwrap();
    if out_text.contains("authority_secret") || out_text.contains("secret_key_hex") {
        return Err(CliError::Verify(
            "refuse to write package containing authority secret material".into(),
        ));
    }
    atomic::atomic_write(Path::new(&out), out_text.as_bytes()).map_err(CliError::Io)?;
    let sidecar = format!("{out}.pubkey");
    atomic::atomic_write(Path::new(&sidecar), pub_hex.as_bytes()).map_err(CliError::Io)?;
    println!("✓ package written to {out} (ed25519 signature; trust pubkey → {sidecar})");
    Ok(())
}

pub fn cmd_verify_package(args: &[String]) -> Result<(), CliError> {
    let target = args
        .first()
        .ok_or_else(|| CliError::Missing("<package.json>".into()))?;
    let trust_path =
        optional_value(args, "--trust-pubkey").unwrap_or_else(|| format!("{target}.pubkey"));
    let trust_hex = fs::read_to_string(&trust_path).map_err(|e| {
        CliError::Verify(format!(
            "trust pubkey required at {trust_path}: {e} — package public key alone is not trusted (P0-2)"
        ))
    })?;
    let trust_hex = trust_hex.trim().to_string();
    let raw =
        fs::read_to_string(target).map_err(|e| CliError::NotFound(format!("{target}: {e}")))?;
    let mut v: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| CliError::BadArgs(format!("invalid package json: {e}")))?;
    let declared_sig = v
        .get("package_signature")
        .and_then(|d| d.as_str())
        .ok_or_else(|| {
            CliError::Verify(
                "missing package_signature — self-hash package_digest is not accepted (P0-2)"
                    .into(),
            )
        })?
        .to_string();
    let sig = declared_sig
        .strip_prefix("ed25519:")
        .unwrap_or(&declared_sig);
    v.as_object_mut()
        .and_then(|m| m.remove("package_signature"))
        .ok_or_else(|| CliError::BadArgs("package_signature not removable".into()))?;
    if let Some(obj) = v.as_object_mut() {
        obj.remove("package_digest");
    }
    let text = serde_json::to_string_pretty(&v)
        .map_err(|e| CliError::BadArgs(format!("re-serialize: {e}")))?;
    aevum_identity::verify_signature_hex(&trust_hex, sig, text.as_bytes()).map_err(|e| {
        CliError::Verify(format!(
            "package_signature invalid against trust pubkey: {e}"
        ))
    })?;
    let auth_pk = v
        .get("authority_public_key")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    if auth_pk != trust_hex {
        return Err(CliError::Verify(
            "authority_public_key does not match trust pubkey (refuse key substitution)".into(),
        ));
    }
    let ledger_entries = v
        .get("ledger_entries")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    verify_signed_ledger_text(ledger_entries, &trust_hex).map_err(|e| {
        CliError::Verify(format!("embedded ledger_entries failed verification: {e}"))
    })?;
    let ledger_n = ledger_entries
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    let audit_d = v
        .get("audit_trail_digest")
        .and_then(|s| s.as_str())
        .unwrap_or("sha256:none");
    if ledger_n > 0 && (audit_d == "sha256:none" || !audit_d.starts_with("sha256:")) {
        return Err(CliError::Verify(
            "package has ledger entries but audit_trail_digest is missing/none".into(),
        ));
    }
    if ledger_n == 0 && audit_d != "sha256:none" {
        return Err(CliError::Verify(
            "package claims audit_trail_digest but ledger_entries is empty".into(),
        ));
    }
    let mission_id = v
        .get("mission")
        .and_then(|m| m.get("mission_id"))
        .and_then(|s| s.as_str())
        .unwrap_or("(unknown)");
    println!("✓ evidence package verified — mission: {mission_id}");
    println!("  signature:  ed25519 (trust pubkey from {trust_path})");
    println!("  ledger:     {ledger_n} signed entr(y/ies) bound to trust key");
    println!(
        "  policy:     {}",
        v.get("policy_bundle_digest")
            .and_then(|s| s.as_str())
            .unwrap_or("?")
    );
    println!("  authority:  {auth_pk}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use aevum_identity::KeyMaterial;

    #[test]
    fn package_ids_prefers_signature_over_raw_hash() {
        let body = r#"{
  "package_signature": "ed25519:abc",
  "ledger_entries": ""
}"#;
        let (sig, content) = package_ids(body);
        assert_eq!(sig.as_deref(), Some("ed25519:abc"));
        assert!(content.starts_with("sha256:"));
    }

    #[test]
    fn package_signature_round_trips_with_trust_pubkey() {
        let key = KeyMaterial::generate();
        let mut pkg = serde_json::Map::new();
        pkg.insert(
            "package_version".into(),
            serde_json::Value::String("aevum.evidence-package/v2".into()),
        );
        pkg.insert(
            "mission".into(),
            serde_json::json!({
                "mission_id": "rt",
                "title": "roundtrip",
                "risk": "R2",
                "constitution_digest": "sha256:abc",
                "authority_actor": "spiffe://local.aevum/agent",
            }),
        );
        pkg.insert(
            "ledger_entries".into(),
            serde_json::Value::String(String::new()),
        );
        let body = serde_json::Value::Object(pkg);
        let text = serde_json::to_string_pretty(&body).unwrap();
        let sig = key.sign(text.as_bytes());
        aevum_identity::verify_signature_hex(
            &hex::encode(key.public_bytes()),
            &sig,
            text.as_bytes(),
        )
        .unwrap();
    }

    #[test]
    fn verify_package_rejects_signed_package_with_garbage_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let key = KeyMaterial::generate();
        let pk = hex::encode(key.public_bytes());
        let mut pkg = serde_json::Map::new();
        pkg.insert(
            "package_version".into(),
            serde_json::Value::String("aevum.evidence-package/v2".into()),
        );
        pkg.insert(
            "mission".into(),
            serde_json::json!({
                "mission_id": "badled",
                "title": "t",
                "risk": "R2",
                "constitution_digest": "sha256:x",
                "authority_actor": "spiffe://local.aevum/agent",
            }),
        );
        pkg.insert(
            "authority_public_key".into(),
            serde_json::Value::String(pk.clone()),
        );
        pkg.insert(
            "policy_bundle_digest".into(),
            serde_json::Value::String("sha256:p".into()),
        );
        pkg.insert(
            "kernel_manifest_digest".into(),
            serde_json::Value::String("sha256:k".into()),
        );
        pkg.insert(
            "ledger_entries".into(),
            serde_json::Value::String("not-valid-jsonl\n".into()),
        );
        pkg.insert(
            "audit_trail_digest".into(),
            serde_json::Value::String("sha256:deadbeef".into()),
        );
        pkg.insert(
            "slop_report_digest".into(),
            serde_json::Value::String("sha256:none".into()),
        );
        pkg.insert(
            "temporal_graph_digest".into(),
            serde_json::Value::String("sha256:none".into()),
        );
        let body = serde_json::Value::Object(pkg);
        let text = serde_json::to_string_pretty(&body).unwrap();
        let sig = key.sign(text.as_bytes());
        let mut signed = body.as_object().unwrap().clone();
        signed.insert(
            "package_signature".into(),
            serde_json::Value::String(format!("ed25519:{sig}")),
        );
        let pkg_path = dir.path().join("bad.json");
        let pk_path = dir.path().join("bad.json.pubkey");
        fs::write(
            &pkg_path,
            serde_json::to_string_pretty(&serde_json::Value::Object(signed)).unwrap(),
        )
        .unwrap();
        fs::write(&pk_path, &pk).unwrap();
        let err = cmd_verify_package(&[pkg_path.to_str().unwrap().into()]).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("ledger") || msg.contains("verification"),
            "got: {msg}"
        );
    }
}
