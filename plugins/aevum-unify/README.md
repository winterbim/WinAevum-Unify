# Aevum Unify — Claude Code Plugin

Trusted Autonomy Hub under Claude Code: authorize · attest · package · anti-slop.

## Install

```bash
# from WinAevum-Unify checkout
claude plugin install ./plugins/aevum-unify
export AEVUM_MISSION=/absolute/path/to/mission
export UNIFY_BIN=$(pwd)/target/debug/unify   # after cargo build -p aevum-unify
```

Or copy into a Claude marketplace / project plugins path.

## Commands

| Command | Purpose |
|---|---|
| `/aevum-status` | Graph + authorizations |
| `/aevum-slop` | Offline slop firewall → Inference |
| `/aevum-golden` | Golden Path (never merges) |
| `/aevum-package` | Evidence package + verify |

## Hooks

`PreToolUse` on Bash/Edit/Write:
- Denies `sh -c` / `bash -c` (D14)
- When `AEVUM_MISSION` is set, checks temporal authorization

## MCP

Template `.mcp.json` uses `${AEVUM_MISSION}` — write a concrete config with:

```bash
unify mcp --mission "$AEVUM_MISSION" --write-config claude
```

## Doctrine

- Never `bypassPermissions`
- Slop/rules findings are Inference only
- `auto_merge=false` on Golden Path
