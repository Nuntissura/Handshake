# Handshake Export Bundles: Surgical Insert Plan (Master Spec + Roadmap)
Version: v0.1  
Status: **Insert-only patch plan** (no rewrites)

## Intent
Define a minimal export framework that:
- ships **Debug Bundle export** exactly as already specified (no changes)
- adds a **Workspace Bundle export** (new) for backup/transfer/fixtures
- keeps **policy + exportability** enforcement (CloudLeakageGuard) and **Display/Export-only filtering**
- integrates into the **roadmap using existing fixed fields only**

## Merge rules (non-negotiable)
1) **Append-only**: do not rewrite existing Master Spec or Roadmap entries.  
2) **No renumbering outside the inserted block** (if a numbering collision occurs, only renumber the *new* subsection).  
3) **If conflict is detected** (contradiction with existing spec text, or roadmap field mismatch): **HALT FOR REVIEW** and do not insert.  
4) **Do not remove technical detail** from existing sections—only cross-reference them.

---

# Two-step integration (must be done in this order)

## Step 1 — Master Spec insert (surgical additions only)
### Target location
Insert the new content **immediately after the existing section** that defines **Debug Bundle export** (e.g., the existing “Debug Bundle export” subsection).

- **Do not modify** the existing Debug Bundle export section.
- Add a new sibling subsection for Workspace Bundle export.

### Master Spec patch text (INSERT)
> **Add the following subsection directly after the existing Debug Bundle Export subsection.**  
> **If the existing section numbering differs, keep the same parent heading and adjust only this new subsection number.**

#### Workspace Bundle Export (v0)
**Purpose**  
Provide a deterministic “backup/transfer/fixture” export path that:
- preserves original imported bytes when present
- exports Handshake canonical workspace state (docs/canvases/tables)
- exports Display-derived renders for portability and review
- remains policy-gated and exportability-safe

**Non-negotiable invariants**
- Export artifacts are derived from **DisplayContent**; filtering/redaction applies at **Display/Export only** and must not mutate Raw/Derived stores.
- **CloudLeakageGuard** must deny inclusion of artifacts marked `exportable=false` unless an explicit policy override exists.
- Export runs as a **mechanical workflow job** (capability-gated, logged, hashed).

**Terms**
- **BundleKind**: `debug_bundle` (existing), `workspace_bundle` (new)
- **ExportProfile**:
  - `SAFE_DEFAULT`: redacts secrets/PII, uses hashes/minimal previews
  - `WORKSPACE`: includes more local context but still redacts secrets/PII
  - `FULL_LOCAL`: full payloads; must not be exportable unless policy explicitly allows

**Bundle format**
- Default output is a **zip** (or folder) containing:
  - `bundle_manifest.json` (required; schema versioned)
  - `workspace/`
    - `docs/<doc_id>.json`
    - `canvases/<canvas_id>.json`
    - `tables/<table_id>.json`
    - `tables/<table_id>.csv`
  - `assets/raw/` (byte-identical originals, if imported assets exist)
  - `assets/rendered/` (Display-derived render outputs: PDF/PNG/SVG/CSV as applicable)
  - `export_report.json` (included/excluded counts + reasons)

**Manifest requirements (`bundle_manifest.json`)**
Must include:
- `bundle_kind`, `schema_version`, `created_at`
- workspace identifier (wsid) and exported entity IDs
- `job_id`, `workflow_run_id`
- `export_profile_id`
- tool/renderer versions used
- input hashes (raw/canonical) and output hash list

**Policy + capabilities**
- Capability gating (minimum):
  - `export.bundle` (initiate export)
  - `fs.write` (destination-scoped)
  - optional `export.include_nonexportable` (explicit and default-deny)
- Policy context for export must be captured and visible (same treatment as other operational actions).
- When `exportable=false` artifacts are encountered:
  - default action: **exclude** and record reason in `export_report.json`
  - surface denial in Problems + Flight Recorder logs

**Determinism**
- Same inputs + same ExportProfile + same renderer/template versions should produce stable hashes for deterministic outputs (platform constraints noted in manifest if needed).

**Observability**
- Emit a distinct Flight Recorder event for Workspace Bundle export (parallel to the debug bundle export event).
- Store/export logs must include: selected IDs, profile, outputs, hashes, denials.

