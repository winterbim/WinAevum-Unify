//! Golden Path — issue → branch → test → package → PR draft (never merge).
//!
//! Real local git + real test commands + Evidence Package. No GitHub API mock:
//! writes a reviewable `pr-draft.json` and optionally invokes `gh pr create`
//! when `AEVUM_GH_PR=1` and `gh` is installed — still never merges.

use std::fs;
use std::path::Path;
use std::process::Command;

use aevum_autonomy_governor::{ApprovalRequirement, AutonomyGovernor, RiskClass};
use aevum_git_provider::{BranchProvider, LocalGit, MemoryRepo};
use aevum_memory_fabric::{open_backend, MemoryBackend};
use serde_json::json;

use crate::graph_cmd::{require_authorized, require_falsifier_if_needed};
use crate::{cmd_package, load_metadata, require_value, sha256_hex, CliError};

pub fn cmd_golden(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let repo = optional(args, "--repo").unwrap_or_else(|| ".".into());
    let title = optional(args, "--title").unwrap_or_else(|| "aevum golden path".into());
    let run_tests = args.iter().any(|a| a == "--run-tests");
    let slug = slugify(&title);
    let branch = optional(args, "--branch").unwrap_or_else(|| format!("aevum/{slug}"));

    let meta = load_metadata(&mission)?;
    let risk = RiskClass::from_label(&meta.mission.risk).unwrap_or(RiskClass::R2);
    require_falsifier_if_needed(&mission, risk)?;

    // Governor: R3+ must have human approval recorded
    let gov = AutonomyGovernor::default();
    match gov.requirement_for(risk) {
        ApprovalRequirement::HumanApproval { reason, .. } => {
            let approvals = Path::new(&mission).join("approvals.jsonl");
            if !approvals.exists() {
                return Err(CliError::Verify(format!(
                    "R3+ blocked: {reason} — write an approval to {}",
                    approvals.display()
                )));
            }
            let raw = fs::read_to_string(&approvals).unwrap_or_default();
            let ok = raw.lines().filter(|l| !l.trim().is_empty()).any(|l| {
                let v: serde_json::Value = serde_json::from_str(l).unwrap_or_default();
                v.get("decision").and_then(|d| d.as_str()) == Some("approved")
            });
            if !ok {
                return Err(CliError::Verify(
                    "R3+ blocked: no approved entry in approvals.jsonl".into(),
                ));
            }
        }
        ApprovalRequirement::Acknowledgement { .. } | ApprovalRequirement::None => {}
    }

    require_authorized(&mission, "git.branch.create")?;

    // 1) Create side branch (never main)
    let mut git = LocalGit::new();
    let repo_handle = MemoryRepo::new(&repo, "main");
    let branch_ref = git
        .create_branch_on(&repo_handle, &branch)
        .map_err(|e| CliError::Verify(e.to_string()))?;
    println!("✓ branch {branch_ref}");

    // 2) Optional real tests
    let mut test_log = String::new();
    if run_tests {
        require_authorized(&mission, "process.exec.argv")?;
        let output = Command::new("cargo")
            .args(["test", "--workspace", "--quiet"])
            .current_dir(&repo)
            .output()
            .map_err(|e| CliError::Io(format!("spawn cargo test: {e}")))?;
        test_log = format!(
            "exit={}\nstdout:\n{}\nstderr:\n{}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let log_path = Path::new(&mission).join("golden-tests.log");
        fs::write(&log_path, &test_log).map_err(|e| CliError::Io(e.to_string()))?;
        if !output.status.success() {
            return Err(CliError::Verify(format!(
                "golden path tests failed — see {}",
                log_path.display()
            )));
        }
        println!("✓ tests passed ({})", log_path.display());
    }

    // 3) Evidence package
    let pkg = Path::new(&mission).join("golden-package.json");
    cmd_package(&[
        "--mission".into(),
        mission.clone(),
        "--out".into(),
        pkg.to_str().unwrap().into(),
    ])?;

    // 4) PR draft (never merge)
    let backend = open_backend(&mission).map_err(|e| CliError::Verify(e.to_string()))?;
    let backend_name = MemoryBackend::name(backend.as_ref());
    let draft = json!({
        "schema": "aevum.pr-draft/v1",
        "title": title,
        "head": branch,
        "base": "main",
        "body": format!(
            "## Summary\nAutomated Golden Path for mission `{}`.\n\n## Evidence\n- package: `{}`\n- graph backend: {}\n- risk: {}\n\n## Policy\n- no auto-merge\n- no deploy\n",
            meta.mission.mission_id,
            pkg.display(),
            backend_name,
            meta.mission.risk
        ),
        "auto_merge": false,
        "package_path": pkg,
        "package_digest": sha256_hex(&fs::read_to_string(&pkg).unwrap_or_default()),
        "mission_id": meta.mission.mission_id,
        "test_log_bytes": test_log.len(),
    });
    let draft_path = Path::new(&mission).join("pr-draft.json");
    fs::write(&draft_path, serde_json::to_string_pretty(&draft).unwrap())
        .map_err(|e| CliError::Io(e.to_string()))?;
    println!("✓ PR draft written to {}", draft_path.display());

    // Optional real `gh pr create` — still never merges
    if std::env::var("AEVUM_GH_PR").ok().as_deref() == Some("1") {
        let status = Command::new("gh")
            .args([
                "pr",
                "create",
                "--title",
                &title,
                "--body-file",
                draft_path.to_str().unwrap(),
                "--base",
                "main",
                "--head",
                &branch,
            ])
            .current_dir(&repo)
            .status()
            .map_err(|e| CliError::Io(format!("gh: {e}")))?;
        if !status.success() {
            return Err(CliError::Verify(
                "gh pr create failed — draft remains for manual open".into(),
            ));
        }
        println!("✓ gh pr create succeeded (no merge)");
    } else {
        println!("  tip: export AEVUM_GH_PR=1 to open via gh (still no merge)");
    }

    Ok(())
}

fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars().flat_map(|c| c.to_lowercase()) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if (c == ' ' || c == '-' || c == '_') && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').chars().take(40).collect()
}

fn optional(args: &[String], key: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == key {
            return args.get(i + 1).cloned();
        }
        i += 1;
    }
    None
}
