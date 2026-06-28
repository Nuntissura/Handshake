# WP-CKC-posekit-overhaul

> Projection only. Authority lives in `.GOV/task_packets/WP-CKC-posekit-overhaul/packet.json`.

- WP_ID: WP-CKC-posekit-overhaul
- BASE_WP_ID: WP-CKC-posekit-overhaul
- **Status:** In Progress
- WORKFLOW_LANE: MANUAL_RELAY
- WORKFLOW_SUBLANE: HOTFIX_ADHOC_MICROTASK
- EXECUTION_OWNER: CODER_A
- AGENTIC_MODE: YES
- WORKFLOW_AUTHORITY: ORCHESTRATOR
- TECHNICAL_ADVISOR: WP_VALIDATOR
- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR
- MERGE_AUTHORITY: INTEGRATION_VALIDATOR
- WP_VALIDATOR_OF_RECORD: UNCLAIMED
- INTEGRATION_VALIDATOR_OF_RECORD: UNCLAIMED
- LOCAL_BRANCH: feat/WP-CKC-posekit-overhaul
- LOCAL_WORKTREE_DIR: ../wtc-ckc-posekit-overhaul
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-CKC-posekit-overhaul
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-CKC-posekit-overhaul/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-CKC-posekit-overhaul/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-CKC-posekit-overhaul/RECEIPTS.jsonl
- MAIN_CONTAINMENT_STATUS: NOT_STARTED
- MERGED_MAIN_COMMIT: NONE
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A
- PACKET_WIDENING_DECISION: NONE
- PACKET_WIDENING_EVIDENCE: N/A
- RISK_TIER: HIGH
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: NONE
- BUILD_ORDER_BLOCKS: NONE
- PRODUCT_WORKTREE: `../wtc-ckc-posekit-overhaul`
- PRODUCT_BRANCH: `feat/WP-CKC-posekit-overhaul`
- SOURCE_WORKTREE: `../wtc-native-editors-v1`
- SOURCE_BRANCH: `feat/WP-KERNEL-012`
- SOURCE_HEAD: `6bb8cfb4f81a06c4718403826f3b30433f74a426`

## Scope

Overhaul CastKit Codex and PoseKit inside Handshake Atelier as native, interconnected Handshake features. Start with non-Rust language surface reduction, then move to visual and behavioral parity with the original proven test app.

## Workflow

Every operator-requested product change gets one `MT-###.json` under this WP before implementation. The MT records target files, expected behavior, risks, validation commands, and closeout state. Active MT state lives in `mt_index.json`.

## Recovery Notes

The hotfix worktree was created as a real git worktree at the source HEAD, then overlaid with the source dirty/untracked product files. `.GOV` is a gov-kernel junction, not a copied folder.

Inherited dirty files are baseline risk, not automatic CKC scope. Future MTs must use path-limited review and staging.
