# Design: Native multi-tenant memory scale (local-first)

**Date:** 2026-08-07  
**Status:** approved (user: “le plus puissant et le plus abouti”)  
**ADR:** ADR-0019  

## Goal

Raise **Managed multi-tenant scale** from ~4/10 to **≥8/10** without cloud/Neo4j vendors.
Keep Trusted Autonomy doctrine: local-first SQLite, epistemic firewall, offline.

## Approaches considered

1. Shared multi-mission SQLite only  
2. Directory-per-mission + scope filters only  
3. **Both (chosen)** — enforced TenantScope everywhere + shared store registry + WAL + measured isolation

## Design

### TenantScope

```rust
pub struct TenantScope {
    pub tenant_id: String,      // default "local"
    pub mission_id: String,
    pub group_id: String,       // always "mission:{mission_id}"
}
```

- Search / assemble / FTS / facts_as_of filter by `mission_id` (and optional `tenant_id` on store rows).
- Cross-mission leak = hard fail in MemoryTruthBench.

### Per-mission path (compat)

Existing `mission_dir/graph.sqlite` continues to work. Scope is still enforced in-process.

### Shared multi-mission store

```
AEVUM_MEMORY_ROOT=~/.aevum/store  (or --store)
  tenants.sqlite   # missions registry + partitioned facts/episodes/nodes
  OR graph.sqlite with mission_id columns
```

- `MissionRegistry`: register/list/open by tenant+mission  
- Schema: `missions(tenant_id, mission_id, …)`, facts/episodes/nodes with indexes  
- FTS includes `mission_id` / `tenant_id` (UNINDEXED) + `MATCH … AND mission_id = ?`  
- WAL + `busy_timeout=5000`  
- Snapshot twin still exportable per mission for packages

### Isolation guarantee

Query for mission A never returns facts stamped mission B (MTB case).

### Scorecard

`scale_managed` → **8** when isolation + multi-mission registry + WAL evidenced.

## Non-goals

- Cloud HA / auto-sharding across machines  
- Third-party memory vendors  

## Acceptance

- MTB isolation case green  
- ATB still 15/15  
- dogfood + prepub-verify green  
- scorecard `scale_managed >= 8`
