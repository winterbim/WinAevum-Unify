# ADR-0003 — PostgreSQL canonical state, SQLite local optional

- **Status:** adopted (file binding of ADR-0004 in the blueprint)
- **Date:** 2026-08-02
- **Authority:** Winter Fernandes
- **Source:** AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §25

## Context

Missions, evidence, attestations and ledger events need durable storage with strong consistency, JSONB and ideally Row Level Security. Local mode requires zero-install operation.

## Decision

- **PostgreSQL** is the canonical store for Team and Enterprise editions.
- **SQLite** is the optional local-mode adapter, behind the same domain ports.

## Rationale

- PostgreSQL provides transactions, JSONB, RLS, durable schema and migrations.
- SQLite allows a single-user local mode without infrastructure.
- The same domain ports hide the storage choice from application logic.

## Consequences

- All aggregates have migrations authored once; SQLite is generated for local dev.
- Soft delete only on business objects; **never on audit/ledger** records.
- IDs are ULID (or UUIDv7), all timestamps UTC.

## Revisit criteria

If multi-region enterprise demand appears, revisit with a primary/replica topology ADR.
