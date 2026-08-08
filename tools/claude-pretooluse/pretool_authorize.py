#!/usr/bin/env python3
"""PreToolUse → Aevum authorize bridge (fail-closed, P0-4).

Reads Claude Code hook JSON on stdin. Always delegates to
`unify pretool-check` (same require_authorized path as the Rust kernel).
Absence of AEVUM_MISSION → deny. Never soft-allows baseline capabilities.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys


def main() -> int:
    raw = sys.stdin.read()
    try:
        payload = json.loads(raw) if raw.strip() else {}
    except json.JSONDecodeError:
        payload = {}

    tool = payload.get("tool_name") or payload.get("toolName") or ""
    tool_input = payload.get("tool_input") or payload.get("input") or {}
    command = ""
    if isinstance(tool_input, dict):
        command = str(tool_input.get("command") or tool_input.get("cmd") or "")

    cap = "process.exec.argv"
    if tool in ("Edit", "Write", "MultiEdit"):
        cap = "graph.write"
    elif tool == "Bash":
        cap = "process.exec.argv"

    mission = os.environ.get("AEVUM_MISSION", "").strip()
    unify = os.environ.get("UNIFY_BIN", "unify")
    argv = [unify, "pretool-check", "--capability", cap, "--tool", str(tool)]
    if mission:
        argv.extend(["--mission", mission])
    if command:
        argv.extend(["--command", command])

    try:
        proc = subprocess.run(argv, capture_output=True, text=True, check=False)
    except FileNotFoundError:
        print(
            json.dumps(
                {
                    "decision": "deny",
                    "reason": f"unify binary not found ({unify}) — fail-closed",
                }
            )
        )
        return 0

    out = (proc.stdout or "").strip()
    if out:
        # Prefer the JSON decision line from unify.
        try:
            decision = json.loads(out.splitlines()[-1])
            print(json.dumps(decision))
            return 0
        except json.JSONDecodeError:
            pass

    print(
        json.dumps(
            {
                "decision": "deny",
                "reason": (proc.stderr or out or "pretool-check failed").strip()[:500],
            }
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
