# TOOLING_GUARDRAILS

Hard rules live in Codex, AGENTS, role protocols, and checks. This file is append-only shared tooling memory for recurring repo bad habits and recurring system/tool limitations.

Rules:
- Append only. Add new entries; do not rewrite or delete old ones.
- Add only recurring patterns, not one-off task incidents.
- Keep each entry short.
- Use `Do`, `Don't`, `Why`, and `Context`.
- Prefer patterns that are likely to recur across sessions, roles, WPs, or models.
- Repeated low-severity friction belongs here if it wastes time often enough.

### TG-001
- Do:
  - Use `just delete-local-worktree <worktree_id> "<approval>"` for local worktree deletion.
- Don't:
  - Do not use `Remove-Item`, `rm`, `del`, or manual fallback delete on repo/worktree paths.
- Why:
  - Malformed targets or bad git state can widen cleanup into repo-loss or disk-loss.
- Context:
  - If `git worktree remove` fails, stop and repair tooling/state instead of deleting by hand.

### TG-002
- Do:
  - Keep committed governance paths repo-relative or environment-derived.
- Don't:
  - Do not hardcode drive letters, UNC roots, or machine-local home paths.
- Why:
  - Governance must survive different machines, drives, and worktree roots.
- Context:
  - Resolve from repo root or script location, not from the current shell drive.

### TG-003
- Do:
  - Use explicit tool `workdir` or `git -C "<path>" ...` for worktree-scoped commands.
- Don't:
  - Do not rely on `cd` or `Set-Location` persisting across automated tool calls.
- Why:
  - Automation shells are often isolated.
- Context:
  - Wrong worktree reads or writes corrupt evidence and validation state.

### TG-004
- Do:
  - Retry with a repo-relative patch path first.
- Don't:
  - Do not assume patch content is wrong when the header path is the failing part.
- Why:
  - Windows path handling can fail before content is applied.
- Context:
  - Recurring in Windows-hosted WP creation/patch flows: valid patch content, failing long header path.

### TG-005
- Do:
  - Use `_` or `-` in all new file and folder names. Applies to governance, product code, and runtime artifacts.
- Don't:
  - Do not create files or folders with spaces in their names. Do not accept spaces from templates, stubs, or auto-generated paths.
