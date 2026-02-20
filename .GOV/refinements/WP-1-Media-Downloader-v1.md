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
- WP_ID: WP-1-Media-Downloader-v1
- CREATED_AT: 2026-02-20T05:44:10Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.133.md
- SPEC_TARGET_SHA1: 9dac473bd1aa01b6d2900874169869c915fc355f
- USER_REVIEW_STATUS: PENDING
- USER_SIGNATURE: <pending>
- USER_APPROVAL_EVIDENCE: <pending> (must equal: APPROVE REFINEMENT WP-1-Media-Downloader-v1)

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- The Master Spec includes key building blocks (Archivist engine, Stage sessions, AllowlistCrawler posture, Export/Materialize contract, capability registry) but does NOT yet define a unified "Media Downloader" surface and its normative requirements (YouTube + Instagram + forum/blog image crawl + generic video).
- The Master Spec defines Archivist operations (`archivist.capture_webpage`, `archivist.archive_video`) but does not specify:
  - batch URL expansion semantics (playlist/channel -> stable per-item queue)
  - caption download requirements and WebVTT sidecar conventions for archived videos
  - forum/blog topic image crawling requirements (pagination, full-res behind thumbnails, skip avatars/emojis/ui chrome)
  - authentication/session sourcing requirements for archivist-like jobs (no password collection, multi-profile sessions, cookie jar handling)
- The Master Spec defines Export/Materialize as the filesystem exfil boundary, but does not define a default user-facing materialization root (e.g., "Handshake_Output") and subfolder conventions for surface outputs.
- Flight Recorder has strict schema enforcement for known FR-EVT families; the spec does not yet define a canonical telemetry pattern for media download progress and per-item outcomes.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Capability checks:
  - Per 11.1, every capability allow/deny MUST be recorded as a Flight Recorder event with `capability_id`, `actor_id`, `job_id` (if applicable), and `decision_outcome`.
- Bronze ingest:
  - For each successfully stored downloaded file (video, image, caption sidecar, manifests), emit FR-EVT-DATA-001 (bronze_record_created) with:
    - ingestion_source: { type: "system", process: "media_downloader" }
    - external_source.url: the source URL (when available)
- Progress telemetry (schema gap):
  - Proposed enrichment below defines a leak-safe structured progress log scheme using the DuckDB Flight Recorder `fr_events` table with event_kind:
    - media_downloader.job_state
    - media_downloader.progress
    - media_downloader.item_result
- Export/materialize:
  - When materializing outputs to a user-chosen path (OutputRootDir/Handshake_Output), emit an ExportRecord with `materialized_paths[]` populated (2.3.10).

### RED_TEAM_ADVISORY (security failure modes)
- SSRF / local network probing: a downloader/crawler can be abused to fetch localhost/private IPs. Default-deny local/private network targets; require explicit allowlist + TTL if ever enabled (align with Stage network policy posture).
- Cookie/token leakage: cookie jars are high sensitivity. Never log cookies, never export cookie jars to OutputRootDir, store in encrypted secrets storage, and ensure artifacts containing cookies are exportable=false by default.
- Command injection: yt-dlp/ffmpeg invocations via proc.exec must never go through a shell; args must be constructed as an argv list and inputs must be validated/sanitized.
- Path traversal: remote titles/filenames must be sanitized to prevent writing outside the allowed output root or artifact store; forbid ".." and absolute paths in any materialized outputs.
- Untrusted media: downloaded files are untrusted inputs. Do not auto-execute; validate containers (ffprobe) and treat parsers as attack surface.
- Denial-of-service: extremely large playlists/topics can exhaust disk/network/CPU. Enforce bounds: max_pages, max_items, rate limits, concurrency caps, retry budgets, and clear cancel controls.
- Policy/legal: downloading must not bypass access controls or DRM; require user confirmation that they have rights to archive the requested content.

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
  - export a Netscape cookies.txt artifact for an allowlisted domain from a Stage Session (never exposed to External Web / Stage Apps directly).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: FAIL
