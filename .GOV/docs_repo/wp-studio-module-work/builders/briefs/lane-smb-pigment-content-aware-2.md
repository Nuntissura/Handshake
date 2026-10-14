---
file_id: BUILDER-BRIEF-LANE-SMB-PIGMENT-CONTENT-AWARE-2
file_kind: builder_brief
updated_at: 2026-10-15T00:00:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-pigment
lane: SMB-PIGMENT-CONTENT-AWARE-2
owned_paths: [crates/hsk-studio-pigment/src/content_aware.rs, crates/hsk-studio-pigment/src/content_aware/**, crates/hsk-studio-pigment/tests/content_aware*.rs, the content-aware call sites in crates/hsk-studio-pigment/src/retouch*.rs and the crop module, the `mod`/`pub use` lines in crates/hsk-studio-pigment/src/lib.rs]
depends_on: [SMB-PIGMENT-CONTENT-AWARE READY at b7880bb]
author_role: lead brief; no product code written
---

<topic id="scope" status="active" summary="Remainder of STU-RAS-143">

## Start state
- Read lane file `builders/lanes/hsk-studio-pigment-content-aware.json` (gaps list) and `crates/hsk-studio-pigment/src/content_aware.rs` at b7880bb. Grep spec module 14 for STU-RAS-143 and the crop/scale clauses; quote them as spec_basis.

## Slices (commit + push each)
1. Wire refusals to the native fill: `retouch::content_aware_fill` and crop `content_aware_fill_on_expand` call `content_aware::fill`. Lead decision 2026-10-15: this re-opens SMB-PIGMENT-RETOUCH (CX-VAL-009); run its existing tests once and keep them green; update any test that asserted the old refusal and say so in the lane notes.
2. Colour adaptation `high` / `very_high`; rotation, scale and mirror adaptation in the search (typed refusal for any setting still not built).
3. Content-aware scale (protect-mask aware, seam/patch based, written from papers only, e.g. Avidan-Shamir 2007 / Simakov 2008 bidirectional similarity) and inpaint as a named API over the fill.
4. Photo-size proof: a 4096x4096 fill within the work/memory budget; record wall time in the receipt and lane notes (performance claim only from a run).

## Rules
- Determinism across thread counts stays bit-identical (extend the existing test).
- Hostile input (V4-02) for every new option; no silent fallback.
- Photoshop parity UNVERIFIED (D5), class Approximate.
- New lane file `builders/lanes/hsk-studio-pigment-content-aware-2.json` (workflow-v3 keys); READY_FOR_VALIDATION per slice on a pushed commit.

</topic>
