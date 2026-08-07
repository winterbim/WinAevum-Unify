# WinAevum-Unify — STATE_OF_TRUTH

**Version:** 2026-08-07 (pre-publication)  
**Authority:** Winter Fernandes  
**Source:** `AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md` + ADRs through ADR-0019
**Public repo:** `https://github.com/winterbim/WinAevum-Unify`

## What is true today

- This monorepo is **WinAevum-Unify** (Aevum Unify implementation: Rust trust path + TS contracts + Mission Control).
- Product category is **Trusted Autonomy**: authorize · attest · package.
- Memory is **native-only** (SQLite + FTS5 + BM25 + RRF + local CE). No third-party memory HTTP adapter (ADR-0018).
- `unify` CLI is the reproducible trust anchor: `new` / `run` / `exec` / `graph` / `golden` / `falsify` / `package`.
- Epistemic firewall: hypothesis and remote/untrusted recall cannot authorize without attested promotion.
- Sentinel doctrine: raw `sh -c` is denied (D14).
- Proof surfaces: AgentTrustBench **15/15**, MemoryTruthBench **9/9** offline, `scripts/aevum-on-aevum.sh` dogfood.
- Multi-tenant local scale: `MultiTenantStore` + `TenantScope` (ADR-0019); set `AEVUM_MEMORY_ROOT`.
- License: MIT OR Apache-2.0.

## Loop / milestone state

| Area | Status | Evidence |
|---|---|---|
| M0 repository truth + CI skeleton | closed | `.project/tasks/AU-M00-L01/`, `.github/workflows/ci.yml` |
| M1–M5 contracts (constitution, risk, agents, policy) | closed | `packages/contracts` vitest |
| M2–M4 sentinel / capabilities / secrets | closed | crate tests |
| M3 attestation + ledger | closed | crate tests |
| M6 evidence + TemporalGraph | closed | ADR-0013, evidence-graph tests |
| M7 autonomy governor | closed | autonomy-governor tests |
| M8 BranchProvider + Golden Path | closed | git-provider, `unify golden` |
| M10 `unify` CLI + package verify | closed | CLI integration tests, dogfood |
| P0–P3 memory fabric | closed | ADR-0014…0018, ATB, MTB |

## Adopted ADRs (index)

See `.project/DECISIONS.md` and `docs/adr/`. Key recent:

| ADR | Title |
|---|---|
| ADR-0013 | Temporal Decision & Evidence Graph |
| ADR-0014 | Memory Fabric + MCP + AgentTrustBench |
| ADR-0015 | SQLite + embeddings + Golden Path + falsifier |
| ADR-0016 | Native Superior Memory (BM25/FTS/contradictions) |
| ADR-0017 | RRF + local cross-encoder |
| ADR-0018 | Native-only memory fabric |

## Proof ledger

Mechanical gate: `python3 scripts/ledger_check.py .project/LEDGER.md` must exit 0.  
Current claims L-01…L-37 are EVIDENCED (no PENDING/CLAIMED/WAIVED).

## Residual gaps (honest, pre-pub)

- Managed multi-tenant cloud scale is not the product focus (local-first by design).
- Optional OpenAI-compatible embeddings are a port; gates never require them.
- No git remote is required for local dogfood; GitHub publication is a separate step (push/PR).
- Mission Control still uses local demo state for some views; trust path authority is the Rust CLI.

## Publication checklist

1. `cargo fmt --check` + `clippy -D warnings` + `cargo test --workspace`
2. ATB 15/15 + MTB 7/7 + dogfood PASS + ledger_check
3. `pnpm install --frozen-lockfile` + lint + build + test
4. LICENSE-MIT + LICENSE-APACHE present
5. README matches current crates and gates
6. Zero competitor memory-vendor names in tree (`rg` clean)
