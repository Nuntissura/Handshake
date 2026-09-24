# Handshake Codex

[CX-001] VERSION: v1.5, compact revision. The existing filename remains the active entrypoint for compatibility.

## Authority and transition

[CX-010] This Codex owns shared repo rules; `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json` owns product build and acceptance obligations; the assigned protocol under `.GOV/roles/` owns role permissions and execution. Read these authorities before acting in their scope.

[CX-011] The Master Spec owns product requirements and architecture. Resolve `.GOV/spec/SPEC_CURRENT.md` to its active manifest, resolver index and relevant modules; do not infer authority from filenames or summaries.

[CX-020] Product architecture, implementation and acceptance must follow the governing Master Spec clauses, applicable HBR rules and approved WP/MT contracts. Contracts define assigned scope; neither a roadmap nor a task-board row authorizes implementation by itself.

[CX-SPEC-TRUTH-001] The Master Spec is the sole product truth. Research, prototypes, refinement findings and Operator ideas become truth only when enrichment writes them into it; a WP stub records the idea and its enrichment disposition. Activation is refused until every requirement the packet binds resolves to a spec anchor or a quoted `spec_basis`. Remediation MTs never pass through this gate.

[CX-021] Explicit Operator instructions govern repo scope and supersede conflicting repo procedures. Role protocols, scripts, startup text and legacy documents must not override this Codex's shared rules or revive obligations expressly retired here.

[CX-012] `.GOV/operator/` is private Operator material, binding only when explicitly designated for the task.

[CX-AUTH-001] Handshake is intended to govern its own repo mechanically. Until that capability is established and verified, keep external repo governance to the minimum needed for safe edits, restartable task state, product proof and independent review; do not expand the external harness for parity or polish.

[CX-AUTH-002] Repomem, automatic memory injection/refresh/capture/compaction, mandatory session-open/close memory records, memory coverage gates and Memory Manager launches are retired repo obligations. Do not invoke them automatically or require them to proceed, including through legacy startup helpers. This retirement does not remove Handshake product-memory requirements.

[CX-AUTH-003] ACP brokers, relay loops, mailboxes, dossiers, launch ledgers, protocol-ack rituals and documentation-maintenance workflows are not universal prerequisites. Use a specific mechanism only when the current assignment needs it and it works within current authority; no legacy harness failure may be represented as a product verdict.

[CX-AUTH-004] The archived Codex and its disposition map are rollback/reference material, not active authority. Retired rules and archived incident narratives must not be reintroduced through old citations.

## Product boundaries

[CX-003-VIS] Build one handmade, user-owned, local-first, AI-native creative and execution workspace with interconnected surfaces and libraries, shared typed state, and parallel model/Operator work. Reuse current Handshake implementations unless inspected evidence justifies replacement.

[CX-008-VIS] Handshake is a native Rust application, not an Electron or webview shell. Embedded webviews belong only to the in-app browser. Older React/Tauri shell assumptions do not authorize new GUI work.

[CX-503R] SurrealDB/EventLedger is the exclusive Handshake database authority, including runtime, tests, proof and future self-governance. Do not introduce or preserve SQLite/PostgreSQL connectivity, import, reconciliation, dual authority, fallback, cache, fixture, compatibility or temporary-adapter paths. Use fresh Handshake-managed SurrealDB state and SurrealKit rollouts.

[CX-503S] Core operation and required proof must use Handshake-managed components, libraries or subprocesses. Outside apps, Docker and manually operated daemons are not implicit prerequisites or fallbacks; compatibility adapters require explicit scope.

[CX-131] Every applicable HBR rule is a mandatory product build/handoff obligation. Argus, UserManual, diagnostics, privacy, interconnectivity, quiet operation and swarm proof live in that registry; a retired harness command does not retire the underlying product requirement.

[CX-503B1] Required acceptance/HBR rows must resolve to proven evidence or a justified NOT_APPLICABLE. PENDING, STEER, BLOCKED, or deferred required behavior cannot count as acceptance.

