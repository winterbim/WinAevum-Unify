# Aevum Unify — Local-First Mission Control (v0.3.0-local)

Aevum Unify is a local-first Audit & Authority Stack that records, signs
and verifies every action performed by AI agents before it lands on your
system. It ships with:

  - Six Rust crates (crates/): identity, attestation, ledger, sentinel-kernel,
    capability-engine, secret-broker.
  - One TypeScript contracts package (packages/contracts/): canonical
    type definitions shared with the UI.
  - One Mission Control web app (apps/mission-control/): Linear-style dark
    UI with nine views and a Cmd+K command palette.
  - 60+ tests passing across Rust + TypeScript (ledger_check gate exit 0).
  - Trust Ledger: append-only, hash-chained, Ed25519-signed.
  - Policy engine: Rego-inspired rule evaluator, fail-closed default-deny.
  - Action Attestation: canonical JSON, sign-then-verify, freshness + replay.
  - Sentinel Kernel: refuses sh -c, shell metacharacters, and grants not in
    the registry.
  - Secret Broker: opaque handles, value-never-stored, replay protection.

## Quickstart (60 seconds)

```
cd "aevum-unify"

pnpm install --frozen-lockfile
pnpm -r lint && pnpm -r build && pnpm -r test

cargo test --workspace

pnpm --filter mission-control dev
```

Open http://localhost:3000 in your browser.

## Try it in 60 seconds

1. Click any mission row in Missions → inspector shows its constitution
   JSON, council members, evidence list.
2. Hit Actions → "Sign & Run" signs an Action Attestation against the active
   mission.
3. Hit "Inject a deny" to see sh -c blocked by deny.r5-by-default.
4. Trust Ledger → click Run verify → success toast confirms the chain.
5. Approvals → click Approve or Reject on any item → its status pill flips
   and a toast confirms.

All state lives in localStorage. Reset (top-right) restores the seed.

Cmd+K opens the command palette. Type:

  - Missions / Council / Evidence / Actions / Policies / Ledger / Settings
  - "Reset demo data" → restore seed
  - "Open mission mis_03 — ..." → jump to mission

## One-shot verification gate

```
python3 .project/verification/M0/audit-script.py
```

Output:

```
[audit-script] final ledger_check exit: 0
[ledger_check] parsed 13 row(s): EVIDENCED=11 PENDING=2 WAIVED=0 CLAIMED=0
```

## Architecture

```
packages/contracts/       canonical TS types (RiskClass, MissionConstitution, ...)
crates/identity/          Ed25519 KeyMaterial + SPIFFE-compatible Identity
crates/attestation/       canonical JSON, sign, verify, replay/freshness check
crates/ledger/            append-only, hash-chained, Ed25519-signed log
crates/sentinel-kernel/   capability manifest, argv shell-metachar check
crates/capability-engine/ role-based capability engine (allow/deny)
crates/secret-broker/     opaque SecretHandle, value-never-stored, replay protection
apps/mission-control/     React + Vite UI (Linear-style dark, Cmd+K palette)
```

## Blueprint Reference

Doctrine source: AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md.
Key sections: 10 (Mission Constitution), 12 (Action Attestation),
13 (Constitution validator), 14 (Council), 15 (Risk Engine & Policy),
16 (Authority), 19 (Evidence Package), 26 (monorepo layout),
37 (open decisions).

## Verified Capabilities

Risk scoring: packages/contracts/src/risk-engine.ts (7 tests pass)
Constitution: packages/contracts/src/constitution.ts (11 tests pass)
Agent registry: packages/contracts/src/agents.ts (5 tests pass)
Policy engine: packages/contracts/src/policy.ts (9 tests pass)
Ed25519: crates/identity (5 tests pass)
Attestation: crates/attestation (8 tests pass)
Trust Ledger: crates/ledger (4 tests pass)
Sentinel Kernel: crates/sentinel-kernel (4 tests pass)
Secret Broker: crates/secret-broker (3 tests pass)
Capability Eng: crates/capability-engine (4 tests pass)
Mission Control UI: apps/mission-control (1 vitest + 60 LOC test pass)

## Status

M0 through M5 functional. M6..M12 are roadmap markers in
.project/tasks/ and will be closed in subsequent sessions.

Signed-off by Winter Fernandes, 2026-08-02.


CLI (M10)

The `unify` binary is the reproducible trust anchor of the stack. Each
subcommand is independently testable and refuses to call `sh -c`.

    cd aevum-unify
    pnpm -r build            # build the apps
    cargo build -p aevum-unify  # build the CLI
    ./target/debug/unify new --constitution constitution.json --out ./mission
    ./target/debug/unify run --mission ./mission --capability git.branch.create --argv "git checkout -b aevum/sec"
    ./target/debug/unify exec --mission ./mission --capability process.exec.argv --argv echo --argv hello
    ./target/debug/unify verify ./mission
    ./target/debug/unify package --mission ./mission --out pkg.json
    ./target/debug/unify verify-package pkg.json   # refuses tampered packages

Subcommands

  new            --constitution <path.json> --out <dir>
  run            --mission <dir> --capability <name> --argv <str>
  exec           --mission <dir> --capability <name> --argv <token> [--argv ...]
  verify         <dir>                                # walks the chain
  package        --mission <dir> --out <file.json>
  verify-package <file.json>                         # re-derives package_digest
