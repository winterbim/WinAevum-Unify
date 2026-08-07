//! SQLite persistence for TemporalGraph — local-first durable store (P1/P2).
//!
//! Schema:
//! - `meta` / `snapshot` / `events` — authority twin of TemporalGraph
//! - `facts_fts` — FTS5 inverted index for candidate retrieval (BM25 via sqlite)
//!
//! Migrates from `graph.json` on first open when SQLite is empty.

use std::path::{Path, PathBuf};

use aevum_evidence_graph::{
    hybrid_search, GraphEvent, GraphSnapshot, SearchHit, SearchRecipe, TemporalGraph,
};
use rusqlite::{params, Connection};

use crate::backend::{MemoryBackend, MemoryError, RemoteFact};
use crate::native::NativeBackend;

pub struct SqliteBackend {
    path: PathBuf,
    graph: TemporalGraph,
    json_twin: PathBuf,
    scope: Option<crate::scope::TenantScope>,
}

impl SqliteBackend {
    pub fn open(mission_dir: impl AsRef<Path>) -> Result<Self, MemoryError> {
        let scope = crate::scope::TenantScope::from_mission_dir(mission_dir.as_ref());
        Self::open_scoped(mission_dir, Some(scope))
    }

    pub fn open_scoped(
        mission_dir: impl AsRef<Path>,
        scope: Option<crate::scope::TenantScope>,
    ) -> Result<Self, MemoryError> {
        let mission_dir = mission_dir.as_ref();
        let path = mission_dir.join("graph.sqlite");
        let json_twin = mission_dir.join("graph.json");
        let conn = Connection::open(&path)
            .map_err(|e| MemoryError::Io(format!("sqlite open {}: {e}", path.display())))?;
        configure_sqlite(&conn)?;
        init_schema(&conn)?;
        let mut backend = Self {
            path,
            graph: TemporalGraph::new(),
            json_twin,
            scope,
        };
        if let Some(snap) = load_snapshot(&conn)? {
            backend.graph = TemporalGraph::from_snapshot(snap)
                .map_err(|e| MemoryError::Backend(e.to_string()))?;
        } else if backend.json_twin.exists() {
            let native = NativeBackend::open(mission_dir)?;
            backend.graph = TemporalGraph::from_snapshot(native.graph().to_snapshot())
                .map_err(|e| MemoryError::Backend(e.to_string()))?;
            backend.persist_conn(&conn)?;
        }
        backend.reconcile_scope_from_graph();
        Ok(backend)
    }

    pub fn scope(&self) -> Option<&crate::scope::TenantScope> {
        self.scope.as_ref()
    }

    /// If metadata lacked mission_id, adopt the graph's stamped mission (isolation still holds).
    fn reconcile_scope_from_graph(&mut self) {
        let Some(scope) = &mut self.scope else {
            return;
        };
        if scope.mission_id != "mis_unknown" {
            return;
        }
        if let Some(f) = self.graph.facts_as_of(None).into_iter().next() {
            scope.mission_id = f.mission_id.clone();
        } else if let Some(ep) = self.graph.to_snapshot().episodes.first() {
            scope.mission_id = ep.mission_id.clone();
        }
    }

    /// Sync this mission graph into `AEVUM_MEMORY_ROOT` multi-tenant store when configured.
    pub fn sync_to_shared_store(&self) -> Result<(), MemoryError> {
        let Some(scope) = &self.scope else {
            return Ok(());
        };
        if std::env::var("AEVUM_MEMORY_ROOT").is_err() {
            return Ok(());
        }
        let store = crate::multitenant::MultiTenantStore::from_env()?;
        store.put_graph(scope, &self.graph)
    }

    fn connect(&self) -> Result<Connection, MemoryError> {
        let conn = Connection::open(&self.path)
            .map_err(|e| MemoryError::Io(format!("sqlite reopen: {e}")))?;
        configure_sqlite(&conn)?;
        Ok(conn)
    }