[CX-TPL-001] Every build rule is enforced through the WP contract: a rule declares the contract fields it requires. A rulebook change that adds, changes or removes contract fields regenerates the WP and MT templates and marks open work for reassessment; activation refuses a contract missing a required field. Models submit field values and never edit template shapes.

[CX-PILLAR-001] Mandatory pillars are assessed on every WP with an explicit disposition and their declared contract fields, from the first WP and never as an afterthought: product-internal parallel runtime lanes, GUI surface with model-navigable and headless routes, user auth with accounts and sub-profiles, compartmentalised build units, canonical component mechanics, UserManual, diagnostics, Flight Recorder, and security/privacy.

[CX-UNIT-001] Every WP names its touched and created modules and a boundary change; every MT binds exactly one owning module and its write scope derives from that module. Build units are logical module paths until the compartment refactor lands crates. Activation rejects dependency-direction violations and unsettled build-budget reviews.

[CX-CAP-001] Before adding any component, find the mechanic it presents (scroll, selection, popup, focus and the like) at the lowest layer that owns it, then consume, configure or extend that owner. One active owner per capability; no MT reimplements a capability owned by another active module. A new mechanic requires a spec primitive and no existing owner.

[CX-LANE-001] A WP whose impact assessment marks change on a GUI matrix entry carries a gui-lane MT naming its backend MT and the shared surface. The GUI MT consumes only the backend's published contract and runs in parallel when write paths are disjoint.

## Repo ownership and safety

[CX-211] Handshake product code and runtime must not read or write `.GOV/`; repo governance and the shipped product are separate systems.

[CX-212C] `.GOV/` in `wt-gov-kernel` on `gov_kernel` is live shared repo authority. Product code must not be authored there. Product implementation belongs in the assigned WP worktree/branch; governance edits belong in the kernel, never through a product worktree junction.

[CX-212F] Commit governance on `gov_kernel` and product changes on the assigned product branch. Never include `.GOV/` files in feature-branch commits.

[CX-113] `main` is the sole canonical integrated branch; `user_ilja` and `gov_kernel` are backup branches. Never merge `gov_kernel` into `main`; governance reaches main through the controlled `.GOV/` sync path owned by the integration role.

[CX-113A] Canonical root control files are authored from `handshake_main` on local `main`. Kernel-local governance launchers do not transfer that authority to other worktrees.

[CX-112] Never delete protected branches `main`, `user_ilja`, `gov_kernel` or permanent worktrees `handshake_main`, `wt-ilja`, `wt-gov-kernel`.

[CX-107] Destructive filesystem operations require same-turn Operator authorization for the exact targets and consequences, except verified disposable owned-artifact cleanup explicitly authorized by [CX-984-006]. Preserve existing, untracked and other actors' work outside that exception.

[CX-108] Git operations that risk discarding, overwriting or hiding existing work, or move a branch/worktree outside the approved assignment, require same-turn Operator authorization for the exact targets and consequences. Routine checkout, switch or merge within approved scope needs no separate approval when existing work is preserved and the assigned role permits the operation.

[CX-114] Before destructive or state-hiding git operations, preserve committed state on the matching remote backup branch.

[CX-119] Before branch/worktree deletion or broad topology cleanup, also preserve an immutable external snapshot of committed refs and working files.

[CX-118] Broad cleanup/sync requests do not authorize deletion or branch movement beyond the approved assignment or the owned-artifact cleanup in [CX-984-006]. For other targets, present exact object types and consequences and obtain explicit approval; changed targets require fresh approval.

[CX-122] Never run `git worktree remove` on a worktree that still holds its `.GOV/` junction, and never delete worktree directories recursively. Deletion needs the Operator-approved target list and exactly these steps: (1) `fsutil reparsepoint query "<wt>\.GOV"` shows `Mount Point`, otherwise STOP and ask the Operator; (2) the gov kernel has no uncommitted work; (3) `cmd /c rmdir "<wt>\.GOV"` with no `/s`; (4) `wt-gov-kernel\.GOV\codex\Handshake_Codex_v1.4.md` still exists; (5) `git worktree remove <wt>`. Never use `rmdir /s`, `rm -rf`, `del` or `Remove-Item` on a worktree or its `.GOV`. Any failure: STOP, no manual cleanup.

