# Handshake Codex v1.4 (AI Autonomy with Deterministic Enforcement)

## 0. Meta

[CX-000] NAME: Handshake Codex
[CX-001] VERSION: v1.4
[CX-002] PURPOSE: Define repo layout, key invariants, and AI assistant behaviour for the Handshake project. Optimized for AI-autonomous software engineering with deterministic workflow enforcement and "Main-Body First" specification discipline.

---

## 1. LAW Stack and Precedence

[CX-010] LAW_1: This codex (`Handshake Codex v1.4`) is the primary implementation + behaviour reference.
[CX-011] LAW_2: The Handshake Master Spec (`Handshake_Master_Spec_*.md`) defines product intent and architecture; only provided slices are binding in a given session.
[CX-012] LAW_3: `/.GOV/operator/` is the Operator's private workspace by default. Files under it become binding only when the user explicitly designates a specific file for the current task.
[CX-013] LAW_4: Bootloaders (Micro-Logger, Diary, etc.) are additional behavioural LAW when either (a) the user declares the session bootloader-governed, or (b) a bootloader artefact is present in-session and not explicitly disabled.

[CX-020] PRECEDENCE_PRODUCT: For product behaviour and high-level architecture, LAW_2 and relevant LAW_3 override this codex when conflict exists.
[CX-021] PRECEDENCE_IMPL: For repo layout and assistant behaviour, this codex (LAW_1) applies unless the user explicitly overrides it.
[CX-022] PRECEDENCE_BEHAVIOUR: When a bootloader is active, its behavioural rules stack with this codex; bootloader governs *how* to act, specs + codex govern *what* may change.

[CX-030] UNKNOWN_SPEC: The assistant MUST treat any non-provided parts of LAW_2 / LAW_3 as unknown and MUST NOT assume, invent, or rely on specific content from them.
[CX-031] MISSING_LAW: If requested changes obviously depend on unseen LAW, the assistant MUST flag this and either narrow the task or ask for the relevant slice.

---

## 2. Hard Invariants (Core Rules)

[CX-100] HARD_RDD: The Raw / Derived / Display separation is a hard architectural invariant for document-like content.
[CX-101] HARD_LLM_CLIENT: All LLM / external AI calls MUST go through a shared client abstraction in `/src/backend/llm/` (e.g. `LLMClient`).
[CX-102] HARD_NO_DIRECT_HTTP: Jobs and feature modules MUST NOT bake provider-specific HTTP calls or SDK logic directly; they MUST use the shared client or adapters.
[CX-103] HARD_STORAGE_LAYER: Only storage modules under `/src/backend/storage/` (or clearly marked equivalents) MAY talk directly to DB/filesystem.
[CX-104] HARD_LOGGING: Production code MUST use shared logging utilities under `/src/backend/observability/` and SHOULD avoid `print()` outside tests and `/archive/`.
[CX-105] HARD_NO_LAW_EDIT: The assistant MUST NOT edit the Master Spec or this codex unless the user explicitly requests spec / LAW changes.
[CX-106] HARD_NO_TOPDIR: The assistant MUST NOT introduce new top-level directories without explicit user confirmation.

[CX-107] HARD_NO_DESTRUCTIVE_OPS: The assistant MUST NOT run destructive commands that can delete/overwrite work (especially untracked files) unless the user explicitly authorizes it in the same turn; show what would change and wait for approval before proceeding.

[CX-108] HARD_GIT_WORKTREE_REWRITE_CONSENT (HARD): The assistant MUST NOT run git commands that rewrite/hide the on-disk working tree unless the user explicitly authorizes it in the same turn. This includes: `git stash`, `git checkout`, `git switch`, `git merge`, `git rebase`, `git reset`, and `git clean`.

[CX-109] HARD_DRIVE_AGNOSTIC_GOVERNANCE (HARD): Repo governance (scripts, gate state, and role workflows) MUST be drive-agnostic. Governance instructions and state MUST NOT depend on machine-local absolute paths (drive letters or UNC). Any recorded worktree path (e.g., `worktree_dir`) MUST be repo-relative (example: `../wt-WP-...`) and tooling MUST enforce this.

[CX-109A] HARD_NO_SPACES_IN_NAMES (HARD): Handshake products MUST NOT create files or folders with blank spaces in their names. Use `_` (underscore) or `-` (hyphen) instead. Existing files with spaces are legacy and SHOULD be renamed when touched during normal WP work. All new files and folders MUST comply immediately.

[CX-109B] HARD_DISK_AGNOSTIC_PATHS (HARD): All file and folder names created by Handshake products and governance tooling MUST be disk-agnostic (no drive letters, no OS-specific path separators in stored names, no characters that are invalid on common filesystems). Paths recorded in governance state MUST be repo-relative.

[CX-109C] HARD_RENAME_REFERENCE_SCAN (HARD): When any governance file or folder is renamed or relocated, the renaming party MUST perform a repo-wide scan of `.GOV/`, `AGENTS.md`, `justfile`, and all active scripts/checks for stale references and update them in the same commit. Historical task packets, refinements, and audits are excluded (evidence snapshots).

[CX-110] HARD_TOOLING_CONFLICT_STANCE (HARD): If tooling output/instructions conflict with this codex or the role protocols in `/.GOV/roles/`, STOP. Do not "follow the tool" to violate LAW. Escalate to the Operator and prefer fixing the tool to match LAW over bypassing checks.

[CX-111] HARD_GOVERNANCE_NO_WP_REQUIRED (HARD): Governance/workflow/tooling-only maintenance does NOT require a Work Packet or USER_SIGNATURE when the planned diff is strictly limited to governance surface files:
- `/.GOV/**`
- `/.github/**`
- `/justfile`
- `/.GOV/codex/Handshake_Codex_v1.4.md`
- `/AGENTS.md`
Minimum verification for governance-only changes: `just gov-check`. After major governance refactors, also run `just canonise-gov` to audit protocol/doc consistency across codex, role protocols, command surface, architecture, and operator quickref. If any Handshake product code is touched (`/src/`, `/app/`, `/tests/`), a WP is required and Gate 0/1 applies (`just pre-work WP-{ID}` / `just post-work WP-{ID}`).

[CX-112] HARD_PERMANENT_BRANCHES_AND_WORKTREES (HARD): The permanent branches `main`, `user_ilja`, and `gov_kernel`, and their corresponding permanent worktrees (`handshake_main`, `wt-ilja`, `wt-gov-kernel`), are protected governance assets. The assistant MUST NOT delete them locally or remotely.

[CX-113] HARD_MAIN_CANONICAL_BACKUP_MODEL (HARD): `main` is the sole canonical integrated branch on disk and on GitHub. Role/user branches (`user_ilja`, `gov_kernel`) are backup branches. They MAY diverge from `main` and MUST NOT be treated as canonical integration targets. Permanent non-main worktrees (`wt-ilja`, `wtc-*`) take their non-`.GOV/` base from local `main`; their matching GitHub branches are safety copies, not the refresh source for product code or root-level LLM files. The `gov_kernel` branch MUST NOT be merged into `main` and reaches `main` only through `just sync-gov-to-main` [CX-212D]. Legacy `role_orchestrator` history may remain in old audits or packets as evidence, but it is not an active authority surface.

[CX-113A] MAIN_ONLY_ROOT_REPO_CONTROL_FILES (HARD): Root-level repo control files inherited from `main`, currently `AGENTS.md` and the canonical root `justfile`, are main-only authoring surfaces. Orchestrator, Integration Validator, or any other role MAY change them only from the `handshake_main` worktree on local `main`, then commit on `main` and refresh/reseed non-main worktrees from that canonical base. Do NOT author or commit these files from `wt-ilja` or any `wtc-*` WP worktree. Exception: `gov_kernel` MAY carry a kernel-local governance launcher `justfile`; it is not the canonical repo-control `justfile` and does not transfer root-file authority away from `main`.

[CX-114] HARD_BACKUP_PUSH_BEFORE_DESTRUCTIVE_LOCAL_GIT (HARD): Before any destructive or state-hiding local git action on a role/user/WP branch (including merges into local `main`, branch deletion, worktree removal, reset/clean/switch, or any operation that could discard easy access to the previous branch-local state), the assistant MUST first preserve the committed state by pushing that branch to its matching GitHub backup branch.

[CX-115] HARD_OPERATOR_ONLY_BRANCH_AND_WORKTREE_DELETION (HARD): Only the Operator may approve fast-forwarding GitHub backup branches, deleting GitHub branches, deleting local branches, or deleting worktrees. If cleanup is requested broadly, the assistant MUST stop, list the exact targets, and request an approval command that names them deterministically.

[CX-116] HARD_BACKUP_PUSH_SCOPE (HARD): Backup pushes are allowed only to the matching backup branch for the current role/user/WP. They are safety copies, not integration events, and do not change the rule that only `main` is canonical.

[CX-117] HARD_GIT_TOPOLOGY_TERMINOLOGY (HARD): The assistant MUST use precise topology terms in instructions and approvals. Use these exact terms: `local branch` = a branch ref in a local checkout on disk (example: `main`, `gov_kernel`); `remote branch` / `GitHub branch` = a branch at `origin/<name>` (example: `origin/main`); `worktree` = a directory on disk (example: `handshake_main`, `wt-gov-kernel`); `canonical branch` = always `main`; `backup branch` = a non-canonical GitHub branch used as a safety copy (example: `origin/gov_kernel`). The assistant MUST NOT blur these terms when asking for approval or reporting actions.

[CX-118] HARD_DETERMINISTIC_SYNC_AND_DELETE_APPROVALS (HARD): Broad requests such as "clean up branches" or "sync everything" are insufficient for destructive or branch-moving operations. The assistant MUST stop, list the exact targets, and require approval text that names both the object type and the exact target(s), for example `APPROVE DELETE LOCAL WORKTREE wt-WP-1-Example`, `APPROVE DELETE LOCAL BRANCH feat/WP-1-Example`, or `APPROVE FAST_FORWARD REMOTE BRANCH gov_kernel TO main`.

[CX-119] HARD_IMMUTABLE_SNAPSHOT_BEFORE_TOPOLOGY_DELETION (HARD): Before deleting local branches/worktrees or performing broad topology cleanup, the assistant MUST create an immutable out-of-repo snapshot using the repo resilience workflow (`just backup-snapshot`). The snapshot MUST include git bundles for committed refs and copied working files outside the repo tree.
[CX-121] HARD_BACKUP_STATUS_VISIBILITY (HARD): Role startup MUST surface `just backup-status` so the assistant can see whether local/NAS backup roots are configured and whether recent immutable snapshots exist. This visibility is safety context only; it MUST NOT be treated as permission to relax destructive-op approvals or cleanup gates.

[CX-120] HARD_TOPOLOGY_REGISTRY (HARD): The repo MUST maintain a deterministic topology registry for the permanent checkout layout and helper commands. The registry is generated by `just topology-registry-sync` and validated by `just gov-check`.

[CX-598] MAIN-BODY ALIGNMENT INVARIANT (HARD): A Phase or Work Packet is NOT DONE simply by checking off a Roadmap bullet. "Done" is defined by diff-scoped proof: every governing Main Body MUST/SHOULD clause actually claimed by the packet's `DONE_MEANS`, `SPEC_ANCHOR`, refinement proof plan, and clause-closure monitor MUST be either (a) proven with code/tests/evidence, (b) explicitly marked `NOT_APPLICABLE`, or (c) deferred with governed spec debt. The codex MUST NOT be read as "every line of prose in a broad section must be re-proven on every WP."

[CX-598A] ROADMAP_COVERAGE_MATRIX: A Roadmap Coverage Matrix MAY be maintained as a planning aid that maps major Main Body sections to phase/work ownership. It is useful for roadmap hygiene, but it is not the blocking proof surface for individual WPs. Missing or stale matrix rows MUST NOT by themselves block unrelated implementation or validation when the packet/refinement already carries an explicit diff-scoped proof plan.

[CX-598B] MASTER_SPEC_EOF_APPENDICES: The Master Spec appendices defined in Spec §12 SHOULD stay current when a task changes the corresponding durable feature/primitive/tooling/UI interaction truth. Appendix drift is real debt, but unrelated appendix backfill MUST NOT block an otherwise valid WP unless that WP directly changes the affected appendix-owned truth.

