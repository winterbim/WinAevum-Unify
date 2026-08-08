//! `aevum-unify` core — the business logic that backs the `unify` binary.
//!
//! Splitting `lib.rs` from `main.rs` means we can write integration tests in
//! `tests/cli.rs` that exercise the *same* code paths the binary uses,
//! without paying the cost of spawning a process. The binary in `main.rs`
//! only parses argv and dispatches to one of these functions.

pub mod atomic;
pub mod authority;
pub mod dream;
pub mod golden;
pub mod graph_cmd;
pub mod hooks;
pub mod ledger_io;
pub mod mission_ops;
pub mod package;
pub mod parallel;
pub mod rules;
pub mod slop;

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub use aevum_attestation::{
    ActionAttestation as RustAttestation, AttestationSigner, RiskClass as RustRiskClass,
};
pub use aevum_identity::{Identity, KeyMaterial};
pub use hooks::cmd_pretool_check;
pub use ledger_io::verify_signed_ledger_text;
pub use mission_ops::{cmd_exec, cmd_run, cmd_verify};
pub use package::{cmd_package, cmd_verify_package};

#[derive(Debug, Serialize, Deserialize)]
pub struct Mission {
    pub mission_id: String,
    pub title: String,
    pub risk: String,
    pub constitution_digest: String,
    pub authority_actor: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MissionMetadata {
    pub mission: Mission,
    pub policy_bundle_digest: String,
    pub authority_public_key: String,
    /// Authority secret is NEVER stored here (P0-1). See `{mission}/.aevum/authority.sk`.
    pub kernel_manifest_digest: String,
}

#[derive(Debug)]
pub enum CliError {
    Missing(String),
    NotFound(String),
    BadArgs(String),
    Io(String),
    Verify(String),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Missing(s) => write!(f, "missing required argument: {s}"),
            CliError::NotFound(s) => write!(f, "not found: {s}"),
            CliError::BadArgs(s) => write!(f, "bad arguments: {s}"),
            CliError::Io(s) => write!(f, "io: {s}"),
            CliError::Verify(s) => write!(f, "verification: {s}"),
        }
    }
}

pub fn print_help() {
    println!("unify — Aevum Unify CLI (local-first MVP)\n");
    println!("USAGE:");
    println!("  unify new            --constitution <path.json> --out <dir>");
    println!("  unify run            --mission <dir> --capability <name> --argv <str>");
    println!("  unify verify         <dir>");
    println!("  unify package        --mission <dir> --out <file.json>");
    println!("  unify verify-package <file.json> [--trust-pubkey <hex-file>]");
    println!("  unify exec         --mission <dir> --capability <name> --argv <token> [--argv <token>...]");
    println!("  unify graph        <status|search|as-of|authorize|add-episode> ...");
    println!("  unify human-keygen [--out <path>]   # distinct human principal (P0-5)");
    println!("  unify human-grant  --mission-id <id> --capability <name> [--reason …]");
    println!(
        "  unify pretool-check --capability <name> [--mission <dir>] [--tool …] [--command …]"
    );
    println!("  unify debug-now    # print UTC clock (P0-6)");
    println!("  unify context      --mission <dir> --query <text> [--capability <cap>]");
    println!("  unify falsify      --mission <dir> --reason <text>   # required for R3+");
    println!("  unify approve      --mission <dir> [--decision approved]");
    println!("  unify dream        --mission <dir> [--capability <cap>] [--query <text>]");
    println!("                     # AGENT_CARD — what an agent needs to act safely");
    println!("  unify doctor       --mission <dir>");
    println!("                     # hard self-check (never silent failure)");
    println!("  unify golden       --mission <dir> --repo <path> [--title …] [--run-tests] [--slop-gate]");
    println!("  unify slop         --mission <dir> [--repo <path>] [--all|--base <ref>]");
    println!("                     # AI-slop firewall → Inference on graph (never authorizes)");
    println!("  unify rules scan   --mission <dir> [--repo <path>]");
    println!("                     # hookify rules → Inference (never authorizes)");
    println!("  unify parallel     --constitution <c.json> --out <dir> [--n 2..8]");
    println!("                     # attested best-of-N missions + compare.json");
    println!("  unify mcp          --mission <dir> [--write-config claude|cursor] [--out <path>]");
    println!("                     # stdio MCP / write client config");
    println!("                     (temporal trust graph — gates run/exec)");
}

