# WinAevum-Unify — STATE_OF_TRUTH

**Version:** 2026-08-08 (Trust × Anti-Slop)  
**Authority:** Winter Fernandes  
**Source:** `AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md` + ADRs through ADR-0020  
**Public repo:** `https://github.com/winterbim/WinAevum-Unify`

## What is true today

- This monorepo is **WinAevum-Unify** (Aevum Unify implementation: Rust trust path + TS contracts + Mission Control).
- Product category is **Trusted Autonomy ∩ offline AI-slop firewall**: authorize · attest · package · gate AI-slop (ADR-0020).
- Memory is **native-only** (SQLite + FTS5 + BM25 + RRF + local CE). No third-party memory HTTP adapter (ADR-0018).
- `unify` CLI is the reproducible trust anchor: `new` / `run` / `exec` / `graph` / `slop` / `golden` / `falsify` / `package`.
- Epistemic firewall: hypothesis, remote/untrusted recall, and **slop findings** cannot authorize without attested promotion (Inference-only ingest).
- Sentinel doctrine: raw `sh -c` is denied (D14).
- Proof surfaces: AgentTrustBench **16/16**, MemoryTruthBench **9/9** offline, `scripts/aevum-on-aevum.sh` + `scripts/dual-dogfood.sh`.
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
| Trust × Anti-Slop | closed | ADR-0020, ATB-16, `unify slop` |

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
| ADR-0019 | Native multi-tenant local scale |
| ADR-0020 | Trusted Autonomy ∩ AI-Slop Firewall |

## Proof ledger

Mechanical gate: `python3 scripts/ledger_check.py .project/LEDGER.md` must exit 0.  
Current claims L-01…L-40 are EVIDENCED (no PENDING/CLAIMED/WAIVED).

## Residual gaps (honest)

- Managed multi-tenant cloud scale is not the product focus (local-first by design).
- Optional OpenAI-compatible embeddings are a port; gates never require them.
- Live `unify slop` needs `slopcheck` on PATH or `SLOPCHECK_BIN`; golden soft-skips if missing unless `--slop-gate`.
- Mission Control still uses local demo state for some views; trust path authority is the Rust CLI.

## Publication checklist

1. `cargo fmt --check` + `clippy -D warnings` + `cargo test --workspace`
2. ATB 16/16 + MTB 9/9 + dogfood PASS + ledger_check
3. `pnpm install --frozen-lockfile` + lint + build + test
4. LICENSE-MIT + LICENSE-APACHE present
5. README matches current crates and gates
6. Zero competitor memory-vendor names in tree (`rg` clean)
