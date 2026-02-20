# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-YouTube-Archive-Downloader-v1

## STUB_METADATA
- WP_ID: WP-1-YouTube-Archive-Downloader-v1
- BASE_WP_ID: WP-1-YouTube-Archive-Downloader
- CREATED_AT: 2026-02-20T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: N/A (Operator request; not currently in §7.6.3 Roadmap)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.133.md 11.1 Capabilities & Consent Model (capability registry; `net.http`, `proc.exec`, `secrets.use`) (Normative)
  - Handshake_Master_Spec_v02.133.md 10.3.3.2 Connectors and capability contracts
  - Handshake_Master_Spec_v02.133.md 2.3.5 Data Architecture: File-Tree Model (Sidecar Files)
  - Handshake_Master_Spec_v02.133.md 6.1.2.2.6 Subtitle / Caption Formats (WebVTT)
  - Handshake_Master_Spec_v02.133.md Engine: Director (Video / Animation) -> `director.transcode` (deterministic transcodes)

## INTENT (DRAFT)
- What: Batch-archive personal media from the web (initially: YouTube videos + forum topic images) into a local-first, resumable workspace ingest job, with explicit network/exec capability gating and evidence-grade logging.
- Why: Preserve personal/family libraries quickly and safely, without depending on continued platform availability.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Worksurface / Shipping (HARD):
    - A single "Archive Downloader" surface in Handshake (one queue + one progress/evidence view) that supports BOTH:
      - YouTube batch video archive
      - Forum topic image archive
    - These source modes ship together in the same Phase 1 delivery for this WP (no split WPs for "crawler vs downloader").
    - Progress UI (HARD):
      - Single-item progress bar (per video/per image) showing: queued/running/done/skipped/failed + bytes/progress where available.
      - Batch progress bar for the entire run showing: total items, completed, skipped, failed, and an ETA when derivable.
  - Input modes:
    - Public mode (no account): archive publicly accessible videos given a channel/playlist/video list.
    - Account mode (optional): OAuth-based account selection for authorized access (no password collection; system browser login only).
    - Forum topic mode (no account if public; account/cookie if required):
      - Given a forum topic URL, download all pages in the topic (pagination) and archive all full-resolution images embedded/linked in posts.
      - Skip profile/author avatars and UI chrome images (site-dependent heuristics + user-configurable exclude patterns).
      - Prefer full-resolution originals behind thumbnails (follow link targets / srcset / data-fullsize patterns; never archive only the low-res thumbnail when a full-res is available).
      - Login through Handshake:
        - Handshake provides a dedicated "Forum Login" flow that uses a system webview/browser session to obtain an authenticated cookie jar/session, stored in the encrypted secrets store.
        - Handshake MUST NOT ask users to type forum passwords into a Handshake-owned form; authentication happens on the forum's own login page.
        - Support multiple forum accounts/profiles and a "no account" mode; selecting an account binds the cookie jar to subsequent fetches.
  - Batch download:
    - Resumable queue, rate limiting, and a "download archive" to avoid re-downloading.
    - Deterministic output naming including `video_id`; stable metadata sidecar JSON.
  - Captions:
    - Download available caption tracks; store as `.vtt` sidecars (preserve original + normalized WebVTT).
  - Merge/transcode:
    - Merge best audio+video streams when needed.
    - Optional normalization via Director `director.transcode` presets (proxy/web/archive).
  - Workspace integration (minimal):
    - Materialize files into workspace Bronze/Raw asset storage + provenance metadata.
    - Emit Flight Recorder events for: request queued, download started/finished/failed, captions saved, transcode finished.
    - Forum topic image archive emits equivalent FR events (topic queued/page fetched/image saved/skipped/failed).
  - Safety:
    - Capabilities required: `net.http:*` + `fs.write:*` (+ `proc.exec:*` if invoking native helpers) + `secrets.use` (account tokens).
    - No cookie jar / credential exfiltration; tokens stored in encrypted secrets store only.
    - Respect site rate limits (polite defaults); require explicit allowlist configuration for non-YouTube domains.
- OUT_OF_SCOPE:
  - Bypassing access controls (private/members-only) without explicit authorized account access.
  - Circumventing DRM or paid content restrictions.
  - Any cloud escalation / sending content externally by default.

## ACCEPTANCE_CRITERIA (DRAFT)
- Given a channel/playlist, the downloader archives all videos it can access, resumes cleanly, and avoids duplicates across runs.
- Given a forum topic URL, the downloader crawls all topic pages, archives full-resolution images (not thumbnails), skips avatars/profile pictures, and resumes cleanly without duplicates.
- Captions (when available) are downloaded and stored as `.vtt` sidecars.
- Optional transcode produces deterministic outputs using Director presets and preserves originals.
- All network/exec actions are capability-gated and produce Flight Recorder events with no secret leakage.
- The UI/worksurface is unified: the same Archive Downloader surface is used for both YouTube and forum modes, and both ship together in this WP.
- The UI shows progress clearly for both single downloads and batches (per-item + overall status bars) for both videos and forum images.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Capability registry includes required IDs (`net.http`, `proc.exec`, `secrets.use`) and the product enforces UnknownCapability rejection.
- Connector/account infrastructure for OAuth token storage (if account mode is implemented).
- Asset ingest + hashing + sidecar conventions; workspace Bronze/Raw storage ready for large media files.

## RISKS / UNKNOWNs (DRAFT)
- Platform policy/TOS changes; fragile parsing if no supported API path exists for a given flow.
- Large storage + throughput; partial downloads; corrupted files; need integrity checks + retries.
- Caption availability differs per video; multiple languages/tracks; auto-captions may be low quality.
- Legal/rights: must ensure the user has rights/permission to archive the requested content.
- Forum variance: different forum engines/themes structure thumbnails/full-res links differently; likely need a small “site adapter” layer and/or per-site selector config.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap), or explicitly treat as an Operator-scoped product addition.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-YouTube-Archive-Downloader-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-YouTube-Archive-Downloader-v1` (in `.GOV/task_packets/`).
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
