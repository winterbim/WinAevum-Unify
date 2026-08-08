#!/usr/bin/env bash
# Native Aevum agent loop (Phase 3) — fully gated by temporal graph.
# doctor → dream (AGENT_CARD) → propose → (falsify R3+) → authorize check
#        → exec argv → rules/slop as Inference → package → verify → dream summary
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

UNIFY="${UNIFY_BIN:-}"
if [[ -z "$UNIFY" ]]; then
  cargo build -p aevum-unify --quiet
  UNIFY="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import sys,json; print(json.load(sys.stdin)["target_directory"])')/debug/unify"
fi
test -x "$UNIFY"

MISSION="${1:?usage: aevum-agent-loop.sh <mission-dir> [repo]}"
REPO="${2:-.}"
export SLOPCHECK_BIN="${SLOPCHECK_BIN:-${HOME}/.local/bin/slopcheck}"
CARD="${MISSION}/agent-card.json"
PKG="${MISSION}/agent-loop-pkg.json"

echo "==> Aevum native agent loop (gated)"

# [0] Self-check first: a sick mission must fail loudly, before any effect.
echo "--> doctor (hard self-check)"
"$UNIFY" doctor --mission "$MISSION"

# [1] AGENT_CARD: what this agent is allowed to be, before it tries to act.
echo "--> dream (AGENT_CARD)"
"$UNIFY" dream --mission "$MISSION" >"$CARD"
echo "AGENT_CARD ($CARD):"
cat "$CARD"

echo "--> graph status"
"$UNIFY" graph status --mission "$MISSION"

# Falsify if R3+ (idempotent challenge record)
RISK="$(python3 - <<PY
import json
from pathlib import Path
m=json.loads(Path("$MISSION/metadata.json").read_text())
print(m.get("mission",{}).get("risk","R2"))
PY
)"
if [[ "$RISK" == R3 || "$RISK" == R4 || "$RISK" == R5 ]]; then
  "$UNIFY" falsify --mission "$MISSION" --reason "agent-loop: independent challenge recorded"
fi

# Attest + safe argv exec (never sh -c)
"$UNIFY" run --mission "$MISSION" --capability process.exec.argv --argv "echo aevum-agent-loop"
"$UNIFY" exec --mission "$MISSION" --capability process.exec.argv \
  --argv echo --argv "aevum-agent-loop-ok"

# Rules + slop as Inference
"$UNIFY" rules scan --mission "$MISSION" --repo "$REPO" || true
if [[ -x "${SLOPCHECK_BIN}" ]] || command -v slopcheck >/dev/null 2>&1; then
  "$UNIFY" slop --mission "$MISSION" --repo "$REPO" --all --warn-only || true
fi

# [n-1] Evidence: build the package and prove its digests.
"$UNIFY" package --mission "$MISSION" --out "$PKG"
"$UNIFY" verify-package "$PKG"

# [n] Close the loop with the post-effect card: what changed about the agent.
echo "--> dream summary (post-effect)"
"$UNIFY" dream --mission "$MISSION" >"$CARD"
DOCTOR_RAW="${MISSION}/.doctor-out.txt"
"$UNIFY" doctor --mission "$MISSION" >"$DOCTOR_RAW"
python3 - <<PY
import json
from pathlib import Path

# doctor prints its JSON report followed by a human line — take the object.
raw = Path("$DOCTOR_RAW").read_text().lstrip()
doc, _ = json.JSONDecoder().raw_decode(raw)
Path("$MISSION/doctor-report.json").write_text(json.dumps(doc, indent=2) + "\n")

card = json.loads(Path("$CARD").read_text())
caps = card.get("authorized_capabilities", [])
print("DREAM SUMMARY")
print(f"  mission      : {card.get('mission_id')} (risk={card.get('risk')})")
print(f"  as_of        : {card.get('as_of')}")
print(f"  authorized   : {len(caps)} -> {', '.join(caps) if caps else '(none)'}")
print(f"  doctor       : {doc.get('verdict', 'unknown')} "
      f"(hard={len(doc.get('hard', []))} soft={len(doc.get('soft', []))})")
print(f"  card         : $CARD")
print(f"  package      : $PKG")
PY
rm -f "$DOCTOR_RAW"

echo "PASS aevum-agent-loop — package=$PKG card=$CARD (auto_merge=false)"
