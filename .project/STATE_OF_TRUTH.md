# Aevum Unify — STATE_OF_TRUTH

**Version :** 2026-08-02 M0 (verification closed)
**Authority :** Winter Fernandes
**Source :** `AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md`

## Loop state

| Loop          | Title                  | Status   | Evidence                                                     |
|---------------|------------------------|----------|--------------------------------------------------------------|
| AU-M00-L01    | repository truth       | closed   | `.project/tasks/AU-M00-L01/evidence-manifest.json`           |
| AU-M00-L02    | contracts package      | closed   | `packages/contracts/src/*.ts` + 4 vitest passing             |
| AU-M00-L04    | CI & evidence          | closed   | `.project/verification/M0/*.log` + ledger_check exit 0       |
| AU-M00-L05    | sentinel inventory     | closed   | `docs/migration/SENTINEL_INVENTORY.md`                       |

## Ce qui est vrai aujourd’hui

- Le blueprint Aevum Unify V1.0 est la source de vérité.
- Ce dépôt est un nouveau monorepo ; aucun code legacy n’a été copié en masse.
- Le prototype Sentinel contient un outil `execute_command` basé sur `sh -c` ; cette voie est **rejetée** pour le chemin agentique.
- Le dépôt est en phase M0 : structure, contrats, skeleton Rust/TS, CI.

## Décisions adoptées

| ADR | Décision | État |
|---|---|---|
| ADR-0001 | Nouveau monorepo `aevum-unify` | adopted |
| ADR-0002 | Rust pour le Kernel et les workers sensibles | adopted |
| ADR-0003 | TypeScript/React pour Mission Control | adopted |
| ADR-0004 | PostgreSQL canonique, SQLite local optionnel | adopted |
| ADR-0005 | Durable workflow via abstraction (Temporal-compatible) | adopted |
| ADR-0006 | OPA/Rego policy-as-code | adopted |
| ADR-0007 | Capabilities typées, pas de shell libre | adopted |
| ADR-0008 | WASI, conteneur rootless, microVM progressive | adopted |
| ADR-0009 | Action Attestation signée | adopted |
| ADR-0010 | Fournisseurs de modèles interchangeables | adopted |
| ADR-0011 | MCP pour outils, A2A pour agents | adopted |
| ADR-0012 | OpenTelemetry pour observabilité | adopted |

## Promises MVP mapping

| Promise | Statut |
|---|---|
| Mission Constitution versionnée | planned M1 |
| Council diversifié | planned M5 |
| Action Attestation | planned M3 |
| Sandbox sans shell libre | planned M2/M4 |
| PR sans merge automatique | planned M8 |
| Evidence Package vérifiable | planned M10 |

## Risques résiduels M0

- Aucune implémentation runtime réelle ; seuls des contrats et skeletons existent.
- Les fournisseurs de modèle, OPA, Temporal et SPIRE sont abstraits par des ports.
- Zéro test unitaire Rust aujourd'hui : la gate `cargo test` valide le harness mais
  pas la logique — la suite de tests est planifiée M1+ (voir `.project/LEDGER.md`
  ligne L-14 = `PENDING`).
- Le lockfile pnpm a été régénéré pour intégrer `@aevum/contracts` ; la prochaine
  boucle doit valider ce pattern en CI strict (work item à traiter en AU-M1).