- CLEARLY_COVERS_REASON: The Master Spec provides Archivist/Stage/export/capability primitives but does not define a Media Downloader surface and its required behaviors (full-res forum image archive, multi-source batch download, output routing). Spec enrichment is required to avoid inventing requirements in a locked packet.
- AMBIGUITY_FOUND: YES
- AMBIGUITY_REASON: Media Downloader is operator-requested but not yet specified as a first-class surface/job family; required behaviors (pagination, full-res inference, auth, output root) must be made normative.

### ENRICHMENT
- ENRICHMENT_NEEDED: YES
- REASON_NO_ENRICHMENT: <not applicable; ENRICHMENT_NEEDED=YES>

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
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

---

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

### 10.14.3 Capability gating (Normative)

Required capabilities (minimum):
- fs.write:artifacts
- net.http (scoped by allowlist policy; deny-by-default for non-allowlisted domains when crawling)
- proc.exec:<archiver_allowlist> (yt-dlp or equivalent)
- proc.exec:<video_allowlist> (ffmpeg/ffprobe when merging/transcoding/probing)
- secrets.use (when using stored sessions/cookies/tokens)

Unknown capability IDs MUST be rejected (HSK-4001).

### 10.14.4 Authentication and sessions (Normative)

Media Downloader MUST support:
- No-account mode (public access only).
- Session mode using Stage Sessions (10.13):
  - The user authenticates on the site-owned login page within a governed WebView session.
  - Handshake MUST NOT request or collect passwords in a Handshake-owned form.
  - Multiple persistent sessions ("accounts") MUST be supported and selectable per job.

Session storage MUST follow Stage session isolation rules (cookies/storage/cache must not bleed).

### 10.14.5 Cookie jar artifacts for external tools (Normative)

If a Media Downloader job uses proc.exec tooling that requires cookies (e.g., yt-dlp):
- The host MUST provide a cookie jar artifact derived from the selected Stage Session or a user-provided cookie artifact.
- Cookie jar artifacts MUST be in Netscape cookies.txt format.
- Cookie jar artifacts MUST be classified "high" and exportable=false by default.
- Cookie jar artifacts MUST NOT be written to OutputRootDir/Handshake_Output.

### 10.14.6 Output routing (Normative)

Canonical storage:
- Downloaded media MUST be stored as workspace artifacts (Bronze/Raw) with stable content hashing (SHA-256) and provenance sidecars.

User-visible materialization (REQUIRED):
- Media Downloader MUST materialize user-visible copies (or hardlinks where supported) under OutputRootDir:
  - <OutputRootDir>/media_downloader/youtube/
  - <OutputRootDir>/media_downloader/instagram/
  - <OutputRootDir>/media_downloader/forumcrawler/
  - <OutputRootDir>/media_downloader/videodownloader/
- Materialization MUST emit an ExportRecord with materialized_paths[] populated (2.3.10).

### 10.14.7 YouTube archive requirements (Normative)

- The system MUST normalize and deduplicate input URLs before enqueue.
- If the input is a playlist/channel-like URL, the system MUST expand it to concrete per-video targets before downloading, so the queue count is stable and visible.
- Downloads MUST be resumable and deduplicated across runs by stable identifiers and/or content hashes.
- When the source provides separate audio/video streams, the system MUST merge into a playable container.
- Captions:
  - When available, the system MUST download caption tracks and store them as WebVTT (.vtt) sidecars.
  - Language metadata MUST be recorded in the sidecar.

### 10.14.8 Generic video downloader requirements (Normative)

- For direct media URLs, the system MUST stream-download to a temporary ".part" file and only finalize after validation.
- The system MUST reject non-media payloads (HTML/JSON/XML/script/error pages) using content-type and sniffing heuristics.
- Accepted video files MUST be validated using ffprobe (or equivalent) before marking success.
- For embed pages, the system MAY perform bounded candidate discovery (bounded BFS) and attempt candidate downloads; if still failing, it MAY fall back to archivist-style extractors when permitted.

