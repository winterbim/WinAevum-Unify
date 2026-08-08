//! Authority + human principal key material (P0-1, P0-5).
//!
//! Mission authority secret lives in `{mission}/.aevum/authority.sk` (mode 600),
//! never in `metadata.json` and never inside an evidence package.
//!
//! Human authorize grants use a key **outside** the mission directory
//! (`$AEVUM_HUMAN_KEY` or `~/.config/aevum/human.sk`). V1 distinct principal =
//! possession of that human key; the agent process must not be given that path.

use std::fs;
use std::path::{Path, PathBuf};

use aevum_identity::{verify_signature_hex, KeyMaterial};

use crate::atomic::{atomic_write, set_mode_600, set_mode_644};
use crate::CliError;

pub fn aevum_dir(mission_dir: &str) -> PathBuf {
    Path::new(mission_dir).join(".aevum")
}

pub fn authority_sk_path(mission_dir: &str) -> PathBuf {
    aevum_dir(mission_dir).join("authority.sk")
}

pub fn authority_pub_path(mission_dir: &str) -> PathBuf {
    aevum_dir(mission_dir).join("authority.pub")
}

pub fn ledger_tip_path(mission_dir: &str) -> PathBuf {
    aevum_dir(mission_dir).join("ledger.tip")
}

pub fn ensure_aevum_dir(mission_dir: &str) -> Result<(), CliError> {
    fs::create_dir_all(aevum_dir(mission_dir))
        .map_err(|e| CliError::Io(format!("creating .aevum: {e}")))
}

pub fn write_authority_keys(mission_dir: &str, key: &KeyMaterial) -> Result<(), CliError> {
    ensure_aevum_dir(mission_dir)?;
    let sk = authority_sk_path(mission_dir);
    let pk = authority_pub_path(mission_dir);
    let sk_hex = hex::encode(key.secret_bytes());
    let pk_hex = hex::encode(key.public_bytes());
    atomic_write(&sk, sk_hex.as_bytes()).map_err(CliError::Io)?;
    set_mode_600(&sk).map_err(CliError::Io)?;
    atomic_write(&pk, pk_hex.as_bytes()).map_err(CliError::Io)?;
    set_mode_644(&pk).map_err(CliError::Io)?;
    Ok(())
}

/// Load authority secret from `.aevum/authority.sk`, migrating off metadata if needed.
pub fn load_authority_secret(mission_dir: &str) -> Result<KeyMaterial, CliError> {
    let sk_path = authority_sk_path(mission_dir);
    if sk_path.exists() {
        let hex_str = fs::read_to_string(&sk_path)
            .map_err(|e| CliError::Io(format!("reading authority.sk: {e}")))?;
        return KeyMaterial::from_secret_hex(hex_str.trim())
            .map_err(|e| CliError::Verify(format!("authority.sk parse: {e}")));
    }
    // One-time migration from legacy metadata.json secret field.
    let meta_path = Path::new(mission_dir).join("metadata.json");
    let raw = fs::read_to_string(&meta_path)
        .map_err(|e| CliError::NotFound(format!("metadata.json: {e}")))?;
    let v: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| CliError::BadArgs(format!("metadata json: {e}")))?;
    let legacy = v
        .get("authority_secret_key_hex")
        .and_then(|s| s.as_str())
        .ok_or_else(|| {
            CliError::Verify(
                "missing .aevum/authority.sk (and no legacy authority_secret_key_hex to migrate)"
                    .into(),
            )
        })?;
    let key = KeyMaterial::from_secret_hex(legacy)
        .map_err(|e| CliError::Verify(format!("legacy secret parse: {e}")))?;
    write_authority_keys(mission_dir, &key)?;
    // Rewrite metadata without the secret field.
    if let Some(obj) = v.as_object() {
        let mut clean = obj.clone();
        clean.remove("authority_secret_key_hex");
        let text = serde_json::to_string_pretty(&serde_json::Value::Object(clean)).unwrap();
        atomic_write(&meta_path, text.as_bytes()).map_err(CliError::Io)?;
        set_mode_600(&meta_path).map_err(CliError::Io)?;
    }
    Ok(key)
}

pub fn load_authority_public_hex(mission_dir: &str) -> Result<String, CliError> {
    let pk = authority_pub_path(mission_dir);
    if pk.exists() {
        let s = fs::read_to_string(&pk)
            .map_err(|e| CliError::Io(format!("reading authority.pub: {e}")))?;
        return Ok(s.trim().to_string());
    }
    let meta = crate::load_metadata(mission_dir)?;
    Ok(meta.authority_public_key)
}

