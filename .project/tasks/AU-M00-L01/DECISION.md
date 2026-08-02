# DECISION — AU-M00-L01

**Closed :** true
**Outcome :** accepted

## What

- 12 ADRs adopted and cross-cited against the blueprint.
- 1 risk register initialised, R-001 binding the rejection of the Sentinel `execute_command` (`sh -c`) path.
- 1 traceability matrix linking every M0 requirement to a component and a test.
- 1 Sentinel inventory classifying each component family (`adopt / rewrite / reject / defer`).

## Why

M0 needs an explicit, immutable substrate (truth, ADR, inventory, traceability) before
any code path lands. Without it, AUDIT retroactively becomes interpretation.

## What is binding after this loop

- ADR-0006 — no raw shell on the agentic path.
- R-001 in the risk register — Sentinel `execute_command` rejected.
- `SENTINEL_INVENTORY.md` adoption decisions (per family).

## Follow-ups

- Loop AU-M00-L02 — contracts package skeleton.
- Loop AU-M00-L04 — CI and evidence (clean build/test/secret scan).
- Loop AU-M00-L05 — Sentinel inventory already run; ports plan written.