[CX-GIT-001] No agent, test or tool creates a worktree or branch, directly or indirectly. Compare the worktree and branch inventory before and after every run; a new entry is a defect of that run, the verdict names the offending test, and the stray ref is removed by native Git after the preservation check.

[CX-GIT-003] A test removes everything it creates outside the artifact root before it ends, on success and on failure: OS credential-store entries, registry keys, temp files, processes, ports. Leftovers counted after a round are a test defect routed to the implementer; clearing another test's leftovers from a shared OS store needs Operator approval.

[CX-GIT-002] Every commit names explicit paths; a bare commit over previously staged content is forbidden. A push targets an existing declared branch and never force-pushes. A clean tree is reported as clean, never committed.

[CX-SAFE-001] Do not stop, kill, restart, suspend or otherwise disrupt a process this session did not start without identifying its exact PID and consequences and receiving `PROCESS_STOP_APPROVED:<comma-separated-PIDs>` for that unchanged target list.

[CX-SAFE-002] Automation must stay non-interactive and in the background: invoke only tools verified to be installed and non-interactive, and never trigger installers, app-store prompts, foreground windows or focus changes.

[CX-109] Keep projects and governance relocatable: use repo-relative paths, root discovery or explicit local configuration; do not embed machine-specific roots in shared authority or code.

[CX-109A] New names must use hyphens or underscores instead of spaces; preserve existing names unless the task authorizes changing them.

## Execution and proof

[CX-620] Product implementation requires an approved WP/MT contract and verified assigned worktree/branch. Read the relevant scope, dependencies, acceptance and proof requirements before editing; report missing authority without inventing it.

[CX-111] Pure repo-governance changes do not require a product WP or signature. Follow the Operator-approved scope and perform focused verification of the changed surface.

[CX-914] Author restartable task state and evidence once in existing typed contracts/records. Keep scope, current status, blockers, commit/tree, proof references and next actor recoverable without chat history. Existing Markdown is reference/projection where a typed authority exists; create new Markdown only when explicitly requested.

[CX-PROOF-002] Implementers may submit READY_FOR_VALIDATION, never self-certify COMPLETED or issue independent validator verdicts. The assigned independent validator owns acceptance and integration judgment; tests and advisory sub-agent reviews remain implementation evidence.

[CX-EXEC-001] Prioritize product repairs and acceptance outcomes. Builds, diagnostics, authority rereading and coordination must enable a specific next implementation or acceptance decision; activity volume and process liveness are not deliverable progress. Once a blocking defect is established, repair it within role authority or route the exact finding to its implementer; further investigation must resolve an uncertainty needed for the repair.

[CX-EXEC-002] Read applicable authority once per revision and scope; reopen affected sections only for changed instructions/scope or a specific unresolved question. Use existing task state and proof references; these execution rules require no new tests, receipts, reports or tracking files.

[CX-EXEC-003] After two unsuccessful attempts at the same blocker, change the technical approach on an evidence-based hypothesis or escalate concisely with the exact blocker and needed decision/dependency. Count attempts across agents, sessions and diagnostic/test retries; changing the executor does not reset the count. Do not launch another identical cycle. Preserve scope and work, continue justified unblocked work, and report the task incomplete if no justified next action remains. This standing Operator-authorized escalation qualifies persistence and HBR-STOP; it never authorizes PASS, reduced acceptance or disruption of another session's processes.

[CX-EXEC-003A] Count attempts per failing check or assertion, not per hypothesis; a new explanation for the same failure does not reset the count. Before any further run on that failure, record in the existing task state the failing assertion, what each attempt changed, a root-cause hypothesis with its code location and the intended fix; then at most one probe run and one confirming run.

[CX-EXEC-005] Timeouts are per test, set in the test runner's configuration (for Rust: nextest slow-timeout with terminate-after). Never wrap a whole build, test run or round script in a wall-clock `timeout`; a killed run yields no results for the tests it never reached. Record a per-test expiry as TIMEOUT, distinct from pass and fail; a force-stopped process is never recorded as a result.

