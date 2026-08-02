# ADR-0004 — Durable workflows through an abstraction compatible with Temporal

- **Status:** adopted (binding of blueprint ADR-0005)
- **Date:** 2026-08-02
- **Authority:** Winter Fernandes
- **Source:** AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §22, ADR-0005 in §8

## Context

Missions can span human approvals, network outages, restarts and long delays. Process memory alone is insufficient.

## Decision

The workflow engine is accessed through a **domain port** in `crates/sentinel-kernel` (and a future `crates/workflow-core`). The Team/Enterprise edition targets **Temporal** as the reference implementation; the local edition uses an SQLite-compatible append-only journal with the same contract surface.

## Rationale

- Replayable, deterministic state reconstruction.
- Human-in-the-loop at risk R3+ is a first-class primitive.
- Provider lock-in is avoided at the domain boundary.

## Consequences

- Every state transition is an idempotent operation bearing an idempotency key.
- A follow-up ADR (OD-004) decides when Temporal becomes mandatory.

## Revisit criteria

If the abstraction leaks provider concepts into domain types, the port must be tightened.
