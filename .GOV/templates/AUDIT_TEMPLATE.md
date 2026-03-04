# Audit Template (Code vs Master Spec)

## METADATA
- AUDIT_ID: <AUDIT-YYYYMMDD-<short-name>>
- DATE_UTC: <YYYY-MM-DDTHH:MM:SSZ>
- AUDITOR: <name/role>
- SPEC_CURRENT_POINTER: .GOV/roles_shared/SPEC_CURRENT.md
- SPEC_TARGET_RESOLVED: <Handshake_Master_Spec_vXX.XXX.md>
- CODE_TARGET:
  - worktree: <path>
  - branch: <name>
  - commit_sha: <sha>
- SCOPE_SUMMARY: <1-3 sentences>
- FOCUS_AREAS:
  - <e.g. LLM-friendly data; Postgres readiness; Locus; Loom; Microtasking; Calendar drift>
- RELATED_WP_IDS:
  - <WP-...>
- OUT_OF_SCOPE:
  - <explicit exclusions>

## METHOD (EVIDENCE-BASED)

Rules:
- "Done" is defined by the current Master Spec main body requirements referenced by each WP.
- Every MUST/SHOULD audited below must map to concrete evidence (file:line) and/or test output.
- Prefer smallest-possible verification set first (targeted tests / module checks), then expand.

Evidence types (use at least one per requirement):
- Code: `path:line` references
- Tests: command + PASS/FAIL + log excerpt (short)
- Static checks: lint/clippy/format/security tools (when applicable)
- Runtime behavior: only if reproducible and logged

## INVENTORY (WHAT EXISTS)

Record the current state before judging correctness:

### Work Packets

| WP_ID | Packet Path | Packet Status | Has Validation Report | Verdict | Notes |
|---|---|---|---|---|---|
| <WP-...> | <.GOV/task_packets/...> | <Ready for Dev/In Progress/Done> | <YES/NO> | <PASS/FAIL/OUTDATED_ONLY/PENDING> | <...> |

### Spec Anchors

| WP_ID | SPEC_ANCHOR (current) | Spec Section(s) | Token(s) to search | Notes |
|---|---|---|---|---|
| <WP-...> | <file + section refs> | <e.g. 2.6.6...> | <unique strings> | <...> |

## AUDIT RESULTS (CODE VS SPEC)

### Summary Table

| WP_ID | Spec Alignment | Security/Hygiene | Test Evidence | Recommendation |
|---|---|---|---|---|
| <WP-...> | <PASS/WARN/FAIL/OUTDATED_ONLY> | <PASS/WARN/FAIL> | <PASS/FAIL/NOT_RUN> | <close / remediation stub / re-open> |

### Detailed Findings (per WP)

#### WP: <WP-...>

- Spec requirement(s):
  - MUST: <...> (Spec: <...>)
- Evidence:
  - Code: <path:line>
  - Tests: `<command>` -> <PASS/FAIL> (log: <path or excerpt>)
- Security/hygiene notes:
  - <...>
- Verdict for this WP (current-spec):
  - <PASS | OUTDATED_ONLY | FAIL>
- Action:
  - <none | create remediation stub | open new WP revision>

## REMEDIATION STUBS (PROPOSED)

Create stubs when current-spec deltas exist (do not rewrite history):

| Base WP ID | Stub ID | Spec Anchor(s) | Problem | Suggested Fix | Risk |
|---|---|---|---|---|---|
| <WP-...> | <WP-...-v1> | <...> | <...> | <...> | <LOW/MED/HIGH> |

## COMMAND LOG

Keep this short and reproducible:

- `<command>` -> <PASS/FAIL> (notes)

## DECISIONS / NOTES

- <decisions taken, open questions, links to PRs/commits>

