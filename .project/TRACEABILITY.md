# TRACEABILITY — Aevum Unify

**Version :** 2026-08-02 M0
**Source :** Blueprint Annexe E + `.project/REQUIREMENTS.json`

Format : `Requirement | Component(s) | Primary Test | Evidence location`

| Requirement     | Components                                            | Primary Test                    | Evidence                                        |
|-----------------|-------------------------------------------------------|---------------------------------|-------------------------------------------------|
| AU-TRUTH-001    | Evidence Graph (planned), `crates/attestation`        | schema round-trip (planned M6)  | `crates/attestation/src/lib.rs`                 |
| AU-SCOPE-001    | Mission service (planned M1), `STATE_OF_TRUTH`        | version mutation test (M1)      | `.project/STATE_OF_TRUTH.md` §Doctrine          |
| AU-SEP-001      | Council Fabric (planned M5)                           | council conflict fixture (M5)   | `.project/RISK_REGISTER.md` R-005               |
| AU-EXEC-001     | `crates/sentinel-kernel`, `crates/capability-engine`  | API compile-time negative test  | `docs/adr/ADR-0006-no-raw-shell.md`            |
| AU-POL-001      | Commit Gate (planned M3), `crates/attestation`        | stale state fixture (M3)        | `docs/adr/ADR-0008-signed-action-attestation.md`|
| AU-SEC-001      | `crates/secret-broker` (skeleton M0)                  | export denial test (M2)         | `crates/secret-broker/src/lib.rs`               |
| AU-AUD-001      | `crates/ledger` (skeleton M0)                         | tamper fixture (M10)            | `docs/adr/ADR-0012-content-addressed-evidence.md`|
| AU-REC-001      | Execution Fabric (planned M4)                         | failure injection (M4)          | `docs/adr/ADR-0007-wasi-container-microvm.md`   |
| AU-COST-001     | Autonomy Governor (planned M7)                        | budget exhaustion (M7)          | `.project/RISK_REGISTER.md` R-012               |
| AU-MVP-001      | GitHub adapter (planned M8)                           | E2E fixture (M8)                | `.project/RISK_REGISTER.md` R-001               |

## Requirement → Blueprint references

| Requirement     | Blueprint refs                          |
|-----------------|------------------------------------------|
| AU-TRUTH-001    | §11 Claim contract                       |
| AU-SCOPE-001    | §10 Mission Constitution                 |
| AU-SEP-001      | §14 Council Fabric, D10                  |
| AU-EXEC-001     | §16.4 No raw shell, D14                  |
| AU-POL-001      | §12 Action Attestation, D16              |
| AU-SEC-001      | §16.2 Secret Broker, D18                 |
| AU-AUD-001      | §19 Trust Ledger                         |
| AU-REC-001      | §17 Execution Fabric, D17                |
| AU-COST-001     | §22.5 Budgets, D22                       |
| AU-MVP-001      | §36 MVP acceptance, interdiction §32.1  |

## Doctrinal anchors

Every requirement in this matrix maps to at least one doctrinal invariant (D01–D24) and
one Architecture Decision Record (ADR-0001 to ADR-0012). The matrix in
`docs/adr/ADR-*.md` cross-references both. A requirement rejected by these anchors must
not be implemented (D21).