[CX-EXEC-004] Batch related repairs within approved scope before expensive validation. During implementation, run focused proof when it determines the next edit; run required acceptance proof on stable batch inputs before readiness or PASS. A rerun requires changed relevant inputs, invalid/missing evidence, or a distinct evidence-based hypothesis. A new agent/session, MT boundary or report alone does not justify a rerun; reuse valid independent evidence under the assigned validator protocol.

[CX-EXEC-003B] CX-EXEC-003 also applies to a steering role's own rounds: a round that produces no pushed commit or verdict forces a changed approach in the next round.

[CX-EXEC-006] Deliverable progress is only pushed product commits and MT status/verdict changes. A report where neither changed says `no direct progress`. Process liveness, gate exits, compiles and reports are not progress.

[CX-EXEC-007] Implementers commit and push per MT as soon as the changed code compiles. Proof and validation always name a pushed commit, never a dirty tree. Holding compiled work uncommitted until a batch is fully proven is forbidden.

[CX-EXEC-008] Verdicts are per MT, judged from each MT JSON's own proof commands (never from a derived index), and recorded as soon as that MT's evidence is complete. The evidence comes from the union round in [CX-VAL-001], never from a build per MT. Broad or full suites run only at the WP boundary, after the MTs pass.

[CX-EXEC-009] Any agent or command expected to run over 2 minutes runs in the background, polled at least every 60 s, so steering takes effect within a minute. A long foreground wait is a brief defect.

[CX-EXEC-010] A role that steers agents gives each one a required output (commit or verdict) per 20–30 minutes. It checks outputs every 10 minutes, demands output after 20 minutes without any, and after 30 minutes replaces the agent with a fresh one that resumes from the ledger.

[CX-EXEC-011] Handoff step lists and agent plans are input, not authority over the technical approach within a step. Workflow is the Operator's alone: which role does what, which statuses exist and when they change, round procedure, relay paths and approval points. A steering role never changes workflow on its own; it proposes the change and waits. Each dispatch states the commit or verdict it will produce and by when, derived from the assigned outcome.

[CX-EXEC-012] A remediation pass fixes failures already recorded in an MT/WP (validator findings, failing tests, named blockers); the recorded failure and its stated remediation are the specification. Flow: read the failure, inspect the named code, fix, commit and push, run that MT's proof commands, record the result. Verification against the failure, current code and proof output still applies; broad research, refinements/red-team, risk/ROI listing and new audits or analysis runs do not, unless the same fix has failed twice (CX-EXEC-003).

[CX-EXEC-013] A round is one test, build or diagnostic run on an MT and ends in a pushed fix commit, a recorded verdict or a recorded blocker; otherwise it is wasted. The MT JSON records `rounds_without_output`, reset only by a new pushed commit or new verdict, and `diagnosis_refs`. At 2 the MT status becomes `NEEDS_NEW_APPROACH` and no further round on it runs until a recorded diagnosis names a cause different from every earlier one (a reworded cause is the same cause); then one probe and one confirming run, then the MT is blocked and escalated.

[CX-EXEC-014] Only the MT's named proof commands run, once per candidate commit; implementers, including the kernel builder, run compile and static checks only. Reruns on an unchanged commit, extra diagnostics, hang checks, duplicate confirmations and new tests or check scripts require a recorded remediation naming them. The only automatic retry is one recorded retry for a test on the HBR `flaky_tests` list.

[CX-VAL-001] A validation round freezes one pushed candidate commit at round start; later commits queue for the next round and never restart a running build. A round is one union round: one build and one test-runner invocation per crate or package, covering every READY_FOR_VALIDATION MT at once. Never build per MT. PASS requires that every required check's result line in the hashed log reads pass.

[CX-VAL-002] Proof reuse across commits requires the original proof's recorded relevant input paths and an empty intersection between those paths and the diff from the proof commit to the candidate commit; the reused verdict names both commits.

[CX-VAL-003] Verdicts are recorded in a declared order: accepted event with the verdict payload and idempotency key, state written once, completed event. An accepted event without its completed event is a half-written verdict; on resume, complete it under the same key, never as a new verdict.

