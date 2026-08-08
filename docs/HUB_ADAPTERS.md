# Connecting IDEs & agents to WinAevum-Unify

Aevum is the **Trusted Autonomy Hub** under Claude Code, Cursor, Windsurf, and Copilot Agent — not a replacement IDE.

## Shared setup

```bash
cargo build -p aevum-unify -p aevum-mcp
export UNIFY_BIN=$PWD/target/debug/unify
export AEVUM_MCP_BIN=$PWD/target/debug/aevum-mcp
export AEVUM_MISSION=/absolute/path/to/mission
# optional
export SLOPCHECK_BIN=$(command -v slopcheck)
```

Create a mission once:

```bash
unify new --constitution constitution.json --out "$AEVUM_MISSION"
```

Write client MCP config:

```bash
unify mcp --mission "$AEVUM_MISSION" --write-config claude   # → .mcp.json
unify mcp --mission "$AEVUM_MISSION" --write-config cursor   # → .cursor/mcp.json
```

## Claude Code

- Install plugin: `plugins/aevum-unify/` (see its README)
- PreToolUse hook denies `sh -c` (D14)
- Slash commands: `/aevum-status`, `/aevum-slop`, `/aevum-golden`, `/aevum-package`
- Never enable `bypassPermissions` for Aevum-governed work

## Cursor

1. `unify mcp --write-config cursor`
2. Add project rule (`.cursor/rules/aevum.mdc` or User Rules):

```
Aevum Hub is authoritative for side-effects.
- Prefer MCP tools aevum_* over raw shell.
- Never use sh -c.
- Golden Path never merges.
- Slop/rule findings are Inference only.
```

3. Use Agent mode with MCP tools enabled for package/golden/slop.

## Windsurf (Cascade)

1. Point Cascade MCP at `aevum-mcp --mission $AEVUM_MISSION`
2. Same doctrine rules as Cursor
3. Prefer Cascade for multi-file edits; Aevum for authorize/attest/package

## GitHub Copilot Agent / coding agent

1. Register MCP server `winaevum-unify` identically
2. Policy: deny free-form shell; require `aevum_exec` argv tokens
3. Treat Copilot as a client — Aevum remains the control plane

## Permission modes → Aevum risk

| Client mode | Aevum |
|---|---|
| plan / ask | read-only assemble/search |
| default / acceptEdits | R1–R2 + authorizes |
| dontAsk | pre-attested caps only |
| bypassPermissions | **forbidden** |

## Parallel best-of-N

```bash
unify parallel --constitution constitution.json --out /tmp/aevum-parallel --n 3
# inspect compare.json — pick winner manually, then attest
```
