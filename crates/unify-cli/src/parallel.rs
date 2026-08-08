//! Parallel attested missions (Cursor-style best-of-N, but packaged).

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::json;

use crate::package::{cmd_package, package_ids};
use crate::{cmd_new, optional_value, require_value, CliError};

/// `unify parallel --constitution <c.json> --out <dir> --n <k>`
/// Creates N mission directories under out, packages each, writes compare.json.
pub fn cmd_parallel(args: &[String]) -> Result<(), CliError> {
    let constitution = require_value(args, "--constitution")?;
    let out = require_value(args, "--out")?;
    let n: usize = optional_value(args, "--n")
        .and_then(|s| s.parse().ok())
        .unwrap_or(2)
        .clamp(2, 8);

    fs::create_dir_all(&out).map_err(|e| CliError::Io(e.to_string()))?;
    let mut packages = Vec::new();
    for i in 0..n {
        let mission = Path::new(&out).join(format!("mission-{i}"));
        let pkg = Path::new(&out).join(format!("pkg-{i}.json"));
        let raw = fs::read_to_string(&constitution)
            .map_err(|e| CliError::Io(format!("read constitution: {e}")))?;
        let mut v: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| CliError::BadArgs(format!("constitution json: {e}")))?;
        if let Some(serde_json::Value::String(id)) = v.get_mut("mission_id") {
            *id = format!("{id}_v{i}");
        }
        let cpath = Path::new(&out).join(format!("constitution-{i}.json"));
        fs::write(&cpath, serde_json::to_string_pretty(&v).unwrap())
            .map_err(|e| CliError::Io(e.to_string()))?;
        cmd_new(&[
            "--constitution".into(),
            cpath.to_string_lossy().into(),
            "--out".into(),
            mission.to_string_lossy().into(),
        ])?;
        cmd_package(&[
            "--mission".into(),
            mission.to_string_lossy().into(),
            "--out".into(),
            pkg.to_string_lossy().into(),
        ])?;
        let body = fs::read_to_string(&pkg).unwrap_or_default();
        let (sig, content_sha) = package_ids(&body);
        packages.push(json!({
            "index": i,
            "mission": mission.display().to_string(),
            "package": pkg.display().to_string(),
            "package_signature": sig.unwrap_or_else(|| "missing".into()),
            "package_content_sha256": content_sha,
        }));
        println!("✓ parallel variant {i} → {}", pkg.display());
    }

    let compare = json!({
        "n": n,
        "variants": packages,
        "note": "Compare signatures / graphs; pick winner then attest — never auto-merge",
        "auto_merge": false,
    });
    let compare_path = Path::new(&out).join("compare.json");
    fs::write(
        &compare_path,
        serde_json::to_string_pretty(&compare).unwrap(),
    )
    .map_err(|e| CliError::Io(e.to_string()))?;
    println!("✓ parallel compare → {}", compare_path.display());
    Ok(())
}

/// Optional: spawn worktrees when --repo is set (best-effort).
pub fn cmd_parallel_worktrees(args: &[String]) -> Result<(), CliError> {
    let repo = require_value(args, "--repo")?;
    let out = require_value(args, "--out")?;
    let n: usize = optional_value(args, "--n")
        .and_then(|s| s.parse().ok())
        .unwrap_or(2)
        .clamp(2, 4);
    fs::create_dir_all(&out).map_err(|e| CliError::Io(e.to_string()))?;
    for i in 0..n {
        let wt = Path::new(&out).join(format!("wt-{i}"));
        let branch = format!("aevum/parallel-{i}");
        let status = Command::new("git")
            .args([
                "-C",
                &repo,
                "worktree",
                "add",
                "-b",
                &branch,
                wt.to_str().unwrap(),
            ])
            .status()
            .map_err(|e| CliError::Io(e.to_string()))?;
        if !status.success() {
            return Err(CliError::Verify(format!(
                "git worktree add failed for {branch}"
            )));
        }
        println!("✓ worktree {i} → {}", wt.display());
    }
    Ok(())
}
