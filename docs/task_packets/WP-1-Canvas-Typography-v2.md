# Task Packet: WP-1-Canvas-Typography-v2

## METADATA
- TASK_ID: WP-1-Canvas-Typography-v2
- WP_ID: WP-1-Canvas-Typography-v2
- BASE_WP_ID: WP-1-Canvas-Typography (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-19T00:42:13.879Z
- REQUESTOR: ilja
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja190120260138

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Canvas-Typography-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement Canvas typography + offline Font Packs (Design Pack 40) and backend-owned Font Registry plumbing (Tauri) plus deterministic frontend font loading (FontFace) per Master Spec v02.113.
- Why: Phase 1 requires offline font packs, a safe font registry/import mechanism, narrow CSP/asset protocol scope, and deterministic typography rendering (no flash of fallback) for Canvas.
- IN_SCOPE_PATHS:
  - app/src/**
  - app/src-tauri/src/**
  - app/src-tauri/tauri.conf.json
  - app/src-tauri/resources/fonts/**
  - app/src-tauri/Cargo.toml
  - app/src-tauri/Cargo.lock
  - app/package.json
  - app/pnpm-lock.yaml
- OUT_OF_SCOPE:
  - Any changes in `src/backend/**` (owned by concurrent backend WPs).
  - Any changes in `tests/` or `scripts/`.
  - Any root-level Cargo workspace dependency changes (`Cargo.toml`, root `Cargo.lock`).

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Canvas-Typography-v2

# Coder (development):
cd app
pnpm run lint
pnpm test

cd src-tauri
cargo fmt
cargo clippy --all-targets --all-features
cargo test

# Full deterministic gates:
cd ../..
just cargo-clean
just post-work WP-1-Canvas-Typography-v2
```

### DONE_MEANS
- `just pre-work WP-1-Canvas-Typography-v2` passes.
- `just post-work WP-1-Canvas-Typography-v2` passes.
- Master Spec v02.113 font pack + registry requirements are implemented:
  - Fonts served only from `{APP_DATA}/fonts/`; Design Pack 40 is available offline and is bootstrapped from embedded resources to `{APP_DATA}/fonts/bundled/` on first run.
  - Tauri asset protocol + CSP are narrowed to the fonts directory (`asset:` and `http://asset.localhost` where required; no broad weakening).
  - Font Manager UI (or system settings) provides Import Font action for `.ttf/.otf/.woff2`.
  - Backend moves imports to `{APP_DATA}/fonts/user/`, dedupes by sha256, and updates/returns `manifest.json` (schemaVersion=1) with path constraints.
  - Frontend loads fonts via `invoke("fonts_list")` + `convertFileSrc` + `FontFace`, awaits `document.fonts.ready`, and avoids fallback flash.
  - Font family/name handling prevents CSS injection (sanitization per spec).
- Licensing artifacts exist for bundled fonts: per-font license files + `THIRD_PARTY_NOTICES` per spec.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.113.md (recorded_at: 2026-01-19T00:42:13.879Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.10.2; and 10.6.1 (Font Packs + Canvas Typography Support Spec v0.1) including Licensing/Packaging, Backend Commands+Manifest, Frontend Loading, Tauri Config, Acceptance Criteria
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packets:
  - `docs/task_packets/WP-1-Canvas-Typography.md` (prior packet; failed validation due to protocol/packet completeness gaps).
  - `docs/task_packets/stubs/WP-1-Canvas-Typography-v2.md` (remediation stub; activation pointer).
- Preserved requirements:
  - Offline Design Pack 40, backend-owned Font Registry/import, deterministic loading (FontFace / document.fonts.ready), CSP/asset protocol narrow scope, and licensing artifacts.
- Changes in v2:
  - Re-anchored to `Handshake_Master_Spec_v02.113.md` main-body sections (11.10.2 and 10.6.1) with a signed Technical Refinement Block.
  - Packet fields completed (scope paths, test plan, done means, bootstrap) to satisfy pre-work gate.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - docs/task_packets/WP-1-Canvas-Typography-v2.md
  - docs/refinements/WP-1-Canvas-Typography-v2.md
  - Handshake_Master_Spec_v02.113.md
  - app/src-tauri/tauri.conf.json
  - app/src-tauri/src/lib.rs
  - app/src-tauri/src/main.rs
  - app/src/components/CanvasView.tsx
- SEARCH_TERMS:
  - "fonts_bootstrap_pack"
  - "fonts_rebuild_manifest"
  - "fonts_list"
  - "fonts_import"
  - "fonts_remove"
  - "convertFileSrc"
  - "FontFace"
  - "document.fonts.ready"
  - "assetProtocol"
  - "font-src"
  - "THIRD_PARTY_NOTICES"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Canvas-Typography-v2
  cd app; pnpm test
  cd src-tauri; cargo test
  ```
- RISK_MAP:
  - "CSP widening" -> "WebView attack surface increase; violates spec security posture"
  - "Path traversal" -> "Font import escape from {APP_DATA}/fonts/**"
  - "CSS injection" -> "Malicious font family names can inject style rules"
  - "Licensing gaps" -> "Bundled fonts shipped without required license artifacts"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