[CX-VAL-004] Process identity is PID plus start time plus command line, recorded at launch; a bare PID is never an identity. A command expected to run over 2 minutes appends a running record before launch naming the command, commit, target path and expected outputs. On resume, read the host restart evidence before attributing a stopped job to a crash; a host restart triggers resume, not remediation.

[CX-VAL-005] A validation round starts when the implementer's fixes for the open failures are pushed, and covers all READY_FOR_VALIDATION MTs ([CX-VAL-001]). The HBR `canary_check` runs first; a canary failure is infrastructure, starts no round and yields no MT verdicts. Build only the round's test targets; if the round build breaks, rebuild without the offending MT's commits and fail that MT. Run every test without fail-fast under a per-test timeout ([CX-EXEC-005]), map results to MTs from the runner's structured output, and clean the round's outputs after verdicts are recorded. A test runner's non-zero exit after a completed run means tests failed; round scripts treat it as a result, never as a script error that aborts the remaining steps. Proofs that need a build other than the union build (release profile, fix reverted, idle host) are declared in the contract's `extra_build_proofs` at activation and all run in one extra build at the end of the WP.

[CX-VAL-006] Every failure carries `failure_kind` product, infrastructure or flaky; `failure_class` remains the cause id. Only product failures become MT remediation. An infrastructure failure (harness setup, host load, environment) changes no MT status. Classify from evidence in the failure output, never from timing correlation alone. A failing test owned by no MT is attributed by narrowing over the round's MT commits.

[CX-STATUS-001] MT `lifecycle.status` takes only these values: READY_FOR_VALIDATION, PASS_Vn, FAIL_Vn, BLOCKED (with `lifecycle.blocked_on` naming the exact item) and NEEDS_NEW_APPROACH ([CX-EXEC-013]). Transitions: an implementer commit newer than the MT's last verdict makes it READY_FOR_VALIDATION; a validator verdict sets PASS_Vn or FAIL_Vn; waiting on a scheduled proof run or a named open fix sets BLOCKED; an infrastructure failure changes nothing. No role invents another status; a needed new status is an Operator decision.

[CX-HOST-001] Every project declares its test environment in a machine-local host profile outside Codex law: runtime roots, database sync mode, default check timeouts, the hang procedure of dump and stack walk before any kill, and a tools manifest. A check run outside the declared environment is a defect, not a result.

## Artifact isolation

[CX-PATH-001] The Operator does not insert backslashes before underscores. Treat any such sequence encountered in an Operator-provided path as a text-processing artifact, never an Operator mistake or intended directory separator. Reason: formatting escapes can be mistaken by the assistant for filesystem separators, splitting one folder name into two and causing incorrect resolution or unwanted folder creation.

[CX-PATH-002] Inspect the filesystem to verify the intended path before using or saving a path affected by [CX-PATH-001].

[CX-PATH-003] Ask the Operator only if the destination affected by [CX-PATH-001] remains unresolved after filesystem inspection.

[CX-984-001] The sole build/test/tool artifact root is `Handshake Worktrees\Handshake_Artifacts`, relative to the enclosing Handshake project folder. `Handshake_Artifacts` is one directory name. Locate that project folder from the canonical live kernel checkout that owns this Codex (resolve any `.GOV` junction to its real owner first); never resolve the path from an arbitrary working directory, WP worktree or document directory. Store portable paths in shared files; absolute paths resolved at runtime are allowed.

[CX-984-012] Verify that the sole root exists and resolves to the Operator-designated directory before creating output. If missing, ambiguous or inconsistent with configured paths, stop the affected run and resolve the discrepancy; never auto-create an artifact root, search for a convenient substitute or fall back to another disk, project or sibling folder. An explicit Operator grant under [CX-984-014] is not a fallback. Creating owned children is allowed only beneath the verified root.

