# Deprecated Governance Archive

This folder preserves deprecated, non-authoritative governance files as historical reference.

Authority stays in live machine-readable contracts and ledgers. For topology, the live authority is `.GOV/roles_shared/records/GOVERNANCE_TOPOLOGY.json`.

## Category Layout

- `topology/`: retired topology registries, projections, and topology-reference files.
- `work_packets/`: deprecated work-packet/refinement/microtask reference snapshots when they do not belong in `.GOV/task_packets/_archive/`.
- `role_surfaces/`: retired role-local protocol, startup, and guidance references.
- `runtime_compatibility/`: deprecated runtime-shape references. Live runtime state must remain outside the repo under `gov_runtime/`.
- `spec/`: deprecated spec helper references that are not active versioned spec bundles. Versioned spec bundles belong under `.GOV/spec/spec_archive/`.
- `operator_notes/`: deprecated operator-local notes retained only for provenance.
- `scripts_checks/`: repo-local reference snapshots for retired scripts/checks only when the Operator explicitly wants them here. Ordinary retired governance scripts/tests should use the external archive rule in Codex.

## Rules

- Do not import archive files from active workflow code.
- Do not use archive files as authority in checks, packets, roles, or topology.
- Do not move active artifacts here by hand when a dedicated archive exists, such as `.GOV/task_packets/_archive/` or `.GOV/spec/spec_archive/`.
- Record replacement authority in the archived file or the relevant changelog/RGF entry.
