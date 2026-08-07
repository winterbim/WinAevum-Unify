# ADR-0018 — Native-only memory fabric (no external memory vendor)

**Status:** adopted  
**Date:** 2026-08-07  
**Authority:** Winter Fernandes / agent session  

## Context

Earlier milestones briefly carried an optional HTTP remote-recall path and
competitor-named docs/benches. That conflicted with the product doctrine:
**100% autonomous**, local-first, and zero brand pollution from third-party
agent-memory products in the trust path or public surface.

## Decision

1. Delete any remote HTTP memory backend from `memory-fabric`.
2. `open_backend` is SQLite (default) or JSON only (`AEVUM_GRAPH_STORE=json`).
3. Remote/untrusted facts may still be ingested as `Inference` via
   `aevum_memory_ingest_remote` / promotion — never as auto-`Authorizes`.
4. Purge competitor names from code comments, ADRs, README, LEDGER, benches,
   and canvases. Scoreboards are Aevum-only integrity/latency metrics.
5. Env flags for optional third-party memory URLs are removed permanently.

## Consequences

- One memory path: native SQLite + FTS5 + BM25 + RRF + local CE + epistemic firewall.
- AgentTrustBench / MemoryTruthBench remain the proof surfaces.
- No reintroduction of vendor-named adapters without a new ADR.

## References

- ADR-0013 … ADR-0017; crates/memory-fabric; crates/memory-truth-bench
