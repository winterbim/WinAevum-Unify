//! Temporal graph CLI + trust-path gate (ADR-0013).
//!
//! Differentiator: the graph is not just memory —
//! `run` / `exec` refuse capabilities without an active `authorizes` fact.

mod gate;
mod io;

pub use gate::{record_denial_episode, require_authorized, require_falsifier_if_needed};
pub use io::{graph_path, load_graph, save_graph, seed_and_persist, GRAPH_FILE};

use std::fs;
use std::path::Path;

use aevum_evidence_graph::{
    hybrid_search, EdgeKind, Episode, EpisodeSource, EpistemicKind, Fact, FirewallVerdict,
    GraphNode, NodeKind, SearchRecipe,
};

use crate::{chrono_now_iso, optional_value, require_value, sha256_hex, CliError};

/// Record an independent falsifier objection (required before R3+ effects).
pub fn cmd_falsify(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let reason = require_value(args, "--reason")?;
    let target = optional_value(args, "--target").unwrap_or_else(|| "mission".into());
    let by = optional_value(args, "--by")
        .unwrap_or_else(|| "spiffe://local.aevum/role/falsifier".into());
    let path = Path::new(&mission).join("falsifier.jsonl");
    let record = serde_json::json!({
        "ts": chrono_now_iso(),
        "role": "falsifier",
        "by": by,
        "target": target,
        "reason": reason,
        "status": "raised"
    });
    let mut text = fs::read_to_string(&path).unwrap_or_default();
    text.push_str(&serde_json::to_string(&record).unwrap());
    text.push('\n');
    fs::write(&path, text).map_err(|e| CliError::Io(e.to_string()))?;
    println!("✓ falsifier challenge recorded → {}", path.display());
    Ok(())
}

/// Human approval line for R3+ golden path.
pub fn cmd_approve(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let decision = optional_value(args, "--decision").unwrap_or_else(|| "approved".into());
    let by = optional_value(args, "--by").unwrap_or_else(|| "human:operator".into());
    let path = Path::new(&mission).join("approvals.jsonl");
    let record = serde_json::json!({
        "ts": chrono_now_iso(),
        "decision": decision,
        "by": by,
        "scope": "mission"
    });
    let mut text = fs::read_to_string(&path).unwrap_or_default();
    text.push_str(&serde_json::to_string(&record).unwrap());
    text.push('\n');
    fs::write(&path, text).map_err(|e| CliError::Io(e.to_string()))?;
    println!("✓ approval recorded → {}", path.display());
    Ok(())
}

pub fn cmd_graph(args: &[String]) -> Result<(), CliError> {
    let sub = args.first().ok_or_else(|| {
        CliError::Missing(
            "graph <status|search|as-of|authorize|add-episode|ingest|contradictions|tenants>"
                .into(),
        )
    })?;
    match sub.as_str() {
        "status" => graph_status(&args[1..]),
        "search" => graph_search(&args[1..]),
        "as-of" => graph_as_of(&args[1..]),
        "authorize" => graph_authorize(&args[1..]),
        "add-episode" => graph_add_episode(&args[1..]),
        "ingest" => graph_ingest(&args[1..]),
        "contradictions" => graph_contradictions(&args[1..]),
        "tenants" => graph_tenants(&args[1..]),
        "help" | "--help" | "-h" => {
            print_graph_help();
            Ok(())
        }
        other => Err(CliError::BadArgs(format!(
            "unknown graph subcommand: {other}"
        ))),
    }
}

fn print_graph_help() {
    println!("unify graph — temporal Decision & Evidence Graph\n");
    println!("  unify graph status         --mission <dir>");
    println!("  unify graph search         --mission <dir> --query <text> [--as-of <iso>]");
    println!("  unify graph as-of          --mission <dir> --at <iso>");
    println!("  unify graph authorize      --mission <dir> --capability <name> --grant-sig <hex> [--reason <text>]");
    println!("                             # P0-5: requires human grant (unify human-grant); self-authorize refused");
    println!("  unify graph add-episode    --mission <dir> --content <text|@file> [--source attested|text|json]");
    println!("  unify graph ingest         --mission <dir> --content <text|@file> [--at <iso>] [--format json|text] [--attested]");
    println!("  unify graph contradictions --mission <dir> [--as-of <iso>] [--resolve]");
    println!("  unify graph tenants        [--root <AEVUM_MEMORY_ROOT>] [--tenant <id>]");
    println!("                             (shared multi-mission store; set AEVUM_MEMORY_ROOT)");
}

