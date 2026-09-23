# .GOV/templates index

## Contract templates (JSON)

| File | Schema | Applies to |
| --- | --- | --- |
| `WORK_PACKET_CONTRACT_TEMPLATE_V2.json` | `hsk.work_packet_contract@2` | every WP after WP-KERNEL-012 |
| `MICRO_TASK_CONTRACT_TEMPLATE_V2.json` | `hsk.microtask_contract@2` | every MT of a V2 WP |
| `REFINEMENT_CONTRACT_TEMPLATE_V2.json` | `hsk.refinement_contract@2` | every refinement of a V2 WP |
| `WORK_PACKET_CONTRACT_TEMPLATE.json` | `hsk.work_packet_contract@1` | WP-KERNEL-012 only (frozen) |
| `MICRO_TASK_CONTRACT_TEMPLATE.json` | `hsk.microtask_contract@1` | WP-KERNEL-012 `MT-*.json` only (frozen) |
| `REFINEMENT_CONTRACT_TEMPLATE.json` | `hsk.refinement_contract@1` | WP-KERNEL-012 `refinement.json` only (frozen) |

V1 remains for WP-KERNEL-012 only. Do not migrate its `@1` files and do not author a new WP from V1. The V1 files are still loaded by `.GOV/roles_shared/checks/template-hbr-fields.mjs` (and `template-hbr-fields.test.mjs`), so they must stay in place.

V2 rules (each file's `_notes` states them): policy lives in `rule_refs` (Codex CX ids); the MT JSON is the status and recovery surface; digests are optional until Handshake exists; keys starting with `_` are author comments; V1 labels map as `PASS_Vn -> passed/pass`, `FAIL_Vn -> needs_remediation/fail`, `PARTIAL -> validating/inconclusive + blockers[] entry`.

## Markdown and other templates

`AI_WORKFLOW_TEMPLATE.md`, `AUDIT_TEMPLATE.md`, `LANDSCAPE_SCAN_TEMPLATE.md`, `MICRO_TASK_TEMPLATE.md`, `REFINEMENT_TEMPLATE.md`, `REPO_GOVERNANCE_CHANGELOG_TEMPLATE.md`, `REPO_GOVERNANCE_TASK_ITEM_TEMPLATE.md`, `SMOKETEST_REVIEW_TEMPLATE.md`, `TASK_PACKET_STUB_TEMPLATE.md`, `TASK_PACKET_TEMPLATE.md`, `WORKFLOW_DOSSIER_TEMPLATE.md`, `WP_COMMUNICATION_THREAD_TEMPLATE.md`, `WP_RECEIPTS_TEMPLATE.md`, `WP_RECEIPTS_TEMPLATE.jsonl`, `WP_RUNTIME_STATUS_TEMPLATE.json`.
