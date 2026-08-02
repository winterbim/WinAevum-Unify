# ADR-0006 — No raw shell command string on the agentic path

- **Status:** adopted
- **Date:** 2026-08-02
- **Authority:** Winter Fernandes
- **Source:** AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §16.4, ADR-0007 in §8

## Context

The Sentinel prototype exposes an integrated `execute_command` tool that ultimately invokes `sh -c`. This execution model bypasses the capability contract and allows shell metacharacter injection.

## Decision

The Kernel exposes **typed capabilities**. Process execution requires:

- a resolved executable path,
- an explicit `argv` array,
- an allowlist or tool manifest,
- a filtered environment,
- a workspace-scoped working directory,
- an enforced sandbox,
- default-deny network,
- bounded CPU / memory / disk / processes / stdout,
- filesystem diff capture,
- a proof of the actual binary executed.

`sh -c`, `bash -c`, `cmd /c` and free-form PowerShell strings are **forbidden** on the agentic path. A human operator may use a strictly audited shell mode outside the agentic route.

## Rationale

- Capabilities are auditable; command strings are not.
- Capability grants carry expiry, scope and revocation; command strings do not.

## Consequences

- The Sentinel `execute_command` module is **rejected** for import (entry recorded in `RISK_REGISTER.md`).
- Negative tests cover shell metacharacters in argv.
- The Capability API surface is the only documented execution entry point.

## Revisit criteria

None. This decision is binding for the product.
