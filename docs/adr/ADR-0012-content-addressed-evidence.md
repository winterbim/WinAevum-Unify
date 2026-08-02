# ADR-0012 — Content-addressed Evidence and signed Trust Ledger

- **Status:** adopted
- **Date:** 2026-08-02
- **Authority:** Winter Fernandes
- **Source:** AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §19

## Context

Evidence and ledger entries must be verifiable offline and survive tampering.

## Decision

- Every Evidence item is content-addressed (`sha256:`).
- The Trust Ledger is an append-only hash-chained log of canonical events, each signed Ed25519 by the active ledger key.
- Evidence Packages embed a manifest with the digests of all artefacts. `unify verify <package>` recomputes digests and signatures.

## Rationale

- Hash chain + signature detects tampering.
- Content addressing allows independent re-fetch and recomputation.
- Verification does not require a live connection to the runtime.

## Consequences

- The Ledger API append-only; corrections are new events.
- A `verify` CLI becomes part of the release artefacts.

## Revisit criteria

If external anchoring (transparency log, WORM) is required (Enterprise), add a follow-up ADR.
