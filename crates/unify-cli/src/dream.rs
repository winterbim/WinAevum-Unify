//! `unify doctor` + `unify dream` — the control plane as the agent sees it.
//!
//! Design rule for this module: build the gate you would want to live under.
//! An agent running inside Aevum should be able to answer three questions with
//! one command each — *is this mission healthy* (`doctor`), *what am I allowed
//! to do right now and how* (`dream`), and *why was I refused* (denial
//! episodes on the temporal graph, surfaced by both).
//!
//! Report building is split from printing so the MCP tools (`aevum_doctor`,
//! `aevum_agent_card`) and AgentTrustBench consume the same values the CLI
//! prints — one truth, three surfaces.

use std::fs;
use std::path::Path;

use aevum_evidence_graph::{may_authorize, EdgeKind, FirewallVerdict, TemporalGraph};
use serde_json::{json, Value};

use crate::graph_cmd::{load_graph, GRAPH_FILE};
use crate::{chrono_now_iso, load_metadata, require_value, CliError};

/// Edge name written by `graph_cmd::record_denial_episode`.
const DENIAL_FACT_NAME: &str = "DENIED_CAPABILITY";

pub const DOCTOR_OK: &str = "AEVUM_DOCTOR_OK";
pub const DOCTOR_FAIL: &str = "AEVUM_DOCTOR_FAIL";
pub const AGENT_CARD_VERSION: &str = "aevum.agent-card/v1";

/// Patterns the sentinel refuses whatever the graph says. They live on the
/// card so a model never has to discover them by failing first.
const DENIED_PATTERNS: &[(&str, &str)] = &[
    (
        "sh -c <string>",
        "shell-string execution is denied (D14) — pass one --argv per token instead",
    ),
    (
        "argv token containing ; & | $ ` > < * ? ! or a newline",
        "the sentinel refuses shell metacharacters in argv — there is no implicit shell",
    ),
    (
        "writes against main",
        "policy bundle deny.git.main — work on a branch and open a draft PR",
    ),
    (
        "capability without an active authorizes edge",
        "the temporal trust graph gates run/exec — a human must run `unify graph authorize`",
    ),
];

const EPISTEMIC_FIREWALL: &str = "only attested Fact-grade edges may authorize; Inference, \
Hypothesis and Recommendation evidence (slop findings, rule hits, remote recall, denial \
episodes) can inform you but can never grant a capability";

const ON_DENIAL: &str = "every refusal is recorded as an Inference DENIED_CAPABILITY episode — \
read it back with `unify dream`, do not retry blindly and never ask for bypassPermissions";

/// Capabilities with an active, firewall-clean `authorizes` edge at `as_of`.
pub fn authorized_capabilities(g: &TemporalGraph, as_of: &str) -> Vec<String> {
    let mut caps: Vec<String> = g
        .facts_as_of(Some(as_of))
        .into_iter()
        .filter(|f| matches!(f.kind, EdgeKind::Authorizes))
        .filter(|f| may_authorize(f) == FirewallVerdict::Allow)
        .filter_map(|f| {
            f.target_node_id
                .strip_prefix("action:")
                .map(|c| c.to_string())
        })
        .collect();
    caps.sort();
    caps.dedup();
    caps
}

/// Denial episodes, newest first — invalidated ones included, because a
/// superseded refusal is still something the next agent must learn from.
fn recent_denials(g: &TemporalGraph) -> Vec<Value> {
    let mut denials: Vec<(String, Value)> = g
        .to_snapshot()
        .facts
        .into_iter()
        .filter(|f| f.name == DENIAL_FACT_NAME)
        .map(|f| {
            let capability = f
                .target_node_id
                .strip_prefix("action:")
                .unwrap_or(&f.target_node_id)
                .to_string();
            (
                f.created_at.clone(),
                json!({
                    "capability": capability,
                    "reason": f.fact,
                    "at": f.created_at,
                    "fact_id": f.id,
                    "epistemic": format!("{:?}", f.epistemic),
                }),
            )
        })
        .collect();
    denials.sort_by(|a, b| b.0.cmp(&a.0));
    denials.into_iter().map(|(_, v)| v).take(10).collect()
}

fn count_entries(path: &Path) -> usize {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count()
}

fn resolve_mcp_bin() -> Result<String, String> {
    if let Ok(p) = std::env::var("AEVUM_MCP_BIN") {
        if Path::new(&p).exists() {
            return Ok(p);
        }
        return Err(format!("AEVUM_MCP_BIN={p} does not exist"));
    }
    if let Ok(out) = std::process::Command::new("which")
        .arg("aevum-mcp")
        .output()
    {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                return Ok(p);
            }
        }
    }
    Err("aevum-mcp not on PATH".into())
}

