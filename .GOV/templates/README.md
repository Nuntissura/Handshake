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

V2 module lifecycle bindings follow Codex CX-MODULE-LIFECYCLE-001/002 and HBR-MODULE-LIFECYCLE-001–007. WP `architecture_binding.module_lifecycle_refs`, refinement `resolved_context.module_lifecycle_refs` and MT `context_capsule.module_maintenance_refs` resolve the owning module's maintenance contract; the WP template's `_module_maintenance_entry_shape` shows its field vocabulary. Product MT `execution.module_change_stage` is `standalone_source` or `host_adoption`. A host-adoption MT fills `execution.source_adoption` and depends on a distinct source MT whose canonical GitHub revision and standalone proof are verified before host writes. A module-only patch uses its declared build/test dependency closure; host integration has separate affected-host proof.

These fields apply to future V2 authoring and explicitly authorized reassessment, including the Studio preparation refactor. They do not re-pin WP-KERNEL-012. This repo keeps transitional admission and reference resolution in the existing role/contract workflow; it does not install the stock schema validator or a new check suite. Apply CX-VERIFY-001–003: scoped diff and contract checks for authoring, reuse valid evidence, and no additional review/evaluation cycle merely to adopt a template. Required runtime/product verdicts remain distinct from authoring checks.

Every V2 WP carries a gameplan at `.GOV/task_packets/<WP_ID>/gameplan.yaml` (an MT with its own steps: `gameplans/<MT_ID>.yaml`, parent = the WP gameplan), named by `gameplan_ref` in the WP and MT contracts, created and edited only with the `gameplan` skill (`init`, `check --role <role> --at <moment>`, `confirm`); the global hook blocks gated commands (test rounds, cache deletion, `git reset --hard`, merges) until the active gameplan passes (Codex CX-GP-001). The closure-loop fields of the MT V2 template (`verdicts[].remediation`, `verdicts[].test_run` binding: `candidate_commit`, `started_at`, `completed_at`, `failures[].artifact_ref`; `blockers[].diagnosis`; `submissions[].diagnosis_ref`; `lifecycle.diagnosis_refs`; `lifecycle.invalidated_by_commit`) are checked by agents by hand against Codex CX-VAL-007, CX-VAL-008 and CX-VAL-009 until Handshake exists.

Each agent sets `GAMEPLAN_ROLE=<its role>` before gated commands and confirms its steps with `gameplan confirm --by <role>` (Codex CX-GP-002). V2 verdict decisions are passed, failed, inconclusive or blocked; PASS_Vn/FAIL_Vn/BLOCKED/NEEDS_NEW_APPROACH stay status labels (Codex CX-STATUS-003); counter precedence and the fix budget of 3 failed verdicts per MT: Codex CX-EXEC-015.

## Markdown and other templates

`AI_WORKFLOW_TEMPLATE.md`, `AUDIT_TEMPLATE.md`, `LANDSCAPE_SCAN_TEMPLATE.md`, `MICRO_TASK_TEMPLATE.md`, `REFINEMENT_TEMPLATE.md`, `REPO_GOVERNANCE_CHANGELOG_TEMPLATE.md`, `REPO_GOVERNANCE_TASK_ITEM_TEMPLATE.md`, `SMOKETEST_REVIEW_TEMPLATE.md`, `TASK_PACKET_STUB_TEMPLATE.md`, `TASK_PACKET_TEMPLATE.md`, `WORKFLOW_DOSSIER_TEMPLATE.md`, `WP_COMMUNICATION_THREAD_TEMPLATE.md`, `WP_RECEIPTS_TEMPLATE.md`, `WP_RECEIPTS_TEMPLATE.jsonl`, `WP_RUNTIME_STATUS_TEMPLATE.json`.
