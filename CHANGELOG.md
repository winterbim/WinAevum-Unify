# Changelog

## Unreleased — `fix/p0-security` (2026-08-08)

### Security (P0)

- Authority secret moved to `{mission}/.aevum/authority.sk` (mode 600); never in packages.
- Evidence packages use Ed25519 `package_signature` + `{pkg}.pubkey` trust sidecar (v2).
- Ledger entries are fully signed (`aevum-ledger` `LedgerEntry`) with tip anchor.
- `unify graph authorize` requires `--grant-sig` from `unify human-grant` (human key outside mission).
- PreToolUse / `unify pretool-check` fail-closed without mission or authorization.
- Clock via `chrono::Utc`; critical writes via tempfile + fsync + rename.
- `doctor` hard-fails on corrupt/unsigned ledgers.

### Doctrine

- Remote HTTP embeddings gated behind cargo feature `remote-embed` (default **off**).
- Bench verdicts: `AEVUM_SELF_RUN_PASS` / `AEVUM_MEMORY_SELF_RUN_PASS` (not “PERFECT”).
- L-37 re-evidenced; L-38 remains CLAIMED until `cargo deny` replaces `test -f` license gate.

### Docs

- `SECURITY.md`, `LIMITATIONS.md`, `docs/REMEDIATION_P0_2026-08-08.md`
- Acceptance scripts: `forge_package.py`, `ledger_mutation_matrix.sh`, `hook-test.sh`, `crash_matrix.sh`
