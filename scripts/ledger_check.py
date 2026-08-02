#!/usr/bin/env python3
"""
ledger_check.py — WinCreator Proof Ledger mechanical gate.

A Markdown table with exactly four columns:

    | ID | Claim | Status | Evidence |

Status values (case-insensitive):
    - CLAIMED    : asserted, no evidence yet. Forbidden at end of loop.
    - EVIDENCED  : linked to proof actually executed and inspected.
    - PENDING    : proof defined but not executable in this context. Honest.
    - WAIVED     : user explicitly accepted proceeding without proof. Recorded debt.

Exit codes:
    0 : every row is EVIDENCED, PENDING or WAIVED AND each row has a non-empty Evidence cell
    2 : a CLAIMED row remains OR an EVIDENCED row has no evidence
    3 : malformed ledger (no rows parsed)

Usage:
    python scripts/ledger_check.py [--self-test] [LEDGER.md ...]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

VALID = {"CLAIMED", "EVIDENCED", "PENDING", "WAIVED"}
TABLE_ROW = re.compile(r"^\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|\s*$")
HEADER = re.compile(r"^\|\s*(?:ID|Id|id)\s*\|")


def parse_ledger(path: Path) -> list[tuple[str, str, str, str]]:
    rows: list[tuple[str, str, str, str]] = []
    for lineno, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if HEADER.match(raw):
            continue
        if "---" in raw and raw.count("|") >= 4:  # markdown separator row
            continue
        m = TABLE_ROW.match(raw)
        if not m:
            continue
        id_, claim, status, evidence = (s.strip() for s in m.groups())
        rows.append((id_, claim, status, evidence))
    return rows


def audit(paths: list[Path]) -> int:
    bad_claimed: list[tuple[Path, str]] = []
    bad_evidenced: list[tuple[Path, str]] = []
    total = 0
    summary: dict[str, int] = {s: 0 for s in VALID}

    for p in paths:
        try:
            rows = parse_ledger(p)
        except Exception as exc:
            print(f"[ledger_check] cannot parse {p}: {exc}", file=sys.stderr)
            return 3
        if not rows:
            print(f"[ledger_check] no rows parsed in {p}", file=sys.stderr)
            return 3
        for id_, claim, status, evidence in rows:
            total += 1
            key = status.upper()
            if key not in VALID:
                print(f"[ledger_check] {p}: unknown status '{status}' on {id_}", file=sys.stderr)
                return 3
            summary[key] += 1
            if key == "CLAIMED":
                bad_claimed.append((p, id_))
            if key == "EVIDENCED" and not evidence:
                bad_evidenced.append((p, id_))

    print(f"[ledger_check] parsed {total} row(s): "
          f"EVIDENCED={summary['EVIDENCED']} PENDING={summary['PENDING']} "
          f"WAIVED={summary['WAIVED']} CLAIMED={summary['CLAIMED']}")

    if bad_claimed or bad_evidenced:
        for p, id_ in bad_claimed:
            print(f"[ledger_check] CLAIMED row must not survive the loop: {p}::{id_}", file=sys.stderr)
        for p, id_ in bad_evidenced:
            print(f"[ledger_check] EVIDENCED row missing evidence link: {p}::{id_}", file=sys.stderr)
        return 2
    return 0


def self_test() -> int:
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        good = Path(tmp) / "good.md"
        good.write_text(
            "| ID | Claim | Status | Evidence |\n"
            "| --- | --- | --- | --- |\n"
            "| X-1 | ok | EVIDENCED | tests/foo.log line 7 |\n"
            "| X-2 | pending | PENDING | n/a |\n"
            "| X-3 | waived | WAIVED | user said move on |\n",
            encoding="utf-8",
        )
        bad = Path(tmp) / "bad.md"
        bad.write_text(
            "| ID | Claim | Status | Evidence |\n"
            "| --- | --- | --- | --- |\n"
            "| X-1 | too optimistic | CLAIMED | |\n",
            encoding="utf-8",
        )
        rc1 = audit([good])
        rc2 = audit([bad])
    if rc1 == 0 and rc2 == 2:
        print("[ledger_check] self-test PASS")
        return 0
    print(f"[ledger_check] self-test FAIL rc1={rc1} rc2={rc2}", file=sys.stderr)
    return 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("ledgers", nargs="*", type=Path)
    args = ap.parse_args()
    if args.self_test:
        return self_test()
    if not args.ledgers:
        print("Usage: ledger_check.py LEDGER.md [LEDGER.md ...]", file=sys.stderr)
        return 3
    return audit(args.ledgers)


if __name__ == "__main__":
    sys.exit(main())
