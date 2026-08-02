# SKEPTIC_CATCHES — Aevum Unify

One line per WinCreator Skeptic catch. Stable patterns must be folded into the
gate definition (Meso+) before the next loop.

## Catches

- **2026-08-02 — Loop AU-M00-L01 (Skeptic 1)** — INSUFFICIENT on every row, by
  mechanical gate (`ledger_check.py` returned exit 2 because all rows were
  CLAIMED, not EVIDENCED). Class: missing promotion pass. Root question the
  Builder should have asked before writing: *"has the proof been executed
  AND captured AND linked to the claim before writing CLAIMED?"*. **Action:**
  always author rows as `PENDING` until first proof, then promote by re-running
  the gate via the audit script.

- **2026-08-02 — Loop AU-M00-L01 (Skeptic 1)** — empty `cargo-fmt.log` flagged
  as suspicious. Class: zero-byte log is acceptable (rustfmt --check is silent
  on success), but the audit script must be explicit about the absence
  (`-v` captures the per-file invocation). **Action:** call `cargo fmt
  --check -v` going forward so the log carries evidence bytes.

- **2026-08-02 — Loop AU-M00-L04 (Skeptic 2a)** — pnpm install with
  `--frozen-lockfile` failed because adding `@aevum/contracts` invalidated
  the lockfile but the workspace did not re-lock. Class: workspace drift.
  Root question: *"after adding a workspace package, did we re-run `pnpm
  install` (non-frozen) to refresh the lock, then re-verify with
  --frozen-lockfile?"*. **Action:** `scripts/verify.sh` (planned AU-M0+1) must
  always pair `--frozen-lockfile` with a prior `pnpm install` to keep the
  lockfile authoritative.

- **2026-08-02 — Loop AU-M00-L04 (Skeptic 2b)** — row L-05 evidence string
  still pointed at the obsolete log even after the lockfile had been resynced.
  Class: stale evidence string. Root question: *"did the underlying log file
  actually contain the claimed fresh state? If not, the audit script must say
  so."*. **Action:** `audit-script.py` now writes an explicit L-05 evidence
  string acknowledging that the log had to be regenerated after `pnpm install`
  non-frozen, naming both the pre-condition and the now-current state.

## Patterns to bake into the next gate

1. CLAIMED rows in the ledger must be promoted by an audit-script that
   re-runs the proof and re-writes the evidence string.
2. Zero-byte log files are valid only if the proof tool is silent on success;
   we explicitly capture verbose output going forward.
3. pnpm workspace changes require an explicit lock sync before
   `--frozen-lockfile` is asserted.
4. Every evidence string must name **which** log file backs it AND confirm that
   the file's current content matches the claim (e.g. "log now contains X",
   not just "log exists").
