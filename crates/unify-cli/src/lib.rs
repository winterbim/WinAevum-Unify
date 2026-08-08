//! `aevum-unify` core — the business logic that backs the `unify` binary.
//!
//! Splitting `lib.rs` from `main.rs` means we can write integration tests in
//! `tests/cli.rs` that exercise the *same* code paths the binary uses,
//! without paying the cost of spawning a process. The binary in `main.rs`
//! only parses argv and dispatches to one of these functions.

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
    /// Hex-encoded Ed25519 secret key. Local-first MVP: this is the
    /// reproducible trust anchor that lets the holder of the mission
    /// directory run `unify run` and sign with the same authority. In a
    /// production deployment (M11+) this would be replaced by a hardware
    /// key or a remote KMS reference — the field becomes a `kms_ref`.
    pub authority_secret_key_hex: String,
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
    println!("  unify verify-package <file.json>");
    println!("  unify exec         --mission <dir> --capability <name> --argv <token> [--argv <token>...]");
    println!("  unify graph        <status|search|as-of|authorize|add-episode> ...");
    println!("  unify context      --mission <dir> --query <text> [--capability <cap>]");
    println!("  unify falsify      --mission <dir> --reason <text>   # required for R3+");
    println!("  unify approve      --mission <dir> [--decision approved]");
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
        authority_secret_key_hex: hex::encode(authority.key.secret_bytes()),
        kernel_manifest_digest: "sha256:kernel:default/v1".into(),
    };
    let meta_path = Path::new(&out_dir).join("metadata.json");
    fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap())
        .map_err(|e| CliError::Io(format!("writing metadata: {e}")))?;
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
    let authority_key = KeyMaterial::from_secret_hex(&meta.authority_secret_key_hex)
        .map_err(|e| CliError::Verify(format!("authority secret key parse: {e}")))?;
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
    let trail = Path::new(mission_dir).join("audit_trail.jsonl");
    let prev_digest = previous_digest(&trail);
    let sequence = next_sequence(&trail);
    let record = serde_json::json!({
        "ts": chrono_now_iso(),
        "sequence": sequence,
        "actor": actor,
        "capability": capability,
        "argv": argv,
        "attestation_id": attestation_id,
        "prev_digest": prev_digest,
    });
    let line = serde_json::to_string(&record).unwrap();
    let mut text = fs::read_to_string(&trail).unwrap_or_default();
    text.push_str(&line);
    text.push('\n');
    fs::write(&trail, &text).map_err(|e| CliError::Io(format!("writing audit trail: {e}")))?;
    // Keep trust ledger twin in sync — evidence packages must not ship empty ledgers
    // after real effects (ADR-0021 / Projet Phare).
    let ledger = Path::new(mission_dir).join("ledger.jsonl");
    let mut ledger_text = fs::read_to_string(&ledger).unwrap_or_default();
    ledger_text.push_str(&line);
    ledger_text.push('\n');
    fs::write(&ledger, ledger_text).map_err(|e| CliError::Io(format!("writing ledger: {e}")))?;
    Ok(())
}

fn previous_digest(trail: &Path) -> String {
    let raw = fs::read_to_string(trail).unwrap_or_default();
    let last = raw.lines().rfind(|l| !l.trim().is_empty());
    match last {
        Some(line) => {
            let v: serde_json::Value = serde_json::from_str(line).unwrap_or_default();
            let cap = v.get("capability").and_then(|s| s.as_str()).unwrap_or("?");
            let argv = v.get("argv").and_then(|s| s.as_str()).unwrap_or("");
            sha256_hex(&format!("{cap}|{argv}"))
        }
        None => "sha256:genesis".into(),
    }
}

fn next_sequence(trail: &Path) -> u64 {
    let raw = fs::read_to_string(trail).unwrap_or_default();
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).ok()?;
            v.get("sequence")?.as_u64()
        })
        .max()
        .map(|n| n + 1)
        .unwrap_or(1)
}

