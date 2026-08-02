# ADR-0005 — OPA/Rego policy-as-code with versioned bundles

- **Status:** adopted (binding of blueprint ADR-0006)
- **Date:** 2026-08-02
- **Authority:** Winter Fernandes
- **Source:** AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §15, ADR-0006 in §8

## Context

Authorization must be deterministic, auditable, and decoupled from both the LLM and the runtime path. Strings of policy in prompts are non-authoritative.

## Decision

All action authorization is evaluated by **OPA/Rego** through the `policy-client` port. Policy bundles are versioned, signed, tested, and referenced by digest from every Action Attestation.

## Rationale

- OPA separates decision from execution.
- Rego is verifiable, declarative and supports property-style testing.
- Bundles carry a digest → integrity is part of the decision evidence.

## Consequences

- A bundle is required to resolve any authorization decision.
- Policy changes require version bump + tests + signature.
- Sample Rego lives in `policies/`.

## Revisit criteria

If a request type cannot be expressed in Rego without side effects, extend with a side-effect-free function and document.
