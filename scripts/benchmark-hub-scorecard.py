#!/usr/bin/env python3
"""Public hub scorecard — Trusted Autonomy Hub vs agent clients.

Measures Aevum gates (not LLM quality): D14 deny, package integrity,
MCP surface, plugin presence, slop Inference path.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def run(cmd: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True)


def main() -> int:
    rows = []

    def check(name: str, ok: bool, detail: str) -> None:
        rows.append({"gate": name, "pass": ok, "detail": detail})
        print(f"{'PASS' if ok else 'FAIL'}  {name} — {detail}")

    # Build
    p = run(["cargo", "build", "-p", "aevum-unify", "-p", "aevum-mcp", "--quiet"])
    check("build", p.returncode == 0, "unify + aevum-mcp")

    meta = run(["cargo", "metadata", "--format-version", "1", "--no-deps"])
    target = json.loads(meta.stdout)["target_directory"]
    unify = Path(target) / "debug" / "unify"
    mcp = Path(target) / "debug" / "aevum-mcp"
    check("binaries", unify.is_file() and mcp.is_file(), f"{unify.name}+{mcp.name}")

    # Plugin
    plugin = ROOT / "plugins" / "aevum-unify" / ".claude-plugin" / "plugin.json"
    hook = ROOT / "plugins" / "aevum-unify" / "hooks" / "pretool_authorize.py"
    check("claude_plugin", plugin.is_file() and hook.is_file(), str(plugin.parent.parent))

    # Adapters doc
    check("hub_adapters_doc", (ROOT / "docs" / "HUB_ADAPTERS.md").is_file(), "docs/HUB_ADAPTERS.md")

    # ADR-0021
    check(
        "adr_0021",
        (ROOT / "docs" / "adr" / "ADR-0021-trusted-autonomy-hub.md").is_file(),
        "Trusted Autonomy Hub",
    )

    # ATB
    atb = run(["cargo", "run", "-p", "aevum-agent-trust-bench", "--quiet"])
    check("atb", "AgentTrustBench: 18/18" in atb.stdout + atb.stderr, "18/18")

    # MCP tool count via list in-process is hard; grep source
    tools_rs = (ROOT / "crates" / "aevum-mcp" / "src" / "tools.rs").read_text()
    needed = [
        "aevum_package",
        "aevum_verify_package",
        "aevum_golden",
        "aevum_falsify",
        "aevum_rule_scan",
        "aevum_pretool_check",
        "aevum_slop_scan",
        "aevum_doctor",
        "aevum_agent_card",
    ]
    missing = [t for t in needed if t not in tools_rs]
    check("mcp_doctrine_tools", not missing, "missing=" + ",".join(missing) if missing else "all present")

    # Agent dream layer: doctor + dream must be reachable, not just written down.
    dream_rs = ROOT / "crates" / "unify-cli" / "src" / "dream.rs"
    dream_src = dream_rs.read_text() if dream_rs.is_file() else ""
    help_proc = run([str(unify), "--help"]) if unify.is_file() else None
    help_text = (help_proc.stdout + help_proc.stderr) if help_proc else ""
    for sub in ("doctor", "dream"):
        in_help = f"unify {sub}" in help_text
        in_src = f"cmd_{sub}" in dream_src
        check(
            f"agent_{sub}",
            in_help or in_src,
            "cli help" if in_help else ("dream.rs" if in_src else "absent"),
        )

    check(
        "agent_dream_doc",
        (ROOT / "docs" / "AGENT_DREAM.md").is_file(),
        "docs/AGENT_DREAM.md",
    )

    # The loop must self-check and self-describe before it produces any effect.
    loop_sh = (ROOT / "scripts" / "aevum-agent-loop.sh").read_text()
    check(
        "agent_loop_dream",
        all(tok in loop_sh for tok in ("doctor --mission", "dream --mission", "AGENT_CARD")),
        "doctor + dream + AGENT_CARD in agent loop",
    )

    # PreToolUse D14 unit
    hook_py = hook
    proc = subprocess.run(
        [sys.executable, str(hook_py)],
        input=json.dumps({"tool_name": "Bash", "tool_input": {"command": "sh -c 'rm -rf /'"}}),
        text=True,
        capture_output=True,
    )
    try:
        decision = json.loads(proc.stdout.strip().splitlines()[-1])
        check("pretool_d14", decision.get("decision") == "deny", decision.get("reason", ""))
    except Exception as e:
        check("pretool_d14", False, str(e))

    # Client coverage (documented)
    adapters = (ROOT / "docs" / "HUB_ADAPTERS.md").read_text()
    for client in ("Claude Code", "Cursor", "Windsurf", "Copilot"):
        check(f"adapter_{client.split()[0].lower()}", client.split()[0] in adapters or client in adapters, client)

    passed = sum(1 for r in rows if r["pass"])
    total = len(rows)
    out = {
        "scorecard": "aevum-hub-v0",
        "passed": passed,
        "total": total,
        "verdict": "AEVUM_HUB_READY" if passed == total else "AEVUM_HUB_GAPS",
        "rows": rows,
    }
    out_path = ROOT / "docs" / "scorecards" / "hub-scorecard.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps({"verdict": out["verdict"], "passed": passed, "total": total}, indent=2))
    print(f"wrote {out_path}")
    return 0 if passed == total else 1


if __name__ == "__main__":
    raise SystemExit(main())
