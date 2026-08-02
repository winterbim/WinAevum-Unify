# AU-M00-L01 — Repository truth

## Objective

Pose M0 of Aevum Unify: a clean monorepo skeleton with the truth artefacts,
ADR record and Sentinel inventory decisions, before any code path is implemented.

## In scope

- `.project/STATE_OF_TRUTH.md` (already present, kept in sync)
- `.project/DECISIONS.md` (ADR index)
- `.project/REQUIREMENTS.json`
- `.project/TRACEABILITY.md`
- `.project/RISK_REGISTER.md`
- `docs/adr/ADR-0001` ... `ADR-0012`
- `docs/migration/SENTINEL_INVENTORY.md`
- `.project/tasks/AU-M00-L01/` evidence

## Out of scope

- Real runtime implementations (deferred to M1-M12).
- External integrations (Temporal, SPIRE, OPA, GitHub) — referenced as ports only.
- Sentinel source copy: the prototype is **not vendored** into this repo
  (Blueprint §2.3 + ADR-0001).

## Acceptance criteria

- AC-01 — 12 ADRs exist with Status `adopted` and a citation of the blueprint.
- AC-02 — `STATE_OF_TRUTH` aligns with `DECISIONS.md`, `RISK_REGISTER.md`, `TRACEABILITY.md`,
  `REQUIREMENTS.json` (no contradiction).
- AC-03 — `RISK_REGISTER.md` records the explicit `rejected` status of the
  Sentinel `execute_command` path (R-001).
- AC-04 — `SENTINEL_INVENTORY.md` classifies every component family
  (`adopt` / `rewrite` / `reject` / `defer`).
- AC-05 — `evidence-manifest.json` references every file produced by this loop.

## Risks

- R-101 — No runtime: build is a skeleton; nothing to test beyond compile + lint.
- R-103 — Sentinel prototype not vendored; the inventory decision must remain
  binding until a concrete port is requested.

## Rollback

Delete `.project/tasks/AU-M00-L01/`, revert newly added `.project/*` and
`docs/adr/*` files. No code change: the build is unchanged.

## Required evidence

- `evidence-manifest.json` (this folder)
- All files listed under AC-01..AC-04.
