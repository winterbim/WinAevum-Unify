#!/usr/bin/env bash
# Pre-publication verification gate — must exit 0 before GitHub push.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

ok() { echo "OK  $*"; }
die() { echo "FAIL $*" >&2; exit 1; }

echo "==> cargo fmt"
cargo fmt --all -- --check

echo "==> cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> cargo test"
cargo test --workspace --quiet

echo "==> AgentTrustBench"
out="$(cargo run -p aevum-agent-trust-bench --quiet 2>&1)"
echo "$out" | tail -3
echo "$out" | grep -q '"verdict": "AEVUM_PERFECT"' || die "ATB verdict"
echo "$out" | grep -q 'AgentTrustBench: 16/16' || die "ATB 16/16"

echo "==> MemoryTruthBench"
out="$(cargo run -p aevum-memory-truth-bench --quiet 2>&1)"
echo "$out" | tail -3
echo "$out" | grep -q '"verdict": "AEVUM_MEMORY_PERFECT"' || die "MTB verdict"
echo "$out" | grep -q 'MemoryTruthBench: 9/9' || die "MTB 9/9"

echo "==> dogfood"
bash scripts/aevum-on-aevum.sh

echo "==> ledger"
python3 scripts/ledger_check.py .project/LEDGER.md

echo "==> pnpm"
CI=1 pnpm install --frozen-lockfile
pnpm -r lint
pnpm -r build
pnpm -r test

echo "==> licenses"
test -f LICENSE && test -f LICENSE-MIT && test -f LICENSE-APACHE

echo "==> no competitor memory-vendor names"
# Pattern is intentional; exclude this script so the checker is not a self-hit.
if rg -i 'graphiti|getzep|\bZep\b|GRAPHITI' \
  --glob '!**/target/**' \
  --glob '!**/node_modules/**' \
  --glob '!scripts/prepub-verify.sh' \
  . >/dev/null; then
  die "competitor names still present"
fi

echo
echo "ALL_GATES_GREEN — ready for GitHub publication"
ok "prepub-verify"
