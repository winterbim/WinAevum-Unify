# WinAevum-Unify — Local-First Trusted Autonomy

**WinAevum-Unify** (Aevum Unify) is a local-first **Audit & Authority** stack: every agent side-effect
must be authorized, attested, and packageable before it lands on your system.

**Product category:** Trusted Autonomy (authorize · attest · package)  
**Not:** a third-party agent-memory clone. Memory is a native SQLite plane
behind an epistemic firewall (ADR-0018).

## What ships

| Layer | Contents |
|---|---|
| Rust trust path | identity, attestation, ledger, sentinel-kernel, capability-engine, secret-broker, autonomy-governor, evidence-graph (`TemporalGraph`), memory-fabric (SQLite+FTS5), git-provider, `unify` CLI |
| Proof benches | AgentTrustBench (15/15), MemoryTruthBench (9/9 offline) |
| MCP | `aevum-mcp` stdio JSON-RPC tools on the real trust path |
| Contracts | `@aevum/contracts` TypeScript types (constitution, risk, policy, temporal graph) |
| UI | Mission Control (Vite/React) — missions, graph, golden path, ledger |

License: **MIT OR Apache-2.0** (`LICENSE-MIT`, `LICENSE-APACHE`).

## Quickstart

```bash
cd aevum-unify
pnpm install --frozen-lockfile
pnpm -r lint && pnpm -r build && pnpm -r test
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p aevum-agent-trust-bench    # AEVUM_PERFECT 15/15
cargo run -p aevum-memory-truth-bench   # AEVUM_MEMORY_PERFECT 9/9
bash scripts/aevum-on-aevum.sh          # dogfood PASS
python3 scripts/ledger_check.py .project/LEDGER.md
```

Mission Control UI:

```bash
pnpm --filter mission-control dev
# http://localhost:3000
```

## CLI (trust anchor)

```bash
cargo build -p aevum-unify
./target/debug/unify new --constitution constitution.json --out ./mission
./target/debug/unify graph status --mission ./mission
./target/debug/unify run --mission ./mission \
  --capability git.branch.create --argv "git checkout -b aevum/sec"
./target/debug/unify exec --mission ./mission \
  --capability process.exec.argv --argv echo --argv hello
./target/debug/unify verify ./mission
./target/debug/unify package --mission ./mission --out pkg.json
./target/debug/unify verify-package pkg.json
```

`run` / `exec` are gated by the temporal graph: a capability without an active
`authorizes` fact is refused. Default store is SQLite (`graph.sqlite` + FTS5,
JSON twin kept); `AEVUM_GRAPH_STORE=json` for JSON-only.

Deterministic ingest / contradictions:

```bash
./target/debug/unify graph ingest --mission ./mission --at <REFERENCE_TIME> --file facts.json
./target/debug/unify graph contradictions --mission ./mission --resolve
```

R3+ missions require `unify falsify` before effects. Golden Path never auto-merges:

```bash
./target/debug/unify golden --mission ./mission --repo . --title "sec fix"
# writes pr-draft.json (auto_merge=false); AEVUM_GH_PR=1 opens via gh (never merges)
```

MCP:

```bash
cargo run -p aevum-mcp -- --mission ./mission
```

Scorecard (Aevum-only metrics):

```bash
python3 scripts/benchmark-memory-scorecard.py
```

## Architecture (short)

```
packages/contracts/       canonical TS types
crates/identity/          Ed25519 KeyMaterial + SPIFFE-compatible Identity
crates/attestation/       canonical JSON, sign, verify, freshness
crates/ledger/            append-only, hash-chained, Ed25519-signed log
crates/sentinel-kernel/   capability manifest, argv shell-metachar check
crates/evidence-graph/    TemporalGraph — episodes, bi-temporal facts, as_of,
                          epistemic firewall, BM25+RRF+local CE hybrid search
crates/memory-fabric/     SqliteBackend (default) + assemble + promote
crates/capability-engine/ role-based allow/deny
crates/secret-broker/     opaque SecretHandle, value-never-stored
crates/aevum-mcp/         MCP stdio choke-point
crates/unify-cli/         unify binary (new/run/exec/graph/golden/…)
apps/mission-control/     React + Vite Mission Control
```

Doctrine source: `AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md` (repo parent / docs).  
ADRs: `docs/adr/` (through ADR-0018). Proof ledger: `.project/LEDGER.md`.

## Status

M0–M10 trust path + temporal graph + native memory fabric evidenced
(see `.project/LEDGER.md` L-01…L-37). CI: `.github/workflows/ci.yml`
(fmt, clippy, cargo test, ATB, MTB, dogfood, pnpm lint/build/test).

Signed-off by Winter Fernandes.
