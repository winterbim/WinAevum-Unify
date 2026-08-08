#!/usr/bin/env bash
# Adversarial re-falsification of P0 trust claims.
# Exit 0 only if every check detects failure / proves the hardened path.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo build -p aevum-unify --quiet
BIN="${UNIFY_BIN:-$ROOT/target/debug/unify}"
WORKDIR="${WORKDIR:-$(mktemp -d /tmp/aevum-refalsify-XXXXXX)}"
REPORT="$WORKDIR/REFALSIFY.md"
HUMAN_SK="$WORKDIR/human.sk"
export AEVUM_HUMAN_KEY="$HUMAN_SK"
export AEVUM_HUMAN_PUB="$WORKDIR/human.pub"

ok() { echo "PASS  $*"; echo "- [x] $*" >>"$REPORT"; }
die() { echo "FAIL  $*" >&2; echo "- [ ] FAIL: $*" >>"$REPORT"; exit 1; }

mkdir -p "$WORKDIR"
{
  echo "# P0 re-falsify — $(date -u +%Y-%m-%dT%H:%MZ)"
  echo
  echo "workdir: \`$WORKDIR\`"
  echo "unify: \`$BIN\`"
  echo
} >"$REPORT"

CONST="$WORKDIR/constitution.json"
cat >"$CONST" <<'JSON'
{
  "mission_id": "mis_refalsify",
  "objective": { "title": "refalsify", "description": "adversarial" },
  "scope": { "includes": ["*"], "excludes": [] },
  "risk": { "preliminary_class": "R2", "rationale": "test" },
  "evidence_required": ["repo_state"]
}
JSON

MISSION="$WORKDIR/mission"
"$BIN" new --constitution "$CONST" --out "$MISSION" >/dev/null

# P0-1
grep -q authority_secret_key_hex "$MISSION/metadata.json" && die "P0-1 metadata still has secret" || ok "P0-1 metadata has no secret field"
test -f "$MISSION/.aevum/authority.sk" || die "P0-1 missing authority.sk"
"$BIN" package --mission "$MISSION" --out "$WORKDIR/empty.json" >/dev/null
if grep -Eiq 'authority_secret|secret_key_hex' "$WORKDIR/empty.json"; then
  die "P0-1 package leaked secret"
else
  ok "P0-1 package has no secret material"
fi

# P0-2 forge
python3 "$ROOT/scripts/forge_package.py" "$WORKDIR/empty.json" "$WORKDIR/forged.json"
cp "$WORKDIR/empty.json.pubkey" "$WORKDIR/forged.json.pubkey"
if "$BIN" verify-package "$WORKDIR/forged.json" >/dev/null 2>&1; then
  die "P0-2 forged package accepted"
else
  ok "P0-2 forged/self-hash package rejected"
fi
cp "$WORKDIR/empty.json" "$WORKDIR/tampered.json"
cp "$WORKDIR/empty.json.pubkey" "$WORKDIR/tampered.json.pubkey"
python3 -c "
import json
from pathlib import Path
p = Path('$WORKDIR/tampered.json')
v = json.loads(p.read_text())
v['mission']['title'] = 'TAMPERED'
p.write_text(json.dumps(v, indent=2))
"
if "$BIN" verify-package "$WORKDIR/tampered.json" >/dev/null 2>&1; then
  die "P0-2 tampered package accepted"
else
  ok "P0-2 tampered package rejected"
fi

# P0-3
bash "$ROOT/scripts/ledger_mutation_matrix.sh" >/dev/null || die "P0-3 mutation matrix"
ok "P0-3 ledger mutation matrix 12/12"

# P0-4
if env -u AEVUM_MISSION "$BIN" pretool-check --capability process.exec.argv >/dev/null 2>&1; then
  die "P0-4 pretool allowed without mission"
else
  ok "P0-4 pretool fail-closed without mission"
fi

# P0-5
if "$BIN" graph authorize --mission "$MISSION" --capability secrets.read --reason x >/dev/null 2>&1; then
  die "P0-5 self-authorize accepted"
else
  ok "P0-5 authorize without --grant-sig refused"
fi
"$BIN" human-keygen --out "$HUMAN_SK" >/dev/null
SIG=$("$BIN" human-grant --mission-id mis_refalsify --capability secrets.read --reason "grant" --human-key "$HUMAN_SK")
"$BIN" graph authorize --mission "$MISSION" --capability secrets.read --reason "grant" --grant-sig "$SIG" \
  >/dev/null || die "P0-5 honest human grant failed"
ok "P0-5 human-grant path works"

# P0-6
NOW=$("$BIN" debug-now)
echo "$NOW" | grep -Eq 'Z$|[+]00:00$' || die "P0-6 clock not UTC ($NOW)"
ok "P0-6 debug-now emits UTC ($NOW)"
bash "$ROOT/scripts/crash_matrix.sh" >/dev/null || die "P0-6 crash matrix"
ok "P0-6 crash matrix 50/50"

# Deep: refuse corrupt ledger package
printf 'garbage\n' >"$MISSION/ledger.jsonl"
if "$BIN" package --mission "$MISSION" --out "$WORKDIR/should-fail.json" >/dev/null 2>&1; then
  die "deep: package accepted corrupt ledger"
else
  ok "deep: refuse to package corrupt ledger"
fi

# Deep: ledger/audit byte divergence
CONST2="$WORKDIR/c2.json"
python3 -c "
import json
from pathlib import Path
v=json.loads(Path('$CONST').read_text()); v['mission_id']='mis_div'; Path('$CONST2').write_text(json.dumps(v,indent=2))
"
M2="$WORKDIR/m2"
"$BIN" new --constitution "$CONST2" --out "$M2" >/dev/null
SIG2=$("$BIN" human-grant --mission-id mis_div --capability git.branch.create --reason "d" --human-key "$HUMAN_SK")
"$BIN" graph authorize --mission "$M2" --capability git.branch.create --reason "d" --grant-sig "$SIG2" >/dev/null
"$BIN" run --mission "$M2" --capability git.branch.create --argv "git checkout -b x" >/dev/null
python3 -c "
from pathlib import Path
p = Path('$M2') / 'audit_trail.jsonl'
p.write_text(p.read_text().replace('git.branch.create', 'git.branch.TAMPERED'))
"
if "$BIN" verify "$M2" >/dev/null 2>&1; then
  die "deep: verify accepted ledger/audit byte divergence"
else
  ok "deep: verify rejects ledger/audit byte divergence"
fi

{
  echo
  echo "## Verdict"
  echo
  echo "\`AEVUM_REFALSIFY_PASS\` — all P0 adversarial checks behaved as designed."
  echo
  echo "Scope: automated refalsify on this checkout (not a 20-agent human panel)."
} >>"$REPORT"

echo
echo "REPORT=$REPORT"
echo "AEVUM_REFALSIFY_PASS"
cat "$REPORT"
