# ADR-0013 — Temporal Decision & Evidence Graph

**Status:** adopted  
**Date:** 2026-08-07  
**Authority:** Winter Fernandes / agent session  

## Context

Aevum Unify specifies a **Decision and Evidence Graph** (blueprint §11)
with typed nodes/relations, freshness, challenges, and an epistemic firewall.
The M6 `EvidenceStore` implemented claim↔evidence checks but lacked temporal
history, episode provenance, and as-of queries.

Depending on a Python + Neo4j/FalkorDB + LLM extraction stack would violate
ADR-0002 (Rust kernel), local-first defaults, and §11.5
("LLM output is not primary evidence").

## Decision

Implement `TemporalGraph` in `crates/evidence-graph`:

| Idea | Aevum implementation |
|---|---|
| Episodes | `Episode` with optional `content_digest`; only `Attested`+digest is primary-evidence eligible |
| Bi-temporal edges | `Fact.valid_at` / `invalid_at` + `created_at` / `expired_at` |
| Invalidate on contradiction | Same name+endpoints → auto-invalidate prior active fact |
| Hybrid search | Keyword + graph distance + optional embeddings (port); never required for gates |
| Prescribed ontology | Fixed to blueprint §11 `NodeKind` / `EdgeKind` |
| Entity extraction | **Not** required for gates; agents assert typed facts with provenance |

Retain `EvidenceStore` for backward-compatible M6 claim/freshness/challenge APIs.

TypeScript mirrors live in `packages/contracts/src/temporal-graph.ts`.

## Consequences

- Positive: point-in-time queries, reconstructible event journal, authorization
  firewall on `authorizes` edges, no Neo4j/Python in the trust path.
- Positive: memory cannot authorize without attested provenance.
- Trade-off: no out-of-box LLM extraction or managed graph-DB scale; those remain
  optional ports behind ADR-0009 / ADR-0003.
- Follow-up: MCP tools for episode/fact/search (ADR-0010), SQLite persistence,
  Mission Control graph view.

## References

- Blueprint §11 Decision and Evidence Graph, §11.5 epistemic firewall, D01, D23
- Existing ADR-0012 content-addressed evidence
