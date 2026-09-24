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

V1 remains for WP-KERNEL-012 only. Do not migrate its `@1` files and do not author a new WP from V1; keep the V1 files in place while WP-KERNEL-012 is open.

V2 rules (each file's `_notes` states them): policy lives in `rule_refs` (Codex CX ids); the MT JSON is the status and recovery surface; digests are optional until Handshake exists; keys starting with `_` are author comments; V1 labels map as `PASS_Vn -> passed/pass`, `FAIL_Vn -> needs_remediation/fail`, `PARTIAL -> validating/inconclusive + blockers[] entry`.

Three lane vocabularies exist and do not mix: the WP `workflow_lane` names the governance workflow that runs the packet (for example `ORCHESTRATOR_MANAGED`); MT `execution.lane` names the kind of MT work (`implementation`, `gui`, ..., `extra_build`); WP `runtime_lanes` declares product-internal parallel runtime lanes (CX-PILLAR-001).

Every V2 WP carries a gameplan at `.GOV/task_packets/<WP_ID>/gameplan.yaml` (an MT with its own steps: `gameplans/<MT_ID>.yaml`, parent = the WP gameplan), named by `gameplan_ref` in the WP and MT contracts, created and edited only with the `gameplan` skill (`init`, `check --role <role> --at <moment>`, `confirm`); the global hook blocks gated commands (test rounds, cache deletion, `git reset --hard`, merges) until the active gameplan passes (Codex CX-GP-001). The closure-loop fields of the MT V2 template (`verdicts[].remediation`, `verdicts[].test_run` binding: `candidate_commit`, `started_at`, `completed_at`, `failures[].artifact_ref`; `blockers[].diagnosis`; `submissions[].diagnosis_ref`; `lifecycle.diagnosis_refs`; `lifecycle.invalidated_by_commit`) are checked by agents by hand against Codex CX-VAL-007, CX-VAL-008 and CX-VAL-009 until Handshake exists.

## Markdown and other templates

`AI_WORKFLOW_TEMPLATE.md`, `AUDIT_TEMPLATE.md`, `LANDSCAPE_SCAN_TEMPLATE.md`, `MICRO_TASK_TEMPLATE.md`, `REFINEMENT_TEMPLATE.md`, `REPO_GOVERNANCE_CHANGELOG_TEMPLATE.md`, `REPO_GOVERNANCE_TASK_ITEM_TEMPLATE.md`, `SMOKETEST_REVIEW_TEMPLATE.md`, `TASK_PACKET_STUB_TEMPLATE.md`, `TASK_PACKET_TEMPLATE.md`, `WORKFLOW_DOSSIER_TEMPLATE.md`, `WP_COMMUNICATION_THREAD_TEMPLATE.md`, `WP_RECEIPTS_TEMPLATE.md`, `WP_RECEIPTS_TEMPLATE.jsonl`, `WP_RUNTIME_STATUS_TEMPLATE.json`.