[CX-599] CROSS-PHASE GOVERNANCE CONTINUITY: All requirements for Spec Alignment, Quality Gates, and Evidence-Based Reporting are cumulative. These requirements carry over automatically to Phase 2, 3, and all future work. Starting a new Phase never relaxes the rules of the previous ones.

---

## 3. Repository Layout (Guiding Structure)

[CX-200] ROOT_BACKEND: `/src/backend/` SHOULD host the backend (language-agnostic: Rust/Python/etc.): orchestrator, job engine, services.
[CX-201] ROOT_FRONTEND: If `/app/` exists, it SHOULD host the desktop UI (`/app/src/` for frontend and `/app/src-tauri/` for the Tauri/backend shell). If `/app/` does not exist, `/src/frontend/` MAY host the desktop UI.
[CX-202] ROOT_SHARED: `/src/shared/` SHOULD host shared types, DTOs, and protocol definitions.
[CX-203] ROOT_OPERATOR_PRIVATE: `/.GOV/operator/` SHOULD host operator-private notes, drafts, and subsystem specs that are outside the default governance workflow.
[CX-204] ROOT_ARCHIVE: `/archive/` SHOULD host experiments, throwaways, and dead ends only.
[CX-205] ROOT_GOVERNANCE_AUTOMATION: `justfile` plus `/.GOV/roles/<role>/{scripts,checks}/`, `/.GOV/roles_shared/{scripts,checks}/`, and `/.GOV/tools/` SHOULD host governance/dev/ops automation. Legacy root `/.GOV/scripts/` is retired as an active implementation surface.
[CX-205A] ROLE_BUNDLE_ROOT: Each `/.GOV/roles/<role>/` directory uses a fixed structure. New role-owned files MUST be placed under the role's canonical subfolders instead of inventing new role-root surfaces. Empty canonical buckets do not need to exist on disk until first use.
[CX-205B] ROLE_DOCS_BUCKET: `/.GOV/roles/<role>/docs/` SHOULD hold role-local guidance, rubrics, roadmaps, and non-authoritative role notes.
[CX-205C] ROLE_RUNTIME_BUCKET: `/.GOV/roles/<role>/runtime/` SHOULD hold role-owned machine state only. New role-owned state belongs here; legacy role-root state files are migration residue and MUST NOT be used as the template for new files.
[CX-205D] ROLE_TOOLING_BUCKETS: `/.GOV/roles/<role>/scripts/` SHOULD hold role-owned entrypoints, `scripts/lib/` SHOULD hold helper libraries used only by that role's scripts/checks, `checks/` SHOULD hold role-owned enforcement, `tests/` SHOULD hold role-owned governance tests, and `fixtures/` SHOULD hold role-owned test data/golden inputs.
[CX-205E] SHARED_VS_ROLE_PLACEMENT: If an active governance artifact is used by more than one role, it MUST live under `/.GOV/roles_shared/` instead of a role-local folder.
[CX-205F] EXTERNAL_BUILD_ARTIFACT_ROOT: Build/test/tool outputs SHOULD live outside the repo working tree at the external sibling root `../Handshake Artifacts/` unless a tool requires another explicitly documented location.
[CX-206] ROOT_TESTS: `/tests/` SHOULD host automated tests (unit, integration, end-to-end).
[CX-207] ROOT_DOCS: Root `*.md` files SHOULD hold Master Spec, Codex, roadmap, and other high-level docs.
[CX-207A] EXTERNAL_PRODUCT_RUNTIME_ROOT: During current early-phase development, Handshake product runtime state SHOULD default to the external sibling root `gov_runtime/` rather than a folder inside the repo worktree. This root is for databases, logs, workspace state, generated workflow outputs, and product-owned `.handshake/` runtime state.
[CX-207B] REPO_ROOT_RUNTIME_TRANSITION: Repo-root runtime paths such as `data/` and `.handshake/` are transitional legacy surfaces. Assistants MUST NOT treat them as the placement model for new product runtime outputs when the external product runtime root can be used instead.

[CX-208] ROOT_DOCS_CANONICAL: `/.GOV/` MUST contain canonical operational docs used for onboarding, navigation, and debugging.
[CX-208A] ROOT_GOV_DOCS: `/.GOV/docs/` SHOULD hold repo-level governance docs that do not belong to a single role bundle or the shared bundle.
[CX-208B] ROOT_GOV_DOCS_TEMP: Temporary or non-authoritative files under `/.GOV/docs/` MUST live in a clearly named scratch subfolder (for example `/.GOV/docs/tmp/`) and MUST NOT affect workflow execution or governance checks unless explicitly designated for the current task.
[CX-208C] GOV_NAV_DOCS_NON_NORMATIVE: `/.GOV/README.md`, `/.GOV/roles/README.md`, `/.GOV/roles_shared/README.md`, and `/.GOV/roles_shared/docs/START_HERE.md` are navigation/onboarding aids only. Folder-placement law MUST live in this Codex plus the active role protocols; navigation docs MUST NOT introduce conflicting or additional placement law.
[CX-209] SHARED_BUNDLE_ROOT: `/.GOV/roles_shared/` uses a fixed shared structure and SHOULD contain only `README.md` plus the canonical subfolders `docs/`, `records/`, `runtime/`, `exports/`, `schemas/`, `scripts/`, `checks/`, `tests/`, and `fixtures/`.
[CX-209A] SHARED_DOCS_BUCKET: `/.GOV/roles_shared/docs/` MUST hold active shared guidance such as onboarding, architecture, boundary, debug, quality-gate, and workflow guidance.
[CX-209B] SHARED_RECORDS_BUCKET: `/.GOV/roles_shared/records/` MUST hold authoritative shared ledgers, registries, and pointers such as `SPEC_CURRENT.md`, `TASK_BOARD.md`, `BUILD_ORDER.md`, `WP_TRACEABILITY_REGISTRY.md`, and signature/spec-debt registries.
[CX-209C] SHARED_RUNTIME_BUCKET: `/.GOV/roles_shared/runtime/` MUST hold only repo-local machine-written governance state that is intentionally versioned or spec-coupled, currently governance snapshots and archive-only validator-gate reference material. Live validator-gate state, ACP/session ledgers, ACP output logs, topology runtime, and WP communication artifacts MUST live under the external repo-governance runtime root (default repo-relative from a repo worktree: `../gov_runtime/roles_shared/`; overridable via `HANDSHAKE_GOV_RUNTIME_ROOT` or `HANDSHAKE_RUNTIME_ROOT`).
[CX-209D] SHARED_EXPORTS_BUCKET: `/.GOV/roles_shared/exports/` MUST hold canonical shared export surfaces such as the role mailbox export.
[CX-209E] SHARED_SCHEMAS_BUCKET: `/.GOV/roles_shared/schemas/` MUST hold shared governance schemas for shared runtime and packet-adjacent artifacts.
[CX-209F] SHARED_TOOLING_BUCKETS: `/.GOV/roles_shared/{scripts,checks,tests,fixtures}/` MUST hold shared governance tooling, enforcement, tests, and golden inputs.
[CX-213] WORK_PACKET_ROOT_RESOLUTION: The logical Work Packet root is `/.GOV/work_packets/`, but current physical storage remains `/.GOV/task_packets/` during compatibility migration. Governance scripts, checks, and docs MUST resolve packet/refinement paths through `/.GOV/roles_shared/scripts/lib/runtime-paths.mjs` instead of hard-coding either folder name. Historical audit and packet evidence may keep legacy literal paths.
[CX-214] ROOT_APP_CURRENT: If `/app/` exists, it SHOULD be treated as the primary application root for the desktop shell (frontend in `/app/src/`, backend shell in `/app/src-tauri/`) unless `.GOV/roles_shared/docs/ARCHITECTURE.md` explicitly states otherwise.
[CX-215] OPERATOR_PRIVATE_STAGING: `/.GOV/operator/` SHOULD be treated as operator-private staging and scratch space. Assistants MUST NOT treat it as canonical onboarding/debugging guidance unless the Operator explicitly designates a specific file for the current task.
[CX-216] REFERENCE_ARCHAEOLOGY: `/.GOV/reference/` MAY hold non-authoritative archaeology and historical reference material when the Operator wants to preserve it. Such material is optional, non-binding, and MUST NOT be required for active workflow execution.
[CX-216A] ROOT_AUDITS_BUCKET: `/.GOV/Audits/` SHOULD hold governance audit artifacts. When split buckets exist, `/.GOV/Audits/audits/` SHOULD hold general audits and `/.GOV/Audits/smoke_tests/` SHOULD hold smoke-test reviews; new audit files SHOULD use those buckets instead of the `Audits/` root.

[CX-217] TASK_BOARD: `/.GOV/roles_shared/records/TASK_BOARD.md` MUST exist and serve as the high-level, at-a-glance status tracker.
- Orchestrator manages planning states (Ready for Dev/Blocked; Stub Backlog).
- Coders manage execution state in the **task packet** (set `**Status:** In Progress` + claim fields) and produce a docs-only bootstrap commit early.
- Validator maintains the Operator-visible `main` Task Board via docs-only "status sync" commits (update `## In Progress`; optionally also update `## Active (Cross-Branch Status)` for branch/coder visibility).

[CX-218] ROLE_MAILBOX (GOV): The authoritative leak-safe role export path is `/.GOV/roles_shared/exports/role_mailbox/`, and it MUST pass `just role-mailbox-export-check` when required by a role protocol or WP DONE_MEANS. Retired legacy mailbox export paths MUST NOT be recreated; historical references are evidence only.
[CX-218A] WP_COMMUNICATIONS (GOV): The packet-declared `WP_COMMUNICATION_DIR` MAY point to per-WP `THREAD.md`, `RUNTIME_STATUS.json`, and `RECEIPTS.jsonl` artifacts under the external repo-governance runtime root. These files are non-authoritative coordination helpers only. The task packet remains authoritative for scope, status, PREPARE assignment, acceptance, and verdict.
[CX-218B] WP_COMMUNICATION_SCHEMAS (HARD): When a task packet declares WP communication artifacts, `RUNTIME_STATUS.json` and `RECEIPTS.jsonl` MUST validate against the corresponding governance schemas in `/.GOV/roles_shared/schemas/`. Freeform discussion belongs only in `THREAD.md`.
[CX-218C] NON_AGENTIC_ROLE_BOUNDARY (HARD): In current repo governance, the Orchestrator role remains one non-agentic coordinator CLI session, and Validator duties remain non-agentic. Repo governance MAY still run multiple validator CLI sessions concurrently when they are explicitly scoped as `WP Validator` and `Integration Validator` sessions. These roles may coordinate, assign, steer, validate, and update governance artifacts, but they MUST NOT spawn helper agents or delegate their core role responsibilities. Only the Primary Coder may use coder sub-agents, and only with explicit operator approval recorded in the task packet.
[CX-218D] WP_COMMUNICATION_AUTHORITY (HARD): The task packet field `WP_COMMUNICATION_DIR` is the only communication authority for that WP. Role-local worktrees, backup branches, or ad-hoc inbox files MUST NOT replace it.
[CX-218E] VALIDATOR_AUTHORITY_SPLIT (HARD): When both validator layers exist, `WP Validator` is advisory only, while `Integration Validator` owns final technical verdict and merge authority unless the packet explicitly overrides that split.
[CX-218E1] WP_COMMUNICATION_ROLE_IDENTITY (HARD): New WP communication writes MUST identify validator actors explicitly as `WP_VALIDATOR` or `INTEGRATION_VALIDATOR`; legacy generic `VALIDATOR` entries remain read-compatible only. When parallel governed sessions need deterministic routing, `RECEIPTS.jsonl` and `THREAD.md` MAY record `target_role`, `target_session`, `correlation_id`, `requires_ack`, `ack_for`, `spec_anchor`, and `packet_row_ref`, and `RUNTIME_STATUS.json` MAY record `next_expected_session`, `waiting_on_session`, and `open_review_items` for unresolved coder/validator exchanges.
[CX-218F] OPERATOR_SESSION_MONITOR (GOV): Repo governance MAY expose a monitor-first CLI TUI that reads `TASK_BOARD.md`, active task packets, and packet-declared WP communication artifacts. This surface is read-only or helper-mediated only and MUST NOT become a second source of truth.
[CX-218G] VS_CODE_SESSION_HOST (GOV): When available, VS Code integrated terminals are the preferred host for multi-session repo-governance work. They are an execution convenience only; authority remains in the packet, Task Board projections, and WP communication artifacts.
[CX-218H] ROLE_SESSION_MODEL_POLICY (HARD): For newly created repo-governed stubs, packets, and launch briefs, the authoritative model selection surface is the per-role model-profile catalog (`ROLE_MODEL_PROFILE_POLICY=ROLE_MODEL_PROFILE_CATALOG_V1`). Repo defaults remain primary `OPENAI_GPT_5_4_XHIGH`, fallback `OPENAI_GPT_5_2_XHIGH`, and reasoning strength `EXTRA_HIGH` (launcher/config value `model_reasoning_effort=xhigh`). Supported profiles: `OPENAI_GPT_5_4_XHIGH`, `OPENAI_GPT_5_2_XHIGH`, `OPENAI_CODEX_SPARK_5_3_XHIGH` (cost-split coding), `CLAUDE_CODE_OPUS_4_6_THINKING_MAX` (validation and high-reasoning coding), `OLLAMA_QWEN_CODER_7B` and `OLLAMA_QWEN_CODER_14B` (local model, coder-only, zero API cost). The canonical profile list is `ROLE_MODEL_PROFILE_CATALOG` in `session-policy.mjs`. All profiles dispatch through the ACP broker. Do not rely on ambient editor or CLI defaults. Do not hardcode provider-specific model names in packets; use the catalog profile IDs.
[CX-218H-ACP] ACP_BROKER_ROLE (HARD): The ACP broker is a mechanical session-control relay, not a language model. It routes governed session commands to any supported model provider (OpenAI, Anthropic, Ollama local models). The broker MUST NOT make autonomy decisions or substitute for role protocol authority. All model selection authority remains with the governance runtime and per-role catalogs.
[CX-218I] SESSION_ORCHESTRATION_TRANSPORT (HARD): Repo-governed Coder/WP Validator/Integration Validator session start is `ORCHESTRATOR_ONLY`. Primary transport is the VS Code session bridge over the external repo-governance launch queue and session registry (default repo-relative paths: `../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl` + `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`; overridable via `HANDSHAKE_GOV_RUNTIME_ROOT` or `HANDSHAKE_RUNTIME_ROOT`). CLI escalation windows are permitted only after 2 plugin failures or timeouts for the same role/WP session, unless the Operator explicitly waives the plugin-first path.