fn graph_tenants(args: &[String]) -> Result<(), CliError> {
    use aevum_memory_fabric::{MultiTenantStore, TenantScope};
    let root = optional_value(args, "--root")
        .or_else(|| std::env::var("AEVUM_MEMORY_ROOT").ok())
        .ok_or_else(|| {
            CliError::Missing("AEVUM_MEMORY_ROOT or --root required for multi-tenant store".into())
        })?;
    let store = MultiTenantStore::open(&root).map_err(|e| CliError::Verify(e.to_string()))?;

    // Optional: sync current mission into the store
    if let Some(mission) = optional_value(args, "--sync-mission") {
        let scope = TenantScope::from_mission_dir(&mission);
        let g = load_graph(&mission)?;
        store
            .put_graph(&scope, &g)
            .map_err(|e| CliError::Verify(e.to_string()))?;
        println!(
            "✓ synced {} / {} → {}",
            scope.tenant_id,
            scope.mission_id,
            store.path().display()
        );
    }

    let tenant_filter = optional_value(args, "--tenant");
    let list = if let Some(t) = tenant_filter {
        store
            .list(&t)
            .map_err(|e| CliError::Verify(e.to_string()))?
    } else {
        store
            .list_all()
            .map_err(|e| CliError::Verify(e.to_string()))?
    };
    println!(
        "✓ multi-tenant store — {} ({} mission(s))",
        store.path().display(),
        list.len()
    );
    for s in list {
        println!(
            "  tenant={} mission={} group={}",
            s.tenant_id,
            s.mission_id,
            s.group_id()
        );
    }
    Ok(())
}

fn graph_status(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let g = load_graph(&mission)?;
    let now = chrono_now_iso();
    let active = g.facts_as_of(Some(&now)).len();
    println!("✓ temporal graph — {}", graph_path(&mission).display());
    println!("  episodes: {}", g.episode_count());
    println!("  nodes:    {}", g.node_count());
    println!("  facts:    {} (active now: {active})", g.fact_count());
    println!("  events:   {}", g.event_log().len());
    for cap in ["git.branch.create", "process.exec.argv"] {
        let ok = g.capability_authorized(cap, &now);
        println!("  auth {cap}: {}", if ok { "ALLOW" } else { "DENY" });
    }
    Ok(())
}

fn graph_search(args: &[String]) -> Result<(), CliError> {
    use aevum_memory_fabric::MemoryBackend;
    let mission = require_value(args, "--mission")?;
    let query = require_value(args, "--query")?;
    let as_of = optional_value(args, "--as-of").unwrap_or_else(chrono_now_iso);
    // Prefer memory-fabric backend (sqlite + semantic hybrid) when available.
    if let Ok(mut backend) = aevum_memory_fabric::open_backend(&mission) {
        let _ = aevum_memory_fabric::ensure_node_embeddings(
            backend.graph_mut(),
            aevum_memory_fabric::default_embedder().as_ref(),
        );
        let hits = MemoryBackend::search(backend.as_ref(), &query, Some(&as_of), 10)
            .map_err(|e| CliError::Verify(e.to_string()))?;
        println!(
            "✓ search — {} hit(s) as_of={as_of} backend={}",
            hits.len(),
            MemoryBackend::name(backend.as_ref())
        );
        for h in hits {
            println!("  {:.3}  {}  {}", h.score, h.fact_id, h.name);
        }
        return Ok(());
    }
    let g = load_graph(&mission)?;
    let recipe = SearchRecipe::new(query).as_of(as_of.clone()).limit(10);
    let hits = hybrid_search(&g, &recipe);
    println!("✓ search — {} hit(s) as_of={as_of}", hits.len());
    for h in hits {
        println!("  [{:.3}] {} — {}", h.score, h.fact_id, h.fact);
    }
    Ok(())
}

