# ADR Index — Aevum Unify

This index points to the durable Architecture Decision Records.
The full content of each ADR lives in `docs/adr/ADR-NNNN-*.md`.

| ADR    | Title                                                | Status   | Date         |
|--------|------------------------------------------------------|----------|--------------|
| ADR-0001 | New monorepo `aevum-unify`                         | adopted  | 2026-08-02   |
| ADR-0002 | Rust for Kernel and sensitive workers             | adopted  | 2026-08-02   |
| ADR-0003 | TypeScript/React for Mission Control              | adopted  | 2026-08-02   |
| ADR-0004 | PostgreSQL canonical, SQLite local optional       | adopted  | 2026-08-02   |
| ADR-0005 | Durable workflow via abstraction (Temporal-ready) | adopted  | 2026-08-02   |
| ADR-0006 | OPA/Rego policy-as-code                           | adopted  | 2026-08-02   |
| ADR-0007 | Typed capabilities, no raw shell                  | adopted  | 2026-08-02   |
| ADR-0008 | WASI, rootless container, progressive microVM     | adopted  | 2026-08-02   |
| ADR-0009 | Signed Action Attestation                         | adopted  | 2026-08-02   |
| ADR-0010 | Interchangeable model providers                   | adopted  | 2026-08-02   |
| ADR-0011 | MCP for tools, A2A for agents                     | adopted  | 2026-08-02   |
| ADR-0012 | OpenTelemetry for observability                   | adopted  | 2026-08-02   |
| ADR-0013 | Temporal Decision & Evidence Graph | adopted | 2026-08-07 |
| ADR-0014 | Memory Fabric + MCP choke-point + AgentTrustBench | adopted | 2026-08-07 |
| ADR-0015 | P1 SQLite + embeddings + Golden Path + falsifier | adopted | 2026-08-07 |
| ADR-0016 | Native Superior Memory (P2) | adopted | 2026-08-07 |
| ADR-0017 | P3 RRF + local cross-encoder | adopted | 2026-08-07 |
| ADR-0018 | Native-only memory fabric | adopted | 2026-08-07 |
| ADR-0019 | Native multi-tenant memory scale | adopted | 2026-08-07 |

## Closed loops (M0)

- **AU-M00-L01** — repository truth: STATE_OF_TRUTH, ADRs, REQUIREMENTS.json,
  TRACEABILITY.md, RISK_REGISTER.md, SENTINEL_INVENTORY.md + task folder.
- **AU-M00-L02** — contracts package skeleton: `packages/contracts/src/*.ts`
  with 4 vitest tests passing.
- **AU-M00-L04** — CI & evidence: cargo fmt/clippy/test green, pnpm install with
  --frozen-lockfile green, pnpm lint/build/test green across workspace,
  `scripts/ledger_check.py --self-test` PASS, WinCreator proof ledger at
  11 EVIDENCED / 2 PENDING / 0 CLAIMED / 0 WAIVED.
- **AU-M00-L05** — Sentinel inventory: decisions per family recorded in
  `docs/migration/SENTINEL_INVENTORY.md`.

## Open contracts to add (deferred to next loops)

- `packages/contracts` full Rust types (currently TS-only) — AU-M0+1.
- Policy bundle signing helper — M3.
- Workflow / Temporal port skeleton — M3+.

Pending open decisions: see `OPEN_DECISIONS.md` and blueprint §37.