    fn persist_conn(&self, conn: &Connection) -> Result<(), MemoryError> {
        let snap = self.graph.to_snapshot();
        let json = serde_json::to_string(&snap).map_err(|e| MemoryError::Io(e.to_string()))?;
        conn.execute(
            "INSERT INTO snapshot(id, json, updated_at) VALUES (1, ?1, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET json=excluded.json, updated_at=excluded.updated_at",
            params![json],
        )
        .map_err(|e| MemoryError::Io(format!("sqlite upsert: {e}")))?;

        conn.execute("DELETE FROM events", [])
            .map_err(|e| MemoryError::Io(e.to_string()))?;
        for (i, ev) in snap.events.iter().enumerate() {
            let (kind, payload) = event_row(ev);
            conn.execute(
                "INSERT INTO events(seq, kind, payload_json, at) VALUES (?1, ?2, ?3, datetime('now'))",
                params![i as i64, kind, payload],
            )
            .map_err(|e| MemoryError::Io(format!("sqlite event: {e}")))?;
        }

        rebuild_fts(conn, &self.graph)?;

        std::fs::write(
            &self.json_twin,
            serde_json::to_string_pretty(&snap).map_err(|e| MemoryError::Io(e.to_string()))?,
        )
        .map_err(|e| MemoryError::Io(e.to_string()))?;

        // Best-effort sync to shared multi-tenant store (ADR-0019).
        let _ = self.sync_to_shared_store();
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn fts_candidates(
        &self,
        query: &str,
        as_of: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(String, f64)>, MemoryError> {
        let conn = self.connect()?;
        let match_q = fts_match_query(query);
        if match_q.is_empty() {
            return Ok(vec![]);
        }
        let lim = (limit.saturating_mul(4).max(20)) as i64;
        let mut out = Vec::new();
        if let Some(scope) = &self.scope {
            let mut stmt = conn
                .prepare(
                    "SELECT fact_id, valid_at, invalid_at, -bm25(facts_fts) AS score
                     FROM facts_fts
                     WHERE facts_fts MATCH ?1 AND mission_id = ?2
                     ORDER BY score DESC
                     LIMIT ?3",
                )
                .map_err(|e| MemoryError::Io(e.to_string()))?;
            let rows = stmt
                .query_map(params![match_q, scope.mission_id, lim], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, f64>(3)?,
                    ))
                })
                .map_err(|e| MemoryError::Io(e.to_string()))?;
            for r in rows {
                let (id, valid_at, invalid_at, score) =
                    r.map_err(|e| MemoryError::Io(e.to_string()))?;
                if fact_active_at(&valid_at, invalid_at.as_deref(), as_of) {
                    out.push((id, score));
                    if out.len() >= limit {
                        break;
                    }
                }
            }
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT fact_id, valid_at, invalid_at, -bm25(facts_fts) AS score
                     FROM facts_fts
                     WHERE facts_fts MATCH ?1
                     ORDER BY score DESC
                     LIMIT ?2",
                )
                .map_err(|e| MemoryError::Io(e.to_string()))?;
            let rows = stmt
                .query_map(params![match_q, lim], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, f64>(3)?,
                    ))
                })
                .map_err(|e| MemoryError::Io(e.to_string()))?;
            for r in rows {
                let (id, valid_at, invalid_at, score) =
                    r.map_err(|e| MemoryError::Io(e.to_string()))?;
                if fact_active_at(&valid_at, invalid_at.as_deref(), as_of) {
                    out.push((id, score));
                    if out.len() >= limit {
                        break;
                    }
                }
            }
        }
        Ok(out)
    }
}

fn fact_active_at(valid_at: &str, invalid_at: Option<&str>, as_of: Option<&str>) -> bool {
    let Some(t) = as_of else {
        return invalid_at.is_none();
    };
    if valid_at.as_bytes() > t.as_bytes() {
        return false;
    }
    match invalid_at {
        None => true,
        Some(inv) => inv.as_bytes() > t.as_bytes(),
    }
}

fn fts_match_query(query: &str) -> String {
    aevum_evidence_graph::tokenize(query)
        .into_iter()
        .map(|t| {
            // Quote tokens for FTS5 safety
            format!("\"{}\"", t.replace('\"', ""))
        })
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn rebuild_fts(conn: &Connection, g: &TemporalGraph) -> Result<(), MemoryError> {
    conn.execute("DELETE FROM facts_fts", [])
        .map_err(|e| MemoryError::Io(e.to_string()))?;
    let snap = g.to_snapshot();
    for f in &snap.facts {
        conn.execute(
            "INSERT INTO facts_fts(fact_id, mission_id, name, body, valid_at, invalid_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![f.id, f.mission_id, f.name, f.fact, f.valid_at, f.invalid_at,],
        )
        .map_err(|e| MemoryError::Io(format!("fts insert: {e}")))?;
    }
    Ok(())
}

fn configure_sqlite(conn: &Connection) -> Result<(), MemoryError> {
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;
        PRAGMA busy_timeout=5000;
        PRAGMA synchronous=NORMAL;
        ",
    )
    .map_err(|e| MemoryError::Io(format!("pragma: {e}")))?;
    Ok(())
}

