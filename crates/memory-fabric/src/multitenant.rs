//! Shared multi-mission SQLite store — local-first managed multi-tenant scale (ADR-0019).
//!
//! One `tenants.sqlite` under `AEVUM_MEMORY_ROOT` holds many missions with
//! WAL, indexed FTS scoped by tenant+mission, and a mission registry.
//! No cloud / Neo4j required.

use std::path::{Path, PathBuf};

use aevum_evidence_graph::{hybrid_search, SearchHit, SearchRecipe, TemporalGraph};
use rusqlite::{params, Connection};

use crate::backend::MemoryError;
use crate::scope::TenantScope;

pub struct MultiTenantStore {
    path: PathBuf,
}

impl MultiTenantStore {
    /// Open (or create) the shared store at `{root}/tenants.sqlite`.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, MemoryError> {
        let root = root.as_ref();
        std::fs::create_dir_all(root)
            .map_err(|e| MemoryError::Io(format!("mkdir memory root: {e}")))?;
        let path = root.join("tenants.sqlite");
        let conn = Connection::open(&path)
            .map_err(|e| MemoryError::Io(format!("tenants.sqlite open: {e}")))?;
        configure_conn(&conn)?;
        init_schema(&conn)?;
        Ok(Self { path })
    }

    /// Open from `AEVUM_MEMORY_ROOT` env (required).
    pub fn from_env() -> Result<Self, MemoryError> {
        let root = std::env::var("AEVUM_MEMORY_ROOT")
            .map_err(|_| MemoryError::NotConfigured("AEVUM_MEMORY_ROOT not set".into()))?;
        Self::open(root)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn connect(&self) -> Result<Connection, MemoryError> {
        let conn = Connection::open(&self.path)
            .map_err(|e| MemoryError::Io(format!("tenants reopen: {e}")))?;
        configure_conn(&conn)?;
        Ok(conn)
    }

    pub fn register(&self, scope: &TenantScope) -> Result<(), MemoryError> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO missions(tenant_id, mission_id, group_id, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))
             ON CONFLICT(tenant_id, mission_id) DO UPDATE SET
               group_id=excluded.group_id,
               updated_at=excluded.updated_at",
            params![scope.tenant_id, scope.mission_id, scope.group_id()],
        )
        .map_err(|e| MemoryError::Io(format!("register mission: {e}")))?;
        Ok(())
    }

    pub fn list(&self, tenant_id: &str) -> Result<Vec<TenantScope>, MemoryError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT tenant_id, mission_id FROM missions
                 WHERE tenant_id = ?1 ORDER BY mission_id",
            )
            .map_err(|e| MemoryError::Io(e.to_string()))?;
        let rows = stmt
            .query_map(params![tenant_id], |row| {
                Ok(TenantScope::new(
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                ))
            })
            .map_err(|e| MemoryError::Io(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| MemoryError::Io(e.to_string()))?);
        }
        Ok(out)
    }

    pub fn list_all(&self) -> Result<Vec<TenantScope>, MemoryError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("SELECT tenant_id, mission_id FROM missions ORDER BY tenant_id, mission_id")
            .map_err(|e| MemoryError::Io(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(TenantScope::new(
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                ))
            })
            .map_err(|e| MemoryError::Io(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| MemoryError::Io(e.to_string()))?);
        }
        Ok(out)
    }

    /// Persist a mission graph into the shared store (snapshot + scoped FTS).
    pub fn put_graph(&self, scope: &TenantScope, g: &TemporalGraph) -> Result<(), MemoryError> {
        self.register(scope)?;
        let conn = self.connect()?;
        let snap = g.to_snapshot();
        let json = serde_json::to_string(&snap).map_err(|e| MemoryError::Io(e.to_string()))?;
        conn.execute(
            "INSERT INTO mission_snapshot(tenant_id, mission_id, json, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))
             ON CONFLICT(tenant_id, mission_id) DO UPDATE SET
               json=excluded.json, updated_at=excluded.updated_at",
            params![scope.tenant_id, scope.mission_id, json],
        )
        .map_err(|e| MemoryError::Io(format!("put snapshot: {e}")))?;

        // Incremental FTS: only this mission's rows
        conn.execute(
            "DELETE FROM facts_fts WHERE tenant_id = ?1 AND mission_id = ?2",
            params![scope.tenant_id, scope.mission_id],
        )
        .map_err(|e| MemoryError::Io(e.to_string()))?;
        for f in &snap.facts {
            conn.execute(
                "INSERT INTO facts_fts(fact_id, tenant_id, mission_id, name, body, valid_at, invalid_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    f.id,
                    scope.tenant_id,
                    scope.mission_id,
                    f.name,
                    f.fact,
                    f.valid_at,
                    f.invalid_at,
                ],
            )
            .map_err(|e| MemoryError::Io(format!("fts insert: {e}")))?;
        }
        Ok(())
    }

    pub fn get_graph(&self, scope: &TenantScope) -> Result<Option<TemporalGraph>, MemoryError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT json FROM mission_snapshot
                 WHERE tenant_id = ?1 AND mission_id = ?2",
            )
            .map_err(|e| MemoryError::Io(e.to_string()))?;
        let mut rows = stmt
            .query(params![scope.tenant_id, scope.mission_id])
            .map_err(|e| MemoryError::Io(e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| MemoryError::Io(e.to_string()))? {
            let json: String = row.get(0).map_err(|e| MemoryError::Io(e.to_string()))?;
            let snap: aevum_evidence_graph::GraphSnapshot = serde_json::from_str(&json)
                .map_err(|e| MemoryError::Backend(format!("snapshot: {e}")))?;
            let g = TemporalGraph::from_snapshot(snap)
                .map_err(|e| MemoryError::Backend(e.to_string()))?;
            Ok(Some(g))
        } else {
            Ok(None)
        }
    }

    /// Scoped hybrid search — never leaks across missions.
    pub fn search(
        &self,
        scope: &TenantScope,
        query: &str,
        as_of: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>, MemoryError> {
        let g = self.get_graph(scope)?.ok_or_else(|| {
            MemoryError::Backend(format!(
                "mission not in store: {}/{}",
                scope.tenant_id, scope.mission_id
            ))
        })?;
        let mut recipe = SearchRecipe::new(query)
            .limit(limit)
            .scoped(&scope.mission_id, scope.group_id());
        if let Some(t) = as_of {
            recipe = recipe.as_of(t);
        }
        let mut hits = hybrid_search(&g, &recipe);

        // FTS boost from shared index (mission-filtered SQL)
        if let Ok(cands) = self.fts_candidates(scope, query, as_of, limit) {
            for (id, score) in cands {
                if let Some(h) = hits.iter_mut().find(|h| h.fact_id == id) {
                    h.score = h.score.max(score * 0.05);
                }
            }
            hits.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            hits.truncate(limit.max(1));
        }
        Ok(hits)
    }

    fn fts_candidates(
        &self,
        scope: &TenantScope,
        query: &str,
        as_of: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(String, f64)>, MemoryError> {
        let conn = self.connect()?;
        let match_q = fts_match_query(query);
        if match_q.is_empty() {
            return Ok(vec![]);
        }
        let mut stmt = conn
            .prepare(
                "SELECT fact_id, valid_at, invalid_at, -bm25(facts_fts) AS score
                 FROM facts_fts
                 WHERE facts_fts MATCH ?1
                   AND tenant_id = ?2
                   AND mission_id = ?3
                 ORDER BY score DESC
                 LIMIT ?4",
            )
            .map_err(|e| MemoryError::Io(e.to_string()))?;
        let rows = stmt
            .query_map(
                params![
                    match_q,
                    scope.tenant_id,
                    scope.mission_id,
                    (limit.saturating_mul(4).max(20)) as i64
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, f64>(3)?,
                    ))
                },
            )
            .map_err(|e| MemoryError::Io(e.to_string()))?;
        let mut out = Vec::new();
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
        Ok(out)
    }

    /// Count registered missions (scale signal).
    pub fn mission_count(&self) -> Result<usize, MemoryError> {
        let conn = self.connect()?;
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM missions", [], |r| r.get(0))
            .map_err(|e| MemoryError::Io(e.to_string()))?;
        Ok(n as usize)
    }
}

