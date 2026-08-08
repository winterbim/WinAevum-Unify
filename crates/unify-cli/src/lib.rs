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
use aevum_ledger::{digest_entry, LedgerEntry, Signature as LedgerSignature};

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
    println!("  unify pretool-check --capability <name> [--mission <dir>] [--tool …] [--command …]");
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

pub fn cmd_run(args: &[String]) -> Result<(), CliError> {
    let mission_dir = require_value(args, "--mission")?;
    let capability = require_value(args, "--capability")?;
    let argv_str = require_value(args, "--argv")?;
    graph_cmd::require_authorized(&mission_dir, &capability)?;
    let meta = load_metadata(&mission_dir)?;
    if let Some(rc) = aevum_autonomy_governor::RiskClass::from_label(&meta.mission.risk) {
        graph_cmd::require_falsifier_if_needed(&mission_dir, rc)?;
    }
    let authority_key = authority::load_authority_secret(&mission_dir)?;
    let actor = Identity {
        spiffe_id: "spiffe://local.aevum/agent/run-cli".to_string(),
        key: authority_key,
        audience: "aevum".to_string(),
    };
    let signer = AttestationSigner::new(actor.clone());
    let attestation = RustAttestation {
        schema_version: "aevum.action-attestation/v1".into(),
        attestation_id: format!("aat_{}", ulid_like()),
        action_id: format!("act_{}", ulid_like()),
        mission_id: meta.mission.mission_id.clone(),
        constitution_version: 1,
        constitution_digest: meta.mission.constitution_digest.clone(),
        policy_bundle_digest: meta.policy_bundle_digest.clone(),
        principal_id: actor.spiffe_id.clone(),
        agent_definition: "unify-cli@0.1".into(),
        council_role: "producer".into(),
        capability: capability.clone(),
        resource: argv_str.clone(),
        parameters_digest: sha256_hex(&argv_str),
        expected_effects: vec![format!("{capability} executed")],
        forbidden_effects: vec!["main modified".into()],
        evidence_required: vec!["repo_state".into()],
        evidence_attached: vec![],
        evidence_completeness: 0.6,
        risk_class: risk_class_from_label(&meta.mission.risk),
        risk_score: 25,
        reversible: true,
        blast_radius: "single_repository".into(),
        approval_ids: vec![],
        not_before: "2026-08-02T00:00:00+00:00".into(),
        expires_at: "2099-01-01T00:00:00+00:00".into(),
        max_uses: 1,
        recovery_strategy: "delete_branch".into(),
        recovery_verified: true,
        nonce: ulid_like(),
        signature: None,
    };
    let signed = signer
        .sign(attestation)
        .map_err(|e| CliError::Verify(format!("signing: {e}")))?;
    signer
        .verify(&signed)
        .map_err(|e| CliError::Verify(format!("verify failed: {e}")))?;
    let sig_value = signed
        .signature
        .as_ref()
        .and_then(|s| s.strip_prefix("ed25519:"))
        .unwrap_or("");
    append_audit_trail(
        &mission_dir,
        &actor.spiffe_id,
        &capability,
        &argv_str,
        &signed.attestation_id,
    )?;
    println!(
        "✓ signed and verified {capability} (attestation_id={})",
        signed.attestation_id
    );
    println!(
        "  signature (first 8 hex): ed25519:{}",
        &sig_value[..8.min(sig_value.len())]
    );
    Ok(())
}

