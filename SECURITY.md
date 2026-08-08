# Security notes — WinAevum-Unify

This document records critical issues found in the adversarial audit of
2026-08-08 and the P0 remediations on branch `fix/p0-security`.

## Findings (audit) → remediation

| ID | Issue | Remediation |
|---|---|---|
| P0-1 | `authority_secret_key_hex` in `metadata.json` | Secret in `{mission}/.aevum/authority.sk` (mode 600); metadata mode 600; never packaged |
| P0-2 | Self-hash `package_digest` forgeable | Ed25519 `package_signature` + trust pubkey sidecar `{pkg}.pubkey` |
| P0-3 | Ledger digest only `capability\|argv` | Signed `LedgerEntry` (all fields) via `aevum-ledger`; tip anchor `.aevum/ledger.tip` |
| P0-4 | PreToolUse fail-open | `unify pretool-check` fail-closed; hook delegates to it |
| P0-5 | Agent self-authorize | `graph authorize` requires `--grant-sig` from `unify human-grant` (human key outside mission) |
| P0-6 | Bad clock + non-atomic writes | `chrono::Utc`; `atomic_write` (tmp+fsync+rename); doctor fails on corrupt ledger |

## Acceptance scripts

- `scripts/forge_package.py`
- `scripts/ledger_mutation_matrix.sh`
- `scripts/hook-test.sh`
- `scripts/crash_matrix.sh`

## Reporting

Open issues via GitHub. Do not treat ATB/MTB scores as third-party security proof.
