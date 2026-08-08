#!/usr/bin/env bash
# aevum-on-aevum — dogfood Aevum Unify against its own repository.
# Exit 0 only if every gate below passes.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
WORK="${TMPDIR:-/tmp}/aevum-aoa-${STAMP}"
MISSION="${WORK}/mission"
PKG="${WORK}/pkg.json"
CONSTITUTION="${WORK}/constitution.json"
BRANCH="aevum/self-test-${STAMP}"
REPORT="${WORK}/REPORT.md"

mkdir -p "$WORK"
echo "==> workdir: $WORK"

# ── 0. Build trust anchor ──────────────────────────────────────────────────
echo "==> cargo build -p aevum-unify"
cargo build -p aevum-unify --quiet
UNIFY="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import sys,json; print(json.load(sys.stdin)["target_directory"])')/debug/unify"
test -x "$UNIFY"
echo "    unify: $UNIFY"
"$UNIFY" --version | tee "${WORK}/version.txt"

# ── 1. Inspect real repo (facts) ───────────────────────────────────────────
RUST_VER="$(python3 - <<'PY'
import re, pathlib
t = pathlib.Path("Cargo.toml").read_text()
m = re.search(r'rust-version\s*=\s*"([^"]+)"', t)
print(m.group(1) if m else "UNKNOWN")
PY
)"
ADR013="docs/adr/ADR-0013-temporal-evidence-graph.md"
test -f "$ADR013"
TEST_CRATES="$(ls crates | wc -l | tr -d ' ')"
HAS_TEMPORAL="$(grep -l 'TemporalGraph' crates/evidence-graph/src/lib.rs || true)"
test -n "$HAS_TEMPORAL"

echo "    rust-version=$RUST_VER crates=$TEST_CRATES adr013=present"

# ── 2. Mission constitution targeting THIS repo ────────────────────────────
python3 - <<PY
import json, pathlib
body = {
  "mission_id": "mis_aevum_on_aevum_${STAMP}",
  "objective": {
    "title": "Aevum-on-Aevum self-test",
    "description": "Exercise unify CLI + TemporalGraph against the aevum-unify monorepo itself."
  },
  "scope": {
    "includes": ["crates/**", "packages/contracts/**", "docs/adr/**", "scripts/**"],
    "excludes": ["node_modules/**", "target/**", "**/*.snap"]
  },
  "risk": {
    "preliminary_class": "R2",
    "rationale": "side-branch only; no merge; no secrets; no deploy"
  },
  "evidence_required": ["repo_state", "tests_pass", "ledger_verify", "deny_shell", "temporal_graph"]
}
pathlib.Path("${CONSTITUTION}").write_text(json.dumps(body, indent=2) + "\n")
print("wrote ${CONSTITUTION}")
PY

# ── 3. unify new / run / exec / verify / package ───────────────────────────
"$UNIFY" new --constitution "$CONSTITUTION" --out "$MISSION"
"$UNIFY" graph status --mission "$MISSION"
"$UNIFY" graph search --mission "$MISSION" --query "constitution authorizes"
"$UNIFY" run --mission "$MISSION" --capability git.branch.create --argv "git checkout -b ${BRANCH}"

# Real git side-branch on this repo (reversible)
BEFORE_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
echo "    current branch: $BEFORE_BRANCH"
"$UNIFY" exec --mission "$MISSION" --capability process.exec.argv \
  --argv git --argv -C --argv "$ROOT" --argv checkout --argv -b --argv "$BRANCH"

# Prove branch exists
git -C "$ROOT" branch --list "$BRANCH" | grep -q "$BRANCH"

# Deny sh -c (must fail)
set +e
"$UNIFY" exec --mission "$MISSION" --capability process.exec.argv \
  --argv sh --argv -c --argv 'echo pwned' >"${WORK}/deny.out" 2>"${WORK}/deny.err"
DENY_EC=$?
set -e
test "$DENY_EC" -ne 0
grep -qi 'denied\|rejected\|sh -c' "${WORK}/deny.err" "${WORK}/deny.out" || \
  grep -qi 'denied\|rejected\|sh -c' "${WORK}/deny.err"