fn append_audit_trail(
    mission_dir: &str,
    actor: &str,
    capability: &str,
    argv: &str,
    attestation_id: &str,
) -> Result<(), CliError> {
    let key = authority::load_authority_secret(mission_dir)?;
    let meta = load_metadata(mission_dir)?;
    let trail = Path::new(mission_dir).join("audit_trail.jsonl");
    let ledger = Path::new(mission_dir).join("ledger.jsonl");
    let existing = fs::read_to_string(&ledger).unwrap_or_default();
    // Refuse to extend a corrupt ledger.
    if let Err(e) = verify_signed_ledger_text(&existing, &meta.authority_public_key, true) {
        return Err(CliError::Verify(format!(
            "refuse to append: ledger corrupt ({e})"
        )));
    }
    let prev = last_entry_digest(&existing);
    let seq = existing
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count() as u64
        + 1;
    let mut entry = LedgerEntry {
        sequence: seq,
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
        previous_digest: prev,
        signature: None,
    };
    entry.sequence = seq;
    let d = digest_entry(&entry);
    let sig_hex = key.sign(d.as_bytes());
    entry.signature = Some(LedgerSignature {
        alg: "ed25519".into(),
        value: sig_hex,
        key_id: meta.mission.authority_actor.clone(),
        public_bytes: hex::encode(key.public_bytes()),
    });
    // Recompute digest after signature is set to None path — digest_entry clears sig.
    let tip_digest = digest_entry(&entry);
    let line = serde_json::to_string(&entry).unwrap();
    let mut new_text = existing;
    if !new_text.is_empty() && !new_text.ends_with('\n') {
        new_text.push('\n');
    }
    new_text.push_str(&line);
    new_text.push('\n');
    atomic::atomic_write(&ledger, new_text.as_bytes()).map_err(CliError::Io)?;
    atomic::atomic_write(&trail, new_text.as_bytes()).map_err(CliError::Io)?;
    authority::write_ledger_tip(mission_dir, &tip_digest, &key)?;
    Ok(())
}

fn last_entry_digest(ledger_text: &str) -> String {
    let last = ledger_text.lines().rfind(|l| !l.trim().is_empty());
    match last {
        Some(line) => match serde_json::from_str::<LedgerEntry>(line) {
            Ok(e) => digest_entry(&e),
            Err(_) => "sha256:genesis".into(),
        },
        None => {
            "sha256:0000000000000000000000000000000000000000000000000000000000000000".into()
        }
    }
}

/// Verify signed ledger JSONL. `require_tip_nonempty` is unused here — tip checked by caller.
pub fn verify_signed_ledger_text(
    text: &str,
    public_hex: &str,
    _allow_empty: bool,
) -> Result<String, String> {
    let mut prev =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string();
    let mut last_digest = prev.clone();
    let mut n = 0u64;
    for (i, line) in text.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        let entry: LedgerEntry =
            serde_json::from_str(line).map_err(|e| format!("entry {i} not LedgerEntry: {e}"))?;
        let seq = (i + 1) as u64;
        if entry.sequence != seq {
            return Err(format!(
                "expected sequence {seq} got {}",
                entry.sequence
            ));
        }
        if entry.previous_digest != prev {
            return Err(format!("seq {seq}: prev_digest mismatch"));
        }
        let sig = entry
            .signature
            .as_ref()
            .ok_or_else(|| format!("seq {seq}: missing signature"))?;
        let d = digest_entry(&entry);
        aevum_identity::verify_signature_hex(public_hex, &sig.value, d.as_bytes())
            .map_err(|e| format!("seq {seq}: bad signature: {e}"))?;
        prev = d.clone();
        last_digest = d;
        n += 1;
    }
    Ok(if n == 0 {
        prev
    } else {
        last_digest
    })
}