fn graph_as_of(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let at = require_value(args, "--at")?;
    let g = load_graph(&mission)?;
    let facts = g.facts_as_of(Some(&at));
    println!("✓ as-of {at} — {} active fact(s)", facts.len());
    for f in facts {
        println!(
            "  {} | {} | {} → {} | {}",
            f.id, f.name, f.source_node_id, f.target_node_id, f.fact
        );
    }
    Ok(())
}

fn graph_authorize(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let capability = require_value(args, "--capability")?;
    let reason = optional_value(args, "--reason")
        .unwrap_or_else(|| format!("explicit authorize for {capability}"));
    let grant_sig = require_value(args, "--grant-sig").map_err(|_| {
        CliError::Verify(
            "graph authorize refuses self-authorize (P0-5): pass --grant-sig from \
             `unify human-grant --mission-id … --capability …` signed by the human key \
             outside the mission directory ($AEVUM_HUMAN_KEY / ~/.config/aevum/human.sk)"
                .into(),
        )
    })?;
    let meta_txt = fs::read_to_string(Path::new(&mission).join("metadata.json"))
        .map_err(|e| CliError::NotFound(format!("metadata: {e}")))?;
    let meta: serde_json::Value = serde_json::from_str(&meta_txt)
        .map_err(|e| CliError::BadArgs(format!("metadata json: {e}")))?;
    let mission_id = meta
        .pointer("/mission/mission_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    crate::authority::verify_human_grant(&mission_id, &capability, &reason, &grant_sig)?;
    let now = chrono_now_iso();
    let mut g = load_graph(&mission)?;

    let ep_id = format!("ep_auth_{}", crate::ulid_like());
    let content = format!(
        "{{\"capability\":\"{capability}\",\"reason\":{},\"grant\":\"human\"}}",
        serde_json::to_string(&reason).unwrap()
    );
    let digest = sha256_hex(&content);
    g.add_episode(Episode {
        id: ep_id.clone(),
        mission_id: mission_id.clone(),
        group_id: format!("mission:{mission_id}"),
        source: EpisodeSource::Attested,
        content,
        content_digest: Some(digest.clone()),
        valid_at: now.clone(),
        created_at: now.clone(),
        actor_id: Some("spiffe://local.aevum/human/operator".into()),
    })
    .map_err(|e| CliError::Verify(format!("episode: {e}")))?;

    let action_id = format!("action:{capability}");
    if g.node(&action_id).is_none() {
        g.upsert_node(GraphNode {
            id: action_id.clone(),
            kind: NodeKind::ActionIntent,
            name: capability.clone(),
            summary: reason.clone(),
            mission_id: mission_id.clone(),
            group_id: format!("mission:{mission_id}"),
            created_at: now.clone(),
            embedding: None,
        });
    }
    if g.node("claim:constitution").is_none() {
        return Err(CliError::Verify(
            "claim:constitution missing — corrupt graph".into(),
        ));
    }

    // Invalidate prior authorizes for this action (supersede).
    let prior: Vec<String> = g
        .facts_as_of(Some(&now))
        .into_iter()
        .filter(|f| f.target_node_id == action_id && matches!(f.kind, EdgeKind::Authorizes))
        .map(|f| f.id.clone())
        .collect();
    for id in prior {
        let _ = g.invalidate_fact(&id, &now, &now);
    }

    let fact = Fact {
        id: format!("fact:auth:{}:{}", capability, crate::ulid_like()),
        kind: EdgeKind::Authorizes,
        source_node_id: "claim:constitution".into(),
        target_node_id: action_id,
        name: "AUTHORIZES".into(),
        fact: reason,
        epistemic: EpistemicKind::Fact,
        episode_ids: vec![ep_id],
        valid_at: now.clone(),
        invalid_at: None,
        created_at: now.clone(),
        expired_at: None,
        fact_digest: Some(digest),
        group_id: format!("mission:{mission_id}"),
        mission_id,
    };
    match aevum_evidence_graph::may_authorize(&fact) {
        FirewallVerdict::Allow => {}
        FirewallVerdict::Deny(r) => return Err(CliError::Verify(r.into())),
    }
    g.assert_fact(fact)
        .map_err(|e| CliError::Verify(format!("assert: {e}")))?;
    save_graph(&mission, &g)?;
    println!("✓ authorized capability `{capability}` in temporal graph");
    Ok(())
}

fn graph_add_episode(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let content_arg = require_value(args, "--content")?;
    let source_s = optional_value(args, "--source").unwrap_or_else(|| "text".into());
    let content = if let Some(path) = content_arg.strip_prefix('@') {
        fs::read_to_string(path).map_err(|e| CliError::Io(format!("read content file: {e}")))?
    } else {
        content_arg
    };
    let source = match source_s.as_str() {
        "attested" => EpisodeSource::Attested,
        "json" => EpisodeSource::Json,
        "message" => EpisodeSource::Message,
        "fact_triple" => EpisodeSource::FactTriple,
        _ => EpisodeSource::Text,
    };
    let meta_txt = fs::read_to_string(Path::new(&mission).join("metadata.json"))
        .map_err(|e| CliError::NotFound(format!("metadata: {e}")))?;
    let meta: serde_json::Value = serde_json::from_str(&meta_txt)
        .map_err(|e| CliError::BadArgs(format!("metadata json: {e}")))?;
    let mission_id = meta
        .pointer("/mission/mission_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let now = chrono_now_iso();
    let mut g = load_graph(&mission)?;
    let id = format!("ep_{}", crate::ulid_like());
    let digest = if matches!(source, EpisodeSource::Attested) {
        Some(sha256_hex(&content))
    } else {
        None
    };
    g.add_episode(Episode {
        id: id.clone(),
        mission_id: mission_id.clone(),
        group_id: format!("mission:{mission_id}"),
        source,
        content,
        content_digest: digest,
        valid_at: now.clone(),
        created_at: now,
        actor_id: Some("spiffe://local.aevum/agent/graph-cli".into()),
    })
    .map_err(|e| CliError::Verify(format!("episode: {e}")))?;
    save_graph(&mission, &g)?;
    println!("✓ episode {id} added");
    Ok(())
}

fn graph_ingest(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let content_arg = require_value(args, "--content")?;
    let format = optional_value(args, "--format").unwrap_or_else(|| "json".into());
    let attested = args.iter().any(|a| a == "--attested");
    let at = optional_value(args, "--at").unwrap_or_else(chrono_now_iso);
    let content = if let Some(path) = content_arg.strip_prefix('@') {
        fs::read_to_string(path).map_err(|e| CliError::Io(format!("read content file: {e}")))?
    } else {
        content_arg
    };
    let meta_txt = fs::read_to_string(Path::new(&mission).join("metadata.json"))
        .map_err(|e| CliError::NotFound(format!("metadata: {e}")))?;
    let meta: serde_json::Value = serde_json::from_str(&meta_txt)
        .map_err(|e| CliError::BadArgs(format!("metadata json: {e}")))?;
    let mission_id = meta
        .pointer("/mission/mission_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let mut g = load_graph(&mission)?;
    let report = match format.as_str() {
        "text" => aevum_memory_fabric::ingest_text_triples(&mut g, &mission_id, &at, &content)
            .map_err(|e| CliError::Verify(e.to_string()))?,
        _ => aevum_memory_fabric::ingest_structured_json(
            &mut g,
            &mission_id,
            &at,
            &content,
            attested,
        )
        .map_err(|e| CliError::Verify(e.to_string()))?,
    };
    for f in g.facts_as_of(Some(&at)) {
        if f.episode_ids.iter().any(|e| e == &report.episode_id) && f.valid_at != at {
            return Err(CliError::Verify(format!(
                "ingest integrity: fact {} valid_at={} != reference_time {at}",
                f.id, f.valid_at
            )));
        }
    }
    save_graph(&mission, &g)?;
    println!(
        "✓ ingest episode={} nodes={} facts={} reference_time={at}",
        report.episode_id, report.nodes_upserted, report.facts_asserted
    );
    Ok(())
}

fn graph_contradictions(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let as_of = optional_value(args, "--as-of").unwrap_or_else(chrono_now_iso);
    let resolve = args.iter().any(|a| a == "--resolve");
    let mut g = load_graph(&mission)?;
    let found = aevum_evidence_graph::detect_contradictions(&g, &as_of);
    println!("✓ contradictions as_of={as_of}: {}", found.len());
    for c in &found {
        println!("  {} ↔ {} — {}", c.left_fact_id, c.right_fact_id, c.reason);
    }
    if resolve {
        let n = aevum_evidence_graph::resolve_parallel_conflicts(&mut g, &as_of)
            .map_err(|e| CliError::Verify(e.to_string()))?;
        save_graph(&mission, &g)?;
        println!("✓ resolved {n} parallel conflict(s) (older invalidated)");
    }
    Ok(())
}

/// Trust-filtered context assembly (raw recall never authorizes).
pub fn cmd_context(args: &[String]) -> Result<(), CliError> {
    use aevum_memory_fabric::{assemble, open_backend, AssemblyRequest};
    let mission = require_value(args, "--mission")?;
    let query = require_value(args, "--query")?;
    let capability = optional_value(args, "--capability");
    let include_remote = args.iter().any(|a| a == "--include-remote");
    let as_of = optional_value(args, "--as-of");
    let backend = open_backend(&mission).map_err(|e| CliError::Verify(e.to_string()))?;
    let ctx = assemble(
        backend.as_ref(),
        &AssemblyRequest {
            query,
            as_of,
            intended_capability: capability,
            limit: 10,
            include_remote,
            mission_id: None,
        },
    )
    .map_err(|e| CliError::Verify(e.to_string()))?;
    println!("{}", serde_json::to_string_pretty(&ctx).unwrap());
    Ok(())
}

/// Hint / spawn path for MCP — prefer the `aevum-mcp` binary for stdio.
/// `unify mcp --mission <dir> [--write-config claude|cursor] [--out <path>]`
pub fn cmd_mcp_hint(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    if !Path::new(&mission).join("metadata.json").exists() {
        return Err(CliError::NotFound(format!(
            "{mission} is not a mission directory"
        )));
    }
    let write_cfg = args
        .windows(2)
        .find(|w| w[0] == "--write-config")
        .map(|w| w[1].clone());
    let out = args
        .windows(2)
        .find(|w| w[0] == "--out")
        .map(|w| w[1].clone());

    let mcp_bin = std::env::var("AEVUM_MCP_BIN").unwrap_or_else(|_| "aevum-mcp".into());
    let fragment = serde_json::json!({
        "mcpServers": {
            "winaevum-unify": {
                "command": mcp_bin,
                "args": ["--mission", mission],
                "env": {
                    "SLOPCHECK_BIN": std::env::var("SLOPCHECK_BIN").unwrap_or_default(),
                    "AEVUM_MISSION": mission
                }
            }
        }
    });

    if let Some(client) = write_cfg {
        let path = out.unwrap_or_else(|| match client.as_str() {
            "cursor" => ".cursor/mcp.json".into(),
            "claude" => ".mcp.json".into(),
            other => format!("mcp.{other}.json"),
        });
        if let Some(parent) = Path::new(&path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| CliError::Io(e.to_string()))?;
            }
        }
        fs::write(
            &path,
            serde_json::to_string_pretty(&fragment).unwrap() + "\n",
        )
        .map_err(|e| CliError::Io(e.to_string()))?;
        println!("✓ wrote {client} MCP config → {path}");
        return Ok(());
    }

    println!("MCP stdio server:");
    println!("  aevum-mcp --mission {mission}");
    println!();
    println!("Cursor/Claude mcp fragment:");
    println!("{}", serde_json::to_string_pretty(&fragment).unwrap());
    println!();
    println!("Write with: unify mcp --mission {mission} --write-config claude|cursor");
    Ok(())
}
