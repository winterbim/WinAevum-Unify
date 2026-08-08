# Limitations — what this audit / release still has not proven

- Automated P0 refalsify is in CI (`scripts/refalsify-p0.sh`); a multi-agent human adversarial panel on a fresh clone is still recommended before a marketing/publication tag.
- Live GitHub PR merge enforcement with `aevum-gate` against a remote repository.
- Long-duration MCP JSON-RPC fuzzing.
- `pnpm audit` / full JS CVE surface (network allowlist blocked in some environments).
- MSRV is `rustc 1.85.0` (edition-2024 transitive crates; CI pins 1.85.0).
- Human security review by a third party.
- Hardware-backed or KMS authority keys (`.aevum/authority.sk` is file-based MVP).
- Enabling `remote-embed` reintroduces TLS/ureq — operators who turn it on accept that surface.
- Guaranteeing the human key is unreachable by the agent (operator must keep `~/.config/aevum/human.sk` outside the agent sandbox).
