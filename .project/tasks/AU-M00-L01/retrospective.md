# Retrospective — AU-M00-L01

## What went well

- Single pass created all required M0 documents.
- Decisions captured in machine-readable JSON where useful.
- Cross-references between ADR ↔ requirement ↔ component ↔ test documented.

## What to improve

- Add an automated check that asserts no doctrine invariant (D01-D24) is violated
  by the documents themselves (textual contradiction scan).
- Wire `unify verify` once the contracts land.

## Carried over

- AU-M00-L02 — contracts skeleton with round-trip tests.
- AU-M00-L04 — CI clean build, evidence manifest aggregator.