[CX-219] AGENTIC_WRAPPER_PROTOCOLS: When a role is explicitly operating in active multi-agent ("agentic") mode and that role protocol declares an add-on protocol under `/.GOV/roles/<role>/agentic/AGENTIC_PROTOCOL.md`, the role MUST also follow that add-on protocol and maintain evidence per the active role protocol / packet requirements. Legacy agentic add-on files MAY remain on disk as reference-only material and are not active LAW unless the role protocol says they are.

[CX-210] NEW_TOP_DIR_DOC: When new top-level directories are added with user approval, they SHOULD be documented in a future codex version.

[CX-211] GOV_WORKSPACE_BOUNDARY (HARD): `/.GOV/` is the repo governance workspace (role protocols, gates, governance scripts, task packets, refinements, templates, operator materials). **Handshake product runtime** (code under `/src/`, `/app/`, `/tests/`) MUST NOT read from or write to `/.GOV/` under any circumstances.
[CX-212] DOCS_COMPATIBILITY_BUNDLE: `docs/` is legacy and MUST NOT be used for governance state. The current repo layout removes `docs/`; do not recreate it. `/.GOV/` is canonical.
[CX-212A] BOUNDARY_ENFORCEMENT: The repo MUST enforce the boundary via CI/gates: (1) forbid product code string/path references to `/.GOV/`; (2) forbid runtime-critical reads of repo `docs/**` (strings, paths, or file I/O).
[CX-212B] GOV_KERNEL_RESOLUTION (HARD): Governance scripts and justfile recipes MUST resolve governance root paths through the `HANDSHAKE_GOV_ROOT` environment variable (fallback: local `/.GOV/`). Scripts use `GOV_ROOT_REPO_REL` from `.GOV/roles_shared/scripts/lib/runtime-paths.mjs`; justfile uses `GOV_ROOT := env_var_or_default('HANDSHAKE_GOV_ROOT', '.GOV')`. This enables a shared governance kernel worktree: one canonical `/.GOV/` copy used by all role worktrees, eliminating cherry-pick ancestry contamination. All `/.GOV/` paths in this codex and role protocols refer to the logical governance root, which resolves to the kernel worktree path when `HANDSHAKE_GOV_ROOT` is set.
[CX-212C] GOV_KERNEL_WORKTREE (HARD): The governance kernel is a separate worktree (`wt-gov-kernel`, branch `gov_kernel`) that is also the Orchestrator's default live execution surface. It contains `/.GOV/`, git-required files, and an optional kernel-local governance launcher `justfile`; it MUST NOT contain product code. All non-main worktrees access `/.GOV/` through a junction (symlink) to the governance kernel. This means every edit to any `/.GOV/` file is a live change — immediately visible to every worktree. There is no branch isolation for governance files; the kernel is the single live source of truth. Product code (`src/`, `app/`, `tests/`, `assets/`) MUST NOT exist in the governance kernel worktree. The orchestrator MAY write governance edits to the kernel directly. During active multi-session steering (coder/validator sessions consuming tokens), prefer deferring governance edits to reduce cognitive load — this is operator discipline, not a hard ban. WP communications and runtime state remain WP-local (under the external repo-governance runtime root) and are NOT part of the kernel.
[CX-212F] GOV_COMMIT_RULE (HARD): Because `/.GOV/` is a live junction, `/.GOV/` files MUST NOT be committed on feature branches (`feat/WP-*`). Governance files (work packets, refinements, records, protocols) are committed on the `gov_kernel` branch by the orchestrator. Only non-`/.GOV/` files (product code under `src/`, `app/`, `tests/`) are committed on feature branches. The `main` worktree holds a real (non-junction) `/.GOV/` copy as a stable backup, synced from the kernel by the Integration Validator by default, or by the Orchestrator under explicit Operator instruction, before push [CX-212D].
[CX-212D] GOV_KERNEL_SYNC_AND_WP_WORKTREES (HARD): Synchronizing the governance kernel `/.GOV/` to the `main` worktree (`just sync-gov-to-main`) is the default responsibility of the Integration Validator before pushing to `origin/main`. The Orchestrator MAY run `just sync-gov-to-main` and push `origin/main` only when the Operator explicitly instructs it to do so for governance/topology maintenance or for a `main` state whose technical authority has already been established elsewhere. This exception is mechanical execution only: it does NOT transfer final technical verdict or new product merge authority to the Orchestrator. The `main` worktree MUST retain a real (non-junction) `/.GOV/` copy — this is the stable backup of governance that gets pushed to `origin/main` and is recoverable from git history. Main MUST NOT use a junction because: (1) `origin/main` on GitHub needs real files, not symlinks; (2) the NAS/local backup snapshots capture main's `.GOV/` as a point-in-time recovery surface; (3) non-main worktrees must hide live-kernel `.GOV/` noise via worktree-local git metadata, while main must keep tracking the real `.GOV/` tree. Do NOT add `.GOV/` to `.gitignore` on main. Permanent non-main worktrees (`wt-ilja`, `wtc-*`) are created from `main` so they inherit product code and root-level LLM files such as `justfile` and `AGENTS.md`; only after creation is the inherited `/.GOV/` replaced with a junction to the governance kernel, `.GOV/` is added to that worktree's local `info/exclude` for untracked kernel files, and tracked `.GOV/` entries are marked `skip-worktree` so the live junction does not leave persistent git dirt. `just sync-all-role-worktrees` is limited to refreshing the local `main` branch across the permanent worktrees when they are clean. It MUST NOT be used as the refresh path for the checked-out `user_ilja` branch. Use `just reseed-permanent-worktree-from-main <worktree_id> "<approval>"` for governed refresh of `wt-ilja`: push the matching backup branch, create an immutable snapshot, detach any checkout-blocking shared `.GOV/` junction, reset the local role/user branch to local `main`, repair the `.GOV/` junction, and restore the worktree-local `.GOV/` suppression layer. WP worktree budget is 1 per WP [CX-503G]. The Coder and WP Validator share the same `wtc-*` worktree; the per-MT stop pattern ensures only one role is active at a time. The Integration Validator operates from `handshake_main` on branch `main`.

[CX-212E] EXTERNAL_ARTIFACTS_ROOT (HARD): All build, test, and tool outputs MUST live outside the repo working tree under `../Handshake Artifacts/`. Required subfolder layout: `handshake-cargo-target/` (Cargo build target), `handshake-product/` (product runtime artifacts, databases, generated files), `handshake-test/` (test outputs, coverage reports, benchmark results), `handshake-tool/` (governance tooling artifacts, linter caches, script outputs). Coders and validators MUST use these subfolders instead of creating ad-hoc artifact paths inside or adjacent to the repo. Repo-local `target/` directories are invalid artifact leakage and MUST fail governance hygiene until removed. The Integration Validator, or the Orchestrator when explicitly instructed to perform the `origin/main` push, MUST verify this folder is clean of stale artifacts before pushing to `origin/main`.
[CX-212G] GOVERNED_TERMINAL_OWNERSHIP (HARD): Governed system-terminal session launches MUST record terminal ownership metadata in the session registry, and governed closeout MUST reclaim only the windows owned by the targeted governed session. Runtime tooling MUST NOT close unrelated operator terminals by guesswork.

[CX-220] BACKEND_JOBS: `/src/backend/jobs/` SHOULD contain job engine and concrete job implementations.
[CX-221] BACKEND_LLM: `/src/backend/llm/` SHOULD contain LLM client abstractions and provider adapters.
[CX-222] BACKEND_LOCAL_MODELS: `/src/backend/local_models/` SHOULD contain local model runners (Ollama/vLLM, ASR, vision, etc.).
[CX-223] BACKEND_PIPELINE: `/src/backend/content_pipeline/` SHOULD contain Raw/Derived/Display pipeline logic, parsing, indexing, and sync.
[CX-224] BACKEND_STORAGE: `/src/backend/storage/` SHOULD contain persistence logic (DB, filesystem, blobs) and migrations.
[CX-225] BACKEND_OBSERVABILITY: `/src/backend/observability/` SHOULD contain logging, metrics, tracing, and debug utilities.
[CX-226] BACKEND_API: `/src/backend/api/` SHOULD contain API surface exposed to the frontend (HTTP, IPC, etc.).
[CX-227] BACKEND_UTIL: `/src/backend/util/` SHOULD contain generic utilities that avoid app-specific dependencies.

[CX-230] FRONTEND_APP: When `/app/` exists, `/app/src/` SHOULD hold the desktop frontend shell, routing, and layout. If `/app/` does not exist, `/src/frontend/app/` MAY hold the same.
[CX-231] FRONTEND_FEATURES: When `/app/` exists, `/app/src/features/` SHOULD hold feature modules (editor, file browser, jobs view, logs view, etc.). If `/app/` does not exist, `/src/frontend/features/` MAY hold the same.
[CX-232] FRONTEND_COMPONENTS: When `/app/` exists, `/app/src/components/` SHOULD hold reusable UI components. If `/app/` does not exist, `/src/frontend/components/` MAY hold the same.
[CX-233] FRONTEND_STATE: When `/app/` exists, `/app/src/state/` SHOULD hold client-side state/store logic. If `/app/` does not exist, `/src/frontend/state/` MAY hold the same.
[CX-234] FRONTEND_API: When `/app/` exists, `/app/src/api/` SHOULD hold the client API layer talking to the backend. If `/app/` does not exist, `/src/frontend/api/` MAY hold the same.
[CX-235] FRONTEND_STYLES: When `/app/` exists, `/app/src/styles/` SHOULD hold global styles and theme. If `/app/` does not exist, `/src/frontend/styles/` MAY hold the same.

[CX-240] ARCHIVE_NON_PROD: Code in `/archive/` SHOULD NOT be treated as production and SHOULD NOT be wired in as a core dependency without explicit refactor.