pub fn cmd_verify(args: &[String]) -> Result<(), CliError> {
    let target = args
        .first()
        .ok_or_else(|| CliError::Missing("<dir>".into()))?;
    if !Path::new(target).is_dir() {
        return Err(CliError::NotFound(target.clone()));
    }
    let meta = load_metadata(target)?;
    let pub_hex = authority::load_authority_public_hex(target)?;
    let ledger = Path::new(target).join("ledger.jsonl");
    let trail = Path::new(target).join("audit_trail.jsonl");
    let ledger_raw = if ledger.exists() {
        fs::read_to_string(&ledger).unwrap_or_default()
    } else {
        String::new()
    };
    let trail_raw = if trail.exists() {
        fs::read_to_string(&trail).unwrap_or_default()
    } else {
        String::new()
    };
    let entry_count = ledger_raw.lines().filter(|l| !l.trim().is_empty()).count();
    let last = verify_signed_ledger_text(&ledger_raw, &pub_hex, true)
        .map_err(|e| CliError::Verify(format!("ledger.jsonl: {e}")))?;
    verify_signed_ledger_text(&trail_raw, &pub_hex, true)
        .map_err(|e| CliError::Verify(format!("audit_trail.jsonl: {e}")))?;
    if !ledger_raw.trim().is_empty() && ledger_raw != trail_raw {
        // Allow trail==ledger content; if both verify independently that's enough,
        // but divergence in line count is a hard fail.
        let lc = ledger_raw.lines().filter(|l| !l.trim().is_empty()).count();
        let tc = trail_raw.lines().filter(|l| !l.trim().is_empty()).count();
        if lc != tc {
            return Err(CliError::Verify(format!(
                "ledger/audit divergence: ledger={lc} audit={tc}"
            )));
        }
    }
    if entry_count > 0 {
        authority::verify_ledger_tip(target, &last, &pub_hex)?;
    }
    println!("✓ trust ledger verified — {entry_count} signed entries on disk");
    println!("  mission: {}", meta.mission.mission_id);
    println!("  risk:    {}", meta.mission.risk);
    println!(
        "  policy:  {}",
        &meta.policy_bundle_digest[..33.min(meta.policy_bundle_digest.len())]
    );
    println!("  chain:   {entry_count} entries linked + tip anchored");
    Ok(())
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
    let last = verify_signed_ledger_text(&ledger_entries, &pub_hex, true)
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
    // Trust pubkey sidecar — verification must not trust the key embedded alone.
    let sidecar = format!("{out}.pubkey");
    atomic::atomic_write(Path::new(&sidecar), pub_hex.as_bytes()).map_err(CliError::Io)?;
    println!("✓ package written to {out} (ed25519 signature; trust pubkey → {sidecar})");
    Ok(())
}

pub fn cmd_exec(args: &[String]) -> Result<(), CliError> {
    let mission_dir = require_value(args, "--mission")?;
    let capability = require_value(args, "--capability")?;
    let argv: Vec<String> = collect_all_argv(args);
    if argv.is_empty() {
        return Err(CliError::Missing("--argv".into()));
    }
    graph_cmd::require_authorized(&mission_dir, &capability)?;
    let meta = load_metadata(&mission_dir)?;
    if let Some(rc) = aevum_autonomy_governor::RiskClass::from_label(&meta.mission.risk) {
        graph_cmd::require_falsifier_if_needed(&mission_dir, rc)?;
    }
    let authority_key = authority::load_authority_secret(&mission_dir)?;
    let actor = Identity {
        spiffe_id: "spiffe://local.aevum/agent/exec".into(),
        key: authority_key,
        audience: "aevum".into(),
    };
    // Sentinel: refuse shell metacharacters anywhere in argv. This is the
    // M11 hook — we use a hard-coded allow-list here to keep the
    // local-first MVP testable.
    for arg in &argv {
        if SHELL_METACHARS.iter().any(|c| arg.contains(*c)) {
            return Err(CliError::Verify(format!(
                "argv entry contains shell metachar: {arg:?}"
            )));
        }
    }
    // We refuse `sh -c` explicitly (D14).
    if argv.len() >= 2 && argv[0] == "sh" && argv.iter().any(|a| a == "-c") {
        return Err(CliError::Verify(
            "command rejected: sh -c is denied (D14)".into(),
        ));
    }
    let cmd = &argv[0];
    let output = std::process::Command::new(cmd)
        .args(&argv[1..])
        .output()
        .map_err(|e| CliError::Io(format!("spawn {cmd}: {e}")))?;
    let exit_code = output.status.code().unwrap_or(-1);
    let argv_for_log = argv.join(" ");
    append_audit_trail(
        &mission_dir,
        &actor.spiffe_id,
        &capability,
        &argv_for_log,
        "exec",
    )?;
    println!("✓ exec argv[0]={cmd} exit={exit_code} audit_record_id=exec");
    if exit_code != 0 {
        return Err(CliError::Verify(format!(
            "exec exited with code {exit_code}"
        )));
    }
    Ok(())
}

