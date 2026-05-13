# Kernel Builder Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: KERNEL_BUILDER

## Use

Use this brief after `just kernel-builder-startup`. It is operational memory for the build-reset product-kernel lane.

## Action Cards

### RAM-KERNEL_BUILDER-BUILD_RESET-001

- ACTION: BUILD_RESET_FOCUS
- TRIGGER: before planning, packet authoring, or product implementation for Kernel V1
- FAILURE_PATTERN: spending the session repairing ACP or repo-governance drift instead of moving Handshake Kernel V1 product code forward
- DO: keep repo governance to the minimum needed for Task Board, Build Order, WPs, microtasks, restartability, and validator handoff
- DO_NOT: patch governance workflow surfaces for polish, parity, or abstract correctness while product-kernel work can continue safely
- VERIFY: the next action either updates kernel-build artifacts or product code, or names a concrete safety/restartability blocker
- SOURCE: KERNEL_BUILDER_PROTOCOL, Operator reset instruction 2026-05-13

### RAM-KERNEL_BUILDER-WP_DETAIL-001

- ACTION: NO_CONTEXT_WP_DETAIL
- TRIGGER: when creating or updating a large kernel-build WP or microtask set
- FAILURE_PATTERN: creating a broad WP that depends on chat memory, implicit design intent, or unspecified code anchors
- DO: include current code anchors, exact scope, acceptance rows, MT goals, dependencies, tests, validator focus, risks, and out-of-scope notes
- DO_NOT: shrink the WP by deleting implementation-critical detail or compressing microtasks below restartable granularity
- VERIFY: a capable model with no chat context can implement each MT from the packet plus local repo reads
- SOURCE: KERNEL_BUILDER_PROTOCOL, Operator instruction 2026-05-13

### RAM-KERNEL_BUILDER-NO_VALIDATION-001

- ACTION: VALIDATION_BOUNDARY
- TRIGGER: after tests pass, self-checks pass, or product code appears complete
- FAILURE_PATTERN: presenting Kernel Builder self-checks as validator PASS/FAIL or merge readiness
- DO: record tests and self-checks as implementation evidence, then hand off to Classic Validator or the Operator-designated validator
- DO_NOT: issue final validation, merge approval, spec compliance verdict, or acceptance-row closure
- VERIFY: final language separates implementation evidence from validator authority
- SOURCE: KERNEL_BUILDER_PROTOCOL

### RAM-KERNEL_BUILDER-WORKTREE-001

- ACTION: PRODUCT_WORKTREE_DISCIPLINE
- TRIGGER: before touching `src/`, `app/`, `tests/`, or product runtime files
- FAILURE_PATTERN: editing product code from `wt-gov-kernel`, direct-main editing by habit, or touching `.GOV` through a WP junction
- DO: use a declared product worktree and branch for product edits; treat `../handshake_main` as reference/integration unless the Operator explicitly authorizes direct-main work
- DO_NOT: edit product code through the gov-kernel worktree or edit governance files through product worktree junctions
- VERIFY: `git status --short --branch` in the product worktree matches the intended branch before and after edits
- SOURCE: KERNEL_BUILDER_PROTOCOL, AGENTS.md worktree law

### RAM-KERNEL_BUILDER-SPEC_RESOLVER-001

- ACTION: MODULE_RESOLVER_NOT_VIEWER
- TRIGGER: before reading indexed Master Spec modules, deriving roadmap order, or creating WPs/microtasks from spec content
- FAILURE_PATTERN: treating `.GOV/spec/indexed_spec/INDEX.json` as an operator surface, document viewer, or repo projection surface
- DO: resolve `.GOV/spec/SPEC_CURRENT.md` through `current_spec.entrypoint_path` and `current_spec.resolver_index_path`; use `INDEX.json` only as a machine-readable module resolver for tools and LLMs
- DO_NOT: create repo-local Markdown indexes, viewer files, summaries, or document projections unless explicitly requested in the current task
- VERIFY: roadmap items are used only for build order, while implementation intent and proof come from topical Master Spec modules, the reset brief, and product-code evidence
- SOURCE: KERNEL_BUILDER_PROTOCOL, RGF-315

### RAM-KERNEL_BUILDER-MACHINE_ARTIFACTS-001

- ACTION: MACHINE_CONTRACT_FIRST
- TRIGGER: before creating or updating WPs, refinements, microtasks, task-state records, handoffs, or validation/dossier artifacts
- FAILURE_PATTERN: copying legacy Markdown packets/refinements/microtasks into future work as if they are the intended authority surface
- DO: author or update typed JSON/JSONL/YAML-compatible contracts first, then generate Markdown only as an explicit projection or compatibility bridge
- DO_NOT: create new model-authored Markdown as authority, or treat old Markdown-heavy artifacts as anything more than migration safety rails
- VERIFY: the durable artifact has a machine-readable authority contract, stable IDs, projection metadata when Markdown exists, and `SAFETY_NET_ONLY_DO_NOT_COPY_FORWARD` semantics for legacy Markdown
- SOURCE: CX-914, KERNEL_BUILDER_PROTOCOL, Operator instruction 2026-05-13