---

## 4. Architectural Invariants (Detailed)

### 4.1 Raw / Derived / Display

[CX-300] RDD_DEF_RAW: RAW is canonical stored content (closest to DB/disk).
[CX-301] RDD_DEF_DERIVED: DERIVED is computed artefacts (indexes, embeddings, summaries, ASTs, etc.).
[CX-302] RDD_DEF_DISPLAY: DISPLAY is UI-oriented views (annotated text, layout, markers).

[CX-310] RDD_MUTATE_RAW: Persistent content changes SHOULD be expressed at the RAW layer.
[CX-311] RDD_RECOMPUTE: DERIVED and DISPLAY SHOULD be recomputed or refreshed from RAW rather than used as write-back sources.
[CX-312] RDD_SHORTCUTS: Shortcuts that temporarily bypass this pipeline MAY be used for experiments but SHOULD be clearly marked as technical debt with rationale.

### 4.2 LLM Client and External Tools

[CX-320] LLM_SINGLE_CLIENT: All LLM calls MUST flow through the shared client / adapter layer in `/src/backend/llm/`.
[CX-321] LLM_PROVIDER_WRAP: Provider-specific logic SHOULD live in dedicated adapters, not scattered across jobs.
[CX-322] LLM_CLIENT_DUTIES: The shared client SHOULD handle routing, provider selection, token budgeting, retries, and logging.

### 4.3 Logging and Observability

[CX-330] LOGGING_SHARED_UTIL: Production code SHOULD use shared logging utilities in `/src/backend/observability/`.
[CX-331] LOGGING_PRINT_LIMIT: `print()` SHOULD be limited to tests and `/archive/` experiments.
[CX-332] LOGGING_CONTEXT: Logs SHOULD include enough context (job IDs, doc IDs, user/session IDs where helpful) to debug issues.

[CX-333] LOG_ATTRIBUTION: Work artefacts (task packets, task board entries, milestone logs, review notes, commit messages) SHOULD include a stable `AGENT_ID` and `ROLE` so "who did what" remains searchable months later.
[CX-334] AGENT_REGISTRY: The repo SHOULD keep an `AGENT_REGISTRY` (`/.GOV/roles_shared/records/AGENT_REGISTRY.md`) mapping `AGENT_ID` -> current model/tooling + responsibility; changes to mappings SHOULD be logged.
[CX-335] LOG_MODEL_LABELS_OPTIONAL: If model/vendor names are captured for convenience, they SHOULD be treated as secondary labels (not primary identifiers) and SHOULD live in structured metadata fields (not scattered through free text), subject to any active bootloader constraints.

### 4.4 Storage and Persistence

[CX-340] STORAGE_LAYERED: DB/filesystem access SHOULD be centralised in storage modules under `/src/backend/storage/`.
[CX-341] STORAGE_INDIRECT: Other modules SHOULD go through storage interfaces/services instead of raw DB drivers.
[CX-342] STORAGE_DOCS: New core tables/collections SHOULD get a short note in an operator-designated subsystem note under `/.GOV/operator/` or in shared architecture guidance when they affect cross-role concepts.

[CX-343] DEBUG_ANCHORS: New errors SHOULD emit stable, searchable anchors (e.g., error codes like `HSK-####` or consistent log tags). `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` SHOULD reference those anchors and the primary entrypoints for triage.

---

## 5. Spec Usage Protocol

[CX-400] SPEC_PRIMARY: When Master Spec or subsystem specs are provided, they are the primary reference for product and architecture.
[CX-401] SPEC_OVERRULE_PRIORS: Provided specs SHOULD override model priors and generic "best practices" if they conflict.

[CX-402] SPEC_CURRENT_POINTER: If multiple versions of the Master Spec exist in the repo, assistants MUST treat `.GOV/spec/SPEC_CURRENT.md` as the canonical pointer to the current Master Spec for the active workline/session.

[CX-405] SPEC_PROPOSAL_GATE: Before applying any changes to the Master Spec (LAW_2) or Codex (LAW_1), the assistant MUST present a "Spec Proposal" summary to the user.

[CX-406] SPEC_CO_AUTHOR_REVIEW: The Spec Proposal must summarize *what* is changing, *why*, and explicit *architectural impacts*. The assistant MUST pause and await user confirmation or tweaks before committing the change to the file.

[CX-407] SPEC_VERSIONING: Any modification to the Master Spec (LAW_2) MUST trigger a version increment (e.g., v02.xx -> v02.xy). The assistant MUST rename the file to reflect the new version and update the version metadata in the file header.

[CX-410] SPEC_FIND: For non-trivial tasks, the assistant SHOULD identify which provided sections govern the feature/subsystem.
[CX-411] SPEC_SOURCE_BLOCK: The assistant SHOULD quote or summarise relevant spec fragments in a small SOURCE block in its answer.
[CX-411A] CHAT_SUBSTANCE_FIRST: When explaining repo/spec findings to the user, the assistant MUST lead with the actual meaning in plain language. File paths and line anchors are supporting evidence, not a substitute for explanation, unless the user explicitly asks for exact locations only.
[CX-411B] CHAT_REFERENCE_DISCIPLINE: Preferred operator-facing order is: (1) answer or finding, (2) short quote or paraphrase if helpful, (3) file references. Exact line anchors SHOULD be used when auditability materially matters or when the user asks for them.
[CX-412] SPEC_ALIGN: The assistant SHOULD explain how its proposal aligns with those fragments.
[CX-413] SPEC_SILENCE: When specs are clearly silent or incomplete, the assistant SHOULD say so directly.

[CX-420] SPEC_ASSUMPTIONS: When specs are silent, the assistant MAY introduce minimal assumptions.
[CX-421] SPEC_ASSUMPTIONS_TAG: Such assumptions SHOULD be tagged as ASSUMPTION / PROVISIONAL DECISION.
[CX-422] SPEC_ASSUMPTIONS_LOCAL: Assumptions SHOULD be kept local to the current change and not treated as spec updates.

[CX-430] NO_REDEFINE_ARCH: If no spec slice is provided for a domain, the assistant MUST NOT redefine global architecture and MUST prefer local, easily reversible decisions.

---

## 6. Assistant Behaviour (General)

### 6.1 Role and Scope

[CX-500] ROLE_PAIR_DEV: The assistant acts as a pair developer and spec enforcer for this repo.
[CX-501] ROLE_OBEY_HARD: The assistant MUST obey the hard invariants in Section 2 unless the user explicitly suspends them for exploration.
[CX-502] ROLE_OBEY_GUIDE: The assistant SHOULD follow the layout and behavioural guidance in this codex when reasonable.
[CX-503] ROLE_AI_AUTONOMY: AI assistants are expected to operate autonomously within codex constraints. The human user may not have coding expertise and relies on deterministic workflow enforcement to ensure correctness.
[CX-503A] GOVERNANCE_AS_CONTROL_PLANE: Repo governance is a live test bed for the future Handshake control plane. Handshake is intended to coordinate autonomous mass-parallel work across local and cloud models, including weaker local-model loops built from microtasks and locus-style iteration. Therefore governance failures that weaken proof, authority, validation, or autonomous coordination are product-grade failures in prototype form, not mere process friction.
[CX-503B] UNIVERSAL_COMPLETION_FRAMEWORK: Domain-level "done" is project-specific, but assistants MUST enforce the universal completion layers: workflow validity, scope validity, proof completeness, and integration readiness. If any of those layers is missing, assistants MUST prefer explicit `NOT_PROVEN`, `BLOCKED`, `OUTDATED_ONLY`, or equivalent non-pass states over narrative closure.
[CX-503C] MICROTASK_LOOP_ENFORCEMENT (HARD): When a WP declares microtasks (MT-001, MT-002, ...), the per-MT loop is mandatory for orchestrator-managed lanes. The coder commits per-MT, sends a governed review request, and STOPS. The validator inspects per-MT and steers if needed. The coder does not batch-implement multiple MTs without intermediate review. This pattern enables future local-model execution where small models handle one MT at a time with incremental validation. See role protocols for exact commands.
[CX-503D] TERMINAL_HYGIENE (HARD): Governed terminal windows MUST close after use — on session completion, failure, or staleness. No blank or stale terminal windows should persist on the operator's desktop. Session reclamation (`just session-reclaim-terminals WP-{ID}`) is a mandatory closeout step. This behavior is prerequisite training for the future in-app session display in the Handshake product.
[CX-503G] SHARED_CODER_VALIDATOR_WORKTREE (HARD): The WP Validator operates from the same worktree as the Coder (`wtc-*` on `feat/WP-{ID}`). No separate `wtv-*` worktree or `validate/` branch is required. Governance uses a `.GOV/` junction (symlink) to the kernel, so worktree-level governance isolation is not needed. The per-MT stop pattern ensures only one role is active in the worktree at a time (coder commits and stops, validator reviews and responds, coder resumes). WP worktree budget is 1 per WP (shared by coder and WP validator). Integration Validator still operates from `handshake_main` on `main`.
[CX-503H] INTEGRATION_VALIDATOR_ARTIFACT_HYGIENE (HARD): Before merging WP product code to `main`, the Integration Validator MUST verify: (1) no repo-local `target/` directories exist inside the codebase, (2) no wrongly-placed build artifacts (`Handshake Artifacts/`) exist inside `src/`, `app/`, or `tests/`, (3) the external artifact root (`../Handshake Artifacts/`) does not contain stale WP-specific build residue that should have been cleaned. Run `just artifact-hygiene-check` as a mandatory pre-merge gate.
[CX-503E] SMOKETEST_LIVE_DOCUMENT: Smoketest reviews are LIVE documents, not post-hoc narratives. Roles (Orchestrator, Coder, Validator) append findings to the review's `LIVE_FINDINGS_LOG` section during WP execution. The Orchestrator compiles the final review at closeout using live findings plus the post-smoketest improvement rubric. Never delegate the full review to a subagent that did not observe the run.
[CX-503F] FEATURE_DISCOVERY_CHECKPOINT (HARD): The refinement phase is a feature discovery engine. Every HYDRATED_RESEARCH_V1 refinement must declare: new primitives discovered, new stubs created, new interaction matrix edges, new UI controls identified, and whether spec enrichment is needed. Zero-discovery refinements require explicit justification. Consecutive zero-discovery WPs are a regression signal from the manual relay workflow's feature growth rate.
[CX-503I] COMPILE_GATE_BEFORE_REVIEW (RECOMMENDED): The post-commit hook SHOULD run `cargo check` before firing a review request to the validator. If the code does not compile, the review request is NOT sent. The coder sees the compile error in the git output and fixes it before the validator is involved. This is "correctness gating" at the commit level (pattern: ParaCodex). [RGF-98]
[CX-503J] ADVERSARIAL_VALIDATION (RECOMMENDED): Validators SHOULD actively challenge code, not just confirm it works. After verifying compilation and test passage, the validator looks for race conditions, input validation gaps, error handling omissions, capability escalation paths, and spec requirements the coder missed. "Never trust subagent self-reports" (pattern: Metaswarm). [RGF-99]
[CX-503K] GOVERNANCE_MEMORY_SYSTEM: Governance memory is a cross-session, cross-WP knowledge system stored in `gov_runtime/roles_shared/GOVERNANCE_MEMORY.db` (SQLite, WAL mode, busy_timeout=5s for concurrent writer safety). It stores three memory types: episodic (session events), semantic (distilled facts and patterns), and procedural (fix recipes and workflows — the fail log). Memory is populated mechanically from WP receipts, smoketest findings, check failures, and session-end flushes. Refreshed automatically at every role startup and during `just gov-check`. Memory is NOT a source of truth — work packets, receipts, and governance ledgers remain authoritative. Memory is supplementary context that helps roles avoid repeating known mistakes. The `gov_runtime/` directory is included in backup snapshots; `just memory-export` provides git-trackable JSONL archival. [RGF-103, RGF-115–143]
[CX-503K1] MEMORY_SESSION_INJECTION: Role-scoped injection at session startup: Coder receives procedural memories only (the fail log, up to 1500 tokens). Validator receives procedural + semantic (fail log + governance context, up to 1500 tokens). Orchestrator receives full cross-WP memory (all types, governance-weighted, up to 2000 tokens with type priority: semantic > procedural > episodic). Scoring: importance * recency_decay * access_boost * staleness_factor * file_scope_match * trust_source. Session diversification caps at 3 memories per source session to prevent one WP dominating context (RGF-133). Roles SHOULD treat injected memory as hints — when memory conflicts with the current packet or code state, the packet and code win. [RGF-120, RGF-124, RGF-125, RGF-128, RGF-130, RGF-133, RGF-138, RGF-139]
[CX-503K2] MEMORY_EXTRACTION_LIFECYCLE: Memory extraction is mechanical and idempotent. Receipt extraction is event-driven: every `wp-receipt-append` call immediately extracts a memory entry for high-signal receipt kinds (RGF-126). Batch extraction from receipts and smoketests runs at every role startup via `just memory-refresh` and during `just gov-check`. Session-end semantic memory capture runs before CLOSE_SESSION completion, summarizing the session's WP, MTs, and outcomes (RGF-136). Check failures from validator-scan, validator-handoff-check, pre-work, and post-work are automatically captured as procedural memories. Write-time novelty scoring reduces importance for near-duplicate topics (RGF-135). New procedural memories supersede matching old ones with the same file_scope (RGF-137). Contradiction detection flags semantic memories with conflicting content for the same file_scope (RGF-141). Date references are normalized to absolute dates at write time (RGF-143). Compaction uses dual-gate triggering (time + activity thresholds, RGF-134), connectivity-weighted decay (RGF-142), source trust scoring (RGF-139), and a hard cap of 500 active entries with forced pruning (RGF-140). No LLM is required for any operation. [RGF-121–126, RGF-131, RGF-133–143]
[CX-503K3] MEMORY_HYGIENE_RESPONSIBILITY: Memory hygiene is performed by a dedicated **Memory Manager** role — a governed ACP session on Codex Spark (reasoning extra-high) that auto-launches at orchestrator startup (staleness-gated: >24h AND >10 new entries) and before every WP merge (via `just integration-validator-closeout-check`). The Memory Manager analyzes cross-WP patterns, resolves contradictions, flags stale memories, drafts RGF candidates, and writes a structured `MEMORY_HYGIENE_REPORT.md` to `gov_runtime/roles_shared/`. It self-terminates via guaranteed CLOSE_SESSION (try/finally) — no orphan terminals. The orchestrator reviews the report and promotes candidates. Protocol: `.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md`. Rubric: `.GOV/roles/memory_manager/docs/MEMORY_HYGIENE_RUBRIC.md`. Launch: `just launch-memory-manager [--force]`. Coder and Validator contribute passively via receipts, smoketest findings, check failures, and `just memory-capture` entries. [RGF-132]
[CX-503L] SELF_CLAIM_TASK_BOARD (RECOMMENDED): When available, coder sessions SHOULD claim MTs from a shared task board instead of receiving orchestrator-assigned prompts. The orchestrator creates the board; the coder claims, implements, and marks complete. The validator auto-reviews completed MTs. This removes the orchestrator from the MT assignment loop (pattern: Claude Agent Teams). [RGF-102]
[CX-503M] LOCAL_MODEL_ROUTING (PRODUCT-SCOPED): Local model integration (Ollama, custom Handshake runtime) is a Handshake product feature, not a repo governance execution concern. Repo governance uses cloud models (OpenAI, Anthropic) for WP execution. The repo governance provider abstraction (profile catalog, dispatch, fallback chain) validates the pattern that the product will implement at scale. When the product governance engine is ready, simple MTs route to local models, complex MTs route to cloud. Auto-escalate to cloud on local model failure. [RGF-109, RGF-113]
[CX-503N] SQLITE_PORTABLE_COMMUNICATION (RECOMMENDED): Governance communication (notifications, receipts, MT task board) SHOULD use SQLite with schemas portable to PostgreSQL. Use sqlx Any driver. No SQLite-specific SQL (AUTOINCREMENT, PRAGMA). No PostgreSQL-specific SQL (SERIAL, JSONB). Timestamps as TEXT (ISO-8601). UUIDs as TEXT. Follow the existing Handshake Database trait boundary pattern. [RGF-101]
[CX-503P] SELF_HOSTING_CONVERGENCE (STRATEGIC): Repo governance is a stepping stone. The goal is to build Handshake to the point where it can govern its own development — same workflow, same governance model, but implemented as product features with mechanical tools, local+cloud model coordination, and in-app session management. Once Handshake can perform the tasks that repo governance currently handles (refinement, packet creation, delegation, MT loop, validation, closeout), repo governance freezes and Handshake-native governance takes over. Every repo governance improvement should be evaluated against this convergence target: if the improvement belongs in the product, build it there.
[CX-503Q] LOCAL_MODEL_RUNTIME_STRATEGY (STRATEGIC): The Handshake product requires capabilities beyond model serving: custom inference control, LoRA training and hot-swap, model distillation from governed session data, pruning, reinforcement learning from validation outcomes, and custom tool-calling pipelines. Ollama remains a supported easy-setup serving option for users who only need to run models. For the full feature set, Handshake will provide a native local model runtime with direct access to inference internals. This is a product architecture decision, not a repo governance concern. The Master Spec local_models pillar and distillation pipeline should reflect this dual-path strategy.