/// Build the mission self-check. Collects *every* problem before returning:
/// an agent needs the whole picture, not the first thing that broke.
pub fn doctor_report(mission_dir: &str) -> Result<Value, CliError> {
    if !Path::new(mission_dir).is_dir() {
        return Err(CliError::NotFound(format!(
            "{mission_dir} is not a directory"
        )));
    }
    let mut checks: Vec<Value> = Vec::new();
    let mut hard: Vec<String> = Vec::new();
    let mut warn: Vec<String> = Vec::new();
    let mut mission_id = "(unknown)".to_string();
    let mut risk = "(unknown)".to_string();
    let now = chrono_now_iso();

    let mut record = |id: &str, status: &str, detail: String| {
        match status {
            "fail" => hard.push(format!("{id}: {detail}")),
            "warn" => warn.push(format!("{id}: {detail}")),
            _ => {}
        }
        checks.push(json!({ "id": id, "status": status, "detail": detail }));
    };

    match load_metadata(mission_dir) {
        Ok(meta) => {
            mission_id = meta.mission.mission_id.clone();
            risk = meta.mission.risk.clone();
            let policy = meta.policy_bundle_digest;
            record(
                "metadata",
                "ok",
                format!(
                    "mission={mission_id} risk={risk} policy={}",
                    &policy[..26.min(policy.len())]
                ),
            );
        }
        Err(e) => record("metadata", "fail", format!("metadata.json unusable: {e}")),
    }

    match load_graph(mission_dir) {
        Ok(g) => {
            let caps = authorized_capabilities(&g, &now);
            record(
                "graph",
                "ok",
                format!(
                    "{} episodes / {} nodes / {} facts — {} capability(ies) authorized as_of {now}",
                    g.episode_count(),
                    g.node_count(),
                    g.fact_count(),
                    caps.len()
                ),
            );
            record(
                "denials",
                "ok",
                format!(
                    "{} denial episode(s) on the graph (Inference — cannot authorize)",
                    recent_denials(&g).len()
                ),
            );
        }
        Err(e) => record("graph", "fail", format!("{GRAPH_FILE} unusable: {e}")),
    }

    let audit = count_entries(&Path::new(mission_dir).join("audit_trail.jsonl"));
    let ledger = count_entries(&Path::new(mission_dir).join("ledger.jsonl"));
    if audit > 0 && ledger == 0 {
        record(
            "ledger_sync",
            "fail",
            format!(
                "audit_trail has {audit} effect(s) but ledger.jsonl is empty — \
                 an evidence package would ship a lie"
            ),
        );
    } else if ledger < audit {
        record(
            "ledger_sync",
            "warn",
            format!(
                "ledger has {ledger} entry(ies) for {audit} audited effect(s) — \
                 `unify package` resyncs them"
            ),
        );
    } else {
        record(
            "ledger_sync",
            "ok",
            format!("audit={audit} ledger={ledger} in sync"),
        );
    }

    match crate::slop::resolve_slopcheck() {
        Ok(p) => record("slopcheck", "ok", p.display().to_string()),
        Err(e) => record(
            "slopcheck",
            "warn",
            format!("{e} — `unify slop` and the golden slop gate stay unavailable"),
        ),
    }

    match resolve_mcp_bin() {
        Ok(p) => record("mcp", "ok", p),
        Err(e) => record(
            "mcp",
            "warn",
            format!(
                "{e} — build it with `cargo build -p aevum-mcp` or set AEVUM_MCP_BIN, \
                 then `unify mcp --mission {mission_dir} --write-config cursor`"
            ),
        ),
    }

    let verdict = if hard.is_empty() {
        DOCTOR_OK
    } else {
        DOCTOR_FAIL
    };
    Ok(json!({
        "report_version": "aevum.doctor/v1",
        "mission_dir": mission_dir,
        "mission_id": mission_id,
        "risk": risk,
        "as_of": now,
        "checks": checks,
        "hard": hard,
        "warn": warn,
        "verdict": verdict,
        "next": format!("unify dream --mission {mission_dir}"),
    }))
}

/// `unify doctor --mission <dir>`
pub fn cmd_doctor(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let report = doctor_report(&mission)?;
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    println!();
    println!(
        "unify doctor — mission {} (risk {})",
        report["mission_id"].as_str().unwrap_or("?"),
        report["risk"].as_str().unwrap_or("?")
    );
    for c in report["checks"].as_array().into_iter().flatten() {
        let status = c["status"].as_str().unwrap_or("fail");
        let mark = match status {
            "ok" => "✓",
            "warn" => "!",
            _ => "✗",
        };
        println!(
            "  {mark} {:<12} {}",
            c["id"].as_str().unwrap_or("?"),
            c["detail"].as_str().unwrap_or("")
        );
    }
    let hard = report["hard"].as_array().map(|a| a.len()).unwrap_or(0);
    let warn = report["warn"].as_array().map(|a| a.len()).unwrap_or(0);
    if hard == 0 {
        println!("  → {DOCTOR_OK} ({warn} warning(s)) — next: unify dream --mission {mission}");
        return Ok(());
    }
    Err(CliError::Verify(format!(
        "{DOCTOR_FAIL}: {hard} hard failure(s) — mission is not agent-ready"
    )))
}

