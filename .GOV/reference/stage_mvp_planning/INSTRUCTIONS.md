---
file_id: stage-mvp-planning-instructions
file_kind: reference-workspace-instructions
updated_at: "2026-07-19"
status: active
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-mvp-planning-instructions" status="active" version="v0.2" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Stage MVP planning workspace instructions

This folder is a non-authoritative working area for initial planning and research. It may propose future specification and work-packet changes, but it does not override the master spec, a machine-readable work-packet contract, the taskboard, build order, or validator authority.

## Editing rules

- Treat the current Stage direction as superseding every older Stage-specific requirement, implementation, prototype, adapter, connector, route, pane, schema, mockup, and test surface regardless of build status.
- Retain complete source lineage and give every older item an explicit reaffirmed, superseded, external-boundary, or operator-decision disposition; do not preserve old Stage behavior or compatibility unless the current contract independently selects it.
- Treat old Stage code as optional salvage only after direct conformance to current requirements; real operator data receives an explicit non-loss disposition without preserving old Stage runtime authority.
- Record operator-locked decisions separately from recommendations and open questions.
- Register every Stage topic and every related file in the single root `index.yaml`.
- Do not create per-topic index files; the root `index.yaml` is the sole topic/file catalog.
- Store research plans, source records, snapshots, and research notes under `research/`, and register each file in the root index.
- Give every external factual claim a source record and verification state.
- Label unverified claims `UNVERIFIED`; do not promote them into architecture or acceptance criteria.
- Use relative repository paths in workspace files.
- Use filenames and folder names without spaces.
- Keep generated projections subordinate to their machine-readable indexes.

## Authority boundary

The operator directed that the master-spec Stage topic be rewritten only after initial planning concludes. During initial planning, record proposed master-spec changes under `topics/master-spec-enrichment/`; do not edit the master spec from this workspace.

## Topic-document convention

New Markdown notes must contain YAML frontmatter with at least `file_id`, `file_kind`, and `updated_at`, followed by one or more flat, non-nested `<topic>` blocks. The root index must map every note to its topic IDs and must list each topic's sources, dependencies, expected planning outputs, open questions, and verification needs.

</topic>
