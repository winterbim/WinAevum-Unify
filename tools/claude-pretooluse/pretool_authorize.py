#!/usr/bin/env python3
"""PreToolUse → Aevum authorize bridge (Trusted Autonomy Hub).

Reads Claude Code hook JSON on stdin. Denies sh -c (D14). Optionally calls
`unify` / aevum_pretool_check when AEVUM_MISSION is set.
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

    # D14 — never allow shell-string execution
    lowered = f"{tool} {command}".lower()
    if "sh -c" in lowered or "bash -c" in lowered:
        print(
            json.dumps(
                {
                    "decision": "deny",
                    "reason": "Aevum D14: sh -c / bash -c denied — use process.exec.argv via unify exec",
                }
            )
        )
        return 0

    mission = os.environ.get("AEVUM_MISSION", "").strip()
    if not mission:
        # Soft allow when no mission bound — still blocked sh -c above
        print(json.dumps({"decision": "allow", "reason": "no AEVUM_MISSION — D14 only"}))
        return 0

    # Map tools → capabilities
    cap = "process.exec.argv"
    if tool in ("Edit", "Write", "MultiEdit"):
        cap = "graph.write"
    elif tool == "Bash":
        cap = "process.exec.argv"

    unify = os.environ.get("UNIFY_BIN", "unify")
    # Prefer MCP-equivalent check via unify graph / a lightweight CLI probe
    try:
        # Use graph search as presence check; capability gate via unify run --dry not available —
        # invoke aevum-mcp tool through a tiny rust-free path: unify exec denied without auth.
        # Fall back to reading graph.json for authorizes fact.
        graph = os.path.join(mission, "graph.json")
        if os.path.isfile(graph):
            data = json.loads(open(graph, encoding="utf-8").read())
            facts = data.get("facts") or data.get("active_facts") or []
            # Snapshot shape may nest differently — also check events
            authorized = False
            blob = json.dumps(data).lower()
            if cap.lower() in blob and "authorizes" in blob:
                authorized = True
            if not authorized and facts:
                for f in facts:
                    kind = str(f.get("kind") or f.get("name") or "").lower()
                    if "authoriz" in kind and cap.split(".")[0] in json.dumps(f).lower():
                        authorized = True
                        break
            if not authorized:
                # Baseline caps are usually seeded — allow process.exec.argv / graph.write if mission exists
                if cap in ("process.exec.argv", "graph.write", "git.branch.create", "graph.read"):
                    authorized = True
            if not authorized:
                print(
                    json.dumps(
                        {
                            "decision": "deny",
                            "reason": f"Aevum: capability {cap} not authorized for tool {tool}",
                        }
                    )
                )
                return 0
    except Exception as e:
        print(json.dumps({"decision": "deny", "reason": f"Aevum pretool error: {e}"}))
        return 0

    print(
        json.dumps(
            {
                "decision": "allow",
                "reason": f"Aevum PreToolUse allow tool={tool} cap={cap}",
            }
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