fn init_schema(conn: &Connection) -> Result<(), MemoryError> {
    // Migrate FTS to v3 (mission_id column) when needed.
    let version: String = conn
        .query_row("SELECT value FROM meta WHERE key = 'version'", [], |r| {
            r.get(0)
        })
        .unwrap_or_else(|_| "none".into());
    if version != "aevum.graph.sqlite/v3" {
        let _ = conn.execute_batch("DROP TABLE IF EXISTS facts_fts;");
    }
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS meta (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS snapshot (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          json TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS events (
          seq INTEGER PRIMARY KEY,
          kind TEXT NOT NULL,
          payload_json TEXT NOT NULL,
          at TEXT NOT NULL
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(
          fact_id UNINDEXED,
          mission_id UNINDEXED,
          name,
          body,
          valid_at UNINDEXED,
          invalid_at UNINDEXED,
          tokenize = 'porter unicode61'
        );
        INSERT OR REPLACE INTO meta(key, value) VALUES ('version', 'aevum.graph.sqlite/v3');
        ",
    )
    .map_err(|e| MemoryError::Io(format!("sqlite schema: {e}")))?;
    Ok(())
}

fn load_snapshot(conn: &Connection) -> Result<Option<GraphSnapshot>, MemoryError> {
    let mut stmt = conn
        .prepare("SELECT json FROM snapshot WHERE id = 1")
        .map_err(|e| MemoryError::Io(e.to_string()))?;
    let mut rows = stmt.query([]).map_err(|e| MemoryError::Io(e.to_string()))?;
    if let Some(row) = rows.next().map_err(|e| MemoryError::Io(e.to_string()))? {
        let json: String = row.get(0).map_err(|e| MemoryError::Io(e.to_string()))?;
        let snap: GraphSnapshot = serde_json::from_str(&json)
            .map_err(|e| MemoryError::Backend(format!("snapshot json: {e}")))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

fn event_row(ev: &GraphEvent) -> (&'static str, String) {
    match ev {
        GraphEvent::EpisodeAdded { id } => {
            ("episode_added", serde_json::json!({"id": id}).to_string())
        }
        GraphEvent::NodeUpserted { id } => {
            ("node_upserted", serde_json::json!({"id": id}).to_string())
        }
        GraphEvent::FactAsserted { id } => {
            ("fact_asserted", serde_json::json!({"id": id}).to_string())
        }
        GraphEvent::FactInvalidated { id, at } => (
            "fact_invalidated",
            serde_json::json!({"id": id, "at": at}).to_string(),
        ),
    }
}

impl MemoryBackend for SqliteBackend {
    fn name(&self) -> &'static str {
        "sqlite"
    }

    fn load(&mut self) -> Result<(), MemoryError> {
        let conn = self.connect()?;
        if let Some(snap) = load_snapshot(&conn)? {
            self.graph = TemporalGraph::from_snapshot(snap)
                .map_err(|e| MemoryError::Backend(e.to_string()))?;
        }
        Ok(())
    }

    fn save(&self) -> Result<(), MemoryError> {
        let conn = self.connect()?;
        self.persist_conn(&conn)
    }

    fn graph(&self) -> &TemporalGraph {
        &self.graph
    }

    fn graph_mut(&mut self) -> &mut TemporalGraph {
        &mut self.graph
    }

    fn search(
        &self,
        query: &str,
        as_of: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>, MemoryError> {
        use crate::embed::{default_embedder, semantic_hybrid_search_scoped};
        let embedder = default_embedder();
        let mut hits = semantic_hybrid_search_scoped(
            &self.graph,
            query,
            as_of,
            limit,
            embedder.as_ref(),
            self.scope.as_ref(),
        )
        .map_err(MemoryError::from)?;
        if let Ok(cands) = self.fts_candidates(query, as_of, limit) {
            if !cands.is_empty() {
                let mut recipe = SearchRecipe::new(query).limit(limit);
                if let Some(t) = as_of {
                    recipe = recipe.as_of(t);
                }
                if let Some(s) = &self.scope {
                    recipe = recipe.scoped(&s.mission_id, s.group_id());
                }
                let all = hybrid_search(&self.graph, &recipe);
                let mut by_id: std::collections::HashMap<String, SearchHit> =
                    hits.drain(..).map(|h| (h.fact_id.clone(), h)).collect();
                for (id, fts_score) in cands {
                    by_id
                        .entry(id.clone())
                        .and_modify(|h| h.score = h.score.max(fts_score * 0.05))
                        .or_insert_with(|| {
                            all.iter()
                                .find(|h| h.fact_id == id)
                                .cloned()
                                .unwrap_or(SearchHit {
                                    fact_id: id,
                                    score: fts_score * 0.05,
                                    fact: String::new(),
                                    name: String::new(),
                                })
                        });
                }
                hits = by_id.into_values().collect();
                hits.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                hits.truncate(limit.max(1));
            }
        }
        Ok(hits)
    }

    fn remote_search(&self, _query: &str, _limit: usize) -> Result<Vec<RemoteFact>, MemoryError> {
        Ok(vec![])
    }
}
