# WinAevum-Unify — Trusted Autonomy Hub

**The reference control plane under Claude Code, Cursor, Windsurf, and Copilot Agent.**

WinAevum-Unify unifies:

1. **Trusted Autonomy** — authorize · attest · package (temporal evidence graph, epistemic firewall)
2. **Deterministic AI-slop firewall** — [slopcheck](https://github.com/winterbim/slopcheck) findings bind as **Inference only** (never authorize)
3. **Universal hub** — MCP + Claude plugin + PreToolUse bridge + IDE adapters (ADR-0021)

No memory vendor. No LLM-as-authority. Golden Path never auto-merges (`auto_merge: false`).
PreToolUse + CLI + MCP are fail-closed when a mission is bound (P0-4). Embedding HTTP (`ureq`) may still be present — treat remote embed as opt-in via env, not “offline by default” until ADR-0018 is re-evidenced.

**Repo:** https://github.com/winterbim/WinAevum-Unify  
**Why this hub:** [docs/HUB_ADAPTERS.md](docs/HUB_ADAPTERS.md)

## What ships

| Layer | Contents |
|---|---|
| Rust trust path | identity, attestation, ledger, sentinel, TemporalGraph, memory-fabric, `unify` CLI |
| Anti-slop + rules | `unify slop`, `unify rules scan` → Inference episodes |
| Parallel | `unify parallel` — attested best-of-N (no auto-merge) |
| Self-run benches | AgentTrustBench **18/18** (`AEVUM_SELF_RUN_PASS`), MemoryTruthBench **9/9** — auto-evaluated in this repo, not third-party verified |
| MCP | package / verify / golden / falsify / slop / rules / pretool / doctor / agent_card |
| Plugin | [`plugins/aevum-unify`](plugins/aevum-unify) for Claude Code |
| Dream | `unify doctor` + `unify dream` — loud denies, AGENT_CARD ([docs/AGENT_DREAM.md](docs/AGENT_DREAM.md)) |
| UI | Mission Control — graph, golden path, packages, ledger |

License: **MIT OR Apache-2.0**.

## Quickstart

```bash
pnpm install --frozen-lockfile && pnpm -r test
cargo test --workspace
cargo run -p aevum-agent-trust-bench    # self-run 18/18 (AEVUM_SELF_RUN_PASS)
cargo run -p aevum-memory-truth-bench   # self-run 9/9 (not third-party)
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
./target/debug/unify doctor --mission ./mission
./target/debug/unify dream --mission ./mission
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
  agent-trust-bench/  ATB 18/18
plugins/aevum-unify/  Claude Code plugin + PreToolUse
apps/mission-control/ UI
```

## Verify before publish

```bash
bash scripts/prepub-verify.sh   # expects ATB 18/18
```

## Release

Tag **v0.2.0-phare** when Phase 0–1 gates are green on `main`. Tag **v0.2.1-dream** when Agent Dream (ATB-18) is green.
