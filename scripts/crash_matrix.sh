#!/usr/bin/env bash
# P0-6: SIGKILL during ledger write must never leave silent corruption
# (doctor agent_ready:true on a broken ledger).
set -euo pipefail
N="${1:-50}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UNIFY="${UNIFY_BIN:-$(cargo metadata --format-version 1 --no-deps --manifest-path "$ROOT/Cargo.toml" | python3 -c 'import sys,json; print(json.load(sys.stdin)["target_directory"])')/debug/unify}"
W=$(mktemp -d)
trap 'rm -rf "$W"' EXIT

printf '%s\n' '{"mission_id":"crash_m","objective":{"title":"t","description":"d"},"scope":{"includes":["*"],"excludes":[]},"risk":{"preliminary_class":"R2","rationale":"t"},"evidence_required":["repo_state"]}' >"$W/c.json"
"$UNIFY" new --constitution "$W/c.json" --out "$W/m" >/dev/null
# Pad ledger so writes are non-trivial
for i in $(seq 1 40); do
  "$UNIFY" exec --mission "$W/m" --capability process.exec.argv --argv echo --argv "pad$i" >/dev/null || true
done

silent=0
intact_or_detected=0
for i in $(seq 1 "$N"); do
  cp -a "$W/m" "$W/try"
  # Race: start exec and kill quickly
  "$UNIFY" exec --mission "$W/try" --capability process.exec.argv --argv echo --argv "race$i" >/dev/null 2>&1 &
  pid=$!
  # sample kill delays
  sleep "0.00$((i % 9))"
  kill -9 "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true

  doc=$("$UNIFY" doctor --mission "$W/try" 2>/dev/null || true)
  ready=$(echo "$doc" | python3 -c 'import sys,json,re; t=sys.stdin.read();
import json as J
m=re.search(r"\{[\s\S]*\}", t)
ready=True
if m:
  try: ready=J.loads(m.group(0)).get("agent_ready", True)
  except: pass
print("true" if ready else "false")' 2>/dev/null || echo true)

  # Corrupt = unparseable line or verify fails
  if "$UNIFY" verify "$W/try" >/dev/null 2>&1; then
    intact_or_detected=$((intact_or_detected + 1))
  else
    if [[ "$ready" == "true" ]]; then
      echo "SILENT_CORRUPT iteration=$i doctor_agent_ready=true"
      silent=$((silent + 1))
    else
      intact_or_detected=$((intact_or_detected + 1))
    fi
  fi
  rm -rf "$W/try"
done

echo "RESULT intact_or_detected=${intact_or_detected}/${N} silent_corruption=${silent}"
if [[ "$silent" -eq 0 && "$intact_or_detected" -eq "$N" ]]; then
  echo "PASS 0 silent corruption"
  exit 0
fi
echo "FAIL"
exit 1