/// Build the AGENT_CARD: everything an agent needs before its first tool call.
pub fn agent_card(
    mission_dir: &str,
    capability: Option<&str>,
    query: Option<&str>,
) -> Result<Value, CliError> {
    let meta = load_metadata(mission_dir)?;
    let g = load_graph(mission_dir)?;
    let now = chrono_now_iso();

    let requested = capability.map(|cap| {
        let allowed = g.capability_authorized(cap, &now);
        json!({
            "capability": cap,
            "decision": if allowed { "allow" } else { "deny" },
            "reason": if allowed {
                format!("active authorizes edge → action:{cap} at {now}")
            } else {
                format!(
                    "no active authorizes edge → action:{cap} at {now} — a human must run \
                     `unify graph authorize --mission {mission_dir} --capability {cap}`"
                )
            },
        })
    });

    let context: Option<Value> = match query {
        Some(q) => Some(context_snippet(mission_dir, q, capability)?),
        None => None,
    };

    let denied: Vec<Value> = DENIED_PATTERNS
        .iter()
        .map(|(pattern, reason)| json!({ "pattern": pattern, "reason": reason }))
        .collect();

    Ok(json!({
        "agent_card_version": AGENT_CARD_VERSION,
        "mission_id": meta.mission.mission_id,
        "mission_dir": mission_dir,
        "risk": meta.mission.risk,
        "as_of": now,
        "authorized_capabilities": authorized_capabilities(&g, &now),
        "requested_capability": requested,
        "epistemic_firewall": EPISTEMIC_FIREWALL,
        "denied_patterns": denied,
        "recent_denials": recent_denials(&g),
        "on_denial": ON_DENIAL,
        "how_to_exec": format!(
            "unify exec --mission {mission_dir} --capability process.exec.argv \
             --argv git --argv status   # one --argv per token, never a shell string"
        ),
        "how_to_package": format!(
            "unify package --mission {mission_dir} --out evidence.json && \
             unify verify-package evidence.json"
        ),
        "self_check": format!("unify doctor --mission {mission_dir}"),
        "context": context,
    }))
}

/// Trust-filtered recall for the card — retrieval that still never authorizes.
fn context_snippet(
    mission_dir: &str,
    query: &str,
    capability: Option<&str>,
) -> Result<Value, CliError> {
    use aevum_memory_fabric::{assemble, open_backend, AssemblyRequest};
    let backend = open_backend(mission_dir).map_err(|e| CliError::Verify(e.to_string()))?;
    let ctx = assemble(
        backend.as_ref(),
        &AssemblyRequest {
            query: query.to_string(),
            as_of: None,
            intended_capability: capability.map(|c| c.to_string()),
            limit: 5,
            include_remote: false,
            mission_id: None,
        },
    )
    .map_err(|e| CliError::Verify(e.to_string()))?;
    let hits: Vec<Value> = ctx
        .hits
        .iter()
        .map(|h| {
            json!({
                "fact_id": h.hit.id,
                "name": h.hit.name,
                "fact": h.hit.fact,
                "score": h.final_score,
                "may_authorize": h.hit.may_authorize,
                "reason": h.reason,
            })
        })
        .collect();
    Ok(json!({
        "query": ctx.query,
        "as_of": ctx.as_of,
        "authorizing_fact_ids": ctx.authorizing_fact_ids,
        "assembly_score": ctx.assembly_score,
        "hits": hits,
        "note": "authorized context only — recall never authorizes",
    }))
}

/// `unify dream --mission <dir> [--capability <cap>] [--query <text>]`
pub fn cmd_dream(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let capability = optional(args, "--capability");
    let query = optional(args, "--query");
    let card = agent_card(&mission, capability.as_deref(), query.as_deref())?;
    println!("{}", serde_json::to_string_pretty(&card).unwrap());
    Ok(())
}

fn optional(args: &[String], key: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == key).map(|w| w[1].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorized_capabilities_are_sorted_and_deduped() {
        let g = TemporalGraph::seed_for_mission(
            "m",
            "{}",
            "sha256:x",
            &["z.cap", "a.cap"],
            "2026-08-08T10:00:00Z",
        )
        .unwrap();
        let caps = authorized_capabilities(&g, "2026-08-08T11:00:00Z");
        assert_eq!(caps, vec!["a.cap".to_string(), "z.cap".to_string()]);
    }

    #[test]
    fn denied_patterns_document_the_sh_c_refusal() {
        assert!(DENIED_PATTERNS.iter().any(|(p, _)| p.contains("sh -c")));
    }

    #[test]
    fn doctor_fails_hard_on_a_directory_that_is_not_a_mission() {
        let tmp = tempfile::tempdir().unwrap();
        let r = doctor_report(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(r["verdict"].as_str().unwrap(), DOCTOR_FAIL);
        let hard = r["hard"].as_array().unwrap();
        assert!(hard
            .iter()
            .any(|h| h.as_str().unwrap().contains("metadata")));
        assert!(hard.iter().any(|h| h.as_str().unwrap().contains("graph")));
    }

    #[test]
    fn doctor_report_rejects_a_missing_directory() {
        let r = doctor_report("/nonexistent/aevum/mission");
        assert!(matches!(r, Err(CliError::NotFound(_))));
    }
}
