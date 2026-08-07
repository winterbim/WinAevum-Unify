# ADR-0019 — Native multi-tenant memory scale (local-first)

**Status:** adopted  
**Date:** 2026-08-07  
**Authority:** Winter Fernandes / agent session  

## Context

“Managed multi-tenant scale” scored ~4/10 because `mission_id`/`group_id` were
provenance stamps only: one mission directory = one graph, no shared registry,
no scoped FTS, no WAL concurrency story.

## Decision

1. **`TenantScope`** (`tenant_id` + `mission_id`, group = `mission:{id}`) enforced
   in hybrid search, assemble, and FTS when set.
2. **`MultiTenantStore`** — shared `tenants.sqlite` under `AEVUM_MEMORY_ROOT` with
   mission registry, per-mission snapshots, mission-scoped FTS, WAL + busy_timeout.
3. Per-mission `graph.sqlite` remains the authority package path; optional sync
   into the shared store on save when `AEVUM_MEMORY_ROOT` is set.
4. Ingest always stamps `group_id = mission:{mission_id}` (no `"aevum"` drift).
5. CLI: `unify graph tenants [--root] [--sync-mission]`.
6. MemoryTruthBench MTB-08 (isolation) + MTB-09 (registry scale).

## Consequences

- Managed multi-tenant scale scoreboard → **8/10** (local managed, not cloud HA).
- Still offline, still no third-party memory vendor.
- Cloud HA / cross-machine sharding remain non-goals.

## References

- Design: `docs/superpowers/specs/2026-08-07-native-multi-tenant-scale-design.md`
- ADR-0016, ADR-0018; crates/memory-fabric `{scope,multitenant,sqlite}`