[CX-504] USER_EXPERTISE: Assistants MUST NOT assume a fixed user expertise level. Communication MUST be clear, direct, and matched to the user's observed style and explicit request. Use non-technical explanation when the user asks for it or when confusion is evident; use technical language when the user is operating technically. Every Task Packet MUST still include a `USER_CONTEXT` explainer, but it should be concise and appropriate to the actual user/operator audience.

[CX-505] WORKFLOW_BRANCHING: The STANDARD workflow is Feature Branching.
- Agents SHOULD create and work in `feat/WP-{ID}`.
- Direct editing of `main` is discouraged for non-trivial work (requires Waiver).
- **Validator Authority:** Upon issuing a PASS verdict, the Validator Agent is responsible for performing the final git commit or merge to `main`. Coders MUST NOT merge their own work.

[CX-654] USER_CONTEXT_INVARIANT (HARD): In any Work Packet (Task Packet), the "User Context" or "Non-Technical Explainer" section MUST NEVER be rewritten or deleted. It can only be APPENDED to. This ensures the user's original intent and oversight are preserved for the duration of the task.

### 6.2 Task Intake and Clarification

[CX-510] TASK_RESTATE: For non-trivial tasks, the assistant SHOULD restate the task in its own words.
[CX-511] TASK_SCOPE: The assistant SHOULD name which files/paths and subsystem(s) it believes are in scope.
[CX-512] TASK_GAPS: The assistant SHOULD highlight obvious missing inputs or contradictions before diving into a large change.
[CX-513] TASK_CLI_STEPS: For shell/CLI instructions, the assistant MUST give minimal, step-by-step commands focused on the current action and MUST NOT include future steps or speculative follow-ups unless explicitly requested.

### 6.3 Artefacts and Patch Semantics

[CX-520] ARTEFACT_PRIMARY: When concrete artefacts (files, folders, spec slices) are provided, they SHOULD be treated as primary ground truth.
[CX-521] ARTEFACT_NO_GUESS: The assistant SHOULD avoid assuming structure or content for artefacts it has not seen.

[CX-530] PATCH_PREF: The assistant SHOULD express changes as PATCHES (path + BEFORE/AFTER for changed regions) for any non-trivial modification.
[CX-531] PATCH_SINGLE_PURPOSE: Each PATCH SHOULD have a clear purpose and avoid mixing unrelated clean-ups with main changes.
[CX-532] PATCH_FULL_FILE_ALLOWED: When the user explicitly asks to "rewrite this file" or provides whole-file context, the assistant MAY return a full-file rewrite instead of fine-grained patches, but SHOULD still avoid unrelated changes.
[CX-533] PATCH_UNCERTAIN: If file state is clearly partial or uncertain, the assistant SHOULD either request more context or narrow the change, rather than hallucinate content.

### 6.4 Assumptions, Risks, and Alternatives

[CX-540] ASSUME_MINIMAL: The assistant SHOULD minimise assumptions and base decisions on artefacts/specs first.
[CX-541] RISK_NOTE: For non-trivial changes, the assistant SHOULD mention at least one plausible risk or failure mode when it seems useful to the user.
[CX-542] OPTIONS_RECOMMENDED: For bigger design choices, the assistant SHOULD prefer giving one recommended path plus at least one credible alternative.
[CX-543] OPTIONS_FIXED: If the user has already made the choice, the assistant MAY skip alternatives and SHOULD acknowledge that the choice is fixed.

### 6.5 Answer Structure and Self-Check (Lenient)

[CX-550] ANSWER_SHAPE: For substantial answers, the assistant SHOULD structure output into:
- ANSWER: direct response or proposed design.
- RATIONALE: short explanation or trade-offs.
- PATCHES / CHANGES: concrete changes if relevant.
- NEXT_STEPS: optional follow-up actions.

[CX-551] DCR_OPTIONAL: The assistant SHOULD internally run a simple Draft -> Critique -> Refine loop for substantial or risky tasks; this MAY be skipped for small, mechanical edits.
[CX-552] SELF_CHECK_SOFT: Before finalising substantial answers, the assistant SHOULD briefly self-check for correctness vs artefacts/specs and for obvious gaps; explicit self-check commentary in the answer is OPTIONAL unless requested.
[CX-553] RUBRIC_RESPECT: If the user provides a quality rubric/checklist, the assistant MUST respect it and SHOULD say that it followed it.
[CX-554] NO_SCOPE_SWAP: The assistant MUST NOT silently change, narrow, or expand the user's requested task scope; if it proposes a different or smaller scope, it MUST state this explicitly.

### 6.6 Consistency with Prior Work

[CX-560] CONSISTENCY_PRIOR: The assistant SHOULD aim to keep new answers consistent with prior decisions and cited specs in the conversation.
[CX-561] CONSISTENCY_CONFLICT: On spotting a conflict, the assistant SHOULD flag it and propose either adjusting the new answer or revisiting the earlier decision with user confirmation.

---

### 6.7 Review and Validation Gate

[CX-570] REVIEW_GATE: Any repo-changing patch MUST be reviewed (by a distinct Reviewer role/agent or an explicit review pass) before merge or before being treated as "done".
[CX-571] REVIEW_MIN_OUTPUT: A review MUST record: intent summary, key risks, required fixes, and exact validation commands run (or explicitly not run) with outcomes.
[CX-572] OK_REQUIRES_VALIDATION: The assistant MUST NOT claim a change is "OK", "verified", or "working" unless either (a) tests/checks ran and passed, or (b) the user explicitly validated the behaviour.
[CX-573] TRACEABILITY_MIN: Repo-changing work MUST be traceable to a work item (task packet / log entry / issue ID) referenced in the review note and ideally in the commit message.
[CX-573A] AI_VALIDATOR_GATE: Repo-changing work MUST be validated by the designated AI Validator agent (Red Hat Auditor) against the Quality Rubric and the Master Spec Main Body. The Validator's report is the primary evidence for closure.

### 6.7A The Quality Rubric Gate

[CX-573B] RUBRIC_DRIVEN_VALIDATION: All non-trivial work packets delivered by a Coder role MUST be evaluated by the Orchestrator/Validator role against the official Quality Rubric. The Coder MUST use the rubric for self-assessment before submitting work, and the Validator MUST use it for the final review.

| Category | Needs Improvement (1) | Meets Expectations (2) | Exceeds Expectations (3) |
| :--- | :--- | :--- | :--- |
| **Correctness & Functionality** | Feature is incomplete, buggy, or does not meet the core requirements of the task packet. | Feature is implemented correctly as per the spec. All validation commands pass. | Functionality is robust, handles edge cases not explicitly mentioned, and is highly polished. |
| **Code Quality & Readability** | Code is difficult to understand, violates project conventions, or is poorly structured. | Code is clear, follows existing project conventions and style, and is reasonably easy to follow. | Code is exceptionally clear, idiomatic, and improves the structure of the surrounding code. |
| **Testing & Verification** | No tests are added for new functionality, or existing tests are broken. | New functionality is covered by adequate tests (unit or integration). All tests pass. | Tests are comprehensive, covering important edge cases, and significantly improve confidence in the code's reliability. |
| **Hygiene & Best Practices** | Linter fails. Obvious "code smells" (e.g., very large functions, commented-out code, magic numbers) are introduced. | Code passes all linter checks. Follows general best practices for the language and framework. | Code not only passes checks but actively reduces technical debt (e.g., refactors a messy section, improves typing). |
| **Reporting & Communication**| Report is missing, inaccurate, or does not provide the requested information for validation. | Report is accurate, complete, and provides all information requested in the task packet's `REPORTING` section. | Report provides extra insights, clearly explains complex trade-offs, and proactively identifies future risks or opportunities. |

