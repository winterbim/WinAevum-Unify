# ADR-0009 — Interchangeable model providers behind a domain port

- **Status:** adopted (binding of blueprint ADR-0010)
- **Date:** 2026-08-02
- **Authority:** Winter Fernandes
- **Source:** AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §10, ADR-0010 in §8

## Context

Models are a replaceable component. Lock-in at the domain level is unacceptable for sovereignty, diversity and resilience.

## Decision

All model interactions go through a `ModelProvider` port. Providers (inference APIs, local runtimes, hosted models) are adapters. Domain types carry no provider identifier outside audit metadata. Profile selection (cost, latency, diversity) is a policy choice, not a prompt string.

## Rationale

- Provider outage degrades the system explicitly, not catastrophically.
- Diversity Gate (M5) depends on interchangeable providers.

## Consequences

- Provider credentials are handled by the Secret Broker.
- Providers receive no implicit authority — their output is always a Claim.

## Revisit criteria

If a domain primitive requires provider-specific guarantees, document the gap with a follow-up ADR.
