//! Mission lifecycle ops: run (attest), exec (spawn), verify (chain).

use std::fs;
use std::path::Path;

use crate::authority;
use crate::graph_cmd;
use crate::ledger_io;
use crate::{
    load_metadata, require_value, risk_class_from_label, sha256_hex, ulid_like,
    verify_signed_ledger_text, AttestationSigner, CliError, Identity, RustAttestation,
};

fn gate_capability(mission_dir: &str, capability: &str) -> Result<(), CliError> {
    graph_cmd::require_authorized(mission_dir, capability)?;
    let meta = load_metadata(mission_dir)?;
    if let Some(rc) = aevum_autonomy_governor::RiskClass::from_label(&meta.mission.risk) {
        graph_cmd::require_falsifier_if_needed(mission_dir, rc)?;
    }
    Ok(())
}

pub fn cmd_run(args: &[String]) -> Result<(), CliError> {
    let mission_dir = require_value(args, "--mission")?;
    let capability = require_value(args, "--capability")?;
    let argv_str = require_value(args, "--argv")?;
    gate_capability(&mission_dir, &capability)?;
    let meta = load_metadata(&mission_dir)?;
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
    ledger_io::append_audit_trail(
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
    if ledger_raw != trail_raw {
        return Err(CliError::Verify(
            "ledger/audit byte divergence — twins must be identical (fail-closed)".into(),
        ));
    }
    let last = verify_signed_ledger_text(&ledger_raw, &pub_hex)
        .map_err(|e| CliError::Verify(format!("ledger.jsonl: {e}")))?;
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

pub fn cmd_exec(args: &[String]) -> Result<(), CliError> {
    let mission_dir = require_value(args, "--mission")?;
    let capability = require_value(args, "--capability")?;
    let argv: Vec<String> = collect_all_argv(args);
    if argv.is_empty() {
        return Err(CliError::Missing("--argv".into()));
    }
    gate_capability(&mission_dir, &capability)?;
    let authority_key = authority::load_authority_secret(&mission_dir)?;
    let actor = Identity {
        spiffe_id: "spiffe://local.aevum/agent/exec".into(),
        key: authority_key,
        audience: "aevum".into(),
    };
    if let Some(bad) = aevum_sentinel_kernel::first_shell_metachar_arg(&argv) {
        return Err(CliError::Verify(format!(
            "argv entry contains shell metachar: {bad:?}"
        )));
    }
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
    ledger_io::append_audit_trail(
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
