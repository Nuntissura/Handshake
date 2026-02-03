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
- WP_ID: WP-1-Artifact-System-Foundations-v1
- CREATED_AT: 2026-02-02T12:16:33.206Z
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md
- SPEC_TARGET_SHA1: 4d406dcc1a75570d2f17659e0ac40d68a22f211a
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja020220261405
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Artifact-System-Foundations-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Non-blocking spec hygiene note: Handshake_Master_Spec_v02.123.md 2.3.10.9 contains a placeholder line ("... (content preserved) ..."). Materialize requirements are specified in 2.3.11, and export pipeline constraints are specified in 2.3.10.1, so this WP can proceed without spec enrichment.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Export runs MUST write an ExportRecord to Flight Recorder / workspace logs (Handshake_Master_Spec_v02.123.md 2.3.10.1-2.3.10.2).
- GC runs MUST emit a `meta.gc_summary` event to Flight Recorder containing counts of pruned vs. spared items (Handshake_Master_Spec_v02.123.md 2.3.11.1 [HSK-GC-003]).

### RED_TEAM_ADVISORY (security failure modes)
- Path traversal / symlink attacks in Materialize can cause arbitrary filesystem writes outside the intended target path.
- ExportGuard / CloudLeakageGuard bypass can leak exportable=false artifacts via local materialize or connector uploads.
- Retention/GC bugs can delete pinned artifacts (irreversible data loss) or keep sensitive artifacts past TTL (privacy leakage).
- Non-canonical hashing (bundle hashing over raw ZIP bytes, unstable ordering) can make evidence bundles non-verifiable and enable "same content, different hash" drift.
- Resource exhaustion: repeated exports/materialize of large bundles can fill disk; retention must remain deterministic and auditable.

### PRIMITIVES (traits/structs/enums)
- ArtifactManifest (2.3.10.6), ExportRecord (2.3.10.2), and BundleIndex canonical hashing (2.3.10.7).
- RetentionPolicy, ArtifactKind, and PruneReport; plus engine.janitor input schema + output contract (2.3.11.0-2.3.11.2).
- ExportTarget and Materialize semantics (2.3.10.1 and 2.3.11).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Main Body sections 2.3.10.1, 2.3.10.6-2.3.10.8, and 2.3.11.0-2.3.11.2 specify the artifact store layout + manifests, canonical hashing rules, retention/pinning/GC requirements, and atomic Materialize + guard constraints. No additional normative text is required to proceed with implementation and validation for this WP.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: 2.3.10.9 is a placeholder, but Materialize requirements are specified in 2.3.11 and export pipeline constraints are specified in 2.3.10.1.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The WP can be fully anchored to existing Main Body requirements in 2.3.10 and 2.3.11. Any cleanup of the 2.3.10.9 placeholder can be handled as separate spec hygiene, but it does not block implementing the required behavior.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 2.3.10.1 Canonical export pipeline (normative)
- CONTEXT_START_LINE: 2518
- CONTEXT_END_LINE: 2550
- CONTEXT_TOKEN: #### 2.3.10.1 Canonical export pipeline (normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 2.3.10 Export & Artifact Production (Unified Contract)
  
  **Why**  
  Exports are the main exfiltration boundary and the primary way Handshake produces deliverables (PDF/PPTX/PNG/ZIP/etc.).  
  Handshake already has ExportGuard (\\u00C2\\u00A72.3.2), ArtifactService (\\u00C2\\u00A72.3.6), and surface-specific exporters, but without one unified contract exports will drift, lose provenance, or bypass safety.
  
  **What**  
  Defines one canonical export pipeline plus the minimum schemas/requirements every exporter MUST follow.
  
  **Jargon**  
  - **Artifact**: immutable output stored inside the workspace (see `ArtifactHandle` schema in \\u00C2\\u00A72.6.6.7.7).  
  - **Export**: a policy-applied projection of content that is allowed to leave the system (ExportGuard; \\u00C2\\u00A72.3.2).  
  - **Materialize**: writing an exported artifact to a user-chosen path (LocalFile) or handing it to a connector.  
  - **Exporter**: a mechanical job that converts DisplayContent \\u00E2\\u2020\\u2019 artifact(s) (no Raw/Derived mutation).  
  - **ExportRecord**: an immutable audit record for one export run.
  
  ---
  
  #### 2.3.10.1 Canonical export pipeline (normative)
  
  1. **Select sources** by `EntityRef` (Raw/Derived content is never edited by export).  
  2. **Build a Display projection** (DisplayContent/layout decisions).  
  3. **Apply ExportGuard** for the chosen `ExportTarget` (\\u00C2\\u00A72.3.2).  
  4. **Run exporter** (mechanical job) to produce one or more `ArtifactHandle`s.  
  5. **(Optional) Materialize** to a path (LocalFile) or pass the artifact to a connector.  
  6. **Write an ExportRecord** to Flight Recorder / workspace logs.
  
  Rules:
  - Exporters MUST NOT mutate Raw/Derived entities.
  - Exporters MUST be invoked via the Orchestrator/Workflow engine (no ad-hoc \\u00E2\\u20AC\\u0153save as\\u00E2\\u20AC\\u009D bypass).
  - Exporters MUST be offline-pure at runtime (no network fetches; all inputs must already exist as workspace entities/artifacts).
  - Any export referencing `exportable=false` artifacts MUST be blocked by CloudLeakageGuard (\\u00C2\\u00A72.6.6.7.11) unless the user explicitly reclassifies and re-runs.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 2.3.10.6 Artifact manifests + on-disk layout (normative)
