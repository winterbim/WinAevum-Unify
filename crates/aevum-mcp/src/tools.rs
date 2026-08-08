//! MCP tool surface — every mutating tool hits the real unify / memory-fabric path.

use std::path::PathBuf;
use std::sync::Mutex;

use aevum_memory_fabric::{
    assemble, ingest_remote_as_inference, open_backend, promote_to_authorize, AssemblyRequest,
    MemoryBackend, NativeBackend,
};
use serde_json::{json, Value};

pub struct ToolCtx {
    pub mission_dir: PathBuf,
    lock: Mutex<()>,
}

impl ToolCtx {
    pub fn new(mission_dir: PathBuf) -> Self {
        Self {
            mission_dir,
            lock: Mutex::new(()),
        }
    }
}

fn tool_defs() -> Vec<Value> {
    vec![
        tool(
            "aevum_graph_status",
            "Show temporal graph status and baseline capability authorizations",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "aevum_graph_search",
            "Hybrid search over the native trust graph",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "as_of": { "type": "string", "description": "ISO-8601 event time" },
                    "limit": { "type": "integer", "default": 10 }
                },
                "required": ["query"]
            }),
        ),
        tool(
            "aevum_context_assemble",
            "Trust-filtered context assembly (retrieval ∩ epistemic ∩ capability).",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "capability": { "type": "string" },
                    "as_of": { "type": "string" },
                    "include_remote": { "type": "boolean", "default": false },
                    "limit": { "type": "integer", "default": 10 }
                },
                "required": ["query"]
            }),
        ),
        tool(
            "aevum_graph_authorize",
            "Attest and authorize a capability in the temporal graph",
            json!({
                "type": "object",
                "properties": {
                    "capability": { "type": "string" },
                    "reason": { "type": "string" }
                },
                "required": ["capability"]
            }),
        ),
        tool(
            "aevum_memory_ingest_remote",
            "Ingest untrusted remote facts as Inference only (cannot authorize until promote)",
            json!({
                "type": "object",
                "properties": {
                    "facts": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "uuid": { "type": "string" },
                                "fact": { "type": "string" },
                                "name": { "type": "string" }
                            },
                            "required": ["uuid", "fact"]
                        }
                    }
                },
                "required": ["facts"]
            }),
        ),
        tool(
            "aevum_memory_promote",
            "Promote a remote inference to an authorizing fact with attested content",
            json!({
                "type": "object",
                "properties": {
                    "remote_fact_id": { "type": "string" },
                    "capability": { "type": "string" },
                    "attested_content": { "type": "string" }
                },
                "required": ["remote_fact_id", "capability", "attested_content"]
            }),
        ),
        tool(
            "aevum_run",
            "Sign an action attestation for a capability (gated by temporal authorizes)",
            json!({
                "type": "object",
                "properties": {
                    "capability": { "type": "string" },
                    "argv": { "type": "string" }
                },
                "required": ["capability", "argv"]
            }),
        ),
        tool(
            "aevum_exec",
            "Execute typed argv under sentinel + temporal authorization (refuses sh -c)",
            json!({
                "type": "object",
                "properties": {
                    "capability": { "type": "string" },
                    "argv": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "argv tokens, NOT a shell string"
                    }
                },
                "required": ["capability", "argv"]
            }),
        ),
        tool(
            "aevum_verify",
            "Verify mission trust ledger chain",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "aevum_slop_scan",
            "Run offline AI-slop firewall (slopcheck) and ingest findings as Inference only — never authorizes",
            json!({
                "type": "object",
                "properties": {
                    "repo": { "type": "string", "description": "repo path to scan (default .)" },
                    "all": { "type": "boolean", "default": true },
                    "base": { "type": "string", "description": "optional git base ref" },
                    "warn_only": { "type": "boolean", "default": false }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "aevum_package",
            "Build evidence package (ledger + audit + slop + graph digests)",
            json!({
                "type": "object",
                "properties": {
                    "out": { "type": "string", "description": "output package json path" }
                },
                "required": ["out"]
            }),
        ),
        tool(
            "aevum_verify_package",
            "Verify evidence package digest integrity",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "package json path" }
                },
                "required": ["path"]
            }),
        ),
        tool(
            "aevum_golden",
            "Golden Path: side-branch + optional tests + package + PR draft (never merges)",
            json!({
                "type": "object",
                "properties": {
                    "repo": { "type": "string" },
                    "title": { "type": "string" },
                    "run_tests": { "type": "boolean", "default": false },
                    "no_slop_gate": { "type": "boolean", "default": false }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "aevum_falsify",
            "Record falsifier challenge (required before R3+ effects)",
            json!({
                "type": "object",
                "properties": {
                    "reason": { "type": "string" }
                },
                "required": ["reason"]
            }),
        ),
        tool(
            "aevum_rule_scan",
            "Scan mission/rules (hookify-style) and ingest matches as Inference only",
            json!({
                "type": "object",
                "properties": {
                    "repo": { "type": "string", "default": "." }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "aevum_doctor",
            "Mission self-check (`unify doctor`): hard failures, soft warnings, verdict — never silent",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "aevum_agent_card",
            "AGENT_CARD (`unify dream`): authorized capabilities, denied patterns, how to exec/package safely",
            json!({
                "type": "object",
                "properties": {
                    "capability": { "type": "string", "description": "narrow the card to one capability" },
                    "query": { "type": "string", "description": "attach a trust-filtered context probe" }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "aevum_pretool_check",
            "PreToolUse bridge: check whether a capability is authorized right now",
            json!({
                "type": "object",
                "properties": {
                    "capability": { "type": "string" },
                    "tool_name": { "type": "string" }
                },
                "required": ["capability"]
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

pub fn tools_list_json() -> Value {
    json!({ "tools": tool_defs() })
}

use crate::silence::quiet;

pub fn dispatch(ctx: &ToolCtx, name: &str, args: &Value) -> Result<String, String> {
    let _guard = ctx.lock.lock().map_err(|e| e.to_string())?;
    let mission = ctx
        .mission_dir
        .to_str()
        .ok_or_else(|| "mission path not utf-8".to_string())?;
    match name {
        "aevum_graph_status" => {
            let b = NativeBackend::open(&ctx.mission_dir).map_err(|e| e.to_string())?;
            let now = "2099-01-01T00:00:00Z";
            Ok(serde_json::to_string_pretty(&json!({
                "backend": b.name(),
                "episodes": b.graph().episode_count(),
                "nodes": b.graph().node_count(),
                "facts": b.graph().fact_count(),
                "git.branch.create": b.graph().capability_authorized("git.branch.create", now),
                "process.exec.argv": b.graph().capability_authorized("process.exec.argv", now),
            }))
            .unwrap())
        }
        "aevum_graph_search" => {
            let q = arg_str(args, "query")?;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            let as_of = args.get("as_of").and_then(|v| v.as_str());
            let b = NativeBackend::open(&ctx.mission_dir).map_err(|e| e.to_string())?;
            let hits = b.search(&q, as_of, limit).map_err(|e| e.to_string())?;
            Ok(serde_json::to_string_pretty(&hits).unwrap())
        }
        "aevum_context_assemble" => {
            let q = arg_str(args, "query")?;
            let cap = args
                .get("capability")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let include_remote = args
                .get("include_remote")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            let as_of = args
                .get("as_of")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let backend = open_backend(&ctx.mission_dir).map_err(|e| e.to_string())?;
            let ctx_out = assemble(
                backend.as_ref(),
                &AssemblyRequest {
                    query: q,
                    as_of,
                    intended_capability: cap,
                    limit,
                    include_remote,
                    mission_id: None,
                },
            )
            .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string_pretty(&ctx_out).unwrap())
        }
        "aevum_graph_authorize" => {
            let cap = arg_str(args, "capability")?;
            let reason = args
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("mcp authorize")
                .to_string();
            quiet(|| {
                aevum_unify::graph_cmd::cmd_graph(&[
                    "authorize".into(),
                    "--mission".into(),
                    mission.into(),
                    "--capability".into(),
                    cap.clone(),
                    "--reason".into(),
                    reason,
                ])
            })
            .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "capability": cap}).to_string())
        }
        "aevum_memory_ingest_remote" => {
            let facts = args
                .get("facts")
                .and_then(|v| v.as_array())
                .ok_or_else(|| "facts array required".to_string())?;
            let mut remotes = Vec::new();
            for f in facts {
                remotes.push(aevum_memory_fabric::RemoteFact {
                    uuid: f
                        .get("uuid")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "fact.uuid required".to_string())?
                        .to_string(),
                    fact: f
                        .get("fact")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "fact.fact required".to_string())?
                        .to_string(),
                    name: f
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    valid_at: None,
                    invalid_at: None,
                    group_id: None,
                });
            }
            let mut backend = open_backend(&ctx.mission_dir).map_err(|e| e.to_string())?;
            let meta = aevum_unify::load_metadata(mission).map_err(|e| e.to_string())?;
            let ids =
                ingest_remote_as_inference(backend.graph_mut(), &meta.mission.mission_id, &remotes)
                    .map_err(|e| e.to_string())?;
            backend.save().map_err(|e| e.to_string())?;
            Ok(serde_json::to_string_pretty(&json!({
                "ingested": ids.len(),
                "fact_ids": ids,
                "note": "ingested as Inference — cannot authorize until aevum_memory_promote"
            }))
            .unwrap())
        }
        "aevum_memory_promote" => {
            let fact_id = arg_str(args, "remote_fact_id")?;
            let cap = arg_str(args, "capability")?;
            let content = arg_str(args, "attested_content")?;
            let mut b = NativeBackend::open(&ctx.mission_dir).map_err(|e| e.to_string())?;
            let meta = aevum_unify::load_metadata(mission).map_err(|e| e.to_string())?;
            let id = promote_to_authorize(
                b.graph_mut(),
                &meta.mission.mission_id,
                &fact_id,
                &cap,
                &content,
            )
            .map_err(|e| e.to_string())?;
            b.save().map_err(|e| e.to_string())?;
            Ok(
                serde_json::to_string_pretty(&json!({ "authorized_fact": id, "capability": cap }))
                    .unwrap(),
            )
        }
        "aevum_run" => {
            let cap = arg_str(args, "capability")?;
            let argv = arg_str(args, "argv")?;
            quiet(|| {
                aevum_unify::cmd_run(&[
                    "--mission".into(),
                    mission.into(),
                    "--capability".into(),
                    cap.clone(),
                    "--argv".into(),
                    argv,
                ])
            })
            .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "capability": cap}).to_string())
        }
        "aevum_exec" => {
            let cap = arg_str(args, "capability")?;
            let argv = args
                .get("argv")
                .and_then(|v| v.as_array())
                .ok_or_else(|| "argv must be string array".to_string())?;
            let mut cmd = vec![
                "--mission".into(),
                mission.to_string(),
                "--capability".into(),
                cap.clone(),
            ];
            for a in argv {
                cmd.push("--argv".into());
                cmd.push(
                    a.as_str()
                        .ok_or_else(|| "argv entries must be strings".to_string())?
                        .to_string(),
                );
            }
            quiet(|| aevum_unify::cmd_exec(&cmd)).map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "capability": cap}).to_string())
        }
        "aevum_verify" => {
            quiet(|| aevum_unify::cmd_verify(&[mission.to_string()])).map_err(|e| e.to_string())?;
            Ok(json!({"ok": true}).to_string())
        }
        "aevum_slop_scan" => {
            let repo = args
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or(".")
                .to_string();
            let mut cmd = vec![
                "--mission".into(),
                mission.to_string(),
                "--repo".into(),
                repo,
            ];
            if args
                .get("warn_only")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                cmd.push("--warn-only".into());
            }
            if let Some(base) = args.get("base").and_then(|v| v.as_str()) {
                cmd.push("--base".into());
                cmd.push(base.to_string());
            } else if args.get("all").and_then(|v| v.as_bool()).unwrap_or(true) {
                cmd.push("--all".into());
            }
            quiet(|| aevum_unify::slop::cmd_slop(&cmd)).map_err(|e| e.to_string())?;
            let report_path = ctx.mission_dir.join("slop-report.json");
            let body = std::fs::read_to_string(&report_path).unwrap_or_else(|_| "{}".into());
            Ok(body)
        }
        "aevum_package" => {
            let out = arg_str(args, "out")?;
            quiet(|| {
                aevum_unify::cmd_package(&[
                    "--mission".into(),
                    mission.to_string(),
                    "--out".into(),
                    out.clone(),
                ])
            })
            .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "out": out}).to_string())
        }
        "aevum_verify_package" => {
            let path = arg_str(args, "path")?;
            let argv = vec![path.clone()];
            quiet(|| aevum_unify::cmd_verify_package(&argv)).map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "path": path}).to_string())
        }
        "aevum_golden" => {
            let mut cmd = vec!["--mission".into(), mission.to_string()];
            if let Some(repo) = args.get("repo").and_then(|v| v.as_str()) {
                cmd.push("--repo".into());
                cmd.push(repo.to_string());
            }
            if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
                cmd.push("--title".into());
                cmd.push(title.to_string());
            }
            if args
                .get("run_tests")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                cmd.push("--run-tests".into());
            }
            if args
                .get("no_slop_gate")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                cmd.push("--no-slop-gate".into());
            }
            quiet(|| aevum_unify::golden::cmd_golden(&cmd)).map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "auto_merge": false}).to_string())
        }
        "aevum_falsify" => {
            let reason = arg_str(args, "reason")?;
            quiet(|| {
                aevum_unify::graph_cmd::cmd_falsify(&[
                    "--mission".into(),
                    mission.to_string(),
                    "--reason".into(),
                    reason.clone(),
                ])
            })
            .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "reason": reason}).to_string())
        }
        "aevum_rule_scan" => {
            let repo = args
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or(".")
                .to_string();
            quiet(|| {
                aevum_unify::rules::cmd_rules_scan(&[
                    "--mission".into(),
                    mission.to_string(),
                    "--repo".into(),
                    repo,
                ])
            })
            .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "note": "rule hits ingested as Inference only"}).to_string())
        }
        "aevum_doctor" => match aevum_unify::dream::doctor_report(mission) {
            Ok(report) => {
                let body = serde_json::to_string_pretty(&report).unwrap();
                let healthy = report
                    .get("hard")
                    .and_then(|v| v.as_array())
                    .is_some_and(|a| a.is_empty());
                // A sick mission must reach the agent as an error, not as a
                // successful call whose body happens to say FAIL.
                if healthy {
                    Ok(body)
                } else {
                    Err(body)
                }
            }
            Err(e) => Err(e.to_string()),
        },
        "aevum_agent_card" => {
            let cap = args.get("capability").and_then(|v| v.as_str());
            let query = args.get("query").and_then(|v| v.as_str());
            match aevum_unify::dream::agent_card(mission, cap, query) {
                Ok(card) => Ok(serde_json::to_string_pretty(&card).unwrap()),
                Err(e) => Err(e.to_string()),
            }
        }
        "aevum_pretool_check" => {
            let cap = arg_str(args, "capability")?;
            let tool_name = args.get("tool_name").and_then(|v| v.as_str()).unwrap_or("");
            // D14: Bash with sh -c style is never allowed
            if tool_name.eq_ignore_ascii_case("Bash") && cap.contains("shell") {
                return Ok(json!({
                    "decision": "deny",
                    "reason": "shell-string tools denied (D14) — use process.exec.argv"
                })
                .to_string());
            }
            match aevum_unify::graph_cmd::require_authorized(mission, &cap) {
                Ok(()) => Ok(json!({
                    "decision": "allow",
                    "capability": cap,
                    "tool_name": tool_name
                })
                .to_string()),
                Err(e) => Ok(json!({
                    "decision": "deny",
                    "capability": cap,
                    "tool_name": tool_name,
                    "reason": e.to_string()
                })
                .to_string()),
            }
        }
        other => Err(format!("unknown tool: {other}")),
    }
}

fn arg_str(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing string arg: {key}"))
}

/// Patch protocol tools/list to use dynamic defs (TOOLS static is placeholder).
pub fn list_tools_value() -> Value {
    tools_list_json()
}
