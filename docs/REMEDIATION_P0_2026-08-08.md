# Rapport de remédiation — 2026-08-08

Branche : `fix/p0-security`  
Audit source : `docs/AUDIT_ADVERSARIAL_2026-08-08.md`

## P0-1 — Clé privée hors metadata / package

**Statut :** critère d'acceptation satisfait (preuve locale).

```text
$ unify package --mission … --out /tmp/pkg-p0.json
✓ package written … (ed25519 signature; trust pubkey → /tmp/pkg-p0.json.pubkey)
$ grep -c "secret" /tmp/pkg-p0.json
0
$ find …/mission -name metadata.json -exec stat -c "%a %n" {} \;
600 …/mission/metadata.json
```

Secret : `{mission}/.aevum/authority.sk` (600).

## P0-2 — Signature Ed25519 du package

**Statut :** critère d'acceptation satisfait.

```text
$ unify verify-package /tmp/pkg-p0.json
✓ evidence package verified — mission: p0_acc
  signature:  ed25519 (trust pubkey from /tmp/pkg-p0.json.pubkey)
legit_rc=0

$ python3 scripts/forge_package.py /tmp/pkg-p0.json /tmp/pkg-forged.json
$ unify verify-package /tmp/pkg-forged.json
… missing package_signature — self-hash package_digest is not accepted (P0-2)
forge_rc=1

$ cp /tmp/pkg-p0.json.pubkey /tmp/pkg-forged.json.pubkey
$ unify verify-package /tmp/pkg-forged.json
… missing package_signature …
forge_with_sidecar_rc=1
```

## P0-3 — Ledger signé tous champs + tip

**Statut :** critère d'acceptation satisfait.

```text
$ bash scripts/ledger_mutation_matrix.sh
DETECT  M01_actor … M12_permute
RESULT detected=12 silent=0 total=12
PASS 12/12 detected, 0 silent
```

## P0-4 — PreToolUse fail-closed

**Statut :** critère d'acceptation satisfait.

```text
$ bash scripts/hook-test.sh
{"decision": "deny", "reason": "no AEVUM_MISSION — fail-closed (P0-4)"}

$ bash scripts/hook-test.sh --mission …/mission-empty secrets.read
{"decision":"deny","reason":"… secrets.read is not authorized …"}

$ for cmd in "bash -lc 'echo x'" "ksh -lc 'echo x'" "env BASH_ENV=/tmp/x bash -i"; do
    bash scripts/hook-test.sh --mission … "$cmd"   # deny each
  done
```

## P0-5 — Interdiction du self-authorize

**Statut :** critère d'acceptation satisfait.

V1 distinct principal : clé humaine hors mission (`unify human-keygen` →
`~/.config/aevum/human.sk` ou `$AEVUM_HUMAN_KEY`). `graph authorize` exige
`--grant-sig` produit par `unify human-grant`.

```text
$ unify graph authorize --mission … --capability secrets.read --reason "auto-octroi, aucun humain"
… refuses self-authorize (P0-5) …
auth_rc=1
$ unify exec --mission … --capability secrets.read …
… not authorized …
exec_rc=1
```

## P0-6 — Horloge + écritures atomiques + doctor

**Statut :** critère d'acceptation satisfait.

```text
$ date -u +%Y-%m-%dT%H:%M:%S
2026-08-08T18:49:55
$ unify debug-now
2026-08-08T18:49:55Z

$ bash scripts/crash_matrix.sh 50
RESULT intact_or_detected=50/50 silent_corruption=0
PASS 0 silent corruption
```

## Modifications de tests (signalées)

Les tests suivants ont été adaptés **parce que le contrat de sécurité a changé**,
pas pour masquer une régression :

- `package_digest` → `package_signature` (unit + CLI + ATB-04)
- ledger fields → `LedgerEntry` (`payload.capability`, `previous_digest`, `signature`)
- `graph authorize` → `--grant-sig` (CLI + ATB-07)
- Verdict ATB `AEVUM_PERFECT` → `AEVUM_SELF_RUN_PASS`

## Revérification indépendante

**Non faite.** Aucune session tierce / clone frais n'a relancé les 20 agents.
**Aucune P0 n'est validée pour publication** tant que cette étape n'est pas faite.

## Nettoyage du langage

- README : retrait de « PERFECT » / « offline by default » absolu ; benches marquées self-run
- LEDGER : L-37, L-38 → `CLAIMED` (downgrade)
- ATB verdict → `AEVUM_SELF_RUN_PASS`
- Ajouts : `SECURITY.md`, `LIMITATIONS.md`

## Gate de publication

| Condition | État |
|---|---|
| 6 P0 avec preuves collées | Oui (cette session) |
| Revérification indépendante | **Non** |
| Langage nettoyé | Oui (partiel ; scorecards historiques peuvent encore dire PERFECT) |
| SECURITY.md + LIMITATIONS.md | Oui |
| Tag / publish | **NO-GO** |

**Décision : pas de publication, pas de nouveau tag.**
