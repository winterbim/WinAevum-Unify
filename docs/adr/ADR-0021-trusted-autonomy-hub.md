# ADR-0021 — Trusted Autonomy Hub (universal control plane)

**Status:** adopted  
**Date:** 2026-08-08  
**Authority:** Winter Fernandes / Projet Phare  

## Context

AI agents and IDEs (Claude Code, Cursor, Windsurf, Copilot Agent) each reinvent
permission UX. WinAevum-Unify already owns authorize · attest · package · anti-slop.
The missing piece is **distribution as the hub under every client**.

## Decision

1. Product identity: **Trusted Autonomy Hub** — OS of trust under agents, not an IDE clone.
2. MCP exposes the full doctrine loop: package, verify-package, golden, falsify, rules, pretool_check.
3. Evidence packages bind ledger + audit_trail_digest + slop_report_digest + temporal_graph_digest; ledger syncs from audit on effects.
4. Permission-mode mapping (clients may label differently):

| Client mode | Aevum |
|---|---|
| plan / read-only | assemble/search only |
| default / acceptEdits | R1–R2 with active authorizes |
| dontAsk | only pre-attested capabilities |
| bypassPermissions | **forbidden** on agentic path |

5. Claude Code plugin (`plugins/aevum-unify`) + PreToolUse bridge + `unify mcp --write-config`.
6. Parallel attested missions (`unify parallel`) for best-of-N without auto-merge.
7. Hookify rules → Inference only (`unify rules scan`).

## Consequences

- Aevum becomes the reference control plane; agents remain interchangeable clients.
- Native Aevum agent loop (Phase 3) must use the same graph gates.

## References

- ADR-0013…0020; crates/aevum-mcp; crates/unify-cli; plugins/aevum-unify
