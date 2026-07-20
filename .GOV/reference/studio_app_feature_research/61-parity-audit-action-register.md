---
file_id: studio-app-feature-research-parity-audit-action-register
topic_id: SFR-PAUDIT-ACTIONS
title: "Parity Audit Action Register (2026-07-20)"
status: draft
summary: "Tracks the recommended research-refactor actions from the 2026-07-20 Studio parity audit, with per-action status, owner-dependency, and evidence."
sources: 4
updated_at: "2026-07-20"
---

## [SFR-PAUDIT-ACTIONS] Parity Audit Action Register

### [SFR-PAUDIT-ACTIONS.summary] Summary

```yaml
audit: "2026-07-20 two-workflow Studio parity + workflow-scenario audit"
registers:
  - 58-parity-feature-gap-register.md
  - 59-workflow-needs-register.md
  - 60-bridge-opportunity-register.md
status_legend:
  DONE: "completed and evidenced in this session"
  IN_PROGRESS: "background work launched, not yet landed"
  OPERATOR_HOST_DEPENDENT: "requires the actual vendor app installed/scriptable on the operator host; a governance model cannot run it headlessly"
  HELD: "explicitly deferred by operator on 2026-07-20"
authority: "Reference/provenance tracker only; not a Work Packet or product authority. Governance-surface work only (.GOV/reference)."
```

### [SFR-PAUDIT-ACTIONS.actions] Actions