pub fn write_ledger_tip(mission_dir: &str, last_digest: &str, key: &KeyMaterial) -> Result<(), CliError> {
    ensure_aevum_dir(mission_dir)?;
    let sig = key.sign(last_digest.as_bytes());
    atomic_write(&ledger_tip_path(mission_dir), sig.as_bytes()).map_err(CliError::Io)?;
    set_mode_600(&ledger_tip_path(mission_dir)).map_err(CliError::Io)?;
    Ok(())
}

pub fn verify_ledger_tip(mission_dir: &str, last_digest: &str, public_hex: &str) -> Result<(), CliError> {
    let tip = ledger_tip_path(mission_dir);
    if !tip.exists() {
        if last_digest.starts_with("sha256:0000") || last_digest == "sha256:genesis" {
            return Ok(());
        }
        return Err(CliError::Verify(
            "missing .aevum/ledger.tip — ledger tip is not anchored".into(),
        ));
    }
    let sig = fs::read_to_string(&tip)
        .map_err(|e| CliError::Io(format!("reading ledger.tip: {e}")))?;
    verify_signature_hex(public_hex, sig.trim(), last_digest.as_bytes())
        .map_err(|e| CliError::Verify(format!("ledger tip signature invalid: {e}")))
}

// ── Human principal (P0-5) ─────────────────────────────────────────────

pub fn default_human_sk_path() -> PathBuf {
    if let Ok(p) = std::env::var("AEVUM_HUMAN_KEY") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config/aevum/human.sk")
}

pub fn default_human_pub_path() -> PathBuf {
    if let Ok(p) = std::env::var("AEVUM_HUMAN_PUB") {
        return PathBuf::from(p);
    }
    let sk = default_human_sk_path();
    sk.with_extension("pub")
}

pub fn human_grant_message(mission_id: &str, capability: &str, reason: &str) -> String {
    format!("aevum.human-grant/v1|{mission_id}|{capability}|{reason}")
}

pub fn cmd_human_keygen(args: &[String]) -> Result<(), CliError> {
    let path = crate::optional_value(args, "--out").map(PathBuf::from);
    let sk_path = path.unwrap_or_else(default_human_sk_path);
    let pk_path = sk_path.with_extension("pub");
    if let Some(parent) = sk_path.parent() {
        fs::create_dir_all(parent).map_err(|e| CliError::Io(format!("mkdir: {e}")))?;
    }
    let key = KeyMaterial::generate();
    atomic_write(&sk_path, hex::encode(key.secret_bytes()).as_bytes()).map_err(CliError::Io)?;
    set_mode_600(&sk_path).map_err(CliError::Io)?;
    atomic_write(&pk_path, hex::encode(key.public_bytes()).as_bytes()).map_err(CliError::Io)?;
    set_mode_644(&pk_path).map_err(CliError::Io)?;
    println!("✓ human key written");
    println!("  secret: {}", sk_path.display());
    println!("  public: {}", pk_path.display());
    println!("  V1 distinct principal: hold this key outside the mission dir / agent sandbox.");
    Ok(())
}

pub fn cmd_human_grant(args: &[String]) -> Result<(), CliError> {
    let mission_id = crate::require_value(args, "--mission-id")?;
    let capability = crate::require_value(args, "--capability")?;
    let reason = crate::optional_value(args, "--reason")
        .unwrap_or_else(|| format!("human grant for {capability}"));
    let sk_path = crate::optional_value(args, "--human-key")
        .map(PathBuf::from)
        .unwrap_or_else(default_human_sk_path);
    let hex_str = fs::read_to_string(&sk_path).map_err(|e| {
        CliError::NotFound(format!(
            "human key {}: {e} — run `unify human-keygen` first",
            sk_path.display()
        ))
    })?;
    let key = KeyMaterial::from_secret_hex(hex_str.trim())
        .map_err(|e| CliError::Verify(format!("human key parse: {e}")))?;
    let msg = human_grant_message(&mission_id, &capability, &reason);
    let sig = key.sign(msg.as_bytes());
    println!("{sig}");
    Ok(())
}

pub fn verify_human_grant(
    mission_id: &str,
    capability: &str,
    reason: &str,
    signature_hex: &str,
) -> Result<(), CliError> {
    let pk_path = default_human_pub_path();
    let pk = fs::read_to_string(&pk_path).map_err(|e| {
        CliError::Verify(format!(
            "human pubkey {}: {e} — authorize requires a distinct human principal \
             (run `unify human-keygen` on the operator machine; never store human.sk in the mission)",
            pk_path.display()
        ))
    })?;
    let msg = human_grant_message(mission_id, capability, reason);
    verify_signature_hex(pk.trim(), signature_hex.trim(), msg.as_bytes()).map_err(|_| {
        CliError::Verify(
            "human grant signature invalid — self-authorize by the agent is refused (P0-5)".into(),
        )
    })
}
