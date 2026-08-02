# RISK_REGISTER — Aevum Unify

**Version :** 2026-08-02 M0
**Authority :** Winter Fernandes
**Source :** AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §21

Format : `R-XXX | Threat | Asset | Class | Mitigation | Owner | Status`

| ID      | Threat                                                          | Asset                | Class   | Mitigation                                                                                           | Owner            | Status   |
|---------|-----------------------------------------------------------------|----------------------|---------|------------------------------------------------------------------------------------------------------|------------------|----------|
| R-001   | Agent path uses `sh -c` (Sentinel legacy `execute_command`)      | Code execution       | Critical| Reject import path; typed capability API only (ADR-0006). Negative tests covering shell metachars.    | Kernel team      | accepted |
| R-002   | Model prompt drives authorization                               | Policy               | Critical| OPA/Rego decision outside the LLM; attestation references a `policy_bundle_digest` (ADR-0005).        | Governor team    | accepted |
| R-003   | Secret value leaks into prompt / logs / receipt                 | Secrets              | Critical| Secret Broker; opaque handles; redaction filter on telemetry; non-exportable handles (ADR).          | Secret Broker    | accepted |
| R-004   | Scope drift during mission                                       | Mission scope        | High    | Versioned Mission Constitution; scope diff before any patch (D02).                                   | Mission service  | accepted |
| R-005   | Producer validates own work (D10 violated)                       | Decision integrity   | High    | Independent verifier agent; council quorum with minority objection surfaced.                         | Council Fabric   | accepted |
| R-006   | Stale approval / policy between prepare and commit               | Authority            | High    | Commit Gate re-evaluates signature, version, freshness (D16).                                         | Kernel           | accepted |
| R-007   | Prompt injection from repository / docs / web pages              | LLM reasoning        | High    | All external content tagged `untrusted` / `embedded_instruction`; cannot modify Constitution/policy. | Cognitive Plane  | accepted |
| R-008   | Attestation replay                                              | Authority            | High    | Nonce + replay cache; single-use execution lease; `expires_at` enforced.                             | Attestation      | accepted |
| R-009   | Ledger tampering                                                | Audit trail          | High    | Append-only hash chain; Ed25519 signatures; content-addressed artefacts (ADR-0012).                   | Ledger           | accepted |
| R-010   | Resource exhaustion (fork bomb, large stdout)                   | Local host           | Medium  | Hard caps: CPU, memory, disk, processes, stdout. Workspace diff capture.                              | Execution Fabric | accepted |
| R-011   | Cross-tenant data leakage                                        | Multi-tenant data    | High    | `tenant_id` mandatory on every critical event; Row Level Security on SaaS edition.                    | Data layer       | accepted |
| R-012   | Cost runaway (tokens, retries, infinite loops)                   | Budget               | Medium  | Mission-level budgets (money/time/tokens/cpu); circuit breaker; D22.                                  | Governor         | accepted |
| R-013   | Egress bypass (DNS rebinding, open HTTP)                         | Network              | High    | Default-deny network; domain+IP+SNI checks; HTTP egress proxy.                                       | Execution Fabric | accepted |
| R-014   | Confused deputy (proxy impersonation)                           | MCP / A2A            | Medium  | Token audience claim; per-tool scopes; A2A Agent Card carries trust domain.                          | Connector layer  | accepted |
| R-015   | Supply chain compromise                                         | Bins / images        | High    | SBOM, image pinning by digest, signature verification (Cosign/in-toto, ADR-0012).                    | Release          | accepted |
| R-016   | Mission resume after partial effect                              | State                | Medium  | Workflow engine replay; Receipt append-only; compensation/rollback documented.                       | Workflow         | accepted |
| R-017   | Policy bundle drift                                             | Policy               | High    | Bundles signed and versioned; attestation references bundle digest.                                   | Governor         | accepted |
| R-018   | Reputation gaming / vendor lock-in                              | Agent selection      | Medium  | Vector reputation per domain; provider port (ADR-0009); diversity gate.                              | Council Fabric   | accepted |
| R-019   | Free-text reasoning exposed in traces                           | Privacy              | Medium  | Spans carry digests and justifications only; no full prompts/results by default.                     | Telemetry        | accepted |
| R-020   | Single model populating producer AND verifier                   | Independence         | High    | Diversity Gate (M5): model/family/prompt/source diversity.                                            | Council Fabric   | accepted |

## Open risks without technical mitigation in M0

| ID      | Issue                                                 | Action                                            |
|---------|-------------------------------------------------------|---------------------------------------------------|
| R-101   | No real runtime implementation yet                   | Plan M1–M4 to land Core capability & policy ports |
| R-102   | No actual model provider wired                       | Land `ModelProvider` port in M0+1                |
| R-103   | No Sentinel source present in this repo              | Inventory + threat review completed (document)    |

See `.project/STATE_OF_TRUTH.md` "Risques résiduels M0" for the short summary version.
