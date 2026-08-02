# Sentinel Inventory — Migration Loop AU-M00-L05

**Version :** 2026-08-02 M0
**Authority :** Winter Fernandes
**Source :** Blueprint §2.3 + ADR-0001

This document inventories the legacy Sentinel prototype (`winterbim/sentinel`) and
records the Aevum Unify migration decision for each component family. No code is
imported into this repository before this audit completes per component.

The actual source of the Sentinel prototype is **not** vendored into `aevum-unify`.
This inventory operates by component family rather than file-by-file.

## Component families

| Family            | Known Sentinel surface                                     | Decision   | Justification                                                                                          | Risk    | First useful in | Tests required (negative)                                 |
|-------------------|------------------------------------------------------------|------------|--------------------------------------------------------------------------------------------------------|---------|------------------|-----------------------------------------------------------|
| Identity authority| OIDC, short-lived identity, mTLS                           | adopt      | Standard primitives. Compatible with SPIFFE (Local = Ed25519/X.509, Team = SPIRE optional).          | medium  | M2               | collision of identity, replay of token                   |
| Vault / secrets   | Encrypted vault, opaque references                         | adopt      | Pattern is sound; cap ABI on the new Secret Broker port.                                              | high    | M2               | exfiltration via log; export of handle value             |
| Capability registry| Catalog of capabilities, schema, leases                   | adopt      | Already structured; port it to `crates/capability-engine`.                                            | high    | M2               | unknown capability; scope drift on grant                 |
| Network guard     | Egress policy, allowed domains                            | adopt      | Reuse as Port; pin DNS resolution in policy.                                                          | high    | M2               | DNS rebinding; private IP leakage                        |
| File guard        | Path canonicalisation, allowed paths                       | rewrite    | Handle symlink race condition explicitly (TOCTOU), mount-based checks.                                 | high    | M2               | symlink race; path traversal; denied path (.env, .ssh)   |
| MCP server        | MCP surface                                                | adopt      | MCP is fine for tools; layer Aevum policy + scopes on top (ADR-0011).                                | medium  | M5               | prompt injection via tool result                        |
| Agentic loop      | Recursive LLM call with tool access                        | rewrite    | Must be replaced by the Cognitive Plane + Action Attestation flow. No direct tool call from the LLM. | critical| M3               | ungrounded authorization; missing audit trail            |
| **execute_command** | **`sh -c` string execution path**                         | **reject** | **Forbidden on the agentic path** (D14, ADR-0006). The capability API replaces this entire family.     | **critical** | —             | shell metacharacter in argv; command substitution         |
| Audit journal     | Signed append-only events                                  | adopt      | Map to Trust Ledger + hash chain.                                                                     | high    | M10              | truncated entry; re-ordered events; signature tampering   |
| Web / Axum surface| HTTP API + UI shell                                        | defer      | Replace with Mission Control (React) + Axum API in Team edition. UX rewrite is on the roadmap.        | low     | M9               | open redirect; unauthenticated endpoint                  |
| Observability     | Trace/log scaffolding                                      | adopt      | Conform to OpenTelemetry (ADR-0011).                                                                   | low     | M2               | PII in trace; missing correlation id                     |
| Configuration     | YAML / env based                                           | adopt      | Keep YAML; harden with schema validation.                                                              | low     | M0+              | unvalidated config injection                             |

## Rejected items (binding)

- **Sentinel `execute_command`** — REJECTED.
  Cannot be ported. Replaced entirely by typed capabilities. ADR-0006 captures this.

## Deferred items

- Axum web surface (UI shell) → replaced by Mission Control.
- Adapter-specific secrets YAML → replaced by Secret Broker handles.

## Process

```
Inventory → Threat Review → Contract Mapping → Minimal Port → Tests → Evidence → Adoption Decision
```

Every `adopt` item above is conditional on reaching the listed milestone with all
listed negative tests passing.

## References

- ADR-0001 — New monorepo `aevum-unify`
- ADR-0006 — No raw shell
- ADR-0007 — WASI / container / microVM tiers
- ADR-0008 — Signed Action Attestation
- ADR-0011 — MCP tools, A2A agents
- Blueprint §2, §16, §21