### 10.14.9 Forum/blog topic image crawl requirements (Normative)

Pagination:
- Given a topic URL, the crawler MUST attempt to crawl all pages in the topic via pagination discovery.
- The crawl MUST be bounded by max_pages with defaults:
  - default max_pages = 1500
  - hard cap max_pages = 5000

Full-resolution preference (HARD):
- The crawler MUST prefer full-resolution images behind thumbnails when a full-size target is available:
  - follow anchor targets that wrap thumbnails
  - use srcset and data-* fullsize attributes where present
  - apply thumbnail-to-fullsize inference heuristics (query param stripping and common path rewrites)

Noise filtering (HARD):
- The crawler MUST skip:
  - profile/author avatars
  - emojis
  - UI chrome icons
  - thumbnails when a full-size target is available

Dedupe:
- The crawler MUST dedupe downloaded images by SHA-256.

Allowlist posture:
- Crawling MUST follow the AllowlistCrawler posture (2.3.8.1):
  - only crawl allowlisted domains or explicit URL lists
  - respect robots/ToS and avoid bulk scraping by default
  - rate-limit requests with polite defaults

Manifests:
- The crawler MUST produce a manifest artifact (CSV or JSON) listing:
  - page_url, discovered_url, chosen_url, sha256, bytes, status, reason_skipped

### 10.14.10 Progress and controls (Normative)

UI progress (HARD):
- The UI MUST display per-item progress and per-batch progress for all Media Downloader modes (videos + crawler).

Queue controls (HARD):
- The queue MUST support: pause/resume, cancel (one/all), retry failed.
- Concurrency MUST be configurable:
  - default = 4
  - allowed range = 1..16

### 10.14.11 Telemetry (Normative)

Bronze ingest:
- For each successfully stored downloaded file, emit FR-EVT-DATA-001 bronze_record_created with:
  - ingestion_source: { type: "system", process: "media_downloader" }
  - external_source.url populated where available

Progress telemetry:
- Implementations MUST emit leak-safe structured progress logs into the DuckDB Flight Recorder fr_events table with:
  - event_kind = "media_downloader.job_state" | "media_downloader.progress" | "media_downloader.item_result"
  - payload includes: job_id, source_kind, url, bytes_downloaded, bytes_total (when known), item_index, item_total, status, error_code (if any)

Capability checks:
- All capability allow/deny decisions MUST be recorded per 11.1 (no secrets in payloads).

### 10.14.12 Policy and limits (Normative)

- The system MUST NOT bypass access controls. Private/members-only content requires an authorized session.
- The system MUST NOT circumvent DRM or paid content restrictions.
- The UI MUST surface a rights warning and require explicit user confirmation for archival actions on third-party platforms.
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 11.1 Capabilities & Consent Model (HSK-4001 UnknownCapability + required axes)
- CONTEXT_START_LINE: 61312
- CONTEXT_END_LINE: 61324
- CONTEXT_TOKEN: HSK-4001: UnknownCapability
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 11.1 Capabilities & Consent Model

  - **Capability Registry & SSoT Enforcement ([HSK-4001]):**
    - The system MUST maintain a centralized `CapabilityRegistry` (SSoT) containing all valid Capability IDs (e.g. `fs.read`, `doc.summarize`, `terminal.exec`).
    - **Hard Invariant:** Any request for a Capability ID not defined in the Registry MUST be rejected with error `HSK-4001: UnknownCapability`. Ad-hoc or "magic string" capabilities are strictly forbidden.
    - **Audit Requirement:** Every capability check (Allow or Deny) MUST be recorded as a Flight Recorder event, capturing: `capability_id`, `actor_id`, `job_id` (if applicable), and `decision_outcome`.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 2.3.8.1 Cache-to-Index Assimilation (LocalWebCacheIndex) / AllowlistCrawler (Tier B)
