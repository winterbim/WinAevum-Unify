# Limitations — what this audit / release still has not proven

- Independent re-verification of the 20 adversarial agents in a **fresh session / fresh clone** (required before any publication tag).
- Live GitHub PR merge enforcement with `aevum-gate` against a remote repository.
- Long-duration MCP JSON-RPC fuzzing.
- `pnpm audit` / full JS CVE surface (network allowlist blocked in some environments).
- Build on exact MSRV `rustc 1.82.0` (lockfile may require newer).
- Human security review by a third party.
- Removal of `ureq` / remote embedder (ADR-0018 still CLAIMED, not EVIDENCED).
- Hardware-backed or KMS authority keys (`.aevum/authority.sk` is file-based MVP).
- Guaranteeing the human key is unreachable by the agent (operator must keep `~/.config/aevum/human.sk` outside the agent sandbox).
