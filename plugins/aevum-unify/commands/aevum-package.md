---
description: Build and verify evidence package (ledger + digests)
argument-hint: "[mission-dir] [out.json]"
---

Package attested evidence:

1. `unify package --mission <mission> --out <out>`
2. `unify verify-package <out>`
3. Confirm ledger_entries non-empty if effects occurred; show audit_trail_digest + slop_report_digest.