fn collect_all_argv(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--argv" {
            if let Some(v) = args.get(i + 1) {
                out.push(v.clone());
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}

const SHELL_METACHARS: &[&str] = &[";", "&", "|", "$", "`", ">", "<", "*", "?", "!", "\n", "\r"];

pub(crate) fn chrono_now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn cmd_debug_now(_args: &[String]) -> Result<(), CliError> {
    println!("{}", chrono_now_iso());
    Ok(())
}

pub fn cmd_verify_package(args: &[String]) -> Result<(), CliError> {
    let target = args
        .first()
        .ok_or_else(|| CliError::Missing("<package.json>".into()))?;
    let trust_path = optional_value(args, "--trust-pubkey").unwrap_or_else(|| format!("{target}.pubkey"));
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
                "missing package_signature — self-hash package_digest is not accepted (P0-2)".into(),
            )
        })?
        .to_string();
    let sig = declared_sig
        .strip_prefix("ed25519:")
        .unwrap_or(&declared_sig);
    v.as_object_mut()
        .and_then(|m| m.remove("package_signature"))
        .ok_or_else(|| CliError::BadArgs("package_signature not removable".into()))?;
    // Drop legacy package_digest if present — not authoritative.
    if let Some(obj) = v.as_object_mut() {
        obj.remove("package_digest");
    }
    let text = serde_json::to_string_pretty(&v)
        .map_err(|e| CliError::BadArgs(format!("re-serialize: {e}")))?;
    aevum_identity::verify_signature_hex(&trust_hex, sig, text.as_bytes()).map_err(|e| {
        CliError::Verify(format!("package_signature invalid against trust pubkey: {e}"))
    })?;
    let mission_id = v
        .get("mission")
        .and_then(|m| m.get("mission_id"))
        .and_then(|s| s.as_str())
        .unwrap_or("(unknown)");
    println!("✓ evidence package verified — mission: {mission_id}");
    println!("  signature:  ed25519 (trust pubkey from {trust_path})");
    println!(
        "  policy:     {}",
        v.get("policy_bundle_digest")
            .and_then(|s| s.as_str())
            .unwrap_or("?")
    );
    println!(
        "  authority:  {}",
        v.get("authority_public_key")
            .and_then(|s| s.as_str())
            .unwrap_or("?")
    );
    Ok(())
}

/// PreToolUse / hook gate — fail-closed (P0-4).
pub fn cmd_pretool_check(args: &[String]) -> Result<(), CliError> {
    let tool = optional_value(args, "--tool").unwrap_or_default();
    let command = optional_value(args, "--command").unwrap_or_default();
    let capability = optional_value(args, "--capability").unwrap_or_else(|| {
        if tool == "Edit" || tool == "Write" || tool == "MultiEdit" {
            "graph.write".into()
        } else {
            "process.exec.argv".into()
        }
    });
    let lowered = format!("{tool} {command}").to_lowercase();
    let shell_deny = [
        "sh -c",
        "bash -c",
        "bash -lc",
        "bash -i",
        "ksh -c",
        "ksh -lc",
        "zsh -c",
        "zsh -lc",
    ];
    if shell_deny.iter().any(|p| lowered.contains(p))
        || lowered.contains("bash_env=")
        || (lowered.contains("env ") && lowered.contains("bash"))
    {
        println!(
            "{}",
            serde_json::json!({
                "decision": "deny",
                "reason": "Aevum D14: shell-string / interactive shell denied — use process.exec.argv"
            })
        );
        return Err(CliError::Verify("pretool deny: D14".into()));
    }
    let mission = optional_value(args, "--mission")
        .or_else(|| std::env::var("AEVUM_MISSION").ok())
        .unwrap_or_default();
    if mission.trim().is_empty() {
        println!(
            "{}",
            serde_json::json!({
                "decision": "deny",
                "reason": "no AEVUM_MISSION — fail-closed (P0-4)"
            })
        );
        return Err(CliError::Verify("pretool deny: no mission".into()));
    }
    match graph_cmd::require_authorized(&mission, &capability) {
        Ok(()) => {
            println!(
                "{}",
                serde_json::json!({
                    "decision": "allow",
                    "reason": format!("authorized cap={capability} tool={tool}")
                })
            );
            Ok(())
        }
        Err(e) => {
            println!(
                "{}",
                serde_json::json!({
                    "decision": "deny",
                    "reason": e.to_string()
                })
            );
            Err(e)
        }
    }
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

    // TEST CHANGE (P0-2): self-hash package_digest replaced by Ed25519 package_signature.
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
}
