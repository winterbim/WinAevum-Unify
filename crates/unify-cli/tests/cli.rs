//! Integration tests for the `unify` CLI.
//!
//! These tests invoke the compiled binary in a temporary directory and verify
//! that each subcommand produces the documented artefacts. They are
//! deliberately filesystem-only — no network, no Postgres.
//!
//! Run with: `cargo test -p aevum-unify --test cli`.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_unify"));
    c.env("RUST_LOG", "off");
    c
}

fn write_min_constitution(dir: &Path, mission_id: &str) -> std::path::PathBuf {
    let p = dir.join("constitution.json");
    let body = serde_json::json!({
        "mission_id": mission_id,
        "objective": { "title": "Test mission", "description": "Validated by integration test." },
        "scope": { "includes": ["src/*.ts"], "excludes": ["tests/*.snap"] },
        "risk": { "preliminary_class": "R2", "rationale": "single-module edit" },
        "evidence_required": ["repo_state", "tests_pass"]
    });
    fs::write(&p, serde_json::to_string_pretty(&body).unwrap()).unwrap();
    p
}

/// TEST CHANGE (P0-5): graph authorize requires a human grant signature.
fn human_grant_sig(tmp: &Path, mission_id: &str, capability: &str, reason: &str) -> String {
    let sk = tmp.join("human.sk");
    if !sk.exists() {
        let out = bin()
            .args(["human-keygen", "--out", sk.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "human-keygen: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let out = bin()
        .env("AEVUM_HUMAN_KEY", &sk)
        .env("AEVUM_HUMAN_PUB", tmp.join("human.pub"))
        .args([
            "human-grant",
            "--mission-id",
            mission_id,
            "--capability",
            capability,
            "--reason",
            reason,
            "--human-key",
            sk.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "human-grant: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn authorize(tmp: &Path, mission: &Path, capability: &str, reason: &str) {
    let meta: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(mission.join("metadata.json")).unwrap()).unwrap();
    let mid = meta["mission"]["mission_id"].as_str().unwrap();
    let sig = human_grant_sig(tmp, mid, capability, reason);
    let out = bin()
        .env("AEVUM_HUMAN_PUB", tmp.join("human.pub"))
        .args([
            "graph",
            "authorize",
            "--mission",
            mission.to_str().unwrap(),
            "--capability",
            capability,
            "--reason",
            reason,
            "--grant-sig",
            &sig,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "authorize: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn new_writes_mission_directory_and_prints_authority() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_test");
    let out_dir = tmp.path().join("mission");

    let output = bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .expect("spawn unify");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let meta = fs::read_to_string(out_dir.join("metadata.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&meta).unwrap();
    assert_eq!(v["mission"]["mission_id"], "mis_test");
    assert_eq!(v["mission"]["risk"], "R2");
    assert!(out_dir.join("ledger.jsonl").exists());
    assert!(out_dir.join("policy.bundle.json").exists());
    assert!(
        out_dir.join("graph.json").exists(),
        "temporal graph must be seeded"
    );
    assert!(v["policy_bundle_digest"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert!(v["authority_public_key"].as_str().unwrap().len() == 64);
    // TEST CHANGE (P0-1): secret must not live in metadata.json
    assert!(v.get("authority_secret_key_hex").is_none());
    assert!(
        out_dir.join(".aevum/authority.sk").exists(),
        "authority secret must be in .aevum/"
    );
}

#[test]
fn new_rejects_missing_constitution() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("mission");
    let output = bin()
        .args([
            "new",
            "--constitution",
            "/tmp/does-not-exist.json",
            "--out",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    assert!(!output.status.success());
    assert!(!out.exists());
}

#[test]
fn run_signs_and_prints_signature() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_run");
    let mission = tmp.path().join("mission");
    bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            mission.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let output = bin()
        .args([
            "run",
            "--mission",
            mission.to_str().unwrap(),
            "--capability",
            "git.branch.create",
            "--argv",
            "git checkout -b aevum/test-mission",
        ])
        .output()
        .expect("spawn");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("signed and verified"),
        "stdout was: {stdout}"
    );
    assert!(stdout.contains("git.branch.create"), "stdout was: {stdout}");
    assert!(stdout.contains("ed25519:"), "stdout was: {stdout}");
}

#[test]
fn run_uses_real_authority_key_from_metadata() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_auth");
    let mission = tmp.path().join("mission");
    bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            mission.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    // Metadata must declare an authority spiffe_id starting with spiffe://
    let meta = fs::read_to_string(mission.join("metadata.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&meta).unwrap();
    assert!(v["mission"]["authority_actor"]
        .as_str()
        .unwrap()
        .starts_with("spiffe://"));
}

#[test]
fn verify_reports_ledger_state() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_verify");
    let mission = tmp.path().join("mission");
    bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            mission.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let output = bin()
        .args(["verify", mission.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("mission_mis_verify") || stdout.contains("mis_verify"));
    assert!(stdout.contains("policy:"), "stdout: {stdout}");
}

#[test]
fn package_emits_digest_and_mission_metadata() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_pkg");
    let mission = tmp.path().join("mission");
    bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            mission.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let out_pkg = tmp.path().join("evidence-package.json");
    let output = bin()
        .args([
            "package",
            "--mission",
            mission.to_str().unwrap(),
            "--out",
            out_pkg.to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out_pkg.exists());

    let body = fs::read_to_string(&out_pkg).unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    // TEST CHANGE (P0-2): v2 packages carry package_signature, not package_digest.
    assert_eq!(v["package_version"], "aevum.evidence-package/v2");
    assert_eq!(v["mission"]["mission_id"], "mis_pkg");
    assert!(v["policy_bundle_digest"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert!(v["authority_public_key"].as_str().unwrap().len() == 64);
    assert!(v["package_signature"]
        .as_str()
        .unwrap()
        .starts_with("ed25519:"));
    assert!(!body.contains("authority_secret"));
    assert!(
        Path::new(&format!("{}.pubkey", out_pkg.display())).exists(),
        "trust pubkey sidecar required"
    );
    assert!(
        v["ledger_entries"].as_str().is_some()
            || v["ledger_entries"].is_object()
            || v["ledger_entries"].is_array()
    );
}
#[test]
fn verify_package_round_trips_after_run() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_round");
    let mission = tmp.path().join("mission");
    bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            mission.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    bin()
        .args([
            "run",
            "--mission",
            mission.to_str().unwrap(),
            "--capability",
            "git.branch.create",
            "--argv",
            "git checkout -b aevum/round-trip",
        ])
        .output()
        .unwrap();
    let out_pkg = tmp.path().join("evidence-package.json");
    bin()
        .args([
            "package",
            "--mission",
            mission.to_str().unwrap(),
            "--out",
            out_pkg.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let output = bin()
        .args(["verify-package", out_pkg.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("verified"), "stdout: {stdout}");
    assert!(stdout.contains("mis_round"), "stdout: {stdout}");
}

#[test]
fn verify_package_detects_tampered_digest() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_tamper");
    let mission = tmp.path().join("mission");
    bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            mission.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let out_pkg = tmp.path().join("evidence-package.json");
    bin()
        .args([
            "package",
            "--mission",
            mission.to_str().unwrap(),
            "--out",
            out_pkg.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let raw = fs::read_to_string(&out_pkg).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v["mission"]["title"] = serde_json::Value::String("tampered".into());
    let s = serde_json::to_string_pretty(&v).unwrap();
    fs::write(&out_pkg, s).unwrap();
    let output = bin()
        .args(["verify-package", out_pkg.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!output.status.success(), "expected tamper detection");
    let stderr = String::from_utf8_lossy(&output.stderr);
    // TEST CHANGE (P0-2): tamper fails signature verification.
    assert!(
        stderr.to_lowercase().contains("signature")
            || stderr.to_lowercase().contains("invalid")
            || stderr.to_lowercase().contains("mismatch"),
        "stderr: {stderr}"
    );
}

#[test]
fn verify_package_rejects_missing_file() {
    let tmp = TempDir::new().unwrap();
    let output = bin()
        .args([
            "verify-package",
            tmp.path().join("nope.json").to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    assert!(!output.status.success());
}

#[test]
fn exec_happy_path_appends_to_audit_trail() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_exec");
    let mission = tmp.path().join("mission");
    bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            mission.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let output = bin()
        .args([
            "exec",
            "--mission",
            mission.to_str().unwrap(),
            "--capability",
            "process.exec.argv",
            "--argv",
            "echo",
            "--argv",
            "aevum hello",
            "--argv",
            "git",
            "--argv",
            "--version",
        ])
        .output()
        .expect("spawn");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("audit_trailer=") || stdout.contains("audit"),
        "stdout: {stdout}"
    );

    let trail = mission.join("audit_trail.jsonl");
    assert!(trail.exists());
    let raw = fs::read_to_string(&trail).unwrap();
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(!lines.is_empty());
    let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    // TEST CHANGE (P0-3): signed LedgerEntry — capability lives in payload.
    assert_eq!(v["payload"]["capability"], "process.exec.argv");
    assert!(v["payload"]["argv"].as_str().unwrap().starts_with("echo"));
    assert!(v["sequence"].as_u64().is_some());
    assert!(v["previous_digest"].as_str().is_some());
    assert!(v["signature"].is_object());
    assert!(v["actor_id"].as_str().unwrap().starts_with("spiffe://"));
}

#[test]
fn exec_rejects_shell_metacharacters() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_shell");
    let mission = tmp.path().join("mission");
    bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            mission.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let output = bin()
        .args([
            "exec",
            "--mission",
            mission.to_str().unwrap(),
            "--capability",
            "process.exec.argv",
            "--argv",
            "sh",
            "--argv",
            "-c",
            "--argv",
            "rm -rf /",
        ])
        .output()
        .expect("spawn");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("reject")
            || stderr.to_lowercase().contains("sh")
            || stderr.to_lowercase().contains("deny"),
        "stderr: {stderr}"
    );
}

#[test]
fn exec_records_failure_exit_code() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_fail");
    let mission = tmp.path().join("mission");
    bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            mission.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let _output = bin()
        .args([
            "exec",
            "--mission",
            mission.to_str().unwrap(),
            "--capability",
            "process.exec.argv",
            "--argv",
            "false",
        ])
        .output()
        .expect("spawn");
    // The audit trail must still record the run even when the spawned
    // command returns non-zero exit code.
    let trail = mission.join("audit_trail.jsonl");
    assert!(trail.exists());
    let raw = fs::read_to_string(&trail).unwrap();
    let last = raw.lines().rfind(|l| !l.trim().is_empty()).unwrap();
    let v: serde_json::Value = serde_json::from_str(last).unwrap();
    assert_eq!(v["payload"]["capability"], "process.exec.argv");
    assert!(v["sequence"].as_u64().is_some());
    assert!(v["actor_id"].as_str().unwrap().starts_with("spiffe://"));
}

#[test]
fn run_appends_to_ledger_jsonl() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_ledger");
    let mission = tmp.path().join("mission");
    bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            mission.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    // First run.
    bin()
        .args([
            "run",
            "--mission",
            mission.to_str().unwrap(),
            "--capability",
            "git.branch.create",
            "--argv",
            "git checkout -b aevum/sec-fix",
        ])
        .output()
        .unwrap();
    // Second run requires explicit human-granted authorize (P0-5).
    authorize(
        tmp.path(),
        &mission,
        "git.commit",
        "self-test authorize commit",
    );
    bin()
        .args([
            "run",
            "--mission",
            mission.to_str().unwrap(),
            "--capability",
            "git.commit",
            "--argv",
            "git commit -m sec",
        ])
        .output()
        .unwrap();

    let trail = mission.join("audit_trail.jsonl");
    let raw = fs::read_to_string(&trail).unwrap();
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected 2 entries, got {lines:?}");
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(first["payload"]["capability"], "git.branch.create");
    assert_eq!(second["payload"]["capability"], "git.commit");
    assert_eq!(first["sequence"], 1);
    assert_eq!(second["sequence"], 2);
    assert!(first["previous_digest"].is_string());
    assert!(second["previous_digest"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert!(first["signature"].is_object());
}

#[test]
fn verify_walks_chain_and_proves_integrity() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_chain");
    let mission = tmp.path().join("mission");
    bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            mission.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    bin()
        .args([
            "run",
            "--mission",
            mission.to_str().unwrap(),
            "--capability",
            "git.branch.create",
            "--argv",
            "git checkout -b aevum/x",
        ])
        .output()
        .unwrap();
    authorize(tmp.path(), &mission, "fs.write", "chain test");
    bin()
        .args([
            "run",
            "--mission",
            mission.to_str().unwrap(),
            "--capability",
            "fs.write",
            "--argv",
            "echo hi",
        ])
        .output()
        .unwrap();

    let output = bin()
        .args(["verify", mission.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("chain:") || stdout.contains("verified"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("2 entries") || stdout.contains("2 signed"),
        "stdout: {stdout}"
    );
}

#[test]
fn run_rejects_unauthorized_capability() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_unauth");
    let mission = tmp.path().join("mission");
    bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            mission.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let output = bin()
        .args([
            "run",
            "--mission",
            mission.to_str().unwrap(),
            "--capability",
            "secrets.read",
            "--argv",
            "cat /etc/shadow",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("not authorized") || err.contains("temporal graph"),
        "stderr: {err}"
    );
}

#[test]
fn graph_status_and_search_work() {
    let tmp = TempDir::new().unwrap();
    let constitution = write_min_constitution(tmp.path(), "mis_graph");
    let mission = tmp.path().join("mission");
    bin()
        .args([
            "new",
            "--constitution",
            constitution.to_str().unwrap(),
            "--out",
            mission.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let status = bin()
        .args(["graph", "status", "--mission", mission.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    let out = String::from_utf8_lossy(&status.stdout);
    assert!(out.contains("episodes:"));
    assert!(out.contains("ALLOW"));

    let search = bin()
        .args([
            "graph",
            "search",
            "--mission",
            mission.to_str().unwrap(),
            "--query",
            "constitution authorizes",
        ])
        .output()
        .unwrap();
    assert!(search.status.success());
    assert!(String::from_utf8_lossy(&search.stdout).contains("hit"));
}
