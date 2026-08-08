//! Graph persistence (JSON twin + optional SQLite).

use std::fs;
use std::path::Path;

use aevum_evidence_graph::{GraphSnapshot, TemporalGraph};

use crate::{chrono_now_iso, CliError};

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
    crate::atomic::atomic_write(&graph_path(mission_dir), text.as_bytes())
        .map_err(|e| CliError::Io(format!("writing graph.json: {e}")))?;
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

pub fn seed_and_persist(
    mission_dir: &str,
    mission_id: &str,
    constitution_src: &str,
    constitution_digest: &str,
) -> Result<(), CliError> {
    let now = chrono_now_iso();
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
