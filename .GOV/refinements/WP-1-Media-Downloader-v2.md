## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Media-Downloader-v2
- CREATED_AT: 2026-02-20T07:51:30Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.134.md
- SPEC_TARGET_SHA1: 3b397673e5e54163846094bd8dfb8919ddc8c88d
- USER_REVIEW_STATUS: PENDING
- USER_SIGNATURE: <pending>
- USER_APPROVAL_EVIDENCE: <pending>

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- No spec gaps remain for this WP. Master Spec v02.134 now contains:
  - 10.14 Media Downloader (Unified Web Media Archiving Surface)
  - 2.3.10.5 OutputRootDir (Default materialization root)
- Remaining work is implementation against explicit normative requirements (queue semantics, auth/session binding, output routing, progress UI, telemetry).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Capability checks:
  - Per 11.1, every capability allow/deny MUST be recorded as a Flight Recorder event with `capability_id`, `actor_id`, `job_id` (if applicable), and `decision_outcome`.
- Bronze ingest:
  - Per 10.14.11, for each successfully stored downloaded file (video, image, caption sidecar, manifests), emit FR-EVT-DATA-001 (bronze_record_created) with:
    - ingestion_source: { type: "system", process: "media_downloader" }
    - external_source.url: the source URL (when available)
- Progress telemetry:
  - Per 10.14.11, emit leak-safe structured progress logs into the DuckDB Flight Recorder `fr_events` table with:
    - event_kind:
      - media_downloader.job_state
      - media_downloader.progress
      - media_downloader.item_result
- Export/materialize:
  - When materializing outputs to OutputRootDir (2.3.10.5), emit an ExportRecord with `materialized_paths[]` populated (2.3.10).

### RED_TEAM_ADVISORY (security failure modes)
- SSRF / local network probing: a downloader/crawler can be abused to fetch localhost/private IPs. Default-deny local/private network targets; require explicit allowlist + TTL if ever enabled (align with Stage network policy posture).
- Cookie/token leakage: cookie jars are high sensitivity. Never log cookies, never export cookie jars to OutputRootDir, store in encrypted secrets storage, and ensure artifacts containing cookies are exportable=false by default.
- Command injection: yt-dlp/ffmpeg invocations via proc.exec must never go through a shell; args must be constructed as an argv list and inputs must be validated/sanitized.
- Path traversal: remote titles/filenames must be sanitized to prevent writing outside the allowed output root or artifact store; forbid ".." and absolute paths in any materialized outputs.
- Untrusted media: downloaded files are untrusted inputs. Do not auto-execute; validate containers (ffprobe) and treat parsers as attack surface.
- Denial-of-service: extremely large playlists/topics can exhaust disk/network/CPU. Enforce bounds: max_pages, max_items, rate limits, concurrency caps, retry budgets, and clear cancel controls.
- Policy/legal: downloads MUST NOT bypass access controls or DRM; require user confirmation that they have rights to archive the requested content.

### PRIMITIVES (traits/structs/enums)
- Media downloader request/plan types:
  - enum MediaSourceKind { youtube, instagram, forumcrawler, videodownloader }
  - struct MediaDownloaderBatchRequest { sources[], source_kind, auth_mode, output_policy, crawl_options?, transcode_preset?, caption_policy? }
  - enum AuthMode { none, stage_session(stage_session_id), cookie_jar(artifact_ref) }
  - struct ForumCrawlOptions { max_pages, delay_ms, allow_cross_domain, follow_content_links, exclude_patterns[] }
  - struct DownloadProgress { bytes_done, bytes_total?, item_index, item_total, eta_seconds? }
- Artifact sidecars/manifests:
  - media_sidecar.json (url, retrieved_at, source_kind, stable_ids, sha256, bytes, captions[], errors[])
  - forumcrawler_manifest.csv/json (page_url, discovered_url, chosen_url, sha256, bytes, status, reason_skipped)
- Stage session -> cookie jar exporter (host-only):
  - Export a Netscape cookies.txt artifact derived from a selected Stage Session for allowlisted domains.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.134 explicitly defines Media Downloader and OutputRootDir as first-class normative requirements; the WP can be implemented without inventing new behavior.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: <none>

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Master Spec v02.134 defines 10.14 Media Downloader requirements and 2.3.10.5 OutputRootDir; no additional normative text is required to proceed.

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 10.14 Media Downloader (Unified Web Media Archiving Surface) (Normative)
- CONTEXT_START_LINE: 61333
- CONTEXT_END_LINE: 61348
- CONTEXT_TOKEN: ## 10.14 Media Downloader
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 10.14 Media Downloader (Unified Web Media Archiving Surface) [ADD v02.134]

  Handshake MUST provide a "Media Downloader" surface for local-first archival of personal media from the External Web into workspace artifacts, with a unified queue, resumability, and evidence-grade logging.

  ### 10.14.1 Scope (Normative)

  The Media Downloader surface MUST ship, in one unified worksurface (no split deliveries for these modes):
  - YouTube batch video archive (video/playlist/channel URLs)
  - Instagram batch archive (profile/post/reel URLs), when accessible via public access or an authorized session
  - Forum/blog topic image archive (topic URLs with pagination)
  - Generic video downloader (direct media URLs and common embed pages)

  ### 10.14.2 Execution path (Normative; no-bypass)

  - All Media Downloader operations MUST run as workflow jobs through the Workflow Engine + Mechanical Tool Bus.
  - Privileged work MUST NOT run directly from UI code.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 2.3.10.5 OutputRootDir (Default materialization root) (Normative)
