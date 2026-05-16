---
file_id: ckc-atelier-lens-mt-execution-index
file_kind: mt_execution_index
updated_at: 2026-05-16
wp_id: WP-1-Atelier-Lens-Consolidation-v1
status: complete
---

# CKC Atelier/Lens Consolidation MT Execution

<topic id="execution-guardrails" status="active" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

This folder contains source-backed execution artifacts for `WP-1-Atelier-Lens-Consolidation-v1`.

The current tranche covers `MT-001` through `MT-012` only. These tasks inventory CKC code, CKC governance sources, existing Handshake Atelier/Lens and adjacent stubs, supersession risk, and current Handshake code anchors. They do not create CKC rebuild work packets.

SQLite is forbidden in Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths. PostgreSQL, EventLedger, ArtifactStore, CRDT/workspace contracts, and governed Handshake runtime surfaces are the accepted translation direction.

</topic>

<topic id="current-tranche" status="complete" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Inventory Artifacts

- `MT-001-006-ckc-code-inventory.md/json`: CKC package/runtime, backend, UI, PoseKit, ComfyUI, and tests inventory.
- `MT-007-008-ckc-spec-taskboard-inventory.md/json`: CKC spec heading/requirement and taskboard state inventory.
- `MT-009-012-handshake-anchor-inventory.md/json`: Handshake Atelier/Lens stubs, adjacent stubs, supersession risk, and code-anchor inventory.
- `MT-001-012-inventory-synthesis.md/json`: controller synthesis after the three inventory lanes complete.
- `MT-001-012-status.json`: completion overlay for official `MT-001` through `MT-012` contracts.

## Execution Rule

Each artifact must preserve source intent first, then translate CKC evidence into Handshake ownership boundaries. CKC remains source evidence until downstream rebuild packets are created after consolidation and research closure.

</topic>