```yaml
actions:
  - id: "ACTION-A1"
    title: "Close the pipeline split (deep-delta rows invisible to coverage matrix)."
    status: "IN_PROGRESS"
    detail: "Extend the card/row/coverage generators (_tools/generate-source-coverage-verification-matrix.py and the 15/39-series) to ingest the 2,330 deep-delta rows (51-55) so they enter coverage/planning. Interim annotation landed: 49-source-coverage-verification-matrix.md now carries a LEAF-PIPELINE-ONLY caveat; 08 SFR-REMAINING-GAP-011 records the mechanism."
    done_this_session:
      - "49 scope caveat added (leaf-pipeline-only, missing_required_fields is not parity proof)."
      - "08 SFR-REMAINING-GAP-011 opened."
    remaining: "Generator code change to parse deep-delta YAML rows and emit deep-row coverage; requires inspecting 51-55 row schema and the existing generator. Non-trivial, deferred to a dedicated generator pass (not rushed)."
  - id: "ACTION-A2"
    title: "Create a feature-level gap register."
    status: "DONE"
    detail: "58-parity-feature-gap-register.md now holds the 58 adversarially-verified NON-AI gaps with severity/verdict/evidence — the register the corpus previously lacked (only process gaps were tracked in 08). 59 and 60 add workflow needs and bridge opportunities."
    evidence:
      - "58-parity-feature-gap-register.md (58 gaps)"
      - "59-workflow-needs-register.md (113 needs)"
      - "60-bridge-opportunity-register.md (86 features)"
      - "_tools/generate-parity-audit-registers.py"
  - id: "ACTION-A3"
    title: "Capture the command-surface / default-shortcut / menu-ID inventories (XAPP-01/02)."
    status: "REPLANNED_ONLINE_CAPTURE"
    detail: "Original plan was to run _installed_export_scripts/{photoshop,indesign,illustrator}-export-inventory.jsx per 32-adobe-installed-ui-export-playbook.md. RETIRED as the primary path: the operator has no Adobe apps or subscriptions, so the installed-app ExtendScript export route is not available (and ExtendScript foregrounds the app, violating the governed headless/quiet law). This also confirms a project invariant: all Studio research must be reproducible from PUBLIC ONLINE SOURCES, not from owning the vendor apps. The goal (default-shortcut inventory XAPP-01, menu-command-ID + scripting-DOM catalogs XAPP-02) is unchanged; the method becomes an online-source capture."
    replacement_method: "Online-source capture pass (no apps required): Photoshop/Illustrator/InDesign default-keyboard-shortcuts help pages; Illustrator executeMenuCommand ID catalog + scripting DOM members (ai-scripting.docsforadobe.dev); InDesign scripting DOM (developer.adobe.com/indesign/uxp/dom); Affinity shortcuts; Figma shortcut list (help article 360040328653). Distill into shortcut/command rows."
    status_detail: "DONE (PARTIAL). Ran online capture (7 lanes pooled 4) -> 65-command-shortcut-capture.md (82 groups). Captured category STRUCTURE + canonical source URLs + representative verbatim samples for PS/AI/ID/Affinity/Figma shortcuts, Illustrator executeMenuCommand entry points, and the InDesign scripting DOM class namespaces. RESIDUAL: the canonical Adobe helpx default-keyboard-shortcuts SPA pages time out for all non-browser clients (WebFetch/curl/Invoke-WebRequest all failed; Jina 422; archive.org blocked), so full verbatim binding tables + the complete executeMenuCommand ID catalog were NOT transcribed. They need a browser-capable fetch (or the SFR-REMAINING-GAP-003 browser-export fallback); doc 65 records the exact URLs + structure a later browser pass completes deterministically."
    evidence:
      - "65-command-shortcut-capture.md (82 groups)"
      - "_audit_20260720/inputs/a3_capture.json"
    tracked_gap: "08 SFR-REMAINING-GAP-014 (now mitigated-partial); full-table fetch blocker under SFR-REMAINING-GAP-003"
  - id: "ACTION-A4"
    title: "Distill in-repo snapshots (zero new research)."
    status: "DONE"
    detail: "Promote content already present verbatim in _source_snapshots/ into feature rows: Figma export color-profile dropdown + resampling + Ignore-overlapping-layers + Slice tool, Figma image import 4096px downscale + format list, Illustrator white-overprint, InDesign CompositeFont/Kashida DOM classes."
    evidence:
      - "62-snapshot-distillation-delta.md"
  - id: "ACTION-A5"
    title: "Targeted re-passes (highest-value gap closers)."
    status: "DONE"
    detail: "Online-source workflow (pooled 4) produced 106 promotable rows across 5 lanes, distilled to 63-parity-repass-delta.md. Figma Sites: the sole CRITICAL gap (per-element Accessibility panel + semantic HTML tags + landmarks + ARIA role/label/current/hidden + heading levels) is now rowed at promotable depth, plus website settings/SEO/CMS. Camera Raw: Point Color, HDR/gain-map, filmstrip Sync/Merge, per-mask Color Grading, Anamorphic Desqueeze, Projection Correction, 1500K, WebP. InDesign: variable fonts + axes, OpenType feature UI, OT-SVG, Document Fonts. Affinity: brush/Soft-Proof/Performance/PDF-import option depth (affinity.help 403 rows marked UNVERIFIED). PS: native AVIF/JXL + gain-map/HDR-output export."
    evidence:
      - "63-parity-repass-delta.md (106 rows)"
      - "_audit_20260720/inputs/a5_repass.json"
  - id: "ACTION-A6"
    title: "Workflow research passes (team/production needs)."
    status: "DONE"
    detail: "Online-source workflow (pooled 4) produced 70 capability rows across 5 lanes, distilled to 64-workflow-needs-research-delta.md, each with how-incumbents-do-it + Handshake-Studio-requirement mapped to CRDT/EventLedger/session/permission primitives: (1) review/approval/sign-off/version-of-record (Ziflow/Filestage/GoProof/PageProof/Frame.io) -> NEED-004/005/017/022; (2) production workflow-state/PM (Creativeforce/verybusy.io/flatplan) -> NEED-009; (3) production-volume performance NFRs -> NEED-026/047/071/095; (4) DAM/PIM + governed library releases (Bynder/CI-HUB/OpenAsset) -> NEED-045/057/101; (5) culling/rating/catalog + lossless round-trip (Capture One/Lightroom) -> NEED-051/063."
    evidence:
      - "64-workflow-needs-research-delta.md (70 capabilities)"
      - "_audit_20260720/inputs/a6_workflow.json"
  - id: "ACTION-A7"
    title: "Record pending operator decisions as artifacts."
    status: "HELD"
    detail: "HELD by operator on 2026-07-20. When resumed: CJK/RTL/ME market-language scope (gates 4 MAJOR gap severities), Illustrator/Photoshop iPad-web exclusion, Illustrator accessible-PDF posture (recommend EXCEED), Figma Community marketplace posture, and Studio's own licensing/business model (surfaced by small-studio/freelancer adoption scenarios NEED-107)."
  - id: "ACTION-A8"
    title: "Stand up a vendor-watch / re-crawl cadence."
    status: "HELD"
    detail: "HELD by operator on 2026-07-20. When resumed: monthly PS/Figma, per-patch Affinity, per-release Adobe; domain-pinned for Affinity (serif.com/affinity.help/affinity.studio); Jina/browser-export fallback per SFR-REMAINING-GAP-003. First sweep: ACR 18.x non-AI, Affinity 2.6.1-2.6.5 + v3 leaf inventory, Figma post-2026-07-09 + beta rollout states. Tracked as 08 SFR-REMAINING-GAP-015."
```

### [SFR-PAUDIT-ACTIONS.sources] Sources

```yaml
sources:
  - { id: PA-S01, path: "58-parity-feature-gap-register.md", note: "Parity feature gap register." }
  - { id: PA-S02, path: "59-workflow-needs-register.md", note: "Workflow needs register." }
  - { id: PA-S03, path: "60-bridge-opportunity-register.md", note: "Bridge opportunity register." }
  - { id: PA-S04, path: "08-gap-resolution-notes.md", note: "SFR-GAP-021 audit pass + SFR-REMAINING-GAP-011..015." }
```