- CONTEXT_START_LINE: 2602
- CONTEXT_END_LINE: 2615
- CONTEXT_TOKEN: AllowlistCrawler (Tier B, optional)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.8.1 Cache-to-Index Assimilation (LocalWebCacheIndex)

  **LocalWebCacheIndex (Tier A cache)**
  - Store external pages fetched by external providers (and optionally AllowlistCrawler).
  - Normalize: strip boilerplate, preserve headings/anchors.
  - Index: same hybrid approach as LocalDocsIndex (keyword + embeddings).
  - TTL + pinning: default TTL; allow pinning \u00e2\u20ac\u0153gold sources\u00e2\u20ac\u009d to prevent eviction.
  **AllowlistCrawler (Tier B, optional)**
  - Only crawl explicit domains or URL lists.
  - Respect robots/ToS and avoid bulk scraping by default.
  - Output to LocalWebCacheIndex and/or LocalDocsIndex.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 10.13 Stage 5.1 Session requirements (cookies/storage isolation; persistent sessions)
- CONTEXT_START_LINE: 59873
- CONTEXT_END_LINE: 59897
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

  **MUST:** no accidental state bleed between sessions.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 2.3.10 Export & Artifact Production (Materialize; ExportRecord)
- CONTEXT_START_LINE: 2684
- CONTEXT_END_LINE: 2715
- CONTEXT_TOKEN: - **Materialize**: writing an exported artifact to a user-chosen path
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 2.3.10 Export & Artifact Production (Unified Contract)

  - **Materialize**: writing an exported artifact to a user-chosen path (LocalFile) or handing it to a connector.
  - **Exporter**: a mechanical job that converts DisplayContent \u00e2\u2020\u2019 artifact(s) (no Raw/Derived mutation).

  #### 2.3.10.1 Canonical export pipeline (normative)

  5. **(Optional) Materialize** to a path (LocalFile) or pass the artifact to a connector.

  - Exporters MUST be invoked via the Orchestrator/Workflow engine (no ad-hoc \u00e2\u20ac\u0153save as\u00e2\u20ac\u009d bypass).
  - Exporters MUST be offline-pure at runtime (no network fetches; all inputs must already exist as workspace entities/artifacts).
  - Any export referencing `exportable=false` artifacts MUST be blocked by CloudLeakageGuard (\u00c2\u00a72.6.6.7.11) unless the user explicitly reclassifies and re-runs.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 11.8 Engine: Archivist (Preservation) operations + capabilities
- CONTEXT_START_LINE: 65872
- CONTEXT_END_LINE: 65917
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

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 11.8 Engine: Director (Video / Animation) -> director.transcode
- CONTEXT_START_LINE: 65220
- CONTEXT_END_LINE: 65278
- CONTEXT_TOKEN: ###### Operation: `director.transcode`
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### Engine: Director (Video / Animation)

  ###### Operation: `director.transcode`

  **Params**
    - `video_ref`: ArtifactHandle (artifact.video)
    - `preset`: proxy|web|archive
    - `format`: mp4|mov

  **Outputs**
  - artifact.video
  ```

#### ANCHOR 7
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 6.1.2.2.6 Subtitle / Caption Formats (WebVTT)
- CONTEXT_START_LINE: 22495
- CONTEXT_END_LINE: 22499
- CONTEXT_TOKEN: Subtitle / Caption Formats
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 6.1.2.2.6 Subtitle / Caption Formats

  | Format | Extension | Support Level | Notes |
  |--------|-----------|--------------|-------|
  | WebVTT | `.vtt`    | \u00e2\u0153\u2026            | Explicitly supported; treated as text with timing metadata. |
  ```

#### ANCHOR 8
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 2.3.8 (batch operations may run in background with progress indicators)
- CONTEXT_START_LINE: 2567
- CONTEXT_END_LINE: 2567
- CONTEXT_TOKEN: progress indicators
- EXCERPT_ASCII_ESCAPED:
  ```text
  - Large batch operations (e.g. imports, full re-index) may run in the background with progress indicators; concrete latency targets are defined and validated using the benchmark harness described in the base research.
  ```
