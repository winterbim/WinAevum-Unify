# ADR-0002 — Rust for Sentinel Kernel and sensitive workers; TypeScript/React for Mission Control

- **Status:** adopted
- **Date:** 2026-08-02
- **Authority:** Winter Fernandes
- **Source:** AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §8, §16, §26

## Context

The Kernel enforces identity, capability, secret and policy boundaries. Sensitive workers (Council orchestration, Execution Fabric, Ledger) handle untrusted input and untrusted code. Mission Control is an interactive product UI.

## Decision

- **Rust stable** (`deny(warnings)`, no `unwrap()` in runtime) for the Sentinel Kernel, capability engine, identity, secret broker, ledger, attestation, workflow core, and execution workers.
- **TypeScript strict** (`noUncheckedIndexedAccess`) for Mission Control, SDKs and selected orchestrators.

## Rationale

- Memory safety, single static binary, mature async runtime (Tokio) for the Kernel.
- Fast product iteration, large ecosystem, accessibility tooling for the UI.

## Consequences

- The Kernel depends on **no UI library and no model provider**.
- Cross-crate dependency rules are enforced (Kernel → no UI/model; UI → API client → public contracts).
- Release artefacts: single-binary `unifyd` + Mission Control bundle.

## Revisit criteria

If Rust async stack proves inadequate for Temporal-style durable workflows at scale, document with a follow-up ADR.