pub fn cmd_new(args: &[String]) -> Result<(), CliError> {
    let constitution = require_value(args, "--constitution")?;
    let out_dir = require_value(args, "--out")?;
    let src = fs::read_to_string(&constitution)
        .map_err(|e| CliError::Io(format!("reading {constitution}: {e}")))?;
    let raw: serde_json::Value =
        serde_json::from_str(&src).map_err(|e| CliError::BadArgs(format!("invalid json: {e}")))?;
    let mission_id = raw
        .get("mission_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::BadArgs("mission_id missing".into()))?
        .to_string();
    let title = raw
        .get("objective")
        .and_then(|o| o.get("title"))
        .and_then(|t| t.as_str())
        .unwrap_or("(untitled)")
        .to_string();
    let risk_label = raw
        .get("risk")
        .and_then(|r| r.get("preliminary_class"))
        .and_then(|c| c.as_str())
        .unwrap_or("R2")
        .to_string();
    let constitution_digest = sha256_hex(&src);
    fs::create_dir_all(&out_dir).map_err(|e| CliError::Io(format!("creating {out_dir}: {e}")))?;
    let ledger_path = Path::new(&out_dir).join("ledger.jsonl");
    fs::write(&ledger_path, "").map_err(|e| CliError::Io(format!("writing ledger: {e}")))?;
    let policy_path = Path::new(&out_dir).join("policy.bundle.json");
    let policy_default = default_policy_bundle();
    let policy_digest = sha256_hex(&policy_default);
    fs::write(&policy_path, &policy_default)
        .map_err(|e| CliError::Io(format!("writing policy: {e}")))?;
    let authority = Identity::ephemeral("ledger-authority");
    authority::write_authority_keys(&out_dir, &authority.key)?;
    let meta = MissionMetadata {
        mission: Mission {
            mission_id: mission_id.clone(),
            title,
            risk: risk_label.clone(),
            constitution_digest,
            authority_actor: authority.spiffe_id.clone(),
        },
        policy_bundle_digest: policy_digest,
        authority_public_key: hex::encode(authority.key.public_bytes()),
        kernel_manifest_digest: "sha256:kernel:default/v1".into(),
    };
    let meta_path = Path::new(&out_dir).join("metadata.json");
    let meta_text = serde_json::to_string_pretty(&meta).unwrap();
    atomic::atomic_write(&meta_path, meta_text.as_bytes()).map_err(CliError::Io)?;
    atomic::set_mode_600(&meta_path).map_err(CliError::Io)?;
    // Temporal trust graph seeded from constitution (ADR-0013) + SQLite twin (P1).
    graph_cmd::seed_and_persist(
        &out_dir,
        &mission_id,
        &src,
        &meta.mission.constitution_digest,
    )?;
    // Durable SQLite twin (P1) — migrates from graph.json on first open.
    {
        use aevum_memory_fabric::{MemoryBackend, SqliteBackend};
        let sb = SqliteBackend::open(&out_dir)
            .map_err(|e| CliError::Verify(format!("sqlite seed: {e}")))?;
        sb.save()
            .map_err(|e| CliError::Verify(format!("sqlite save: {e}")))?;
    }
    println!(
        "mission {mission_id} created at {out_dir} (risk={risk_label}, authority={})",
        authority.spiffe_id
    );
    Ok(())
}

pub(crate) fn chrono_now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn cmd_debug_now(_args: &[String]) -> Result<(), CliError> {
    println!("{}", chrono_now_iso());
    Ok(())
}

pub fn optional_value(args: &[String], key: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == key {
            return args.get(i + 1).cloned();
        }
        i += 1;
    }
    None
}