- CONTEXT_START_LINE: 2766
- CONTEXT_END_LINE: 2782
- CONTEXT_TOKEN: OutputRootDir
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.10.5 OutputRootDir (Default materialization root) (Normative) [ADD v02.134]

  Handshake MUST support a user-configurable filesystem directory used as the default target for "materialize" operations across surfaces ("OutputRootDir").

  Rules (HARD):
  - OutputRootDir MUST be user-configurable (global default; may be overridden per-export).
  - OutputRootDir MUST be treated as an exfil boundary and MUST use the Export pipeline (2.3.10.1) + ExportGuard/CloudLeakageGuard where applicable.
  - ExportRecord.materialized_paths[] MUST be populated for any materialize operation.
  - Implementations MUST NOT record raw file bytes in Flight Recorder; only artifact handles, hashes, and bounded metadata.

  Default (SHOULD):
  - Default OutputRootDir SHOULD resolve to a user-writable location containing a folder named "Handshake_Output" (platform-specific resolution).

  Convention (SHOULD, but REQUIRED for Media Downloader below):
  - Surfaces SHOULD write under:
    <OutputRootDir>/<surface_id>/<feature_id>/...
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 11.1 Capabilities & Consent Model (HSK-4001 UnknownCapability)
- CONTEXT_START_LINE: 61476
- CONTEXT_END_LINE: 61488
- CONTEXT_TOKEN: HSK-4001: UnknownCapability
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 11.1 Capabilities & Consent Model

  - **Capability Registry & SSoT Enforcement ([HSK-4001]):**
    - The system MUST maintain a centralized `CapabilityRegistry` (SSoT) containing all valid Capability IDs (e.g. `fs.read`, `doc.summarize`, `terminal.exec`).
    - **Hard Invariant:** Any request for a Capability ID not defined in the Registry MUST be rejected with error `HSK-4001: UnknownCapability`. Ad-hoc or "magic string" capabilities are strictly forbidden.
    - **Audit Requirement:** Every capability check (Allow or Deny) MUST be recorded as a Flight Recorder event, capturing: `capability_id`, `actor_id`, `job_id` (if applicable), and `decision_outcome`.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 10.13 Stage Session requirements (cookies/storage isolation; persistent sessions)
- CONTEXT_START_LINE: 59894
- CONTEXT_END_LINE: 59908
- CONTEXT_TOKEN: A **Stage Session** is a browser profile
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 5.1 Session requirements

  A **Stage Session** is a browser profile with isolated:

  - cookies
  - cache
  - localStorage/indexedDB
  - service workers
  - permission decisions (where feasible)

  **MUST:** allow multiple sessions simultaneously.

  **SHOULD:** support two session types:
  - **Ephemeral**: destroyed on close (nonPersistent store).
  - **Persistent**: stored under a per-session directory, encrypted when workspace encryption is enabled.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 2.3.8.1 AllowlistCrawler posture (Tier B)
- CONTEXT_START_LINE: 2614
- CONTEXT_END_LINE: 2617
- CONTEXT_TOKEN: AllowlistCrawler (Tier B, optional)
- EXCERPT_ASCII_ESCAPED:
  ```text
  **AllowlistCrawler (Tier B, optional)**
  - Only crawl explicit domains or URL lists.
  - Respect robots/ToS and avoid bulk scraping by default.
  - Output to LocalWebCacheIndex and/or LocalDocsIndex.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 11.8 Engine: Archivist (Preservation) operations + capabilities
- CONTEXT_START_LINE: 66036
- CONTEXT_END_LINE: 66056
- CONTEXT_TOKEN: #### Engine: Archivist (Preservation)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### Engine: Archivist (Preservation)

  ##### Capabilities

  - `fs.write:artifacts`
  - `net.http`
  - `proc.exec:<archiver_allowlist>`

  ###### Operation: `archivist.capture_webpage`
  ###### Operation: `archivist.archive_video`
  ```

#### ANCHOR 7
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 11.8 Engine: Director (Video / Animation) -> director.transcode
- CONTEXT_START_LINE: 65434
- CONTEXT_END_LINE: 65442
- CONTEXT_TOKEN: ###### Operation: `director.transcode`
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### Operation: `director.transcode`

  **Params**
    - `video_ref`: ArtifactHandle (artifact.video)
    - `preset`: proxy|web|archive
    - `format`: mp4|mov

  **Outputs**
  - artifact.video
  ```

#### ANCHOR 8
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 6.1.2.2.6 Subtitle / Caption Formats (WebVTT)
- CONTEXT_START_LINE: 22516
- CONTEXT_END_LINE: 22520
- CONTEXT_TOKEN: Subtitle / Caption Formats
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 6.1.2.2.6 Subtitle / Caption Formats

  | Format | Extension | Support Level | Notes |
  |--------|-----------|--------------|-------|
  | WebVTT | `.vtt`    | \u00e2\u0153\u2026            | Explicitly supported; treated as text with timing metadata. |
  ```

#### ANCHOR 9
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 2.3.8 (batch operations may run in background with progress indicators)
- CONTEXT_START_LINE: 2569
- CONTEXT_END_LINE: 2569
- CONTEXT_TOKEN: progress indicators
- EXCERPT_ASCII_ESCAPED:
  ```text
  - Large batch operations (e.g. imports, full re-index) may run in the background with progress indicators; concrete latency targets are defined and validated using the benchmark harness described in the base research.
  ```
