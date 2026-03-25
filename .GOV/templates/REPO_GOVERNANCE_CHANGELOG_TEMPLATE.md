# Repo Governance Changelog Template

Use this template for `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`.

## Metadata Template

```md
## Metadata

- SCHEMA_VERSION: `hsk.repo_governance_changelog@0.1`
- STATUS: ACTIVE
- PURPOSE: durable governance-only change history for the repo governance kernel
- VERSIONING_RULE: `CHANGESET_VERSION` uses sortable `YYYY.MM.DD.N`
- LINKAGE_RULE: every entry must cite a stable `CHANGESET_ID` plus the driving `AUDIT_ID` and/or `SMOKETEST_REVIEW_ID`
- RELATED_TASK_BOARD: `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
```

## Entry Template

```md
### <CHANGESET_VERSION> / <CHANGESET_ID>

- STATUS: APPLIED
- SUMMARY: <one-sentence summary>
- CHANGE_TYPE: <POLICY_HARDENING|RECORDKEEPING_HARDENING|TOOLING_HARDENING|OTHER>
- DRIVER_EVIDENCE:
  - `<AUDIT_ID>`
  - `<SMOKETEST_REVIEW_ID or N/A>`
- SURFACES:
  - `<path>`
- FOLLOW_ON_ITEMS:
  - `<RGF-... or NONE>`
- OUTCOME: <one-sentence result>
```

## Naming Rules

- `CHANGESET_VERSION`: `YYYY.MM.DD.N`
- `CHANGESET_ID`: `GOV-CHANGE-YYYYMMDD-NN`
- One applied governance change or tightly related batch per entry.
