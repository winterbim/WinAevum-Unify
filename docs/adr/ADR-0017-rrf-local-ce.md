# ADR-0017 — P3 push: RRF + local cross-encoder

**Status:** adopted  
**Date:** 2026-08-07  
**Authority:** Winter Fernandes / agent session  

## Context

P2 delivered native BM25/FTS/ingest/MTB. To force memory-plane leadership on the
autonomous path, Aevum needed multi-signal fusion without external memory services.

## Decision

1. Hybrid search uses **RRF** over BM25 / embed / local CE / graph ranks, then
   epistemic trust weight (`query.rs`).
2. **Local cross-encoder surrogate** = deterministic Jaccard+coverage on tokens
   (offline; no neural vendor dependency).
3. MemoryTruthBench expands (RRF ranking + SQLite default backend).

## Consequences

- Integrity-weighted memory scoreboard improves on measured cases.
- Managed cloud scale remains an accepted later gap.

## References

- ADR-0016; MemoryTruthBench; scripts/benchmark-memory-scorecard.py