fn configure_conn(conn: &Connection) -> Result<(), MemoryError> {
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
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS missions (
          tenant_id TEXT NOT NULL,
          mission_id TEXT NOT NULL,
          group_id TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          PRIMARY KEY (tenant_id, mission_id)
        );
        CREATE TABLE IF NOT EXISTS mission_snapshot (
          tenant_id TEXT NOT NULL,
          mission_id TEXT NOT NULL,
          json TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          PRIMARY KEY (tenant_id, mission_id)
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(
          fact_id UNINDEXED,
          tenant_id UNINDEXED,
          mission_id UNINDEXED,
          name,
          body,
          valid_at UNINDEXED,
          invalid_at UNINDEXED,
          tokenize = 'porter unicode61'
        );
        CREATE INDEX IF NOT EXISTS idx_missions_tenant ON missions(tenant_id);
        ",
    )
    .map_err(|e| MemoryError::Io(format!("multitenant schema: {e}")))?;
    Ok(())
}

fn fts_match_query(query: &str) -> String {
    aevum_evidence_graph::tokenize(query)
        .into_iter()
        .map(|t| format!("\"{}\"", t.replace('\"', "")))
        .collect::<Vec<_>>()
        .join(" OR ")
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

#[cfg(test)]
mod tests {
    use super::*;
    use aevum_evidence_graph::{relate_fact, seed_entity, Episode, EpisodeSource, EpistemicKind};

    #[test]
    fn isolation_across_missions() {
        let dir = tempfile::tempdir().unwrap();
        let store = MultiTenantStore::open(dir.path()).unwrap();
        let a = TenantScope::local("mis_a");
        let b = TenantScope::local("mis_b");

        let mut ga = TemporalGraph::new();
        ga.add_episode(Episode {
            id: "ep_a".into(),
            mission_id: a.mission_id.clone(),
            group_id: a.group_id(),
            source: EpisodeSource::Text,
            content: "secret-a".into(),
            content_digest: None,
            valid_at: "2026-01-01T00:00:00Z".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            actor_id: None,
        })
        .unwrap();
        ga.upsert_node(seed_entity(
            "n1",
            "N1",
            &a.mission_id,
            &a.group_id(),
            "2026-01-01T00:00:00Z",
        ));
        ga.upsert_node(seed_entity(
            "n2",
            "N2",
            &a.mission_id,
            &a.group_id(),
            "2026-01-01T00:00:00Z",
        ));
        ga.assert_fact(relate_fact(
            "f_a",
            "n1",
            "n2",
            "SECRET",
            "alpha-token-unique-aaa",
            "ep_a",
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00Z",
            &a.mission_id,
            &a.group_id(),
            EpistemicKind::Fact,
        ))
        .unwrap();
        store.put_graph(&a, &ga).unwrap();

        let mut gb = TemporalGraph::new();
        gb.add_episode(Episode {
            id: "ep_b".into(),
            mission_id: b.mission_id.clone(),
            group_id: b.group_id(),
            source: EpisodeSource::Text,
            content: "secret-b".into(),
            content_digest: None,
            valid_at: "2026-01-01T00:00:00Z".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            actor_id: None,
        })
        .unwrap();
        gb.upsert_node(seed_entity(
            "n1",
            "N1",
            &b.mission_id,
            &b.group_id(),
            "2026-01-01T00:00:00Z",
        ));
        gb.upsert_node(seed_entity(
            "n2",
            "N2",
            &b.mission_id,
            &b.group_id(),
            "2026-01-01T00:00:00Z",
        ));
        gb.assert_fact(relate_fact(
            "f_b",
            "n1",
            "n2",
            "SECRET",
            "beta-token-unique-bbb",
            "ep_b",
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00Z",
            &b.mission_id,
            &b.group_id(),
            EpistemicKind::Fact,
        ))
        .unwrap();
        store.put_graph(&b, &gb).unwrap();

        assert_eq!(store.mission_count().unwrap(), 2);
        let hits_a = store
            .search(
                &a,
                "alpha-token-unique-aaa",
                Some("2026-02-01T00:00:00Z"),
                10,
            )
            .unwrap();
        assert!(hits_a.iter().any(|h| h.fact_id == "f_a"));
        assert!(!hits_a.iter().any(|h| h.fact_id == "f_b"));

        let hits_b = store
            .search(
                &b,
                "beta-token-unique-bbb",
                Some("2026-02-01T00:00:00Z"),
                10,
            )
            .unwrap();
        assert!(hits_b.iter().any(|h| h.fact_id == "f_b"));
        assert!(!hits_b.iter().any(|h| h.fact_id == "f_a"));

        // Cross-query: searching A's store for B's token must not return B's fact
        let leak = store
            .search(
                &a,
                "beta-token-unique-bbb",
                Some("2026-02-01T00:00:00Z"),
                10,
            )
            .unwrap();
        assert!(!leak.iter().any(|h| h.fact_id == "f_b"));
    }
}