# Run evidence-graph + contracts tests as attested evidence
cargo test -p aevum-evidence-graph --quiet 2>&1 | tee "${WORK}/evidence-graph-tests.log"
pnpm --filter @aevum/contracts exec vitest run --pool=threads src/temporal-graph.test.ts 2>&1 | tee "${WORK}/contracts-temporal.log"

"$UNIFY" verify "$MISSION" | tee "${WORK}/verify.out"
grep -qi 'verified\|links' "${WORK}/verify.out"

"$UNIFY" package --mission "$MISSION" --out "$PKG"
"$UNIFY" verify-package "$PKG"

# Tamper reject
python3 - <<PY
import json, pathlib
p = pathlib.Path("${PKG}")
data = json.loads(p.read_text())
# mutate a visible field if present
if "mission" in data and isinstance(data["mission"], dict):
    data["mission"]["mission_id"] = data["mission"].get("mission_id","x") + "-TAMPERED"
elif "title" in data:
    data["title"] = str(data.get("title","")) + "-TAMPERED"
else:
    data["_tamper"] = True
pathlib.Path("${WORK}/pkg-tampered.json").write_text(json.dumps(data, indent=2))
PY
set +e
"$UNIFY" verify-package "${WORK}/pkg-tampered.json" >"${WORK}/tamper.out" 2>"${WORK}/tamper.err"
TAMPER_EC=$?
set -e
test "$TAMPER_EC" -ne 0

# ── 4. TemporalGraph dogfood on real project facts (Rust one-shot) ─────────
cargo test -p aevum-evidence-graph --test temporal --quiet 2>&1 | tee -a "${WORK}/evidence-graph-tests.log"

python3 - <<PY
"""Assert TemporalGraph contracts against live repo observations (TS mirror)."""
from pathlib import Path
import re, sys
root = Path("${ROOT}")
cargo = (root / "Cargo.toml").read_text()
adr = (root / "docs/adr/ADR-0013-temporal-evidence-graph.md").read_text()
lib = (root / "crates/evidence-graph/src/lib.rs").read_text()
assert 'rust-version = "1.82"' in cargo or 'rust-version="1.82"' in cargo.replace(" ", "")
assert "TemporalGraph" in lib
assert "adopted" in adr.lower()
assert "Temporal" in adr or "temporal" in adr or "Native" in adr
# bi-temporal helpers exist
fact_rs = (root / "crates/evidence-graph/src/fact.rs").read_text()
assert "valid_at" in fact_rs and "invalid_at" in fact_rs
print("temporal dogfood assertions: OK")
PY

# ── 5. Cleanup side branch (return to previous) ────────────────────────────
git -C "$ROOT" checkout "$BEFORE_BRANCH" >/dev/null 2>&1 || true
git -C "$ROOT" branch -D "$BRANCH" >/dev/null 2>&1 || true

# ── 6. Report ──────────────────────────────────────────────────────────────
{
  echo "# Aevum-on-Aevum REPORT"
  echo
  echo "- stamp: \`$STAMP\`"
  echo "- root: \`$ROOT\`"
  echo "- work: \`$WORK\`"
  echo "- rust-version: \`$RUST_VER\`"
  echo "- crates: \`$TEST_CRATES\`"
  echo "- ADR-0013: present"
  echo "- unify version: \`$(cat "${WORK}/version.txt")\`"
  echo "- side branch created then deleted: \`$BRANCH\`"
  echo "- deny sh -c: exit $DENY_EC (expected non-zero)"
  echo "- tampered package: exit $TAMPER_EC (expected non-zero)"
  echo "- verify: $(tr '\n' ' ' < "${WORK}/verify.out")"
  echo
  echo "## Gates"
  echo "- [x] unify new/run/exec/verify/package/verify-package"
  echo "- [x] real git side-branch on self"
  echo "- [x] sh -c denied"
  echo "- [x] evidence-graph + temporal tests"
  echo "- [x] contracts temporal-graph tests"
  echo "- [x] tamper detection"
  echo "- [x] TemporalGraph/ADR-0013 live assertions"
} | tee "$REPORT"

echo
echo "PASS aevum-on-aevum — report: $REPORT"
