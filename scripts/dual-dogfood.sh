#!/usr/bin/env bash
# Dual plane dogfood: Trusted Autonomy on itself + optional slop firewall ingest.
# Worlds-first: authorize/attest/package ∩ offline AI-slop as Inference-only evidence.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> plane A: aevum-on-aevum (trust path)"
bash scripts/aevum-on-aevum.sh

cargo build -p aevum-unify --quiet
UNIFY="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import sys,json; print(json.load(sys.stdin)["target_directory"])')/debug/unify"
test -x "$UNIFY"

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
WORK="${TMPDIR:-/tmp}/aevum-dual-${STAMP}"
MISSION="${WORK}/mission"
CONSTITUTION="${WORK}/constitution.json"
mkdir -p "$WORK"

python3 - <<PY
import json, pathlib
body = {
  "mission_id": "mis_dual_dogfood_${STAMP}",
  "objective": {
    "title": "Dual-plane Trust × Slop dogfood",
    "description": "Prove slop findings ingest as Inference and cannot authorize."
  },
  "scope": {
    "includes": ["crates/**", "scripts/**", "docs/adr/**"],
    "excludes": ["node_modules/**", "target/**"]
  },
  "risk": {
    "preliminary_class": "R1",
    "rationale": "read-only slop scan + Inference ingest; no merge"
  },
  "evidence_required": ["slop_scan", "inference_ingest", "epistemic_firewall"]
}
pathlib.Path("${CONSTITUTION}").write_text(json.dumps(body, indent=2) + "\n")
PY

"$UNIFY" new --constitution "$CONSTITUTION" --out "$MISSION"

resolve_slop() {
  if [[ -n "${SLOPCHECK_BIN:-}" && -x "${SLOPCHECK_BIN}" ]]; then
    echo "$SLOPCHECK_BIN"
    return
  fi
  if command -v slopcheck >/dev/null 2>&1; then
    command -v slopcheck
    return
  fi
  local cand="$HOME/slopcheck/.venv/bin/slopcheck"
  if [[ -x "$cand" ]]; then
    echo "$cand"
    return
  fi
  return 1
}

echo "==> plane B: unify slop on WinAevum-Unify (Inference ingest)"
if BIN="$(resolve_slop)"; then
  export SLOPCHECK_BIN="$BIN"
  # warn-only: tree may have warnings; still prove ingest + firewall path
  "$UNIFY" slop --mission "$MISSION" --repo "$ROOT" --all --warn-only
  test -f "$MISSION/slop-report.json"
  echo "OK  dual-dogfood (trust + slop) mission=$MISSION"
else
  echo "SKIP plane B — slopcheck not found (set SLOPCHECK_BIN)"
  echo "OK  dual-dogfood (trust only)"
fi