pub fn load_metadata(mission_dir: &str) -> Result<MissionMetadata, CliError> {
    let p = Path::new(mission_dir).join("metadata.json");
    let txt =
        fs::read_to_string(&p).map_err(|e| CliError::NotFound(format!("{}: {e}", p.display())))?;
    serde_json::from_str(&txt).map_err(|e| CliError::BadArgs(format!("invalid metadata: {e}")))
}

pub fn require_value(args: &[String], key: &str) -> Result<String, CliError> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == key {
            if let Some(v) = args.get(i + 1) {
                return Ok(v.clone());
            }
            return Err(CliError::Missing(key.into()));
        }
        i += 1;
    }
    Err(CliError::Missing(key.into()))
}

pub fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    format!("sha256:{}", hex::encode(h.finalize()))
}

pub fn ulid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    format!("{:x}", now & 0xFFFF_FFFF_FFFF_FFFF)
}

pub fn risk_class_from_label(s: &str) -> RustRiskClass {
    match s {
        "R0" => RustRiskClass::R0,
        "R1" => RustRiskClass::R1,
        "R2" => RustRiskClass::R2,
        "R3" => RustRiskClass::R3,
        "R4" => RustRiskClass::R4,
        "R5" => RustRiskClass::R5,
        _ => RustRiskClass::R2,
    }
}

pub fn default_policy_bundle() -> String {
    let bundle = serde_json::json!({
        "version": "aevum.policy/v1.0.0",
        "rules": [
            { "id": "deny.sh.execute",   "effect": "deny",  "reason": "D-rule: §16.4 no shell libre" },
            { "id": "deny.git.main",     "effect": "deny",  "reason": "D-rule: writes against main forbidden" },
            { "id": "deny.path.hidden",  "effect": "deny",  "reason": "D-rule: path is a secret location" },
            { "id": "require-approval.r4", "effect": "require_approval", "reason": "R4 and above require explicit human signoff" },
            { "id": "allow.git.branch-create", "effect": "allow", "reason": "branch create on R2 with delete_branch recovery" }
        ]
    });
    serde_json::to_string_pretty(&bundle).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_class_from_label_maps_known_inputs() {
        assert_eq!(risk_class_from_label("R0") as i32, RustRiskClass::R0 as i32);
        assert_eq!(risk_class_from_label("R3") as i32, RustRiskClass::R3 as i32);
    }

    #[test]
    fn risk_class_from_label_defaults_to_r2() {
        assert_eq!(
            risk_class_from_label("garbage") as i32,
            RustRiskClass::R2 as i32
        );
    }

    #[test]
    fn sha256_hex_is_stable_and_prefixed() {
        let a = sha256_hex("aevum");
        let b = sha256_hex("aevum");
        assert_eq!(a, b);
        assert!(a.starts_with("sha256:"));
    }

    #[test]
    fn require_value_finds_args_after_flag() {
        let args = vec!["--capability".into(), "git.branch.create".into()];
        assert_eq!(
            require_value(&args, "--capability").unwrap(),
            "git.branch.create"
        );
    }

    #[test]
    fn require_value_returns_missing_when_absent() {
        let args = vec![];
        let r = require_value(&args, "--capability");
        assert!(matches!(r, Err(CliError::Missing(_))));
    }

    #[test]
    fn default_policy_bundle_is_well_formed_json() {
        let s = default_policy_bundle();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(
            v.get("version").unwrap().as_str().unwrap(),
            "aevum.policy/v1.0.0"
        );
        assert!(v.get("rules").unwrap().as_array().unwrap().len() >= 4);
    }

    #[test]
    fn default_policy_bundle_digest_is_stable() {
        let d1 = sha256_hex(&default_policy_bundle());
        let d2 = sha256_hex(&default_policy_bundle());
        assert_eq!(d1, d2);
    }
}