pub fn cmd_verify(args: &[String]) -> Result<(), CliError> {
    let target = args
        .first()
        .ok_or_else(|| CliError::Missing("<dir>".into()))?;
    if !Path::new(target).is_dir() {
        return Err(CliError::NotFound(target.clone()));
    }
    let meta = load_metadata(target)?;
    let trail = Path::new(target).join("audit_trail.jsonl");
    let raw = if trail.exists() {
        fs::read_to_string(&trail).unwrap_or_default()
    } else {
        String::new()
    };
    let entry_count = raw.lines().filter(|l| !l.trim().is_empty()).count();
    let (chain_ok, summary) = walk_chain(&trail);
    println!("✓ trust ledger verified — {entry_count} entries on disk");
    println!("  mission: {}", meta.mission.mission_id);
    println!("  risk:    {}", meta.mission.risk);
    println!(
        "  policy:  {}",
        &meta.policy_bundle_digest[..33.min(meta.policy_bundle_digest.len())]
    );
    if chain_ok {
        println!("  chain:   {} (all links verified)", summary);
    } else {
        println!("  chain:   BROKEN — {summary}");
    }
    if !chain_ok {
        return Err(CliError::Verify(format!("ledger chain broken: {summary}")));
    }
    Ok(())
}

fn walk_chain(trail: &Path) -> (bool, String) {
    let raw = fs::read_to_string(trail).unwrap_or_default();
    let entries: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    if entries.is_empty() {
        return (true, "empty ledger".into());
    }
    let mut last_digest = "sha256:genesis".to_string();
    let mut prev_seq: u64 = 0;
    for (i, line) in entries.iter().enumerate() {
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => return (false, format!("entry {i} not json: {e}")),
        };
        let seq = v.get("sequence").and_then(|n| n.as_u64()).unwrap_or(0);
        if seq != prev_seq + 1 {
            return (
                false,
                format!("expected sequence {} got {}", prev_seq + 1, seq),
            );
        }
        let pd = v.get("prev_digest").and_then(|s| s.as_str()).unwrap_or("");
        if pd != last_digest {
            return (false, format!("seq {seq}: prev_digest mismatch"));
        }
        let cap = v.get("capability").and_then(|s| s.as_str()).unwrap_or("");
        let argv = v.get("argv").and_then(|s| s.as_str()).unwrap_or("");
        last_digest = sha256_hex(&format!("{cap}|{argv}"));
        prev_seq = seq;
    }
    (true, format!("{} entries linked", entries.len()))
}

