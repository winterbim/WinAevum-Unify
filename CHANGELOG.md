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

### Quality / CI

- CLI modules: `ledger_io`, `package`, `mission_ops`, `hooks`; `graph_cmd/{io,gate,mod}`.
- Append/verify via `TrustLedger`; exec metachar via sentinel shared policy.
- Golden/parallel emit `package_signature` (+ informational content sha).
- `verify-package` binds trust key ↔ `authority_public_key` and verifies embedded ledger.
- `verify`/`doctor` fail-closed on ledger/audit byte divergence.
- MSRV **1.85** (CI toolchain 1.85.0) — unlocks edition-2024 transitive crates.
- L-38: `deny.toml` + `cargo deny` in prepub **and** CI.
- `scripts/refalsify-p0.sh` + CI step; report `docs/REFALSIFY_P0_2026-08-08.md`.

### Doctrine

- Remote HTTP embeddings gated behind cargo feature `remote-embed` (default **off**).
- Bench verdicts: `AEVUM_SELF_RUN_PASS` / `AEVUM_MEMORY_SELF_RUN_PASS` (not “PERFECT”).
- L-37 / L-38 EVIDENCED.

### Docs

- `SECURITY.md`, `LIMITATIONS.md`, `docs/REMEDIATION_P0_2026-08-08.md`
- Acceptance scripts: `forge_package.py`, `ledger_mutation_matrix.sh`, `hook-test.sh`, `crash_matrix.sh`
