//! Temporal graph CLI + trust-path gate (ADR-0013).
//!
//! Differentiator: the graph is not just memory —
//! `run` / `exec` refuse capabilities without an active `authorizes` fact.

use std::fs;
use std::path::Path;

use aevum_evidence_graph::{
    hybrid_search, EdgeKind, Episode, EpisodeSource, EpistemicKind, Fact, FirewallVerdict,
    GraphNode, GraphSnapshot, NodeKind, SearchRecipe, TemporalGraph,
};

use crate::{chrono_now_iso, require_value, sha256_hex, CliError};

pub const GRAPH_FILE: &str = "graph.json";

pub fn graph_path(mission_dir: &str) -> std::path::PathBuf {
    Path::new(mission_dir).join(GRAPH_FILE)
}

pub fn load_graph(mission_dir: &str) -> Result<TemporalGraph, CliError> {
    let p = graph_path(mission_dir);
    if !p.exists() {
        return Err(CliError::Verify(format!(
            "missing {GRAPH_FILE} — mission was not seeded with a temporal graph (re-run unify new)"
        )));
    }
    let raw = fs::read_to_string(&p).map_err(|e| CliError::Io(format!("reading graph: {e}")))?;
    let snap: GraphSnapshot = serde_json::from_str(&raw)
        .map_err(|e| CliError::BadArgs(format!("invalid graph.json: {e}")))?;
    TemporalGraph::from_snapshot(snap).map_err(|e| CliError::Verify(format!("graph load: {e}")))
}

pub fn save_graph(mission_dir: &str, g: &TemporalGraph) -> Result<(), CliError> {
    let snap = g.to_snapshot();
    let text = serde_json::to_string_pretty(&snap)
        .map_err(|e| CliError::Io(format!("serialize graph: {e}")))?;
    fs::write(graph_path(mission_dir), text)
        .map_err(|e| CliError::Io(format!("writing graph.json: {e}")))?;
    // Keep SQLite twin in sync when durable store is enabled (default).
    let store = std::env::var("AEVUM_GRAPH_STORE").unwrap_or_else(|_| "sqlite".into());
    if store != "json" {
        use aevum_memory_fabric::{MemoryBackend, SqliteBackend};
        if let Ok(mut sb) = SqliteBackend::open(mission_dir) {
            *sb.graph_mut() = TemporalGraph::from_snapshot(snap)
                .map_err(|e| CliError::Verify(format!("sqlite twin: {e}")))?;
            sb.save()
                .map_err(|e| CliError::Io(format!("sqlite twin save: {e}")))?;
        }
    }
    Ok(())
}

/// Seed graph at mission creation — constitution is primary attested evidence.
pub fn seed_and_persist(
    mission_dir: &str,
    mission_id: &str,
    constitution_src: &str,
    constitution_digest: &str,
) -> Result<(), CliError> {
    let now = chrono_now_iso();
    // Baseline capabilities the local-first MVP may exercise without extra authorize.
    // Higher-risk caps must be added via `unify graph authorize`.
    let caps = [
        "git.branch.create",
        "process.exec.argv",
        "graph.read",
        "graph.write",
    ];
    let g = TemporalGraph::seed_for_mission(
        mission_id,
        constitution_src,
        constitution_digest,
        &caps,
        &now,
    )
    .map_err(|e| CliError::Verify(format!("seed graph: {e}")))?;
    save_graph(mission_dir, &g)?;
    Ok(())
}

/// Trust gate: capability must be actively authorized in the temporal graph.
pub fn require_authorized(mission_dir: &str, capability: &str) -> Result<(), CliError> {
    let g = load_graph(mission_dir)?;
    let now = chrono_now_iso();
    if g.capability_authorized(capability, &now) {
        return Ok(());
    }
    Err(CliError::Verify(format!(
        "capability `{capability}` is not authorized by the temporal graph at {now} \
         (need active authorizes edge → action:{capability}; use `unify graph authorize`)"
    )))
}

/// R3+ requires an independent falsifier challenge on record (Council invariant).
pub fn require_falsifier_if_needed(
    mission_dir: &str,
    risk: aevum_autonomy_governor::RiskClass,
) -> Result<(), CliError> {
    use aevum_autonomy_governor::RiskClass;
    if risk.rank() < RiskClass::R3.rank() {
        return Ok(());
    }
    let path = Path::new(mission_dir).join("falsifier.jsonl");
    if !path.exists() {
        return Err(CliError::Verify(
            "R3+ blocked: missing falsifier.jsonl — run `unify falsify --mission … --reason …`"
                .into(),
        ));
    }
    let raw = fs::read_to_string(&path).map_err(|e| CliError::Io(e.to_string()))?;
    let count = raw.lines().filter(|l| !l.trim().is_empty()).count();
    if count == 0 {
        return Err(CliError::Verify(
            "R3+ blocked: falsifier.jsonl is empty".into(),
        ));
    }
    // Must include at least one entry from a falsifier role
    let has_falsifier = raw.lines().filter(|l| !l.trim().is_empty()).any(|l| {
        let v: serde_json::Value = serde_json::from_str(l).unwrap_or_default();
        v.get("role").and_then(|r| r.as_str()) == Some("falsifier")
    });
    if !has_falsifier {
        return Err(CliError::Verify(
            "R3+ blocked: no falsifier-role challenge recorded".into(),
        ));
    }
    Ok(())
}

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
    println!("  unify graph authorize      --mission <dir> --capability <name> [--reason <text>]");
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

    let ep_id = format!("ep_auth_{}", crate::ulid_like());
    let content = format!(
        "{{\"capability\":\"{capability}\",\"reason\":{}}}",
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
        actor_id: Some("spiffe://local.aevum/agent/graph-cli".into()),
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

fn optional_value(args: &[String], key: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == key {
            return args.get(i + 1).cloned();
        }
        i += 1;
    }
    None
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
pub fn cmd_mcp_hint(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    if !Path::new(&mission).join("metadata.json").exists() {
        return Err(CliError::NotFound(format!(
            "{mission} is not a mission directory"
        )));
    }
    println!("MCP stdio server:");
    println!("  aevum-mcp --mission {mission}");
    println!();
    println!("Cursor mcp.json fragment:");
    println!(
        "{}",
        serde_json::json!({
            "mcpServers": {
                "aevum": {
                    "command": "aevum-mcp",
                    "args": ["--mission", mission]
                }
            }
        })
    );
    Ok(())
}
