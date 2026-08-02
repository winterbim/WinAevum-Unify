# AU-M00-L01 Plan

1. Read `STATE_OF_TRUTH.md` and the blueprint index to identify all M0 deliverables.
2. Author the 12 ADRs in `docs/adr/`, each file cross-citing its source blueprint ADR/§.
3. Author `.project/DECISIONS.md` as the human-facing ADR index.
4. Author `.project/REQUIREMENTS.json` with M0 atomic requirements.
5. Author `.project/TRACEABILITY.md` mapping requirements to components and tests.
6. Author `.project/RISK_REGISTER.md` with R-001 explicitly marking the `sh -c` path as rejected.
7. Author `docs/migration/SENTINEL_INVENTORY.md` mapping each Sentinel family to its
   migration decision and the negative tests required.
8. Author this task folder (TASK.md, intake.json, plan.md, requirements.json, risks.json,
   verification.json, evidence-manifest.json, DECISION.md, retrospective.md).

## Constraints honored

- D01..D24 invariants are referenced but not modified.
- ADR-0001, ADR-0006, ADR-0011 are explicitly cited as binding.
- No change to runtime code (M1+).
