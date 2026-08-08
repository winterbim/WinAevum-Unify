# WinAevum-Unify — STATE_OF_TRUTH

**Version:** 2026-08-08 (P0 remédiation + quality hardening)  
**Authority:** Winter Fernandes  
**Source:** Blueprint + ADRs through ADR-0021 + Agent Dream + P0 security  
**Public repo:** `https://github.com/winterbim/WinAevum-Unify`  
**Branch:** `fix/p0-security`

## What is true today

- **WinAevum-Unify** is the Trusted Autonomy Hub: authorize · attest · package · anti-slop under Claude/Cursor/Windsurf/Copilot agents (ADR-0021).
- Memory is native-only by default (SQLite + FTS5 + BM25 + RRF + local CE). Remote embeddings require feature `remote-embed` + `AEVUM_ALLOW_REMOTE_EMBED=1` (ADR-0018 re-evidenced 2026-08-08).
- CLI: `new` / `run` / `exec` / `graph` / `slop` / `rules` / `parallel` / `golden` / `falsify` / `doctor` / `dream` / `package` / `verify-package` / `human-keygen` / `human-grant` / `pretool-check` / `debug-now` / `mcp --write-config`.
- Evidence packages are **Ed25519-signed** (`aevum.evidence-package/v2`) with trust pubkey sidecar; authority secret lives in `{mission}/.aevum/authority.sk` (never packaged).
- Ledger entries are signed end-to-end with tip anchor; `unify verify` requires ledger/audit byte-identity + tip.
- `verify-package` verifies envelope signature, key bind, and embedded ledger (not signature-only).
- Dependency license gate: `cargo deny check` (`deny.toml`, L-38 EVIDENCED).
- `graph authorize` requires a human grant signature (distinct principal — P0-5).
- PreToolUse + `pretool-check` + MCP authz are fail-closed (P0-4).
- Agent Dream: loud Inference denials; `unify doctor` / `unify dream`.
- Self-run benches: AgentTrustBench **18/18** (`AEVUM_SELF_RUN_PASS`), MemoryTruthBench **9/9** (`AEVUM_MEMORY_SELF_RUN_PASS`) — not third-party verified.
- Golden Path never auto-merges (`auto_merge: false`).
- License: MIT OR Apache-2.0.

## Adopted ADRs (recent)

| ADR | Title |
|---|---|
| ADR-0018 | Native-only memory fabric |
| ADR-0019 | Native multi-tenant local scale |
| ADR-0020 | Trusted Autonomy ∩ AI-Slop Firewall |
| ADR-0021 | Trusted Autonomy Hub |

## Proof ledger

`python3 scripts/ledger_check.py .project/LEDGER.md` — see LEDGER for CLAIMED vs EVIDENCED.

## Residual gaps (honest)

- Automated P0 refalsify is CI-gated (`scripts/refalsify-p0.sh`); a multi-agent human panel on a fresh clone is still recommended before a marketing tag.
- Human key must stay outside the agent sandbox (operator responsibility).
- Live GitHub PR merge with `aevum-gate` against remote not dogfooded end-to-end.
- Managed multi-tenant cloud is not the focus.
- Live slop needs `slopcheck` / `SLOPCHECK_BIN`.
