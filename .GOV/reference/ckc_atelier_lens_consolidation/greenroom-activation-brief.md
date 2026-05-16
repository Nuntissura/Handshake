---
file_id: ckc-greenroom-activation-brief
file_kind: greenroom_reference_brief
updated_at: 2026-05-16
status: reference_only_not_execution_authority
wp: WP-1-Atelier-Lens-Consolidation-v1
supersedes_premature_wp: WP-1-Atelier-Lens-CKC-Greenroom-v1
---

<topic id="greenroom-reference-brief" status="active" version="v2" wp="WP-1-Atelier-Lens-Consolidation-v1" summary="Greenroom reference brief for Atelier/Lens consolidation" updated_at="2026-05-16">

# Greenroom Reference Brief

Do not activate `WP-1-Atelier-Lens-CKC-Greenroom-v1`. That Greenroom WP framing was premature. Greenroom is today's reference and research activity feeding `WP-1-Atelier-Lens-Consolidation-v1`.

## Operator Intent

The operator wants CastKit Codex rebuilt in Handshake as Atelier/Lens language and tech stack, because CKC was a working expression of one of the original reasons for building with LLMs: prompt diaries, character/media workflows, and Atelier/Lens creative production.

The operator expects overlap between Atelier/Lens and CKC. CKC partly carried the same goals and intent, but was built outside Handshake and may be more evolved in some behaviors. Do not throw away any Atelier/Lens intent. Do not ignore CKC extras. Fold CKC into Atelier/Lens with no-loss preservation and explicit conflict handling.

CKC features created from unexpected need and convenience are valid requirement evidence. Each such feature must be mapped to folded, dependency, deferred, conflict, or operator-decision-needed.

The operator explicitly requires preservation-first consolidation:

- Do not reject or neglect existing Atelier/Lens paths.
- Preserve old WP intent, intended goals, and technical requirements.
- Change or layer only when conflicts emerge.
- Preserve CKC evolved behavior where it fills gaps, adds convenience, or shows better working product shape.
- Treat overlap as consolidation input, not as a reason to delete either source.
- Use massive WPs and 60+ microtasks if that speeds the build while keeping restartability.

## Greenroom Scope

The Greenroom activity does not implement product code and is not its own active WP. It produces source evidence for the Atelier/Lens consolidation WP.

In scope:

- CKC code greenroom.
- CKC spec/taskboard requirement extraction.
- Handshake stub preservation.
- Atelier/Lens versus CKC overlap matrix.
- CKC evolved-feature and convenience-driven requirement register.
- Conflict matrix.
- Translation matrix.
- Fixture/test inventory.
- Microtask map.
- Later CKC rebuild handoff notes after consolidation and research.

Out of scope:

- Rust implementation.
- Tauri commands.
- React UI.
- ComfyUI runtime integration.
- PoseKit calibration implementation.
- Creating CKC Kernel or Vertical Slice stubs now.

## Corrected Downstream Sequence

The Greenroom activity feeds:

1. `WP-1-Atelier-Lens-Consolidation-v1`
   - Single preservation-first consolidation stub for Atelier/Lens and CKC overlap.

2. CKC research basis
   - Current external and source-level research required before implementation planning.

3. Later CKC rebuild stubs
   - Kernel, vertical slice, and any additional packet families are created only after consolidation, greenroom review, CKC research, and conflict classification are complete.

## Required Consolidation Gates

- Confirm spec anchors from v02.185.
- Attach this reference folder as source evidence.
- Include the Greenroom microtask buckets as source material or a no-loss expansion of them.
- Include acceptance criteria that prevent dropping old stub intent.
- Include acceptance criteria that prevent ignoring CKC overlap and CKC convenience-driven extras.
- Include explicit absolute rejection: SQLite is not accepted anywhere in Handshake, including runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths. Also reject Electron IPC, localhost intake authority, and `.GOV` product outputs.
- Obtain `USER_SIGNATURE`.
- Create official consolidation packet.
- Only then move the consolidation packet from Stub Backlog to Ready for Dev.
- Do not create CKC rebuild stubs until the consolidation, greenroom review, and CKC research are complete.

## Greenroom Done Means

The Greenroom activity is done when:

- `greenroom-requirements-register.md` and JSON equivalent are accepted or superseded by official packet artifacts.
- `greenroom-translation-matrix.md` and JSON equivalent are accepted or superseded.
- `greenroom-microtask-map.md` and JSON equivalent are accepted or superseded.
- The Atelier/Lens versus CKC overlap matrix is accepted or superseded.
- The CKC evolved-feature and convenience-driven requirement register is accepted or superseded.
- Later CKC rebuild packet stubs can be generated without rereading every CKC source file and without dropping existing Atelier/Lens intent.
- Every preserved old WP goal and CKC feature is either carried into consolidation, retained as separate dependency, deferred, marked conflict, or explicitly marked operator-decision-needed.

</topic>
