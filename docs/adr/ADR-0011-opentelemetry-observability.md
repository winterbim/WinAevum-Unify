# ADR-0011 — OpenTelemetry for traces, metrics and logs

- **Status:** adopted (binding of blueprint ADR-0012)
- **Date:** 2026-08-02
- **Authority:** Winter Fernandes
- **Source:** AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §22

## Context

Observability data must be portable across vendors and consistent across services.

## Decision

OpenTelemetry is the unified observability layer. Mandatory correlation identifiers are: `mission_id`, `loop_id`, `agent_run_id`, `action_id`, `lease_id`, `approval_id`, `trace_id` and `tenant_id` when applicable.

## Rationale

- Vendor neutrality;
- Widespread instrumentation libraries;
- Compatible with the SDK / collector separation required for enterprise.

## Consequences

- Telemetry exporters are adapters; the domain emits spans with the mandatory ids.
- Free-text reasoning is **never** emitted in spans; only digests, justifications and decisions.

## Revisit criteria

None in M0.
