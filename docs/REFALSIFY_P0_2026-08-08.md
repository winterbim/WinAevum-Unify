# P0 re-falsify — 2026-08-08T19:32Z

workdir: `/tmp/aevum-refalsify-CBvYj5`
unify: `/tmp/cursor-sandbox-cache/9025f1ad6e988a5f37c96e681a41090c/cargo-target/debug/unify`

- [x] P0-1 metadata has no secret field
- [x] P0-1 package has no secret material
- [x] P0-2 forged/self-hash package rejected
- [x] P0-2 tampered package rejected
- [x] P0-3 ledger mutation matrix 12/12
- [x] P0-4 pretool fail-closed without mission
- [x] P0-5 authorize without --grant-sig refused
- [x] P0-5 human-grant path works
- [x] P0-6 debug-now emits UTC (2026-08-08T19:32:10Z)
- [x] P0-6 crash matrix 50/50
- [x] deep: refuse to package corrupt ledger
- [x] deep: verify rejects ledger/audit byte divergence

## Verdict

`AEVUM_REFALSIFY_PASS` — all P0 adversarial checks behaved as designed.

Scope: automated refalsify on this checkout (not a 20-agent human panel).