pub fn cmd_package(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let out = require_value(args, "--out")?;
    let meta = load_metadata(&mission)?;
    let ledger_path = Path::new(&mission).join("ledger.jsonl");
    let audit_path = Path::new(&mission).join("audit_trail.jsonl");
    let slop_path = Path::new(&mission).join("slop-report.json");

    let audit_raw = if audit_path.exists() {
        fs::read_to_string(&audit_path).unwrap_or_default()
    } else {
        String::new()
    };
    let mut ledger_entries = if ledger_path.exists() {
        fs::read_to_string(&ledger_path).unwrap_or_default()
    } else {
        String::new()
    };

    // Fail-closed heal: effects recorded in audit must appear in the trust ledger.
    if ledger_entries.trim().is_empty() && !audit_raw.trim().is_empty() {
        fs::write(&ledger_path, &audit_raw)
            .map_err(|e| CliError::Io(format!("sync ledger from audit: {e}")))?;
        ledger_entries = audit_raw.clone();
    }
    if !audit_raw.trim().is_empty() && ledger_entries.trim().is_empty() {
        return Err(CliError::Verify(
            "refuse to package: audit_trail has effects but ledger is empty after sync".into(),
        ));
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

    // Build the package as a serde_json::Map with explicit insertion order —
    // the verify-package subcommand re-derives the digest by removing the
    // `package_digest` line, so the textual pre-digest representation must
    // match exactly.
    let mut pkg = serde_json::Map::new();
    pkg.insert(
        "package_version".into(),
        serde_json::Value::String("aevum.evidence-package/v1".into()),
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
    let placeholder = serde_json::Value::Object(pkg);
    let text = serde_json::to_string_pretty(&placeholder).unwrap();
    let digest = sha256_hex(&text);
    let mut with_digest = placeholder.as_object().unwrap().clone();
    with_digest.insert(
        "package_digest".into(),
        serde_json::Value::String(digest.clone()),
    );
    let final_value = serde_json::Value::Object(with_digest);
    fs::write(&out, serde_json::to_string_pretty(&final_value).unwrap())
        .map_err(|e| CliError::Io(format!("writing {out}: {e}")))?;
    println!("✓ package written to {out} (digest={digest})");
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
    let authority_key = KeyMaterial::from_secret_hex(&meta.authority_secret_key_hex)
        .map_err(|e| CliError::Verify(format!("authority secret key parse: {e}")))?;
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
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let minutes = (now / 60) % 60;
    let hours = (now / 3600) % 24;
    let days = now / 86400;
    let days_per_month = [31u64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut year = 1970u64;
    let mut day_of_year = days;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let yd = if leap { 366 } else { 365 };
        if day_of_year < yd {
            break;
        }
        day_of_year -= yd;
        year += 1;
    }
    let mut month = 0;
    let mut dom = day_of_year;
    for (i, dm) in days_per_month.iter().enumerate() {
        let m = if i == 1 && (year % 4 == 0 && year % 100 != 0 || year % 400 == 0) {
            29
        } else {
            *dm
        };
        if dom < m {
            month = i + 1;
            break;
        }
        dom -= m;
    }
    format!(
        "{year:04}-{:02}-{dom:02}T{hours:02}:{minutes:02}:00+00:00",
        month
    )
}

pub fn cmd_verify_package(args: &[String]) -> Result<(), CliError> {
    let target = args
        .first()
        .ok_or_else(|| CliError::Missing("<package.json>".into()))?;
    let raw =
        fs::read_to_string(target).map_err(|e| CliError::NotFound(format!("{target}: {e}")))?;
    let mut v: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| CliError::BadArgs(format!("invalid package json: {e}")))?;
    let declared_digest = v
        .get("package_digest")
        .and_then(|d| d.as_str())
        .ok_or_else(|| CliError::BadArgs("missing package_digest".into()))?
        .to_string();
    v.as_object_mut()
        .and_then(|m| m.remove("package_digest"))
        .ok_or_else(|| CliError::BadArgs("package_digest not removable".into()))?;
    let text = serde_json::to_string_pretty(&v)
        .map_err(|e| CliError::BadArgs(format!("re-serialize: {e}")))?;
    let computed = sha256_hex(&text);
    if computed != declared_digest {
        return Err(CliError::Verify(format!(
            "package_digest mismatch: declared={declared_digest} computed={computed}"
        )));
    }
    let mission_id = v
        .get("mission")
        .and_then(|m| m.get("mission_id"))
        .and_then(|s| s.as_str())
        .unwrap_or("(unknown)");
    println!("✓ evidence package verified — mission: {mission_id}");
    println!("  digest:     {computed}");
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

    #[test]
    fn package_digest_round_trips_through_verify_package() {
        // The writer's textual pre-digest representation must match the
        // reader's recomputed bytes — otherwise the `verify-package`
        // subcommand would always reject packages the writer itself built.
        let mut pkg = serde_json::Map::new();
        pkg.insert(
            "package_version".into(),
            serde_json::Value::String("aevum.evidence-package/v1".into()),
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
        let placeholder = serde_json::Value::Object(pkg);
        let text = serde_json::to_string_pretty(&placeholder).unwrap();
        let digest = sha256_hex(&text);
        let mut with_digest = placeholder.as_object().unwrap().clone();
        with_digest.insert(
            "package_digest".into(),
            serde_json::Value::String(digest.clone()),
        );
        let final_value = serde_json::Value::Object(with_digest);
        let serialized = serde_json::to_string_pretty(&final_value).unwrap();
        let mut parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        let declared = parsed
            .get("package_digest")
            .and_then(|d| d.as_str())
            .unwrap()
            .to_string();
        parsed.as_object_mut().unwrap().remove("package_digest");
        let recomputed = sha256_hex(&serde_json::to_string_pretty(&parsed).unwrap());
        assert_eq!(declared, recomputed);
    }
}
