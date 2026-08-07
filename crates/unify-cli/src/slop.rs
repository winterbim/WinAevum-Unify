//! `unify slop` — run offline AI-slop firewall and bind findings to the trust graph.
//!
//! Worlds-first: deterministic slop gate ∩ temporal epistemic firewall.
//! Blocking findings fail the command; all findings ingest as Inference only.

use std::path::{Path, PathBuf};
use std::process::Command;

use aevum_memory_fabric::{
    ingest_slop_report, open_backend, MemoryBackend, SlopReport, SqliteBackend,
};

use crate::graph_cmd::{load_graph, save_graph};
use crate::{load_metadata, require_value, CliError};

pub fn cmd_slop(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let repo = optional(args, "--repo").unwrap_or_else(|| ".".into());
    let base = optional(args, "--base");
    let all = args.iter().any(|a| a == "--all");
    let warn_only = args.iter().any(|a| a == "--warn-only");
    let skip_ingest = args.iter().any(|a| a == "--no-ingest");

    let bin = resolve_slopcheck()?;
    let mut cmd = Command::new(&bin);
    cmd.arg("--json");
    if all {
        cmd.arg("--all");
    } else if let Some(b) = &base {
        cmd.arg("--base").arg(b);
    } else {
        // Default: staged/working-tree scan (slopcheck default) — use --all for WT vs HEAD
        cmd.arg("--all");
    }
    cmd.current_dir(&repo);

    let output = cmd
        .output()
        .map_err(|e| CliError::Io(format!("spawn slopcheck ({}): {e}", bin.display())))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.trim().is_empty() && !output.status.success() {
        return Err(CliError::Verify(format!("slopcheck failed: {stderr}")));
    }

    let report = parse_report(&stdout)?;
    let blocking = report.blocking;
    let warns = report
        .findings
        .iter()
        .filter(|f| f.severity != "block")
        .count();

    println!(
        "✓ slopcheck — {} blocking, {} warning(s) (bin={})",
        blocking,
        warns,
        bin.display()
    );
    for f in report.findings.iter().take(20) {
        let tag = if f.severity == "block" {
            "BLOCK"
        } else {
            "warn"
        };
        println!("  {tag}  {}:{}  [{}] {}", f.path, f.line, f.rule, f.message);
    }
    if report.findings.len() > 20 {
        println!("  … +{} more", report.findings.len() - 20);
    }

    if !skip_ingest {
        let meta = load_metadata(&mission)?;
        let mut g = load_graph(&mission)?;
        let now = chrono_now();
        let ingest = ingest_slop_report(&mut g, &meta.mission.mission_id, &report, &now)
            .map_err(|e| CliError::Verify(e.to_string()))?;
        save_graph(&mission, &g)?;
        // Keep sqlite twin
        if let Ok(mut sb) = SqliteBackend::open(&mission) {
            *sb.graph_mut() = g;
            let _ = sb.save();
        }
        // Also via open_backend path for consistency
        let _ = open_backend(&mission);
        println!(
            "✓ ingested as Inference — episode={} facts={}",
            ingest.episode_id, ingest.facts_asserted
        );
        println!("  (slop findings cannot authorize — epistemic firewall)");
    }

    let out_path = Path::new(&mission).join("slop-report.json");
    std::fs::write(
        &out_path,
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| stdout.to_string()),
    )
    .map_err(|e| CliError::Io(e.to_string()))?;
    println!("✓ report → {}", out_path.display());

    if blocking > 0 && !warn_only {
        return Err(CliError::Verify(format!(
            "slop gate blocked: {blocking} blocking finding(s) — fix before golden/package"
        )));
    }
    Ok(())
}

fn parse_report(stdout: &str) -> Result<SlopReport, CliError> {
    let v: serde_json::Value = serde_json::from_str(stdout)
        .map_err(|e| CliError::Verify(format!("slopcheck json: {e}; out={stdout}")))?;
    // Support both {findings,blocking} and raw array
    if v.get("findings").is_some() {
        serde_json::from_value(v).map_err(|e| CliError::Verify(e.to_string()))
    } else if v.is_array() {
        let findings: Vec<aevum_memory_fabric::SlopFinding> =
            serde_json::from_value(v).map_err(|e| CliError::Verify(e.to_string()))?;
        let blocking = findings.iter().filter(|f| f.severity == "block").count() as u32;
        Ok(SlopReport { findings, blocking })
    } else {
        Err(CliError::Verify("unexpected slopcheck json shape".into()))
    }
}

pub fn resolve_slopcheck() -> Result<PathBuf, CliError> {
    if let Ok(p) = std::env::var("SLOPCHECK_BIN") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Ok(pb);
        }
    }
    // Sibling checkout common on this machine
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{home}/slopcheck/.venv/bin/slopcheck"),
        format!("{home}/.local/bin/slopcheck"),
        "/usr/local/bin/slopcheck".into(),
    ];
    for c in candidates {
        let p = PathBuf::from(&c);
        if p.exists() {
            return Ok(p);
        }
    }
    // PATH lookup
    if let Ok(out) = Command::new("which").arg("slopcheck").output() {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                return Ok(PathBuf::from(p));
            }
        }
    }
    Err(CliError::NotFound(
        "slopcheck not found — set SLOPCHECK_BIN or install https://github.com/winterbim/slopcheck"
            .into(),
    ))
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

fn chrono_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let rem = secs % 86400;
    let h = rem / 3600;
    let m = (rem % 3600) / 60;
    let s = rem % 60;
    format!("2026-08-08T{h:02}:{m:02}:{s:02}Z")
}