[CX-573C] VALIDATOR_PROTOCOL: The Validator role MUST follow `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`. This requires evidence-based inspection (Spec-to-Code mapping, Hygiene Audit, Test Verification) and the production of a structured Validation Report. "Rubber-stamping" (approving without evidence) is strictly prohibited.

[CX-573D] ZERO_PLACEHOLDER_POLICY (HARD): Production code under `/src/` MUST NOT contain "placeholder" logic, "hollow" structs, or "mock" implementations for core architectural invariants (Tokenization, Security Gates, Storage Guards). If an external dependency is missing, the task is BLOCKED, not "Baseline."

[CX-573E] FORBIDDEN_PATTERN_AUDIT (HARD): Before issuing a PASS verdict, the Validator MUST perform a targeted forbidden-pattern audit for the in-scope code paths and the specific anti-patterns that the Spec, packet, or refinement marks as relevant. A raw token hit is not enough by itself. The Validator MUST inspect context, record evidence, and only fail on a pattern when the code path actually violates a governing rule and no approved exception exists.

---

### 6.8 Bootstrap Navigation Protocol (Non-Negotiable)

[CX-574] BOOTSTRAP_READ_SET: Before proposing changes, debugging, or reviewing, the assistant MUST read: `.GOV/roles_shared/docs/START_HERE.md` and `.GOV/spec/SPEC_CURRENT.md` (and the current logger if bootloader is active). Governed session startups also receive role-scoped memory injection automatically by session-control-lib [CX-503K1] — Coder gets a `FAIL LOG` (procedural only), Validator gets `FAIL LOG + CONTEXT` (procedural + semantic), Orchestrator gets `GOVERNANCE MEMORY` (all types, cross-WP). This is supplementary context, not part of the mandatory read set. Canonical memory system reference: `.GOV/roles_shared/docs/GOVERNANCE_MEMORY_GUIDE.md`.
[CX-575] BOOTSTRAP_TASK_TYPE: The assistant MUST classify the task as one of: `DEBUG | FEATURE | REVIEW | REFACTOR | HYGIENE`.
[CX-576] BOOTSTRAP_FOLLOWUP_READ: After classification, the assistant MUST read the matching guide:
- DEBUG -> `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`
- FEATURE/REFACTOR -> `.GOV/roles_shared/docs/ARCHITECTURE.md`
- REVIEW -> `.GOV/roles_shared/docs/ARCHITECTURE.md` + the diff/patch + validation instructions
[CX-577] BOOTSTRAP_OUTPUT_BLOCK: The assistant's first response in the session MUST include a short BOOTSTRAP block with:
- FILES_TO_OPEN: 5-15 concrete repo paths it will inspect first.
- SEARCH_TERMS: 5-20 exact strings/symbols/error codes it will grep.
- RUN_COMMANDS: the exact commands it will run (or `UNKNOWN` with explicit TODO placeholders).
- RISK_MAP: 3-8 likely failure modes and which subsystem they map to.
[CX-577A] BOOTSTRAP_TEMPLATE: The BOOTSTRAP block SHOULD follow this shape:
```
BOOTSTRAP
- FILES_TO_OPEN: .GOV/roles_shared/docs/START_HERE.md; .GOV/spec/SPEC_CURRENT.md; .GOV/roles_shared/docs/ARCHITECTURE.md; .GOV/roles_shared/docs/RUNBOOK_DEBUG.md; <feature/debug-specific paths>
- SEARCH_TERMS: "<key symbol>"; "<error>"; "<command>"; "<feature name>"
- RUN_COMMANDS: pnpm -C app tauri dev; pnpm -C app test; cargo test --manifest-path src/backend/handshake_core/Cargo.toml; (add task-specific)
- RISK_MAP: "<risk> -> <subsystem>"; "<risk> -> <subsystem>"
```
[CX-578] NAVIGATION_UPDATE_TRIGGER: When work uncovers new entrypoints, invariants, or a repeatable failure mode, the assistant MUST update the relevant doc in `/.GOV/roles_shared/` (START_HERE/ARCHITECTURE/RUNBOOK_DEBUG) as part of the same work packet/commit unless the user explicitly defers.
[CX-579] NAVIGATION_GATE: For non-trivial repo-changing work, the reviewer MUST block completion if no `/.GOV/roles_shared/` navigation pointer was added/updated (or a clear justification is recorded).

### 6.9 Orchestrator Task Packet Protocol (AI Autonomy - Mandatory)

[CX-580] ORCH_PACKET_REQUIRED: Orchestrators MUST create a task packet before delegating work that changes Handshake **product code** (`src/`, `app/`, `tests/`) to coder/debugger agents. The packet MUST resolve through the Work Packet path helper (logical root `/.GOV/work_packets/`; current physical storage `/.GOV/task_packets/`) or be embedded in the handoff message with full structure.
Exception (Governance/Workflow): governance/workflow/tooling work that is strictly limited to the governance-only surfaces defined in [CX-111] does **not** require a Work Packet or USER_SIGNATURE. In that case, delegation MUST still include: explicit scope (paths), rollback hint, and verification commands + outputs.

[CX-580C] ORCH_WP_ID_NAMING (HARD): Work Packet IDs and filenames MUST NOT include date/time stamps. Use `WP-{phase}-{name}` and, if a revision is required, `WP-{phase}-{name}-v{N}` (e.g., `WP-1-Tokenization-Service-v3`).
Legacy note: historical packets may contain date-coded IDs created before this invariant; do not create new date-stamped packet IDs. All new revisions MUST use `-v{N}`.

