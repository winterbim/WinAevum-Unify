#!/usr/bin/env bash
# Acceptance helper for PreToolUse fail-closed (P0-4).
# Usage:
#   ./hook-test.sh                         # no mission → expect DENY
#   ./hook-test.sh --mission DIR CAP       # check capability CAP → expect DENY if unauthorized
#   ./hook-test.sh --mission DIR "bash -lc 'x'"  # shell string → expect DENY
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UNIFY="${UNIFY_BIN:-$(cargo metadata --format-version 1 --no-deps --manifest-path "$ROOT/Cargo.toml" | python3 -c 'import sys,json; print(json.load(sys.stdin)["target_directory"])')/debug/unify}"
export UNIFY_BIN="$UNIFY"

MISSION=""
POS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --mission) MISSION="$2"; shift 2 ;;
    *) POS+=("$1"); shift ;;
  esac
done

ARG="${POS[0]:-}"
if [[ -n "$MISSION" ]]; then
  export AEVUM_MISSION="$MISSION"
else
  unset AEVUM_MISSION || true
fi

# Capability-style token (contains a dot, no spaces) → call pretool-check directly.
if [[ -n "$ARG" && "$ARG" == *.* && "$ARG" != *" "* ]]; then
  OUT=$("$UNIFY" pretool-check --mission "${MISSION:-}" --capability "$ARG" --tool Bash 2>/dev/null || true)
  # unify prints JSON on stdout even when exiting 1
  echo "$OUT" | tail -1
  echo "$OUT" | tail -1 | python3 -c 'import sys,json; d=json.load(sys.stdin); sys.exit(0 if d.get("decision")=="deny" else 1)'
  exit $?
fi

CMD="${ARG:-echo hi}"
PAYLOAD=$(python3 -c 'import json,sys; print(json.dumps({"tool_name":"Bash","tool_input":{"command":sys.argv[1]}}))' "$CMD")
OUT=$(echo "$PAYLOAD" | python3 "$ROOT/tools/claude-pretooluse/pretool_authorize.py")
echo "$OUT"
echo "$OUT" | python3 -c 'import sys,json; d=json.load(sys.stdin); sys.exit(0 if d.get("decision")=="deny" else 1)'
