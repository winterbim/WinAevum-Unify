# ADR-0015 — P1 Durable Memory, Embeddings, Golden Path, Falsifier Gate

**Status:** adopted  
**Date:** 2026-08-07  
**Authority:** Winter Fernandes / agent session  

## Context

P0 delivered Memory Fabric + MCP + AgentTrustBench. To strengthen the memory
plane *without* surrendering the trust plane, P1 must:
1. persist the temporal graph durably (SQLite local-first),
2. add semantic hybrid retrieval as an optional port (never a gate),
3. ship a real Golden Path (issue → branch → test → package → PR draft, never merge),
4. enforce Council falsifier before R3+ effects,
5. surface the graph in Mission Control.

## Decision

- `SqliteBackend` (`graph.sqlite`) is the default store (`AEVUM_GRAPH_STORE=sqlite`);
  migrates from `graph.json` and keeps a JSON twin for CLI compatibility.
- Embedding port: `HashingEmbedder` (offline, deterministic) + optional
  OpenAI-compatible HTTP when keys are set. Wired into hybrid search via
  `semantic_hybrid_search`; gates never require embeddings.
- `unify golden`: real `LocalGit` branch + optional `cargo test` + evidence
  package + `pr-draft.json` (`auto_merge: false`); optional `gh pr create` when
  `AEVUM_GH_PR=1` — never merges.
- `unify falsify` / `unify approve`: R3+ `run`/`exec`/`golden` require a
  falsifier-role challenge; golden also requires human approval via governor.
- Mission Control: Temporal Graph view + Golden Path copy aligned with CLI.

## Consequences

- AgentTrustBench expands to 15 cases (ATB-13..15).
- SQLite is the local authority store; remote recall never auto-authorizes.
- Embeddings improve recall only — authorize still requires attested facts.

## References

- ADR-0013, ADR-0014; path-to-number-one strategy
