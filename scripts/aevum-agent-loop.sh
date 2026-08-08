#!/usr/bin/env bash
# Native Aevum agent loop (Phase 3) — fully gated by temporal graph.
# propose → (optional falsify R3+) → authorize check → exec argv → slop → package
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

echo "==> Aevum native agent loop (gated)"
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

PKG="${MISSION}/agent-loop-pkg.json"
"$UNIFY" package --mission "$MISSION" --out "$PKG"
"$UNIFY" verify-package "$PKG"

echo "PASS aevum-agent-loop — package=$PKG (auto_merge=false)"
