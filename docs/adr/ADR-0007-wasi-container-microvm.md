# ADR-0007 — WASI first, rootless container second, progressive microVM

- **Status:** adopted (binding of blueprint ADR-0008)
- **Date:** 2026-08-02
- **Authority:** Winter Fernandes
- **Source:** AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §17, ADR-0008 in §8

## Context

Different actions have different reversal, sensitivity and trust profiles. A single isolation tier is wasteful and insufficient.

## Decision

The Execution Fabric exposes three isolation tiers, chosen per capability:

1. **WASI/Wasmtime** — deterministic tools, parsing, validation, signed computations.
2. **Rootless container (Podman/Docker rootless)** — build, test, dependency management, full repositories.
3. **MicroVM (Firecracker/Kata/gVisor)** — high-risk R4+ execution, enterprise sensitive workflows.

Promotion between tiers is gated by the policy bundle, never by the agent prompt.

## Consequences

- Each tier ships with a manifest schema (`aevum.sandbox/v1`).
- Image digests are pinned; SBOM is required for tier 2 and 3.
- MicroVM is disabled by default (feature flag).

## Revisit criteria

OD-002 and OD-003 in blueprint §37 capture the precise Wasmtime and microVM choice at the appropriate milestones.
