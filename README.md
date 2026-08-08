# WinAevum-Unify — Trusted Autonomy Hub

**The reference control plane under Claude Code, Cursor, Windsurf, and Copilot Agent.**

WinAevum-Unify unifies:

1. **Trusted Autonomy** — authorize · attest · package (temporal evidence graph, epistemic firewall)
2. **Deterministic AI-slop firewall** — [slopcheck](https://github.com/winterbim/slopcheck) findings bind as **Inference only** (never authorize)
3. **Universal hub** — MCP + Claude plugin + PreToolUse bridge + IDE adapters (ADR-0021)

No memory vendor. No LLM-as-authority. No `bypassPermissions`. Offline by default. Golden Path never auto-merges.

**Repo:** https://github.com/winterbim/WinAevum-Unify  
**Why this hub:** [docs/HUB_ADAPTERS.md](docs/HUB_ADAPTERS.md)

## What ships

| Layer | Contents |
|---|---|
| Rust trust path | identity, attestation, ledger, sentinel, TemporalGraph, memory-fabric, `unify` CLI |
| Anti-slop + rules | `unify slop`, `unify rules scan` → Inference episodes |
| Parallel | `unify parallel` — attested best-of-N (no auto-merge) |
| Proof | AgentTrustBench **17/17**, MemoryTruthBench **9/9**, hub scorecard |
| MCP | package / verify-package / golden / falsify / slop / rules / pretool_check |
| Plugin | [`plugins/aevum-unify`](plugins/aevum-unify) for Claude Code |
| UI | Mission Control — graph, golden path, packages, ledger |

License: **MIT OR Apache-2.0**.

## Quickstart

```bash
pnpm install --frozen-lockfile && pnpm -r test
cargo test --workspace
cargo run -p aevum-agent-trust-bench    # AEVUM_PERFECT 17/17
cargo run -p aevum-memory-truth-bench   # AEVUM_MEMORY_PERFECT 9/9
python3 scripts/benchmark-hub-scorecard.py
bash scripts/aevum-on-aevum.sh
bash scripts/dual-dogfood.sh
bash scripts/aevum-agent-loop.sh /path/to/mission .
```

Mission Control: `pnpm --filter mission-control dev` → http://localhost:3000

## CLI (trust anchor)

```bash
cargo build -p aevum-unify -p aevum-mcp
./target/debug/unify new --constitution constitution.json --out ./mission
./target/debug/unify slop --mission ./mission --repo . --all
./target/debug/unify rules scan --mission ./mission --repo .
./target/debug/unify parallel --constitution constitution.json --out /tmp/aevum-p --n 3
./target/debug/unify package --mission ./mission --out pkg.json
./target/debug/unify mcp --mission ./mission --write-config claude
```

## Hub clients

| Client | How |
|---|---|
| Claude Code | Install `plugins/aevum-unify` + set `AEVUM_MISSION` |
| Cursor | `unify mcp --write-config cursor` |
| Windsurf / Copilot Agent | Point MCP at `aevum-mcp --mission …` |

See [docs/HUB_ADAPTERS.md](docs/HUB_ADAPTERS.md).

## Architecture

```
crates/
  evidence-graph/     TemporalGraph + epistemic firewall
  memory-fabric/      SQLite + slop/rules ingest (Inference)
  unify-cli/          unify binary (trust anchor)
  aevum-mcp/          MCP stdio choke-point
  agent-trust-bench/  ATB 17/17
plugins/aevum-unify/  Claude Code plugin + PreToolUse
apps/mission-control/ UI
```

## Verify before publish

```bash
bash scripts/prepub-verify.sh   # expects ATB 17/17
```

## Release

Tag **v0.2.0-phare** when Phase 0–1 gates are green on `main`.
