# ADR-0016 — Native Superior Memory (P2)

**Status:** adopted  
**Date:** 2026-08-07  
**Authority:** Winter Fernandes / agent session  

## Context

Generic agent-memory stacks often fail classes Aevum must not copy:
LLM-invented `valid_at`, noisy extraction, driver as-of leaks, and treating
recall as authority. The product goal is **100% autonomous** memory that is
correct under adversarial cases, while keeping the trust plane as the choke point.

## Decision

1. **Nominal path = native only** — SQLite + FTS5 + in-process BM25 hybrid +
   hashing embeddings. No external graph DB required.
2. **`valid_at` = episode REFERENCE_TIME only** — deterministic ingest
   (`unify graph ingest`) never substitutes wall-clock "today".
3. **Contradiction engine** — detect/resolve parallel conflicts + Refutes
   without LLM (`unify graph contradictions`).
4. **MemoryTruthBench** — offline adversarial cases; CI gate.

## Consequences

- Memory-plane scores rise via measured BM25/FTS/as-of/ingest integrity.
- LLM extraction stays non-authoritative by doctrine (ADR-0013 §11.5).
- Managed cloud scale remains a later gap, not a reason to abandon local-first.

## References

- ADR-0013, ADR-0014, ADR-0015; MemoryTruthBench; scripts/benchmark-memory-scorecard.py