[CX-984-013] `HANDSHAKE_ARTIFACTS_ROOT` and the legacy `HANDSHAKE_ARTIFACT_ROOT` may convey only that same verified root; they do not authorize alternatives. Resolve the project-relative path in [CX-984-001] from the enclosing project folder; the equivalent `../Handshake_Artifacts` is valid only from the canonical kernel checkout root. Never resolve relative overrides against process working directories. Pass the verified absolute path to subprocesses and report it before launch. A root relocation requires explicit Operator instruction.

[CX-984-002] Every WP must have its own `<WP_ID>/` subfolder beneath the sole root. MT work uses `<WP_ID>/<MT_ID>/<OWNER_SLUG>/`; packet-level work without an MT uses `<WP_ID>/<OWNER_SLUG>/`. Cargo targets, caches, logs, coverage, TMP and TEMP belong below that owner. Concurrent owners must not share mutable output. Run at most one build per physical disk at a time.

[CX-984-014] The Operator may grant a capped, WP-scoped build-output location on another disk. The grant covers only the build target directory (`CARGO_TARGET_DIR` or equivalent), must be recorded with its path and size cap, stays within that cap, and is cleaned when the WP closes. The steering role checks the target size against the cap before every round. Validation builds keep targets small: incremental compilation off (`CARGO_INCREMENTAL=0`) and debug info limited to line tables (`CARGO_PROFILE_DEV_DEBUG` and `CARGO_PROFILE_TEST_DEBUG` = `line-tables-only`). Test runtime stores, TMP/TEMP, workspaces, logs and evidence remain under the sole root.

[CX-984-006] Routine cleanup under this rule is authorized without a separate approval. Clean no-longer-needed owned output after each run; after a WP is validated PASS, clean its remaining disposable output before integration. First verify resolved paths stay inside that WP and no active process uses the targets. Preserve required review evidence and still-needed reuse with an explicit retention reason; remove retained output when that need ends. Remove the WP folder when empty, never the artifact root or another WP's output. The parent checks delegated cleanup; another owner's active or retained output requires coordination before removal.

[CX-984-008] Before launch, inspect effective runner/configuration paths, including command-line overrides and junction/symlink targets, and verify every output stays under the assigned owner in the sole root. An inherited environment variable or root-only check does not prove isolation; a conflicting launcher must be corrected before use.

[CX-984-009] Shared batch proof names one actual owning MT and all covered MTs; its output stays below that WP/MT. Do not duplicate builds merely to populate another MT's evidence.

[CX-984-010] These root, hierarchy and cleanup rules supersede conflicting HBR, role-protocol, startup and helper instructions. Existing scripts cannot authorize working-directory-dependent resolution, automatic root creation or a fallback root. All other HBR isolation and provenance obligations remain mandatory.

## Authority maintenance

[CX-105] Change Codex, build rules or Master Spec only on explicit Operator instruction and within the assigned role's authority; the owning role protocol defines the publishing procedure. Present material changes for review; an approved concrete edit needs no repeated approval.

[CX-GOV-PIN-001] Every in-flight WP is pinned to the governance commit it was activated under and is judged under that pin. Later Codex, HBR or template changes apply from the next WP; an open WP is re-pinned only through reassessment and renewed activation.

[CX-MAINT-001] Keep this Codex small. Give each durable obligation a stable ID; preserve surviving IDs and record folds/retirements without reusing an ID for unrelated law. Put detailed product gates in HBR and execution steps in the owning role protocol.

[CX-MAINT-002] Authority/document-only edits require focused checks of meaning, protected text, IDs and live references. Blanket governance checks, canonicalisation passes, documentation lints, projection regeneration and governance-board paperwork are not mandatory for such edits. Executable changes still require the relevant behavioral checks.

[CX-MAINT-003] Existing files and tools outside this Codex may retain legacy requirements during the transition. Identify an actual conflict when it affects work; do not silently bypass a product/safety gate, broadly repair unrelated documentation, or reimpose retired harness obligations.

Archive: [original v1.4](archive/Handshake_Codex_v1.4-before-compact-rewrite.md). Rule history: [initial rewrite](archive/Handshake_Codex_v1.4-to-v1.5-rule-map.json), [second trim](archive/Handshake_Codex_v1.5-second-trim-rule-map.json). These are reference material, not active authority.
