# Aevum Unify — Runbook

This runbook is the canonical entry point for any engineer or agent touching
the repository. It links to doctrine, ADR and skill files and explains the
M0 verification gate.

## Source of truth

- Authority: **Winter Fernandes**.
- Doctrine: `AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md` at the repo root.
- Project state: `.project/STATE_OF_TRUTH.md`.
- ADR register: `.project/DECISIONS.md` and `docs/adr/ADR-NNNN-*.md`.

## M0 verification gate

```
# from repo root
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
pnpm install --frozen-lockfile
pnpm -r lint
pnpm -r build
pnpm -r test
python3 scripts/ledger_check.py .project/LEDGER.md
```

Each command must return exit 0; capture every run into
`.project/verification/<milestone>/<command>.log` for the audit trail.

## Reproducible promotion script

`scripts/ledger_check.py` is the mechanical gate. It exits non-zero if any
CLAIMED row remains, if any EVIDENCED row lacks an evidence string, or if
the ledger is malformed. Always run with `--self-test` after a fresh clone
to confirm the gate itself is intact.

## Per-loop checklist (from blueprint Annexe D.1)

- [ ] `STATE_OF_TRUTH.md` re-read.
- [ ] Single measurable objective.
- [ ] In-scope / out-of-scope list.
- [ ] Acceptance criteria.
- [ ] Risks.
- [ ] Files inspected (recorded in the task folder).
- [ ] Rollback plan.
- [ ] Expected evidence.

## Skill invocation

When authoring M0+ work, load:

```
skill_view(name="skill-wincreator")
```

and follow the protocol: classify the level, open the Loop Panel,
write CLAIMED rows, run the gate, delegate the Skeptic,
promote to EVIDENCED only after the Skeptic's VERIFIED.

## Hard rules

- **No `sh -c` on the agentic path** (ADR-0006). If a tool feels like it needs
  a shell, the design is wrong.
- **No provider lock-in** in domain contracts (ADR-0009).
- **No CLAIMED row in a closed loop** (WinCreator rule).
- **No silent scope change** (doctrine D02).

## Tasks tracked in this repo

- `.project/tasks/AU-M00-L01/` — closed-loop evidence for M0 truth.
- `.project/verification/M0/`     — raw proof artefacts.
- `.project/LEDGER.md`            — WinCreator proof ledger (current state).
