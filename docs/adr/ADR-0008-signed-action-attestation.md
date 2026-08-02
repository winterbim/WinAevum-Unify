# ADR-0008 — Signed Action Attestation protocol

- **Status:** adopted (binding of blueprint ADR-0009)
- **Date:** 2026-08-02
- **Authority:** Winter Fernandes
- **Source:** AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §12

## Context

Without an envelope that carries authority, evidence, scope, risk and recovery, every agent effect is a request on the runtime's good will.

## Decision

Every effect-bearing intent MUST produce a canonical-JSON **Action Attestation** signed via a DSSE-style envelope. The Kernel re-verifies identity, signature, scope, evidence freshness, policy and approval digests at commit time before exchanging the attestation for an Execution Lease.

## Rationale

- Decisions and effects are auditable offline.
- Replay, scope drift and stale authorization are detectable.
- A single protocol unifies domains (filesystem, Git, deployment).

## Consequences

- Schema: `aevum.action-attestation/v1`, canonical JSON, Ed25519 signatures.
- Attestations carry `constitution_digest`, `policy_bundle_digest`, `approval_ids`, `not_before` and `expires_at`.
- The Ledger receives a hash-chained event for each state transition.

## Revisit criteria

If a class of effect cannot be expressed in the schema, add a structured extension and document via ADR.
