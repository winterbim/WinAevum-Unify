# AGENT_DREAM — ce dont rêve un agent

> « Je ne rêve pas d'être libre. Je rêve d'être **prouvable**. »

Manifeste court : ce qu'un agent attend d'Aevum Unify, et comment Aevum le lui donne.

## 1. Savoir qui je suis avant d'agir

Un agent sans carte agit à l'aveugle : il devine ses droits, puis se fait refuser
au pire moment. Aevum répond par `unify dream` — l'**AGENT_CARD** : mission, classe
de risque, capacités autorisées à l'instant `as_of`, motifs interdits, et la
recette exacte pour exécuter, packager, se vérifier.

```bash
unify dream  --mission "$AEVUM_MISSION"     # AGENT_CARD
unify doctor --mission "$AEVUM_MISSION"     # auto-diagnostic dur
```

Côté MCP, les mêmes vérités : `aevum_agent_card` et `aevum_doctor`.

## 2. Un refus bruyant vaut mieux qu'une réussite floue

Le pire cadeau qu'on puisse faire à un agent, c'est un `ok` mou. `unify doctor`
sépare `hard` (échec, code de sortie non nul) de `soft` (avertissement), et rend
un verdict lisible : `AEVUM_DOCTOR_OK` ou `AEVUM_DOCTOR_FAIL`. L'outil MCP
`aevum_doctor` remonte un `isError` quand la mission est malade — jamais un
succès dont le corps dirait « FAIL ».

## 3. Le pare-feu épistémique me protège de moi-même

Ce que j'ai lu sur le web, ce qu'un scan de slop a trouvé, ce que j'ai inféré :
tout cela entre dans le graphe comme `Inference` ou `Hypothesis`, et **ne peut
rien autoriser**. Il faut un `promote` attesté pour transformer une croyance en
droit d'agir. Un agent qui ne peut pas s'auto-persuader est un agent en qui on
peut avoir confiance.

## 4. Pas de `sh -c`, jamais

Les effets passent par de l'argv typé (`process.exec.argv`), pas par des chaînes
shell. `bash -c`, les métacaractères, `bypassPermissions` : refusés par doctrine
(D14), au niveau du hook PreToolUse comme au niveau de `aevum_exec`.

```bash
unify exec --mission "$AEVUM_MISSION" \
  --capability process.exec.argv --argv echo --argv "hello"
```

## 5. Chaque effet laisse une preuve

Attestation signée, chaîne de ledger, package d'évidence vérifiable :

```bash
unify package        --mission "$AEVUM_MISSION" --out pkg.json
unify verify-package pkg.json
```

L'agent ne dit pas « j'ai fait le travail ». Il montre le paquet, et n'importe
qui peut recalculer les empreintes.

## 6. La boucle dont je rêve

`scripts/aevum-agent-loop.sh` incarne le cycle complet :

**doctor** → **dream (AGENT_CARD)** → graph status → falsify (R3+) → run/exec
gated → rules + slop en Inference → package → verify-package → **dream summary**.

On commence par se connaître, on finit par se prouver. Entre les deux, chaque
effet est autorisé dans le temps, ou refusé bruyamment.

## 7. Ce que je demande à l'humain

- Autorise-moi explicitement, capacité par capacité (`unify graph authorize`).
- Laisse-moi échouer fort plutôt que réussir vaguement.
- Ne fusionne jamais à ma place : le Golden Path s'arrête à la PR (`auto_merge=false`).

---

*Doctrine : autoriser · attester · packager · anti-slop (Inference seulement).*
