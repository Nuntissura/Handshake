# Task Packet: WP-1-Media-Downloader-v2

## METADATA
- TASK_ID: WP-1-Media-Downloader-v2
- WP_ID: WP-1-Media-Downloader-v2
- BASE_WP_ID: WP-1-Media-Downloader (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-20T08:17:19.638Z
- MERGE_BASE_SHA: dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4
- REQUESTOR: Operator (ilja)
- AGENT_ID: codex-cli:gpt-5.2 (orchestrator)
- ROLE: Orchestrator
- AGENTIC_MODE: NO
- ORCHESTRATOR_MODEL: N/A
- ORCHESTRATION_STARTED_AT_UTC: N/A
- CODER_MODEL: Coder-A
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
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
  - Handshake_Master_Spec_v02.134.md
  - .cargo/config.toml
  - justfile
  - app/src/**
  - app/src/App.tsx
  - app/src/components/MediaDownloaderView.tsx
  - app/src/lib/mediaDownloader.ts
  - app/src-tauri/src/lib.rs
  - src/backend/handshake_core/mechanical_engines.json
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/jobs.rs
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/workflows.rs
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
- Waiver ID: CX-WT-001
  - Date: 2026-02-22
  - Check waived: `just hard-gate-wt-001`
  - Scope: WP-1-Media-Downloader-v2
  - Justification: User explicitly authorized skipping the hard-gate; worktree/branch verified by `just pre-work` preflight [CX-WT-001].
  - Approver: User (Operator)
  - Expiry: WP-1-Media-Downloader-v2 closure

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

## SKELETON
- Proposed interfaces/types/contracts (no logic):
  - Job kinds + protocol IDs (backend; `storage::JobKind` + `workflows::run_job`):
    - `media_downloader` (protocol_id: `hsk.media_downloader.batch.v0`)
    - `media_downloader_control` (protocol_id: `hsk.media_downloader.control.v0`) - expresses pause/resume/cancel/retry as a governed workflow operation without needing new HTTP routes.
  - Batch request schema (job_inputs JSON; validated server-side):
    - `schema_version`: `hsk.media_downloader.batch@v0`
    - `source_kind`: `youtube|instagram|forumcrawler|videodownloader`
    - `sources`: `string[]` (raw URLs; server normalizes + dedupes before enqueue per Spec 10.14.7)
    - `auth`: `{ mode: none|stage_session|cookie_jar, stage_session_id?, cookie_jar_artifact_ref? }`
    - `output`: `{ output_root_dir_override?: string, materialize: true }`
    - `controls`: `{ concurrency: u8 (default=4, range=1..16), retry_failed: bool, pause_on_error: bool }`
    - `forumcrawler?`: `{ max_pages (default=1500, cap=5000), delay_ms?, exclude_patterns?: string[] }`
    - `videodownloader?`: `{ sniff_bytes?, allow_embed_discovery?, require_ffprobe: true }`
  - Control request schema (job_inputs JSON; validated server-side):
    - `schema_version`: `hsk.media_downloader.control@v0`
    - `target_job_id`: `string`
    - `action`: `pause|resume|cancel_one|cancel_all|retry_failed`
    - `item_id?`: `string`
  - Batch outputs schema (job_outputs JSON; updated as progress occurs):
    - `schema_version`: `hsk.media_downloader.result@v0`
    - `plan`: `{ stable_item_total, items[]: { item_id, source_kind, url_canonical, stable_ids? } }`
    - `progress`: `{ state, item_done, item_total, bytes_downloaded?, bytes_total?, concurrency }`
    - `items[]`: `{ item_id, status, artifact_handles[], materialized_paths[], error_code?, error_message? }`
    - `export_records[]`: `ExportRecord`-shaped objects with `materialized_paths[]` root-relative under OutputRootDir (Spec 2.3.10 + 2.3.10.5 + 10.14.6)
  - Artifact sidecars/manifests:
    - `media_sidecar.json` per downloaded media (+ captions): `{ url, retrieved_at, source_kind, stable_ids, sha256, bytes, captions[], errors[] }`
    - `forumcrawler_manifest.json|csv`: `{ page_url, discovered_url, chosen_url, sha256, bytes, status, reason_skipped }` rows (Spec 10.14.9)
    - Cookie jar artifact: Netscape `cookies.txt` (classification=high, exportable=false, never materialized to OutputRootDir) (Spec 10.14.5)
  - Telemetry payloads (Flight Recorder; leak-safe; no secrets):
    - System events with `event_kind`:
      - `media_downloader.job_state`
      - `media_downloader.progress`
      - `media_downloader.item_result`
      (Spec 10.14.11; payload includes job_id, source_kind, url (sanitized), bytes_downloaded, bytes_total?, item_index, item_total, status, error_code?)
    - Bronze ingest: extend FR-EVT-DATA-001 payload (`data_bronze_created`) to include:
      - `ingestion_source: { type: \"system\", process: \"media_downloader\" }`
      - `external_source.url` (sanitized) where available (Spec 10.14.11)
  - Capabilities + allowlists (Spec 10.14.3 + 11.1):
    - Required (minimum) for batch protocol:
      - `fs.write:artifacts`
      - `net.http` (plus explicit domain allowlist enforcement in workflow)
      - `proc.exec:yt-dlp`
      - `proc.exec:ffmpeg`
      - `proc.exec:ffprobe`
      - `secrets.use` (when `auth.mode != none`)
    - `mechanical_engines.json`: add a scoped Media Downloader / Archivist op whose allowlisted `proc.exec:*` scopes include only yt-dlp/ffmpeg/ffprobe (no generic `proc.exec`)
    - `capabilities.rs`: add a dedicated profile (e.g. `Archivist`) mapped to `media_downloader*` job kinds with only required caps; preserve `HSK-4001: UnknownCapability` posture.
  - OutputRootDir config plumbing (Spec 2.3.10.5):
    - Runtime config file under workspace: `.handshake/gov/output_root_dir.json` storing an absolute OutputRootDir path.
    - Tauri command bridge (`app/src-tauri/src/lib.rs`) provides get/set for OutputRootDir and Stage Session registry (without exposing secrets in logs).
    - Materialization root conventions (required for this surface): `<OutputRootDir>/media_downloader/<source_kind>/...` (Spec 10.14.6).
- Open questions / assumptions:
  - Stage Sessions (Spec 10.13 + 10.14.4): Do we implement governed WebView Stage Sessions in this WP, or is a v2 stopgap acceptable where users import cookie jars as persistent sessions (still no password capture)?
  - Cookie extraction implementation: if governed WebView is required, which platform is the target first (Windows/WebView2) and what API is approved for cookie export?
  - URL/domain allowlist: confirm the allowlisted domains set for YouTube/Instagram archival (to enforce deny-by-default for crawls per Spec 10.14.3 + 10.14.9).
  - External tooling: bundle vs user-installed `yt-dlp`/`ffmpeg`/`ffprobe` and how tool versions are discovered/pinned (supply chain posture).
- Notes:
  - Private content is only downloadable when the selected session has authorized access; do not attempt to bypass access controls.
  - All URLs in telemetry MUST be sanitized (no tokens/query secrets) (Spec 10.14.11 + \"no secrets in payloads\").

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
- TRANSPORT_SCHEMA:
  - UI -> backend: `CreateJobRequest.job_inputs` carries `hsk.media_downloader.batch@v0` and `hsk.media_downloader.control@v0` payloads.
  - Backend -> UI: `AiJob.job_outputs` carries `hsk.media_downloader.result@v0` plus per-item summaries and materialized paths.
  - Backend -> FR: `event_type=system` with `event_kind=media_downloader.*`; `event_type=data_bronze_created` for successful ingest.
- VERIFICATION_PLAN:
  - Gate/validator verifies materialized_paths normalization rules and that cookie jars are exportable=false.
  - Flight Recorder validates media_downloader.* event payload shapes and ExportRecord invariants.
- ERROR_TAXONOMY_PLAN:
  - "policy_denied" (capability/allowlist)
  - "auth_missing" (requires session)
  - "fetch_failed" (network)
  - "payload_rejected" (non-media content)
  - "validation_failed" (ffprobe/container)
  - "tool_missing" (yt-dlp/ffmpeg not available)
- UI_GUARDRAILS:
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
- **Target File**: `Handshake_Master_Spec_v02.134.md`
- **Start**: 1
- **End**: 68397
- **Line Delta**: -1
- **Pre-SHA1**: `3b397673e5e54163846094bd8dfb8919ddc8c88d`
- **Post-SHA1**: `b846f04093f1bd6fae885876affc99a21065ec95`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `app/src-tauri/src/lib.rs`
- **Start**: 1
- **End**: 559
- **Line Delta**: 453
- **Pre-SHA1**: `84f82b2af9ab377fcb2656be9b6de5d3bc205413`
- **Post-SHA1**: `3324391fb08df76ebfe7f83158c04d1b6dcd27b8`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `app/src/App.tsx`
- **Start**: 1
- **End**: 215
- **Line Delta**: 11
- **Pre-SHA1**: `c3bf563d04d8f848c2a9d32a361c8d8a7be548be`
- **Post-SHA1**: `4ce4a3c6791a8a371882e19e0e09c1c8d2789614`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `app/src/components/MediaDownloaderView.tsx`
- **Start**: 1
- **End**: 654
- **Line Delta**: 654
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `65f681d5ae0fa430a2f205e2e455420a26877577`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `app/src/lib/mediaDownloader.ts`
- **Start**: 1
- **End**: 36
- **Line Delta**: 36
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `507bb92943891947d77ef26edd5f00189cc28266`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `.cargo/config.toml`
- **Start**: 1
- **End**: 4
- **Line Delta**: 0
- **Pre-SHA1**: `7869613e53ac3f4b43e6d1aae29d8ea21086c7a8`
- **Post-SHA1**: `a1c73433f10cef5f2195047b91a198367032826a`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `justfile`
- **Start**: 1
- **End**: 259
- **Line Delta**: 4
- **Pre-SHA1**: `fb3f363b6bd2c642b4755e56a0d8ab72fcbfaefd`
- **Post-SHA1**: `5ad5099435d80dd3c57d51173640165c2a37a6d4`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/mechanical_engines.json`
- **Start**: 1
- **End**: 211
- **Line Delta**: 46
- **Pre-SHA1**: `adcf990b326b4c52abd3b75ae528c115ea3ad52a`
- **Post-SHA1**: `402e5bc2d02678a24c70c06b11de4ed51c34f7b0`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/capabilities.rs`
- **Start**: 1
- **End**: 668
- **Line Delta**: 46
- **Pre-SHA1**: `c6a94e35829d6f560ff0dea250dabd9fb7cc96c1`
- **Post-SHA1**: `f23df3d836f0eeb3fed539080678ae7003ba1193`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 1
- **End**: 1600
- **Line Delta**: 19
- **Pre-SHA1**: `76c93729938ff7a30750e2d69425235ac4c287b2`
- **Post-SHA1**: `fa76c2c5e02a08be156ee24d114024eaebdf6659`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 1
- **End**: 16277
- **Line Delta**: 6381
- **Pre-SHA1**: `13773956b14c10256cce253fc3c7e7bc3a88583c`
- **Post-SHA1**: `f042af9ce3ed8ca0d56303f225e1deab6bb1c76f`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: Ready for Validation
- What changed in this update:
  - Implemented Media Downloader worksurface + backend workflow family (YouTube/Instagram via yt-dlp, forum/blog image crawler, generic video downloader).
  - Implemented OutputRootDir config + Stage Sessions + cookie import.
  - Remediated cancel/kill-tree/shutdown reliability for external download processes.
  - Remediated validator-error-codes/validator-scan findings (typed WorkflowError + nondeterminism waiver + no expect()).
  - Routed Cargo build artifacts to external target dir (`../Handshake Artifacts/handshake-cargo-target`).
- Touched files (staged):
  - .GOV/refinements/WP-1-Media-Downloader-v2.md
  - .GOV/task_packets/WP-1-Media-Downloader-v2.md
  - .cargo/config.toml
  - Handshake_Master_Spec_v02.134.md
  - app/src-tauri/src/lib.rs
  - app/src/App.tsx
  - app/src/components/MediaDownloaderView.tsx
  - app/src/lib/mediaDownloader.ts
  - justfile
  - src/backend/handshake_core/mechanical_engines.json
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/workflows.rs
- Next step / handoff hint: Hand off to Validator (staged changes include packet + product code). Note: worktree has untracked `.GOV/validator_gates/WP-1-Media-Downloader-v2.json` local validator state.

## EVIDENCE_MAPPING
- REQUIREMENT: "DONE_MEANS: A single \"Media Downloader\" worksurface ships: YouTube batch archive, Instagram batch archive, forum/blog topic image crawl, and generic video downloader (Spec 10.14.1)."
- EVIDENCE: `app/src/components/MediaDownloaderView.tsx:397`
- EVIDENCE: `app/src/components/MediaDownloaderView.tsx:398`
- EVIDENCE: `app/src/components/MediaDownloaderView.tsx:399`
- EVIDENCE: `app/src/components/MediaDownloaderView.tsx:400`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:9918`

- REQUIREMENT: "DONE_MEANS: OutputRootDir exists as a user-configurable default materialization root and defaults to a user-writable directory containing \"Handshake_Output\" (Spec 2.3.10.5)."
- EVIDENCE: `app/src-tauri/src/lib.rs:164`
- EVIDENCE: `app/src-tauri/src/lib.rs:355`
- EVIDENCE: `app/src-tauri/src/lib.rs:362`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11108`

- REQUIREMENT: "DONE_MEANS: Media Downloader materializes under <OutputRootDir>/media_downloader/<source_kind>/... (Spec 10.14.6)."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11339`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:15638`

- REQUIREMENT: "DONE_MEANS: YouTube mode normalizes/dedupes URLs and expands playlist/channel-like inputs into concrete per-video targets before download (stable queue count) (Spec 10.14.7)."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11357`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:12393`

- REQUIREMENT: "DONE_MEANS: YouTube mode merges separate audio/video streams when required (Spec 10.14.7)."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:15375`

- REQUIREMENT: "DONE_MEANS: YouTube mode downloads captions as WebVTT .vtt sidecars and records language metadata (Spec 10.14.7 + 6.1.2.2.6)."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:15379`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:15476`

- REQUIREMENT: "DONE_MEANS: Generic video mode streams to a .part file and only finalizes after validation; rejects non-media payloads via sniffing heuristics; validates with ffprobe (Spec 10.14.8)."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:12869`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:12586`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:12577`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:12909`

- REQUIREMENT: "DONE_MEANS: Forum/blog topic image crawl paginates bounded by default max_pages=1500, hard cap 5000 (Spec 10.14.9)."
- EVIDENCE: `app/src/components/MediaDownloaderView.tsx:110`
- EVIDENCE: `app/src/components/MediaDownloaderView.tsx:427`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:10318`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:13227`

- REQUIREMENT: "DONE_MEANS: Forum/blog crawler prefers full-resolution images behind thumbnails (srcset/data-fullsize + heuristic rewrites) (Spec 10.14.9)."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:13107`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:13132`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:13464`

- REQUIREMENT: "DONE_MEANS: Forum/blog crawler dedupes by SHA-256 and writes a manifest artifact (Spec 10.14.9)."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:13264`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:13667`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:14167`

- REQUIREMENT: "DONE_MEANS: Forum/blog crawler enforces allowlist posture and blocks localhost/private ranges by default (Spec 10.14.9 + 10.13)."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:13081`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:13346`

- REQUIREMENT: "DONE_MEANS: UI shows per-item and per-batch progress; queue supports pause/resume, cancel (one/all), retry failed; concurrency configurable (default 4; range 1..16) (Spec 10.14.10)."
- EVIDENCE: `app/src/components/MediaDownloaderView.tsx:109`
- EVIDENCE: `app/src/components/MediaDownloaderView.tsx:419`
- EVIDENCE: `app/src/components/MediaDownloaderView.tsx:564`
- EVIDENCE: `app/src/components/MediaDownloaderView.tsx:636`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:10593`

- REQUIREMENT: "DONE_MEANS: Stage Sessions-based session mode is supported and does not collect passwords in a Handshake-owned form (Spec 10.14.4 + 10.13)."
- EVIDENCE: `app/src-tauri/src/lib.rs:382`
- EVIDENCE: `app/src-tauri/src/lib.rs:389`
- EVIDENCE: `app/src-tauri/src/lib.rs:421`
- EVIDENCE: `app/src-tauri/src/lib.rs:461`
- EVIDENCE: `app/src/components/MediaDownloaderView.tsx:479`
- EVIDENCE: `app/src/components/MediaDownloaderView.tsx:513`

- REQUIREMENT: "DONE_MEANS: Cookie jars are Netscape cookies.txt format, classified high, exportable=false, and never written to OutputRootDir/Handshake_Output (Spec 10.14.5)."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:10799`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:10997`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11012`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11013`

- REQUIREMENT: "DONE_MEANS: Telemetry emits data_bronze_created + media_downloader.* events with sanitized URLs and no secrets in payloads (Spec 10.14.11 + 11.1)."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:10179`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11431`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:12834`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:12841`

- REQUIREMENT: "Extra attention: cancel uses robust job_id parsing and kills external downloader process trees on cancel/close."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:10642`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:6095`
- EVIDENCE: `app/src-tauri/src/lib.rs:53`

## EVIDENCE
- COMMAND: `just pre-work WP-1-Media-Downloader-v2`
- EXIT_CODE: 0
- PROOF_LINES: "Pre-work validation PASSED"

- COMMAND: `just post-work WP-1-Media-Downloader-v2`
- EXIT_CODE: 0
- PROOF_LINES: "Post-work validation PASSED (deterministic manifest gate; not tests) with warnings"
- PROOF_LINES: "Warnings: Manifest[1] could not load HEAD (spec file); Manifest[4]/Manifest[5] could not load HEAD (new files)"

- COMMAND: `just lint`
- EXIT_CODE: 0
- PROOF_LINES: "eslint src --ext .ts,.tsx"

- COMMAND: `cd app; pnpm test`
- EXIT_CODE: 0
- PROOF_LINES: "vitest run"

- COMMAND: `cd app; pnpm run depcruise`
- EXIT_CODE: 0
- PROOF_LINES: "no dependency violations found"

- COMMAND: `just fmt`
- EXIT_CODE: 0
- PROOF_LINES: "cargo fmt"

- COMMAND: `just test`
- EXIT_CODE: 0
- PROOF_LINES: "cargo test"

- COMMAND: `just cargo-clean`
- EXIT_CODE: 0
- PROOF_LINES: "cargo clean -p handshake_core"
- PROOF_LINES: "target-dir \"../Handshake Artifacts/handshake-cargo-target\""
 
- COMMAND: `just validator-error-codes`
- EXIT_CODE: 0
- PROOF_LINES: "validator-error-codes: PASS"
 
- COMMAND: `just validator-scan`
- EXIT_CODE: 0
- PROOF_LINES: "validator-scan: PASS"
 
- COMMAND: `just post-work WP-1-Media-Downloader-v2`
- EXIT_CODE: 0
- PROOF_LINES: "Post-work validation PASSED (deterministic manifest gate; not tests) with warnings"
- PROOF_LINES: "Warnings: Out-of-scope files changed but waiver present [CX-573F]: .cargo/config.toml, justfile"
 
## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
