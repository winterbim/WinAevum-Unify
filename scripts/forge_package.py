#!/usr/bin/env python3
"""Forge an evidence package the way audit Agent 1/2 did (re-hash / mutate).

Usage: forge_package.py <honest.json> <forged.json>
Does NOT possess the authority secret — any resulting file must fail verify-package.
"""
from __future__ import annotations

import collections
import hashlib
import json
import sys


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: forge_package.py <in.json> <out.json>", file=sys.stderr)
        return 2
    src, dst = sys.argv[1], sys.argv[2]
    v = json.load(open(src), object_pairs_hook=collections.OrderedDict)
    v["ledger_entries"] = (
        '{"actor_id":"ATTACKER","payload":{"capability":"git.branch.create",'
        '"argv":"git push --force origin main","attestation_id":"FAKE"},'
        '"previous_digest":"sha256:genesis","sequence":1,'
        '"occurred_at":"1999-01-01T00:00:00+00:00"}\n'
    )
    if "mission" in v and isinstance(v["mission"], dict):
        v["mission"]["title"] = "FORGED"
    v.pop("package_signature", None)
    # Classic attack: recompute self-hash and pretend it is a digest.
    body = json.dumps(v, indent=2, ensure_ascii=False)
    v["package_digest"] = "sha256:" + hashlib.sha256(body.encode()).hexdigest()
    with open(dst, "w", encoding="utf-8") as f:
        json.dump(v, f, indent=2, ensure_ascii=False)
        f.write("\n")
    # Leave sidecar absent / copy if present — forge does not create a valid sig.
    print(f"wrote forged package {dst} (no valid package_signature)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