**Explicitly out of scope (v0)**
- Round-trip writers for proprietary formats (DOCX/PPTX/XLSX)
- cloud upload/sharing
- any export path that mutates Raw/Derived stores

---

## Step 2 — Roadmap append (use fixed fields only; append-only)
### Target location
Append entries into **existing fixed fields per phase**, without changing prior text.  
If the roadmap phase template differs from the expected fixed fields, **HALT FOR REVIEW**.

### Roadmap patch text (APPEND)

## Phase 1 (MVP)
**Goal**  
- Add: Minimum viable export to prevent lock-in and enable reproducible debugging (bundles).

**MUST deliver**  
- Add: Bundle Export Framework v0  
  - Debug Bundle export: implement end-to-end exactly as specified in the Master Spec’s Debug Bundle section (no edits).  
  - Workspace Bundle export v0: backup/transfer/fixture export for docs/canvases/tables + raw assets (when present).

**Key risks addressed in Phase 1**  
- Add:
  - No redaction-safe evidence packet for LLM coders/validators (Debug Bundle).  
  - Data lock-in / inability to back up workspace state early (Workspace Bundle).  
  - Accidental leakage through export (exportable=false enforcement + policy gating).

**Mechanical Track**  
- Add job profiles (capability-gated; logged; hashed):
  - `debug_bundle_export_v0`
  - `workspace_bundle_export_v0`

**Atelier Track**  
- Add note:
  - Workspace/Debug Bundles may include Atelier artifacts **only if policy allows**; filtering remains Display/Export-only.

**Distillation Track**  
- Add note:
  - Distillation log artifacts must respect `exportable` flags so bundles cannot leak local-only payloads.

**Vertical slice**  
- Append (do not replace existing slice):
  - Export a Workspace Bundle for a non-trivial workspace and verify: manifest + doc/canvas/table snapshots + export report.  
  - Export a Debug Bundle for one AI job and verify required files + SAFE_DEFAULT redaction mode.

**Acceptance criteria**  
- Add:
  - Debug Bundle meets required structure and emits its export event (per existing Master Spec).  
  - Workspace Bundle contains required tree and manifest; produces stable hashes when rerun with identical inputs/profile.  
  - Policy context is captured/visible for export actions.  
  - Attempt to include `exportable=false` artifacts without explicit policy is denied, logged, and surfaced.

**Explicitly OUT of scope**  
- Add:
  - Format round-tripping (DOCX/PPTX/XLSX writers)  
  - cloud bundle sharing/upload  
  - export workflows that mutate Raw/Derived stores

---

## Phase 2 (Ingestion + Shadow Workspace)
**Goal**  
- Add: Bundle export covers imported files + ingestion outputs in a portable, policy-safe way.

**MUST deliver**  
- Add:
  - Workspace Bundle v0 expands to include imported raw assets + key derived/canonical snapshots produced by ingestion.

**Key risks addressed in Phase 2**  
- Add:
  - Ingested content cannot be backed up/moved while preserving provenance/IDs.

**Mechanical Track**  
- Add:
  - `workspace_bundle_export_v0` supports inclusion of imported raw assets + selected derived sidecars (policy-gated).

**Vertical slice**  
- Append:
  - Import a PDF/DOCX, run ingestion, export Workspace Bundle; verify original bytes + canonical snapshot + Display-derived render included.

**Acceptance criteria**  
- Add:
  - export_report lists inclusions/exclusions and reasons; denials visible in Problems + Flight Recorder.  
  - exported entities preserve stable IDs referenced by jobs/workflows.

**Explicitly OUT of scope**  
- Add:
  - “Rehydrate full index from bundle” as a supported workflow (future phase).  
  - publishing bundles as shareable links.

---

# Conflict checklist (HALT FOR REVIEW triggers)
- The Master Spec already defines a different “workspace export” mechanism that contradicts the above invariants.
- The Roadmap phase template lacks any of the fixed fields used above (Goal/MUST deliver/Key risks/Acceptance criteria/Explicitly OUT of scope/Tracks/Vertical slice).
- Any existing roadmap entry promises immediate DOCX/PPTX/XLSX round-trip export in Phase 1.
- Any existing policy model contradicts `exportable=false` default-deny semantics for export.
