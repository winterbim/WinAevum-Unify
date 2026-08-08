//! PreToolUse / agent-hook gate — fail-closed (P0-4).

use crate::graph_cmd;
use crate::{optional_value, CliError};

/// `unify pretool-check` — deny shell-strings and unauthorized tools.
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
        "sh -c", "bash -c", "bash -lc", "bash -i", "ksh -c", "ksh -lc", "zsh -c", "zsh -lc",
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