- CONTEXT_START_LINE: 2604
- CONTEXT_END_LINE: 2635
- CONTEXT_TOKEN: #### 2.3.10.6 Artifact manifests + on-disk layout (normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.10.6 Artifact manifests + on-disk layout (normative)
  
  Artifacts MUST be first-class workspace objects with:
  - an immutable payload (bytes or directory payload)
  - a sidecar manifest (`artifact.json`)
  - a stable `content_hash: Hash` (Hash = SHA-256; \\u00C2\\u00A72.6.6.7.5)
  
  On-disk (inside each workspace):
  - `.handshake/artifacts/L3/<artifact_id>/payload` (file) OR `payload/` (directory artifact)
  - `.handshake/artifacts/L3/<artifact_id>/artifact.json`
  - same layout for `L2/` and `L1/` (LayerGuard + promotion gates apply)
  
  `artifact.json` minimum schema:
  
  ```text
  ArtifactManifest
  - artifact_id: UUID
  - layer: (L1|L2|L3|L4)
  - kind: (file | tool_output | transcript | dataset_slice | prompt_payload | report | bundle)
  - mime: string
  - filename_hint?: string
  - created_at: Timestamp
  - created_by_job_id?: UUID
  - source_entity_refs[]?: EntityRef
  - source_artifact_refs[]?: ArtifactHandle
  - content_hash: Hash
  - size_bytes: int
  - classification: (low | medium | high)
  - exportable: bool
  - retention_ttl_days?: int
  - pinned?: bool
  ```
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 2.3.10.7 Bundles + canonical hashing (normative) and 2.3.10.8 Retention, pinning, and garbage collection (normative)
- CONTEXT_START_LINE: 2637
- CONTEXT_END_LINE: 2657
- CONTEXT_TOKEN: #### 2.3.10.7 Bundles + canonical hashing (normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  Rules:
  - For `determinism_level=bitwise`, `content_hash` MUST match the exact payload bytes.
  - For directory artifacts and for `determinism_level=structural`, exporters MUST define the canonical hash basis (e.g. normalized entry list + per-entry hashes) and log it in `ExportRecord.warnings[]` if not bitwise.
  
  ---
  
  #### 2.3.10.7 Bundles + canonical hashing (normative)
  
  When an exporter emits a bundle (e.g. Debug Bundle ZIP):
  - `determinism_level` SHOULD be `structural` unless bitwise ZIP determinism is guaranteed.
  - `content_hash` MUST be computed over a canonical `BundleIndex` (sorted paths + per-item content_hash + size_bytes), not over raw ZIP bytes unless bitwise is guaranteed.
  - Bundles MUST include an embedded `bundle_index.json` OR emit it as a sibling artifact referenced by the ExportRecord.
  
  ---
  
  #### 2.3.10.8 Retention, pinning, and garbage collection (normative)
  
  - `retention_ttl_days` MUST be set for `prompt_payload` and other high-sensitivity artifacts.
  - Expired, unpinned artifacts MUST be garbage-collected.
  - GC MUST be deterministic and auditable (emit a `gc_report` artifact + log record containing deleted artifact_ids + reason).
  - Workspaces SHOULD enforce a size quota; quota evictions MUST never delete pinned artifacts.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 2.3.11 Retention & Pruning (MEX v1.2)
