# Task Packet: WP-1-Media-Downloader-v2

## METADATA
- TASK_ID: WP-1-Media-Downloader-v2
- WP_ID: WP-1-Media-Downloader-v2
- BASE_WP_ID: WP-1-Media-Downloader (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-20T08:17:19.638Z
- MERGE_BASE_SHA: dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4 (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
- REQUESTOR: Operator (ilja)
- AGENT_ID: codex-cli:gpt-5.2 (orchestrator)
- ROLE: Orchestrator
- AGENTIC_MODE: NO
- ORCHESTRATOR_MODEL: N/A
- ORCHESTRATION_STARTED_AT_UTC: N/A
- CODER_MODEL: Coder-A
- CODER_REASONING_STRENGTH: HIGH
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja200220260908
- PACKET_FORMAT_VERSION: 2026-02-01

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: ALLOWED
- OPERATOR_APPROVAL_EVIDENCE: coder A  can use agents.
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Media-Downloader-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement the Master Spec v02.134 "Media Downloader" surface (YouTube + Instagram + forum/blog topic image crawl + generic video) with a unified queue, resumability, progress UI, Stage Sessions-based auth, and OutputRootDir materialization.
- Why: Preserve personal/family media libraries locally-first with evidence-grade logging, without depending on third-party platform responsiveness.
- IN_SCOPE_PATHS:
  - app/src/** (Media Downloader worksurface UI + progress)
  - app/src-tauri/src/lib.rs (command bridge)
  - src/backend/handshake_core/src/jobs.rs (job queue/progress plumbing)
  - src/backend/handshake_core/src/workflows.rs (workflow ops; Media Downloader job family)
  - src/backend/handshake_core/src/storage/mod.rs (OutputRootDir + materialize routing)
  - src/backend/handshake_core/src/flight_recorder/mod.rs (media_downloader.* telemetry validators)
  - src/backend/handshake_core/src/capabilities.rs (capability IDs + UnknownCapability posture)
  - src/backend/handshake_core/src/runtime_governance.rs (consent gating for net/proc/secrets)
  - src/backend/handshake_core/mechanical_engines.json (proc.exec allowlists for yt-dlp/ffmpeg/ffprobe; if required)
- OUT_OF_SCOPE:
  - Any attempt to bypass access controls (private/members-only requires an authorized session).
  - Any DRM circumvention or paid content restriction bypass.
  - Loom integration / Lens tagging / Atelier workflows (separate WP per roadmap).
  - Any cloud escalation changes (not required for this WP).

## Dependencies
- Depends on: None (Stage Sessions requirements are implemented within this WP; no Stage MVP UI dependency).
- Blocks: WP-1-Video-Archive-Loom-Integration-v1 (video browsing/search integration depends on successful local ingestion).

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Media-Downloader-v2

# Frontend:
just lint
cd app; pnpm test
cd app; pnpm run depcruise

# Backend:
just fmt
just test

# Optional (slower, but preferred before handoff if feasible):
# just validate

just cargo-clean
just post-work WP-1-Media-Downloader-v2 --range dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4..HEAD
```

### DONE_MEANS
- A single "Media Downloader" worksurface ships that includes (in one delivery): YouTube batch archive, Instagram batch archive, forum/blog topic image crawl (pagination), and generic video downloader (Spec 10.14.1).
- OutputRootDir exists as a user-configurable default materialization root and defaults to a user-writable directory containing "Handshake_Output" (Spec 2.3.10.5); Media Downloader materializes under:
  - `<OutputRootDir>/media_downloader/youtube/`
  - `<OutputRootDir>/media_downloader/instagram/`
  - `<OutputRootDir>/media_downloader/forumcrawler/`
  - `<OutputRootDir>/media_downloader/videodownloader/`
- YouTube mode:
  - Normalizes + deduplicates URLs and expands playlist/channel-like inputs into concrete per-video targets before download (stable queue count) (Spec 10.14.7).
  - Merges separate audio/video streams into a playable container when required (Spec 10.14.7).
  - Downloads available captions as WebVTT `.vtt` sidecars and records language metadata (Spec 10.14.7 + 6.1.2.2.6).
- Generic video mode:
  - Direct media downloads stream to a `.part` file and only finalize after validation; rejects non-media payloads via content-type + sniffing heuristics; validates with ffprobe (Spec 10.14.8).
- Forum/blog topic image crawl:
  - Crawls pagination for a topic (bounded by default max_pages=1500, hard cap 5000) (Spec 10.14.9).
  - Prefers full-resolution images behind thumbnails (anchor-follow, srcset/data-fullsize, heuristic rewrites) and skips profile avatars/emojis/UI chrome; dedupes by SHA-256; writes a manifest artifact (Spec 10.14.9).
- Progress + controls:
  - UI shows per-item and per-batch progress for videos and crawler; queue supports pause/resume, cancel (one/all), retry failed; concurrency configurable (default 4; range 1..16) (Spec 10.14.10).
- Auth/session:
  - Supports no-account mode; supports Stage Sessions-based session mode with multiple persistent sessions selectable per job, and does not collect passwords in a Handshake-owned form (Spec 10.14.4 + 10.13).
  - Cookie jar artifacts for external tools are Netscape cookies.txt format, classified high, exportable=false by default, and are never written to OutputRootDir/Handshake_Output (Spec 10.14.5).
- Telemetry:
  - Emits FR-EVT-DATA-001 bronze_record_created for downloaded files and media_downloader.* progress/item events per Spec 10.14.11; capability allow/deny is recorded per 11.1; no secrets appear in payloads.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.134.md (recorded_at: 2026-02-20T08:17:19.638Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.134.md 10.14 Media Downloader (Unified Web Media Archiving Surface) (Normative)
  - Handshake_Master_Spec_v02.134.md 2.3.10.5 OutputRootDir (Default materialization root) (Normative)
  - Handshake_Master_Spec_v02.134.md 10.13 Stage Session requirements (auth/session isolation)
  - Handshake_Master_Spec_v02.134.md 11.1 Capabilities & Consent Model (HSK-4001 UnknownCapability; audit requirement)
  - Handshake_Master_Spec_v02.134.md 11.8 Engine: Archivist + Director (archiving + transcode/probe)
  - Handshake_Master_Spec_v02.134.md 6.1.2.2.6 Subtitle / Caption Formats (WebVTT)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior artifacts:
  - .GOV/task_packets/stubs/WP-1-Media-Downloader-v1.md (stub; not executable)
  - .GOV/refinements/WP-1-Media-Downloader-v1.md (refinement only; ENRICHMENT_NEEDED=YES so no packet was created)
- Preserved requirements (carried forward):
  - One unified worksurface shipping YouTube/Instagram/forum crawler/generic video together.
  - Progress UI (per-item + batch) and queue controls.
  - Full-resolution forum image preference + noise filtering + pagination.
  - Captions (.vtt) download when available.
  - Session-based auth via governed webview sessions (no password collection).
- Changed in v2:
  - Spec enriched to v02.134 (10.14 + 2.3.10.5); this packet is the first executable packet for the Base WP.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.134.md (read: 2.3.10.5 + 10.14 + 10.13 + 11.1)
  - .GOV/refinements/WP-1-Media-Downloader-v2.md
  - src/backend/handshake_core/src/jobs.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - app/src/App.tsx
  - app/src/lib/api.ts (invoke patterns + types)
  - app/src-tauri/src/lib.rs
- SEARCH_TERMS:
  - "materialize_local_dir"
  - "ExportRecord"
  - "proc.exec"
  - "net.http"
  - "HSK-4001"
  - "capability"
  - "job progress"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Media-Downloader-v2
  just lint
  just test
  ```
- RISK_MAP:
  - "SSRF / local network probing" -> "may exfiltrate LAN metadata; must default-deny localhost/private IP targets and enforce allowlist posture"
  - "Cookie/token leakage" -> "session compromise; cookie jars must be secrets, never logged, exportable=false, never materialized to OutputRootDir"
  - "External tool supply chain (yt-dlp/ffmpeg)" -> "version drift or license issues; pin and allowlist proc.exec; avoid shell invocation"
  - "Untrusted media payloads" -> "parser vulnerabilities; validate containers; never auto-open"
  - "Disk/CPU DoS" -> "large playlists/topics; enforce max_pages/items, rate limits, concurrency caps, and cancel controls"
  - "Policy/rights" -> "must not bypass access controls/DRM; require rights warning confirmation UX"

## SKELETON
- Proposed interfaces/types/contracts:
  - Media Downloader job family: enqueue requests for youtube/instagram/forumcrawler/videodownloader with shared progress reporting (Spec 10.14).
  - OutputRootDir config plumbing for default materialization root (Spec 2.3.10.5).
  - Stage Session binding for auth-required fetches; host cookie-jar artifact export for proc.exec tools (Spec 10.14.4-10.14.5).
- Open questions:
  - Tooling delivery: bundle vs user-installed yt-dlp/ffmpeg; how versions are pinned and verified (Spec 11.7/OSS policy).
  - Minimal Stage Sessions implementation required for Media Downloader auth if Stage UI is not yet built (Spec 10.13).
- Notes:
  - Private content is only downloadable when the selected session has authorized access; do not attempt to bypass access controls.

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: UI (untrusted input) -> host workflow/job execution (capability-gated side effects)
- SERVER_SOURCES_OF_TRUTH:
  - CapabilityRegistry (reject UnknownCapability; audit all allow/deny)
  - Allowlist policy + URL validation (deny localhost/private IP targets by default)
  - OutputRootDir resolution + normalized root-relative materialized_paths[]
  - Secrets storage for sessions/cookies (cookie jars never leave secrets/artifact store)
- REQUIRED_PROVENANCE_FIELDS:
  - job_id, workflow_id (if applicable), actor_id, capability_id, decision_outcome
  - external_source.url (when available), sha256, bytes, materialized_paths[]
  - stage_session_id (when using session auth)
- VERIFICATION_PLAN:
  - Gate/validator verifies materialized_paths normalization rules and that cookie jars are exportable=false.
  - Flight Recorder validates media_downloader.* event payload shapes and ExportRecord invariants.
- ERROR_TAXONOMY_PLAN:
  - "policy_denied" (capability/allowlist/rights)
  - "auth_missing" (requires session)
  - "fetch_failed" (network)
  - "payload_rejected" (non-media content)
  - "validation_failed" (ffprobe/container)
  - "tool_missing" (yt-dlp/ffmpeg not available)
- UI_GUARDRAILS:
  - Explicit rights warning + confirmation before starting downloads.
  - Clear display of selected session/no-account mode per job.
  - Prominent pause/cancel controls and bounded defaults (max pages/items).
- VALIDATOR_ASSERTIONS:
  - Media Downloader behaviors match Spec 10.14 + OutputRootDir 2.3.10.5.
  - Capability checks and ExportRecord/materialize evidence exist; no secrets in Flight Recorder.

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `path/to/file`
- **Start**: <line>
- **End**: <line>
- **Line Delta**: <adds - dels>
- **Pre-SHA1**: `<hash>`
- **Post-SHA1**: `<hash>`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Media-Downloader-v2/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
