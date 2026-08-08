#!/usr/bin/env bash
# P0-3 acceptance: every mutation of a signed ledger must fail `unify verify`.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UNIFY="${UNIFY_BIN:-$(cargo metadata --format-version 1 --no-deps --manifest-path "$ROOT/Cargo.toml" | python3 -c 'import sys,json; print(json.load(sys.stdin)["target_directory"])')/debug/unify}"
W=$(mktemp -d)
export W
trap 'rm -rf "$W"' EXIT

printf '%s\n' '{"mission_id":"mut_matrix","objective":{"title":"t","description":"d"},"scope":{"includes":["*"],"excludes":[]},"risk":{"preliminary_class":"R2","rationale":"t"},"evidence_required":["repo_state"]}' >"$W/c.json"
"$UNIFY" new --constitution "$W/c.json" --out "$W/m" >/dev/null
"$UNIFY" run --mission "$W/m" --capability git.branch.create --argv "git checkout -b a" >/dev/null
"$UNIFY" run --mission "$W/m" --capability git.branch.create --argv "git checkout -b b" >/dev/null
"$UNIFY" exec --mission "$W/m" --capability process.exec.argv --argv echo --argv hi >/dev/null
cp "$W/m/ledger.jsonl" "$W/baseline.ledger"
cp -a "$W/m/.aevum" "$W/baseline.aevum"

detected=0
silent=0

restore() {
  rm -rf "$W/m"
  "$UNIFY" new --constitution "$W/c.json" --out "$W/m" >/dev/null
  cp "$W/baseline.ledger" "$W/m/ledger.jsonl"
  cp "$W/baseline.ledger" "$W/m/audit_trail.jsonl"
  rm -rf "$W/m/.aevum"
  cp -a "$W/baseline.aevum" "$W/m/.aevum"
}

run_mut() {
  local name="$1"
  local py="$2"
  restore
  python3 -c "$py"
  if "$UNIFY" verify "$W/m" >/dev/null 2>&1; then
    echo "SILENT  $name"
    silent=$((silent + 1))
  else
    echo "DETECT  $name"
    detected=$((detected + 1))
  fi
}