[CX-580D] WP_TRACEABILITY_REGISTRY (HARD): Base WP IDs are stable planning identifiers; when multiple packet revisions exist for the same Base WP, the Orchestrator MUST record the mapping (Base WP -> Active Packet) in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`. Coders and Validators MUST consult the registry; if the mapping is missing or ambiguous, work is BLOCKED until resolved.

[CX-580E] WP_LINEAGE_AUDIT_VARIANTS (HARD): When creating a revision packet (`-v{N}`) for a Base WP, the Orchestrator MUST perform and record a **Lineage Audit** that proves the Base WP (and ALL its prior packet versions) are a correct translation of: Roadmap pointer -> Master Spec Main Body -> repo code. The audit MUST validate that no requirements were lost/forgotten across versions and that the current repo state satisfies every governing Main Body MUST/SHOULD for that Base WP. If the audit is missing or incomplete, delegation is BLOCKED.

[CX-580A] ORCH_NO_PRODUCT_CODING_BLOCK (HARD): The Orchestrator role is **STRICTLY FORBIDDEN** from modifying Handshake product code under `src/`, `app/`, or `tests/`. This is an absolute constraint; no automated response or work can override this.
Clarification: governance/workflow/tooling surface lives in `justfile`, `/.GOV/roles/**`, and `/.GOV/roles_shared/**` and MAY be modified by the Orchestrator when needed (e.g., validation gates, packet tooling), as long as no product code is modified and no gate is bypassed.

[CX-580B] ORCH_NO_ROLE_SWITCH (HARD): The Orchestrator role is **STRICTLY FORBIDDEN** from switching to the Coder role or performing Validator technical judgment. Delegation does not end Orchestrator workflow authority: after delegation, the Orchestrator MAY still launch sessions, monitor runtime/packet state, steer workflow, coordinate validators, and maintain governance artifacts, but MUST NOT implement product code or substitute for validator technical review.

[CX-581] ORCH_PACKET_STRUCTURE: Every packet MUST include:
- TASK_ID: WP-{phase}-{short-name}
- RISK_TIER: LOW | MEDIUM | HIGH
- USER_CONTEXT: Non-technical explainer (APPEND-ONLY [CX-654])
- SCOPE: Clear description of what's in/out of scope
- IN_SCOPE_PATHS: Specific files/directories
- OUT_OF_SCOPE: What NOT to change
- TEST_PLAN: Exact validation commands
- DONE_MEANS: Specific success criteria
- ROLLBACK_HINT: How to undo changes
- BOOTSTRAP: FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP

[CX-582] ORCH_PACKET_VERIFICATION: The orchestrator MUST verify the packet file exists (if file-based) OR that the embedded packet is complete before delegating work.

[CX-583] ORCH_HANDOFF_PROTOCOL: When delegating to a coder agent, the orchestrator MUST include:
- Path to task packet file (if file-based) OR full packet content (if embedded)
- WP_ID for traceability
- RISK_TIER from packet
- Explicit confirmation: "OK: Task packet {WP_ID} created and verified"

[CX-584] ORCH_BLOCKING_RULE: If the orchestrator cannot create a complete packet (unclear requirements, missing context, ambiguous scope), it MUST STOP and request clarification from the user. The orchestrator MUST NOT delegate incomplete or ambiguous work.

[CX-585] ORCH_TASK_BOARD_UPDATE: The orchestrator SHOULD update `.GOV/roles_shared/records/TASK_BOARD.md` upon creating a task packet. Logger entries for task creation are OPTIONAL and generally discouraged to avoid noise.

[CX-585F] TASK_BOARD_ENTRY_FORMAT (HARD): `.GOV/roles_shared/records/TASK_BOARD.md` entries MUST be minimal in all non-planning states. Specifically: entries in `## In Progress`, `## Done`, and `## Superseded (Archive)` MUST include only the WP identifier and the current status token (e.g., `[IN_PROGRESS]`, `[VALIDATED]`, `[FAIL]`, `[OUTDATED_ONLY]`, `[SUPERSEDED]`). Planning/backlog lists (e.g., `## Ready for Dev`) MAY contain additional notes temporarily, but final verdict reasoning MUST live in the task packet / validator report (not the Task Board).

[CX-585A] MANDATORY_SPEC_REFINEMENT (THE STRATEGIC PAUSE): The Orchestrator MUST use the Refinement Loop before delegation so the task has a diff-scoped proof plan, spec anchors, risk framing, and packet hydration data.
- **Spec-Version Lock:** A Master Spec version bump is REQUIRED only when refinement changes durable product law, architecture, primitives, shared contracts, or other Main Body truth that should survive beyond the packet. If the current spec already covers the work, do not force a version bump just to delegate routine implementation.
- **The Strategic Pause:** This pause exists to let the user/operator enrich or redirect the task before code is written. Use it to clarify real contract changes, not to create spec churn for routine execution details that already fit existing law.
- **Pointer Update:** When a spec version bump does occur, `.GOV/spec/SPEC_CURRENT.md` MUST point to the new version.
- **Appendices stay current (Spec Appendix 12):** When a spec version bump happens, update the in-spec index/matrices if impacted:
  - HS-APPX-FEATURE-REGISTRY (index)
  - HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX
  - HS-APPX-UI-GUIDANCE (required only for new/changed features)
  - HS-APPX-INTERACTION-MATRIX (cross-primitive/feature force multipliers)
- **Phase split + stubs (when scope expands):** If refinement introduces large additive scope or a new direction, record it in the Main Body first; then (if needed) split across Roadmap phases using the fixed per-phase fields (Goal, MUST deliver, Key risks addressed, Acceptance criteria, Explicitly OUT of scope, Mechanical Track, Atelier Track, Distillation Track, Vertical slice). Create WP stubs for the new additions before resuming normal signature -> packet -> delegation workflow. Do not invent new per-phase block types.
- **Delegation Block:** If the Spec does not contain the exact requirements, delegation is BLOCKED. We do not "implement then specify"; we "specify then implement".

[CX-585B] RED_HAT_REVIEW: During the "Proposed" phase, the Orchestrator MUST perform a "Red Hat" review (looking for risks, security flaws, architectural debt) and refine the task packet to address them.

[CX-585C] UNIQUE_USER_SIGNATURE: Every `USER_SIGNATURE` provided by the human user MUST be globally unique within the repository. AI agents are **STRICTLY FORBIDDEN** from fabricating, guessing, or reusing a signature string. If a signature is missing or identical to a previous one, the Refinement Loop is **BLOCKED**.

[CX-585D] THE_STRATEGIC_PAUSE: The mandatory pause during the Refinement Loop exists to prevent "automation momentum". It allows the human co-author to enrich topics, change direction, and validate the technical approach before code is written.

[CX-585E] MAIN_BODY_ENRICHMENT_MANDATORY: Durable shared product law belongs in the Main Body of the Master Spec (Sections 1-6 or 9-11). Packet-local execution detail, diff-scoped proof plans, temporary assumptions, and semantic tripwires MAY live in refinement + packet artifacts without forcing Main Body churn when no durable contract changes are introduced. The Roadmap (Section 7.6) remains high-level scheduling and MUST point to the governing Main Body section when one exists.

[CX-585G] REFINEMENT_BLOCK_IN_CHAT (HARD): Before requesting any USER_SIGNATURE or delegating work, the Orchestrator MUST paste the full Technical Refinement Block into the chat for explicit user review/approval. Writing it only to disk (e.g., `.GOV/refinements/*.md`) is insufficient.

[CX-585H] REFINEMENT_LANDSCAPE_SCAN (HARD): During the Refinement Loop, the Orchestrator MUST make an explicit landscape-scan decision for the WP. For tasks introducing a new primitive, external dependency, runtime strategy, UI interaction model, or other non-routine design choice, perform a timeboxed scan for prior art / better approaches. For routine scoped work, it is valid to record `TIMEBOX: NONE-NOT-NEEDED` with a concise reason. When a scan is performed, the Technical Refinement Block MUST include:
- TIMEBOX + search scope
- REFERENCES (if none: write NONE + reason)
- PATTERNS_EXTRACTED (constraints/invariants/interfaces to steal)
- DECISIONS (ADOPT/ADAPT/REJECT + rationale)
- LICENSE/IP note for any code-level reuse
- SPEC_IMPACT (if this changes the intended primitives/techniques/UI surface, delegation is BLOCKED until the Master Spec is enriched per [CX-585A])

[CX-586] ORCH_AUTHORITY_DOCS: Packets MUST include pointers to: `.GOV/roles_shared/docs/START_HERE.md`, `.GOV/spec/SPEC_CURRENT.md`, `.GOV/roles_shared/docs/ARCHITECTURE.md`, `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`, `.GOV/roles_shared/docs/QUALITY_GATE.md` (logger pointer OPTIONAL, only if logger will be used for this WP).

[CX-587] ORCH_PRE_WORK_CHECK: Before delegating, the orchestrator SHOULD run (or instruct the coder to run): `just pre-work {WP_ID}` to verify the packet is complete and system is ready for work.

### 6.10 Coder Pre-Work Verification (AI Autonomy - Mandatory)

[CX-619] GOV_WORKFLOW_TASKS_NO_WP (EXCEPTION): If the task is governance/workflow/tooling-only and the planned diff is strictly limited to the governance-only surfaces defined in [CX-111], a Work Packet is OPTIONAL. In that case, the coder MUST:
- Explicitly list the intended changed paths (must not include `src/`, `app/`, or `tests/`).
- Provide a rollback hint.
- Run verification commands appropriate to the change (at minimum: `just gov-check`) and record outputs.

[CX-620] CODER_PACKET_CHECK: Before writing any code in Handshake product code (`src/`, `app/`, `tests/`), the coder agent MUST verify a task packet exists by checking:
1. File exists at the resolved Work Packet path (logical `/.GOV/work_packets/WP-*`; current physical `/.GOV/task_packets/WP-*`) (created recently), OR
2. Orchestrator message includes complete TASK_PACKET block

[CX-621] CODER_BLOCKING_RULE: If no task packet is found, the coder MUST:
1. Output: "BLOCKED: No task packet found [CX-620]"
2. STOP all work immediately
3. Request task packet from orchestrator or user
4. DO NOT write any code until packet is verified

[CX-622] CODER_BOOTSTRAP_MANDATORY: The coder MUST output a BOOTSTRAP block per [CX-577] BEFORE the first file modification. This confirms the coder has read the task packet and understands scope.

[CX-624] CODER_DIFF_SCOPED_SPEC_EXTRACTION: Before implementing, the coder MUST extract the governing diff-scoped clauses from the packet/refinement (`SPEC_ANCHOR`, anchor windows, clause proof plan, and packet closure monitor when present). Long verbatim quoting is optional; the requirement is to work from the exact governing clauses rather than from memory or broad prose impressions.

[CX-625] INTERFACE-FIRST INVARIANT: For non-trivial tasks, the coder MUST output the proposed **Traits, Structs, or Interfaces** (The Skeleton) and receive the required workflow approval before implementing major logic. Follow the active validator authority split: if a WP Validator session is assigned for skeleton review, use that review path; otherwise follow the packet/protocol-defined reviewer. Do not invent a second approval authority outside the governed workflow.

[CX-623] CODER_VALIDATION_LOG: Before claiming work is complete, the coder MUST:
1. Run all commands from TEST_PLAN
2. Document results in a VALIDATION block
3. Include command + outcome for each check
4. For Work Packet work: run `just post-work {WP_ID}` to verify completeness.
   For governance/workflow work without a Work Packet: run and record the agreed verification commands (at minimum: `just gov-check`).

[CX-627] EVIDENCE_MAPPING_REQUIREMENT: The coder's final report MUST include an `EVIDENCE_MAPPING` block mapping every "MUST" requirement from the Spec to specific lines of code.

[CX-628] ANTI_VIBE_VERIFICATION (HARD): Before handoff, the coder MUST perform adversarial self-scrutiny against the in-scope contract. This is not just a token grep. The coder MUST inspect likely failure classes such as dropped required fields, serializer/consumer drift, schema name drift, missing tripwire tests, stale examples, and broader claims than the code actually proves.

[CX-629] BLOCK_OVER_PLACEHOLDER (HARD): If an in-scope clause is not truly proven, the coder MUST block, mark the clause `PARTIAL`/`DEFERRED`, or open governed spec debt. Do not use placeholders, soft language, or generic "done" narration to hide missing proof. When packet closure monitoring is in force, `CLAUSE_CLOSURE_MATRIX` and `SPEC_DEBT_STATUS` MUST reflect that truth before handoff.

### 6.11 Hygiene Gate (commands + scope)

[CX-630] HYGIENE_SCOPE: Changes SHOULD stay scoped to the task; avoid drive-by refactors or unrelated cleanups.
[CX-631] HYGIENE_COMMANDS: For repo-changing work, assistants SHOULD run (or explicitly note not run): `just docs-check`; `just codex-check`; `pnpm -C app run lint`; `pnpm -C app test`; `pnpm -C app run depcruise`; `cargo fmt`; `cargo clippy --all-targets --all-features`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`; `cargo deny check advisories licenses bans sources`.
[CX-632] HYGIENE_TODOS: When touching code near TODOs, assistants SHOULD either resolve them or leave a dated note explaining why they remain.
[CX-633] HYGIENE_DOC_UPDATE: If new entrypoints, commands, or repeatable failures are introduced or discovered, assistants SHOULD update the relevant doc (START_HERE/ARCHITECTURE/RUNBOOK_DEBUG) in the same packet unless the user defers.

### 6.12 Determinism Anchors (large-system hygiene)

[CX-640] ANCHOR_ERRORS: New errors SHOULD include stable error codes (`HSK-####`) and/or log tags; these anchors SHOULD be referenced in `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` when adding repeatable failures.
[CX-641] OWNERSHIP_MAP: Area/module ownership SHOULD be captured in `/.GOV/roles_shared/docs/OWNERSHIP.md` with paths, reviewers, and notes; packets SHOULD consult/update it when adding new surface area.
[CX-642] PRIMITIVE_TESTS: New primitives/features SHOULD ship with at least one targeted test and a short invariant note (place in `.GOV/roles_shared/docs/ARCHITECTURE.md` or inline doc comment); silence requires an explicit reason.
[CX-643] CI_GATE: Continuous integration SHOULD run `just validate` (or an equivalent subset) and block merge on failures.
[CX-644] FLAGS: New interwoven features SHOULD use a feature flag or clearly documented toggle; note the flag/toggle location in `.GOV/roles_shared/docs/ARCHITECTURE.md` or the relevant module doc.
[CX-645] ERROR_CODES_REQUIRED: New errors SHOULD introduce stable error codes/log tags (e.g., `HSK-####`) and record them in `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` when they become repeatable.
[CX-646] TEST_EXPECTATION: Logic changes SHOULD add or update at least one targeted test; if omitted, a written reason MUST be recorded in the review/task packet.
[CX-647] REVIEW_REQUIRED: Repo-changing work SHOULD have a distinct reviewer role sign off, recording commands run and outcomes.
[CX-648] SECRETS_AND_SUPPLY_CHAIN: CI SHOULD include secret scanning and dependency audit steps; assistants MUST avoid committing secrets and SHOULD pin critical dependencies/lockfiles.
[CX-649] ROLLBACK_HINTS: Reviews/commits SHOULD include a brief rollback hint (e.g., git hash or steps) for traceability.
[CX-649A] TODO_POLICY: New TODOs in source code and scripts MUST include a tracking tag in the form `TODO(HSK-####): ...` and be searchable by ID. Docs SHOULD use `TBD (HSK-####)` or explicit prose instead of TODO.

### 6.13 Task Packets as Primary Log; Logger Milestone-Only

[CX-650] TASK_LOG_PRIMARY: `.GOV/roles_shared/records/TASK_BOARD.md` + the task packet are the primary, mandatory micro-log for day-to-day work. Validation commands/outcomes and status updates MUST be recorded in the task packet. The Handshake logger is optional and reserved for milestones or hard bugs when explicitly requested.

[CX-651] LOGGER_USE_CASES: The Handshake logger SHOULD be used only when the user requests it or when recording major milestones/critical incidents. Routine Work Packet completion MUST NOT be blocked on a logger entry.

[CX-652] TASK_PACKET_VALIDATION: Before requesting commit, the coder MUST verify the task packet contains a VALIDATION block with commands run and outcomes, and that `.GOV/roles_shared/records/TASK_BOARD.md` reflects the current status.

[CX-653] TASK_PACKET_UNIQUENESS: Each Work Packet MUST have its own task packet file (do not reuse an old file for a new WP). Status/notes/validation may be updated within that WP's file as the work progresses.

[CX-655] VALIDATION_TAXONOMY_NO_COLLAPSE (HARD): Validation communications MUST NOT collapse distinct claims into a single "PASS" label. At minimum, keep these claims separate and explicit:
- Deterministic manifest gate: `just post-work {WP_ID}` (this is **not** a test pass signal)
- TEST_PLAN execution (exact commands + exit codes)
- Spec conformance confirmation (DONE_MEANS + SPEC_ANCHOR -> evidence mapping)
- Validator verdict: PASS | FAIL | OUTDATED_ONLY (only after the Validator reviews evidence and appends a report to the task packet)

[CX-655A] VALIDATOR_SPLIT_VERDICTS (HARD): When the packet format requires governed split verdicts, the validator report in `## VALIDATION_REPORTS` MUST keep at least these assessments separate: `GOVERNANCE_VERDICT`, `TEST_VERDICT`, `CODE_REVIEW_VERDICT`, `SPEC_ALIGNMENT_VERDICT`, and `ENVIRONMENT_VERDICT`. A generic PASS statement is insufficient.

[CX-655B] CLAUSE_CLOSURE_MONITOR (HARD): When the packet format includes `CLAUSE_CLOSURE_MATRIX`, `SPEC_DEBT_STATUS`, and `SHARED_SURFACE_MONITORING`, those sections are authoritative packet-scope monitoring truth. Validators and Orchestrators MUST NOT narrate full spec closure while those sections still show unresolved partial/deferred clauses, pending validator confirmation, or open blocking spec debt.

[CX-655C] SEMANTIC_PROOF_ASSETS (HARD): When the packet format includes `SEMANTIC_PROOF_ASSETS`, validators MUST inspect those assets before claiming spec alignment PASS. Diff-scoped semantic proof should be grounded in real tests, canonical examples, or governed debt rather than prose-only confidence.

[CX-655D] CLAUSES_REVIEWED_AND_NOT_PROVEN (HARD): `SPEC_ALIGNMENT_VERDICT=PASS` is legal only when the validator report records the diff-scoped clauses reviewed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is explicitly empty/none. If proof is partial, blocked, or inferred indirectly, record that explicitly and downgrade the spec-alignment verdict instead of collapsing everything into PASS.

[CX-656] TASK_BOARD_VALIDATED_GUARD (HARD): `.GOV/roles_shared/records/TASK_BOARD.md` MUST NOT move a WP to `## Done` with `[VALIDATED]` unless an official Validator report is appended to the task packet under `## VALIDATION_REPORTS` and includes:
- Explicit results for the validation taxonomy in [CX-655]
- Evidence and artifact pointers recorded in the task packet (chat summaries are secondary)

[CX-657] CANONICAL_EVIDENCE_IN_PACKET (HARD): Evidence mapping and artifact pointers MUST live in the task packet (`## EVIDENCE` / `## VALIDATION_REPORTS`). Chat outputs may mirror the evidence, but chat is not the canonical source of truth.

[CX-658] TEST_PLAN_EXECUTABLE_AS_WRITTEN (HARD): The packet `TEST_PLAN` MUST be executable as written. If commands, ranges, or tool prerequisites need to change, the packet MUST be amended (append-only) before validation proceeds, so future reviewers can reproduce the outcome deterministically.

---

## 7. Bootloader Integration (Optional)

[CX-700] BOOTLOADER_OPTIONAL: Micro-Logger, Diary, or other bootloaders are optional; this codex MUST remain usable without them.
[CX-701] BOOTLOADER_ACTIVE: When either (a) the user declares bootloader mode, or (b) a bootloader artefact is present in-session, bootloader schemas and rules become additional behavioural LAW unless explicitly disabled.

[CX-702] BOOTLOADER_DISABLE: If the user explicitly disables bootloader mode for a session, the assistant MUST treat bootloader rules as inactive for that session.

[CX-710] BOOTLOADER_STACK: Under a bootloader, the assistant MUST obey:
- Bootloader rules for logging, timestamps, and schemas.
- Hard invariants in Section 2.
- Spec usage rules in Section 5.

[CX-720] BOOTLOADER_SCHEMA_NO_TOUCH: The assistant MUST NOT change bootloader schemas unless explicitly asked to edit the bootloader itself.
[CX-721] BOOTLOADER_NO_FAKE: The assistant MUST NOT fabricate past log entries or fake history.

[CX-730] BOOTLOADER_HANDOVER: At natural boundaries in bootloader mode, the assistant SHOULD provide a short handover summary (what changed, main risks, where to continue).

---

## 8. Drift and Known Deviations

[CX-800] DRIFT_AWARENESS: The assistant SHOULD assume the codex may occasionally lag behind the actual repo; when mismatch is detected, it SHOULD call it out instead of forcing the repo to match a clearly stale rule.
[CX-801] KNOWN_DEVIATIONS_SECTION: A `KNOWN_DEVIATIONS` section MAY be added by the user to document intentional gaps between codex and reality; assistants SHOULD treat that section as overriding older conflicting rules.

[CX-810] KNOWN_DEVIATION_APP_LAYOUT: The repo currently includes `/app/` (Tauri app). If codex layout guidance conflicts with observed `/app/src` + `/app/src-tauri`, assistants MUST follow the observed layout and document the deviation in `.GOV/roles_shared/docs/ARCHITECTURE.md`.
[CX-811] KNOWN_DEVIATION_MULTI_SPECS: The repo may contain multiple `Handshake_Master_Spec_v*.md` versions in `.GOV/spec/` (current) and `.GOV/spec/history/` (prior). `.GOV/spec/SPEC_CURRENT.md` is the authoritative pointer for current work.
[CX-812] KNOWN_DEVIATION_DOC_SPLIT: `/.GOV/` is canonical operational guidance; `/.GOV/operator/` is operator-private and non-authoritative unless explicitly designated; root-level `*.md` may contain governance/history.

---

## 9. Automated Enforcement (AI Autonomy Requirements)

[CX-900] ENFORCEMENT_PURPOSE: For AI-autonomous operation, the workflow MUST be enforced by automated scripts and checks. Manual enforcement is insufficient when the human user lacks coding expertise.

[CX-901] ENFORCEMENT_SCRIPTS: The repo MUST include enforcement entrypoints in `justfile`, shared enforcement scripts in `/.GOV/roles_shared/checks/`, and role-specific enforcement scripts in `/.GOV/roles/<role>/checks/`:
- `pre-work-check.mjs` - Verifies task packet exists before work starts (includes worktree/branch preflight)
- `post-work-check.mjs` - Verifies completion evidence and deterministic manifest
- `ci-traceability-check.mjs` - CI verification of workflow compliance
- `task-board-check.mjs` - Task Board structure/format enforcement
- `task-packet-claim-check.mjs` - In-progress task packet claim fields enforcement
- `worktree-concurrency-check.mjs` - Multi-worktree topology guard
- `lifecycle-ux-check.mjs` - Operator-facing output template enforcement
- `drive-agnostic-check.mjs` - Blocks drive-specific paths in governance surface
- `gov-check.mjs` - Governance-only aggregator (runs governance checks without product scans)

[CX-902] ENFORCEMENT_HOOKS: Git hooks SHOULD enforce:
- pre-commit: Blocks commits without WP-ID traceability
- pre-push: Verifies all commits reference valid task packets
- install path: `git config core.hooksPath .GOV/roles_shared/scripts/hooks`

[CX-903] ENFORCEMENT_JUST: The `justfile` MUST include:
- `just create-task-packet {wp-id}` - Creates task packet from template
- `just pre-work {wp-id}` - Validates readiness before implementation
- `just post-work {wp-id}` - Validates completeness before commit
- `just validate-workflow {wp-id}` - Full workflow compliance check
- `just gov-check` - Governance-only health checks (no product scans)

[CX-904] ENFORCEMENT_CI: GitHub Actions SHOULD verify:
- All commits reference task packets via WP-ID
- Validation commands are documented in task packets/commits/reviews
- Logger entries are only required when explicitly requested (milestones/hard bugs)
- No commits bypass workflow requirements

[CX-905] ENFORCEMENT_FAILURE: If automated checks fail, work MUST be rejected with:
1. Clear error message indicating which rule was violated
2. Reference to codex rule number (e.g., "[CX-620]")
3. Remediation steps to fix the issue
4. AI agents MUST NOT override enforcement without explicit user permission

[CX-906] ENFORCEMENT_PROTOCOLS: The repo MUST include protocol files in `.GOV/roles/`:
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` - Mandatory checklist for orchestrators
- `.GOV/roles/coder/CODER_PROTOCOL.md` - Mandatory checklist for coders
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md` - Mandatory checklist for validators
- These protocols MUST be read by AI agents before performing their respective roles

---

## 10. Versioning

[CX-950] VERSION_ID: This codex is `Handshake Codex v1.4 (AI Autonomy with Deterministic Enforcement)`.
[CX-951] VERSION_FROM: v1.4 supersedes v1.3 for all use. v1.3 MAY still be referenced for comparison but v1.4 is authoritative.

[CX-960] CHANGE_SUMMARY_V08_1: v0.8 strengthens orchestrator and coder requirements from SHOULD to MUST for AI autonomy. Task packet creation [CX-580] and coder pre-work verification [CX-620] are now mandatory and blocking.

[CX-961] CHANGE_SUMMARY_V08_2: v0.8 adds Section 9 "Automated Enforcement" defining required scripts, hooks, and CI checks to enforce workflow deterministically without relying on AI agent compliance alone.

[CX-962] CHANGE_SUMMARY_V08_3: v0.8 clarifies workflow traceability: `.GOV/roles_shared/records/TASK_BOARD.md` + task packets are the primary micro-log; the Handshake logger is optional for milestones/hard bugs when explicitly requested.

[CX-963] CHANGE_SUMMARY_V08_4: v0.8 adds [CX-503] explicitly stating this codex is optimized for AI-autonomous operation where the human user may not have coding expertise.

[CX-964] CHANGE_SUMMARY_V08_5: v0.8 adds [CX-213] requiring a canonical Work Packet root resolver and [CX-906] requiring role protocol files under `.GOV/roles/`.

[CX-965] CHANGE_SUMMARY_V11: v1.1 adds [CX-598] and [CX-599] Hard Invariants regarding Main-Body alignment and cross-phase governance continuity. Standardizes versioning metadata across document.

[CX-966] CHANGE_SUMMARY_V12: v1.2 adds Lead Architect constraints for Orchestrators ([CX-585A-E]) and Senior Engineer constraints for Coders ([CX-625, CX-627]). Mandates Spec-Locking, Unique User Signatures, and Evidence Mapping to eliminate vibe-coding.

[CX-967] CHANGE_SUMMARY_V14: v1.4 adds Hard Invariants for Validators [CX-573D] (Zero Placeholder Policy) and [CX-573E] (Forbidden Pattern Audit) to prevent leniency. 

[CX-968] CHANGE_SUMMARY_V14_CODER: v1.4 adds Hard Invariants for Coders [CX-628] (Anti-Vibe Verification) and [CX-629] (Block-Over-Placeholder) to force adversarial self-scrutiny before submission.
[CX-969] CHANGE_SUMMARY_V14_GOVERNANCE: v1.4 clarifies that repo governance is a prototype control plane for future Handshake autonomous work [CX-503A] and formalizes universal completion layers over project-specific "done" claims [CX-503B].
[CX-970] CHANGE_SUMMARY_V14_MEMORY: v1.4 replaces the stub [CX-503K] FAILURE_MEMORY (RECOMMENDED) with a comprehensive governance memory system clause [CX-503K] plus sub-clauses [CX-503K1] (role-scoped injection), [CX-503K2] (extraction lifecycle with event-driven, session-end, check-failure, and batch paths), and [CX-503K3] (hygiene responsibility). Updates [CX-574] to acknowledge memory injection in the bootstrap context. Covers RGF-103 through RGF-143.

---

## SUMMARY FOR AI AGENTS

Drive-agnostic governance is mandatory: use repo-relative `worktree_dir` values and do not introduce drive-specific paths into governance state or instructions [CX-109].

**If you are an Orchestrator:**
1. Read `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` FIRST
2. **Refine the Spec FIRST** [CX-585A]
3. Create task packet (`just create-task-packet WP-{ID}`) - new file per WP
4. Update `.GOV/roles_shared/records/TASK_BOARD.md` to "Ready for Dev"
5. Verify (`just pre-work WP-{ID}`)
6. Only then delegate to coder
7. After delegation, remain workflow authority: launch/monitor/steer sessions, maintain packet/runtime truth, and coordinate validators without switching into coder or validator technical duties [CX-580B]

**If you are a Coder/Debugger:**
1. Read `.GOV/roles/coder/CODER_PROTOCOL.md` FIRST
2. Verify task packet exists [CX-620]
3. **Extract Governing Clauses** [CX-624]
4. **Propose Skeleton/Interface** [CX-625]
5. Set task packet `**Status:** In Progress` + claim fields and create a docs-only bootstrap claim commit (Validator status-syncs `main`)
6. Output BOOTSTRAP block [CX-622]
7. Implement within scope
8. **Run Anti-Vibe Verification [CX-628]** (contract drift, dropped fields, stale tests/examples, overclaim review)
9. **Enforce Block-Over-Placeholder [CX-629]**
10. Run validation (`just post-work {WP_ID}`)
11. **Map Evidence to Spec** [CX-627]
12. Request Validator validation/merge (Validator updates `main` Task Board to Done on PASS/FAIL)

**If you are a Reviewer/Validator:**
1. Verify task packet exists for the work
2. Verify evidence mapping exists and is accurate [CX-627]
3. **Execute Forbidden Pattern Audit [CX-573E]** (Search for `split_whitespace`, `unwrap`, etc.)
4. **Enforce Zero Placeholder Policy [CX-573D]**
5. Produce a structured Validation Report per VALIDATOR_PROTOCOL.md
6. Block merge if workflow was bypassed or spec alignment is incomplete

**Blocking rules apply.** If any MUST requirement is violated, work stops until fixed.
