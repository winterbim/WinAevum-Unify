# WinAevum-Unify — STATE_OF_TRUTH

**Version:** 2026-08-08 (Projet Phare — Trusted Autonomy Hub)  
**Authority:** Winter Fernandes  
**Source:** Blueprint + ADRs through ADR-0021  
**Public repo:** `https://github.com/winterbim/WinAevum-Unify`

## What is true today

- **WinAevum-Unify** is the Trusted Autonomy Hub: authorize · attest · package · anti-slop under Claude/Cursor/Windsurf/Copilot agents (ADR-0021).
- Memory is native-only (SQLite + FTS5 + BM25 + RRF + local CE). ADR-0018.
- CLI: `new` / `run` / `exec` / `graph` / `slop` / `rules` / `parallel` / `golden` / `falsify` / `package` / `mcp --write-config`.
- Evidence packages bind ledger (synced from audit), `audit_trail_digest`, `slop_report_digest`, `temporal_graph_digest`.
- MCP doctrine tools include package, verify-package, golden, falsify, rule_scan, pretool_check, slop_scan.
- Claude Code plugin: `plugins/aevum-unify` (PreToolUse D14 + slash commands).
- Proof: AgentTrustBench **17/17**, MemoryTruthBench **9/9**, hub scorecard, dual-dogfood, agent-loop.
- `bypassPermissions` is forbidden on the agentic path.
- License: MIT OR Apache-2.0.

## Adopted ADRs (recent)

| ADR | Title |
|---|---|
| ADR-0018 | Native-only memory fabric |
| ADR-0019 | Native multi-tenant local scale |
| ADR-0020 | Trusted Autonomy ∩ AI-Slop Firewall |
| ADR-0021 | Trusted Autonomy Hub |

## Proof ledger

`python3 scripts/ledger_check.py .project/LEDGER.md` — L-01…L-41 EVIDENCED.

## Residual gaps (honest)

- Managed multi-tenant cloud is not the focus.
- Mission Control still mixes demo seed with hub views (packages/golden updated for hub).
- Live slop needs `slopcheck` / `SLOPCHECK_BIN`.
