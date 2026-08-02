# ADR-0001 — New monorepo `aevum-unify`

- **Status:** adopted
- **Date:** 2026-08-02
- **Authority:** Winter Fernandes
- **Source:** AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §2.2, §26

## Context

Aevum Unify fuses two legacy codebases: `winterbim/aevum-council` and `winterbim/sentinel`. Sentinel's prototype exposes a `execute_command` tool built on `sh -c` strings. Both codebases contain incompatible historical assumptions and undocumented contracts.

## Decision

Aevum Unify is built in a **new monorepo** `aevum-unify`. Legacy code is treated as a conceptual and audited library — never merged wholesale.

## Rationale

- eliminates incompatible historical dependencies and assumptions;
- establishes shared contracts before any code import;
- prevents a known weakness from becoming an implicit invariant;
- preserves traceability between any reused component and its audit;
- enables a local-first and enterprise-ready architecture from day one.

## Migration rule

Every reused component follows:

```
Inventory → Threat Review → Contract Mapping → Minimal Port → Tests → Evidence → Adoption Decision
```

Each import receives an id, a justification, an owner, a risk list, negative tests, and one of: `adopted`, `rewritten`, `rejected`, `deferred`.

## Consequences

- A clean build must be reproducible on a fresh machine.
- STATE_OF_TRUTH and traceability must be updated on every component reuse.
- The known Sentinel `sh -c` path is explicitly **rejected** for the agentic route (see ADR-0007).

## Revisit criteria

If the clean-room strategy blocks MVP delivery past M3, revisit with ADR update.
