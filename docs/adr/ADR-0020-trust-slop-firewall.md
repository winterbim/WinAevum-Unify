# ADR-0020 — Trusted Autonomy ∩ AI-Slop Firewall (unprecedented)

**Status:** adopted  
**Date:** 2026-08-08  
**Authority:** Winter Fernandes / agent session  

## Context

No existing stack combines (1) a local-first authorize/attest/package trust plane
with (2) a deterministic offline AI-slop diff gate whose findings bind into the
same temporal graph **without** becoming authorizing evidence.

## Decision

1. `unify slop` runs [slopcheck](https://github.com/winterbim/slopcheck) and
   writes `slop-report.json`.
2. Findings ingest via `ingest_slop_report` as **Inference** only (`SLOP_BLOCK` /
   `SLOP_WARN` / `SLOP_CLEAN`) — epistemic firewall forbids authorization.
3. Golden Path enables the slop gate by default (`--no-slop-gate` to skip;
   `--slop-gate` to require binary).
4. MCP tool `aevum_slop_scan` exposes the same path.
5. AgentTrustBench ATB-16 proves slop Inference cannot authorize.

## Consequences

- Product identity: **WinAevum-Unify** = Trusted Autonomy + anti-slop evidence plane.
- Completely novel vs memory-only or linter-only tools.
- Requires `slopcheck` on PATH / `SLOPCHECK_BIN` for live scans.

## References

- ADR-0013…0019; crates/memory-fabric/src/slop_ingest.rs; crates/unify-cli/src/slop.rs