- CONTEXT_START_LINE: 2683
- CONTEXT_END_LINE: 2742
- CONTEXT_TOKEN: ### 2.3.11 Retention & Pruning (MEX v1.2)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 2.3.11 Retention & Pruning (MEX v1.2)
  
  #### 2.3.11.0 Normative Data Structures
  
  ```rust
  /// [HSK-GC-001] Retention Policy Schema
  /// Defines how artifacts and logs are pruned to prevent disk bloat.
  #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
  pub struct RetentionPolicy {
      pub kind: ArtifactKind,
      pub window_days: u32,   // Default: 30 for Logs, 7 for Cache
      pub min_versions: u32,  // Default: 3; keep even if expired
  }
  
  #[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum ArtifactKind { 
      Log,      // Flight Recorder traces (.jsonl)
      Result,   // AI Job outputs / EngineResults
      Evidence, // Context snapshots (ACE-RAG)
      Cache,    // Web/Model cache
      Checkpoint // Durable workflow snapshots
  }
  
  #[derive(Debug, Serialize, Deserialize)]
  pub struct PruneReport {
      pub timestamp: DateTime<Utc>,
      pub items_scanned: u32,
      pub items_pruned: u32,
      pub items_spared_pinned: u32,
      pub items_spared_window: u32,
      pub total_bytes_freed: u64,
  }
  ```
  
  #### 2.3.11.1 Hard Invariants
  
  1.  **[HSK-GC-002] Pinning Invariant:** Any artifact or log entry marked `is_pinned: true` (in SQLite metadata or sidecar) MUST be excluded from automated GC runs.
  2.  **[HSK-GC-003] Audit Trail:** Every GC run MUST emit a `meta.gc_summary` event to the Flight Recorder containing counts of pruned vs. spared items.
  3.  **[HSK-GC-004] Atomic Materialize:** The `PruneReport` MUST be written as a versioned artifact before old logs are unlinked.
  
  #### 2.3.11.2 Mechanical Engine Contract: engine.janitor (v1.2)
  
  - **Operation:** `prune`
  - **Input Schema:** `{ policies: Vec<RetentionPolicy>, dry_run: bool }`
  - **Output:** `PruneReport` (as defined above)
  - **Side Effects:** Deletion of files from `artifacts/` and `logs/` roots that exceed `window_days` and are not pinned or required for `min_versions`.
  
  **[HSK-GC-005] Janitor Decoupling (Normative):**
  The Janitor service MUST NOT hold a direct reference to a database pool. It MUST interact with the storage layer exclusively via the `Database` trait or a dedicated `JanitorStorage` interface. This ensures that maintenance tasks remain portable across SQLite and PostgreSQL backends.
  
  
  Materialize = writing an existing artifact payload to:
  - `ExportTarget::LocalFile` (path chosen by user)
  - a connector upload stream
  
  Rules:
  - Materialize MUST be atomic (write temp + fsync + rename) and MUST prevent path traversal.
  - Materialize MUST NOT bypass ExportGuard/CloudLeakageGuard; the exporter pipeline (\\u00C2\\u00A72.3.10.1) still applies.
  - `ExportRecord.materialized_paths[]` MUST be written for LocalFile targets.
  ```