- Why:
  - Spaces break `cmd.exe` quoting, junction creation (`mklink /J`), `rmdir`, shell pipelines, and copy-paste of paths. Root cause of the `D:\D:\` malformed junction bug in wt-ilja [CX-109A].
- Context:
  - Existing repo paths with spaces (e.g., `Handshake Worktrees/`, `Handshake_Artifacts/`) are legacy. Full rename is planned but deferred. All NEW paths must comply immediately.

### TG-006
- Do:
  - Inspect worktree topology before assuming Git tracking state for governance files.
- Don't:
  - Do not infer "repo ignores this folder" from one hidden file or from one worktree's local behavior.
- Why:
  - Worktree-local `.git/info/exclude`, `skip-worktree`, and kernel-junction topology can hide or expose files differently from repo-wide `.gitignore`, and wrong assumptions lead to bad cutovers.
- Context:
  - Before retiring or replacing a public governance surface, check `git ls-files <path>`, `git check-ignore -v <path>`, the current worktree/branch, and local `.git/info/exclude`.

### Append Template
- Do:
  - <short required action>
- Don't:
  - <short forbidden action>
- Why:
  - <short reason>
- Context:
  - <short tool failure or state pattern>

### TG-007
- Do:
  - Wire every new script or check into `fail-capture-lib.mjs`. Add `import { registerFailCaptureHook, failWithMemory } from "<relative-path>/fail-capture-lib.mjs";` and call `registerFailCaptureHook("filename.mjs", { role: "ROLE" });` after imports. Replace or delegate `fail()` to `failWithMemory()`.
- Don't:
  - Do not create scripts with standalone `function fail() { console.error(...); process.exit(1); }` that silently discard the error context.
- Why:
  - Script failures are written to the governance memory DB and surfaced via `memory-recall` before future actions. Without the wiring, the same failure repeats across sessions with no memory of the fix.
- Context:
  - `fail-capture-lib.mjs` is at `.GOV/roles_shared/scripts/lib/fail-capture-lib.mjs`. All 67 existing scripts are already wired. New scripts must follow the same pattern.

### TG-008
- Do:
  - Route variadic governance `just` wrappers through `node-argv-proxy.mjs` when free-text flags may contain PowerShell metacharacters such as parentheses, braces, quotes, commas, or JSON.
- Don't:
  - Do not forward raw `*FLAGS` straight into Node for wrappers like `repomem`, `memory-capture`, or similar commands that accept arbitrary operator text.
- Why:
  - PowerShell can misparse the arguments before Node receives them, which makes the failure look like a downstream script bug even when the wrapper is the real problem.
- Context:
  - Recurring on `just repomem close ... --decisions "..."` and structured memory-capture flows. The safe pattern is `.GOV/roles_shared/scripts/lib/node-argv-proxy.mjs`.

### TG-009
- Do:
  - Use safe quoting for `rg` in PowerShell, and use `rg -- '<pattern>'` when the pattern can begin with `--`.
- Don't:
  - Do not assume a failed `rg` pattern with spaces, alternation, or leading dashes means the files are missing or the repo state changed.
- Why:
  - PowerShell and ripgrep option parsing can eat or reinterpret the pattern before the actual search runs.
- Context:
  - Recurring on governance triage where the intended search term contains spaces, alternation, or a literal token that starts with `--`.

### TG-010
- Do:
  - Parse packet identity fields by extracting the canonical token you need, such as the leading 40-hex SHA in `MERGE_BASE_SHA`.
- Don't:
  - Do not validate or compare explanatory packet fields as if the full rendered field value were the raw machine token.
- Why:
  - Signed packet fields can include human-readable explanatory suffix text while still carrying one authoritative machine token.
- Context:
  - Recurring on packet-baseline worktree creation and repair where `MERGE_BASE_SHA` is displayed with extra context after the SHA.

### TG-011
- Do:
  - When `wp-receipt-append` for `REPAIR` or other repair-class receipts times out, inspect receipts, runtime projection, and session-registry truth before retrying.
- Don't:
  - Do not treat the shell timeout as proof that the receipt failed to land or that the governed auto-relay did not already fire.
- Why:
  - Receipt append can finish the write and trigger inline runtime re-projection or session wake-up before the shell tool times out.
- Context:
  - Recurring on governed repair flows where one command both writes evidence and performs the immediate next mechanical wake.

### TG-012
- Do:
  - Add new functions to an existing same-domain lib when the capability reads the same data sources or extends the same pipeline.
  - Keep CLI entry points as thin wrappers (< 30 lines) over library exports, or add flags to existing CLIs.
- Don't:
  - Do not create a new `.mjs` script without first ruling out the existing file that covers the same domain.
  - Do not put business logic in CLI scripts; keep it in importable library functions.
- Why:
  - File sprawl makes the governance surface harder to navigate, test, and maintain. Every new file adds import paths, test files, build-order entries, and cognitive overhead.
- Context:
  - Recurring pattern: a new capability (metrics, idle ledger, scope classification) gets its own script when it should be an export on the existing domain lib.

### TG-013
- Do:
  - Emit repo-relative or workspace-relative paths on operator-facing governance surfaces, for example `.GOV/...`, `../wtc-...`, `../handshake_main`, or `../gov_runtime/...`.
  - Keep absolute paths as internal script resolution detail only.
- Don't:
  - Do not print drive-specific or host-specific absolute paths in orchestrator diagnostics, monitor output, protocol examples, packet guidance, or workflow-dossier narration.
- Why:
  - Absolute paths are host-specific noise. They leak workstation topology into governance truth, make logs harder to compare across machines, and directly violate drive-agnostic governance rules.
- Context:
  - Recurring on stale-worktree diagnostics, operator monitor summaries, and protocol/examples that accidentally surface `D:/...` or other absolute paths instead of governed relative forms.

### TG-014
- Do:
  - Keep every discovered worktree `.cargo/config.toml` aligned to the canonical external artifact root from project invariants.
  - Treat Cargo `target-dir` drift and sibling artifact-root aliases such as `../Handshake Artifacts/` as governance-blocking hygiene issues and fix them before role launch or `gov-flush`.
- Don't:
  - Do not allow coder or validator worktrees to introduce alternate artifact roots such as spaced-path variants or repo-local `target/` fallbacks.
- Why:
  - Artifact-root drift makes build outputs appear random, breaks deterministic cleanup, and allows governance publish to succeed while artifact retention and NAS backup fail later in the flush pipeline.
- Context:
  - Recurring when product worktrees drift away from the canonical `Handshake_Artifacts/handshake-cargo-target` root and the mismatch only becomes visible during late `gov-flush` cleanup.

### TG-015
- Do:
  - Treat Workflow Dossier sections, imports, and generated telemetry as diagnostic evidence.
  - Preserve write-lane separation: Orchestrator notes near the top, ACP/session-control traces at EOF, terminal repomem snapshots at EOF after ACP settles.
- Don't:
  - Do not turn malformed dossier markdown, missing placeholders, duplicate live sections, or failed dossier imports into product outcome blockers.
  - Do not use mid-run manual dossier narration as a substitute for role-bound `just repomem ... --wp WP-{ID}` checkpoints.
- Why:
  - The dossier is allowed to be partial or malformed so it can preserve diagnostic evidence during live WP pressure without blocking the workflow it is observing.
- Context:
  - Recurring on closeout flows where support-surface debt was mistaken for validator authority or where ACP EOF appends conflicted with manually maintained live sections.

### TG-016
- Do:
  - Treat public `justfile` recipes with default arguments (`arg=""`) as user-facing command-surface entries that still require docs.
  - After adding or changing a recipe, run `just canonise-gov` and inspect recipe coverage warnings plus the structured review brief.
- Don't:
  - Do not rely on regexes that only match recipes without `=` in their argument list.
  - Do not assume a missing canonise warning proves a recipe is documented unless the parser covers default-arg recipes.
- Why:
  - Default-argument recipes are common for optional WP ids and flags; parser blind spots let live commands drift out of `COMMAND_SURFACE_REFERENCE.md` and `ROLE_WORKFLOW_QUICKREF.md`.
- Context:
  - Found when `artifact-root-preflight wp-id=""` was undocumented but did not appear in the original canonise warning set.