run_mut M01_actor '
import json,os,pathlib
p=pathlib.Path(os.environ["W"])/"m/ledger.jsonl"
lines=p.read_text().splitlines()
e=json.loads(lines[0]); e["actor_id"]="ATTACKER"; lines[0]=json.dumps(e,separators=(",",":"))
t="\n".join(lines)+"\n"; p.write_text(t); (pathlib.Path(os.environ["W"])/"m/audit_trail.jsonl").write_text(t)
'
run_mut M02_ts '
import json,os,pathlib
p=pathlib.Path(os.environ["W"])/"m/ledger.jsonl"
lines=p.read_text().splitlines()
e=json.loads(lines[0]); e["occurred_at"]="1999-01-01T00:00:00Z"; lines[0]=json.dumps(e,separators=(",",":"))
t="\n".join(lines)+"\n"; p.write_text(t); (pathlib.Path(os.environ["W"])/"m/audit_trail.jsonl").write_text(t)
'
run_mut M03_attestation '
import json,os,pathlib
p=pathlib.Path(os.environ["W"])/"m/ledger.jsonl"
lines=p.read_text().splitlines()
e=json.loads(lines[1]); e["payload"]["attestation_id"]="FAKE"; lines[1]=json.dumps(e,separators=(",",":"))
t="\n".join(lines)+"\n"; p.write_text(t); (pathlib.Path(os.environ["W"])/"m/audit_trail.jsonl").write_text(t)
'
run_mut M04_sequence '
import json,os,pathlib
p=pathlib.Path(os.environ["W"])/"m/ledger.jsonl"
lines=p.read_text().splitlines()
e=json.loads(lines[1]); e["sequence"]=99; lines[1]=json.dumps(e,separators=(",",":"))
t="\n".join(lines)+"\n"; p.write_text(t); (pathlib.Path(os.environ["W"])/"m/audit_trail.jsonl").write_text(t)
'
run_mut M05_capability '
import json,os,pathlib
p=pathlib.Path(os.environ["W"])/"m/ledger.jsonl"
lines=p.read_text().splitlines()
e=json.loads(lines[0]); e["payload"]["capability"]="git.push"; lines[0]=json.dumps(e,separators=(",",":"))
t="\n".join(lines)+"\n"; p.write_text(t); (pathlib.Path(os.environ["W"])/"m/audit_trail.jsonl").write_text(t)
'
run_mut M06_argv_last '
import json,os,pathlib
p=pathlib.Path(os.environ["W"])/"m/ledger.jsonl"
lines=p.read_text().splitlines()
e=json.loads(lines[-1]); e["payload"]["argv"]="git push --force"; lines[-1]=json.dumps(e,separators=(",",":"))
t="\n".join(lines)+"\n"; p.write_text(t); (pathlib.Path(os.environ["W"])/"m/audit_trail.jsonl").write_text(t)
'
run_mut M07_prev_digest '
import json,os,pathlib
p=pathlib.Path(os.environ["W"])/"m/ledger.jsonl"
lines=p.read_text().splitlines()
e=json.loads(lines[1]); e["previous_digest"]="sha256:"+"0"*64; lines[1]=json.dumps(e,separators=(",",":"))
t="\n".join(lines)+"\n"; p.write_text(t); (pathlib.Path(os.environ["W"])/"m/audit_trail.jsonl").write_text(t)
'
run_mut M08_middle '
import json,os,pathlib
p=pathlib.Path(os.environ["W"])/"m/ledger.jsonl"
lines=p.read_text().splitlines()
e=json.loads(lines[1]); e["payload"]["argv"]="middle-tamper"; lines[1]=json.dumps(e,separators=(",",":"))
t="\n".join(lines)+"\n"; p.write_text(t); (pathlib.Path(os.environ["W"])/"m/audit_trail.jsonl").write_text(t)
'
run_mut M09_last_actor '
import json,os,pathlib
p=pathlib.Path(os.environ["W"])/"m/ledger.jsonl"
lines=p.read_text().splitlines()
e=json.loads(lines[-1]); e["actor_id"]="LAST"; lines[-1]=json.dumps(e,separators=(",",":"))
t="\n".join(lines)+"\n"; p.write_text(t); (pathlib.Path(os.environ["W"])/"m/audit_trail.jsonl").write_text(t)
'
run_mut M10_delete_middle '
import os,pathlib
p=pathlib.Path(os.environ["W"])/"m/ledger.jsonl"
lines=p.read_text().splitlines(); del lines[1]
t="\n".join(lines)+"\n"; p.write_text(t); (pathlib.Path(os.environ["W"])/"m/audit_trail.jsonl").write_text(t)
'
run_mut M11_duplicate_last '
import os,pathlib
p=pathlib.Path(os.environ["W"])/"m/ledger.jsonl"
lines=p.read_text().splitlines(); lines.append(lines[-1])
t="\n".join(lines)+"\n"; p.write_text(t); (pathlib.Path(os.environ["W"])/"m/audit_trail.jsonl").write_text(t)
'
run_mut M12_permute '
import os,pathlib
p=pathlib.Path(os.environ["W"])/"m/ledger.jsonl"
lines=p.read_text().splitlines(); lines[0],lines[1]=lines[1],lines[0]
t="\n".join(lines)+"\n"; p.write_text(t); (pathlib.Path(os.environ["W"])/"m/audit_trail.jsonl").write_text(t)
'

echo "RESULT detected=${detected} silent=${silent} total=$((detected+silent))"
if [[ "$detected" -ge 12 && "$silent" -eq 0 ]]; then
  echo "PASS ${detected}/$((detected+silent)) detected, 0 silent"
  exit 0
fi
echo "FAIL need >=12 detected and 0 silent"
exit 1
