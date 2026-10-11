---
file_id: BUILDER-BRIEF-HSK-STUDIO-LAYOUT
file_kind: builder_brief
updated_at: 2026-10-11T05:00:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-layout
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-LAYOUT` / key `layout` / crate `hsk-studio-layout`; foundation MT-35537 (test target `threaded_reflow`, cases `two_frame_flow_overset`, `cyclic_missing_font_budget_cancel_stale`); write scope `handshake-studio-assets/crates/hsk-studio-layout/**`. Files per MT: `src/paragraph.rs`, `src/threaded_reflow.rs`, `tests/threaded_reflow.rs`.
- Responsibility (verbatim): "Publication/page/threaded-frame composition, overset/preflight, tables and multilingual paragraph algorithms."
- Foundation capability: "Compose real frames with line layout/overset diagnostics using shared type/nib contracts."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-type`, `hsk-studio-nib`, `hsk-studio-observe`. Slice uses accord, type (types only), observe; folio/nib adapters are later (see non-goals).
- Forbidden: handshake_core/native/SurrealDB; a second shaper (consume Type-shaped runs, STU-ARC-007 one owner per capability); a component/responsive solver (that is `components`); egui/wgpu. Layout is a dependency of `components` and `interop-layout` - keep its public types plain and stable.
- Conventions: edition 2024, rust-version 1.97.1, path deps `{ path="../hsk-studio-x", version="=0.1.0", default-features=false }`, `pub const DESCRIPTOR`, example `layout-consumer` (`--descriptor`), 30 s `bounded_case`. Root `Cargo.toml` steward-owned; dep needs via `builders/dep-requests.jsonl`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`, 14.6 l.5237+)
- STU-LAY-017 (l.5727) flowed copy = `StudioTextStory` through threaded text frames; STU-LAY-018 + STU-LAY-122 (l.5807+): ONE story flows through an ordered chain of frames (five thread fields); overset = text that does not fit the last frame, reported not truncated; STU-LAY-118..121 frame prefs/auto-size/first-baseline/vertical justification; STU-LAY-024 + STU-LAY-126 (l.5957): text wrap per object (contour/bounding-box/jump) - obstacle subtraction at each baseline.
- STU-DOC-003 (l.261): typography in points, explicit unit on every length; STU-ARC-007 (one owner per capability), STU-ARC-011/012/013 (revision footprints, bounded work, cancel between chunks), STU-ARC-016.
- MT-35537 acceptance (verified in the MT json): "Revision-bound placement proposal in points, glyph/source ranges and overset findings; MissingFont/CyclicThread/UnsupportedComposition/BudgetExceeded/Cancelled/StaleRevision retain story"; "Latin foundation never certifies RTL/CJK/kinsoku/ruby/vertical feature scope".
- Later scope: 14.6.1-14.6.11 (pages/spreads/sections/tables/styles/long documents/prepress), 14.7 typography (composer model l.8111, H&J l.8192, hyphenation l.8245, CJK/vertical l.8409, OpenType l.8514), STU-LAY-199 derivation index. MT plan domains: `_mt_plan_06_layout.json` (MT-020 and ~50 layout MTs), `_mt_plan_0708_typography_color.json`.

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, ~700-1000 LOC)
Prep reuse: visual-team `preparation.json#/representative_proposals[2]` (MT-35537; "Resolve shared Type shaping, paragraph breaks, columns/baselines/wrap and threaded frames; return overset spans. Component responsive constraints remain Components"; provider "Designcraft compose candidate through Type/Nib contracts; no duplicate shaping or component solver owner"; oracle "InDesign-produced exact story/font/settings fixtures with line/glyph/frame-position and overset comparison. Separate pinned RTL/CJK cases; Latin proof cannot cover them"); Craft reference (pinned cache `.../gov_runtime/tools/studio-craft-source-cache/designcraft-66c7ce7e.../crates/compose`: shaping/line breaking/columns/threading/wrap/overset; licence verified MIT OR Apache-2.0 for all five Craft projects; any copied code still needs pinned-hash provenance per STU-ARC-018); `.GOV/reference/studio_app_feature_research/03-indesign-feature-map.md`, `05-studio-primitive-map.md` (typography/layout engine split), `19-studio-local-first-rust-posture.md`.
Reuse (verified by grep): `type::{SourceRange{start:u64,end:u64}, Glyph{glyph_id,cluster:SourceRange,x_advance_pt,y_advance_pt,x_offset_pt,y_offset_pt}, Run (glyphs(), source(), direction(), font_size_pt(), bidi_level()), ParagraphResult (source(), runs()), Prepared (paragraphs(), coverage(), disposition()), Direction, Error::{MissingFont,UnavailableFont,Canceled,Budget,...}}`; `accord::{CancellationToken, Length/Unit::Points, DomainId}`; `observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`. Folio has NO frame/thread type (grep: none) - frames are caller-supplied until a frame contract lands (spec gap to flag, not to invent). A real `TextEngine::shaped_runs` call needs ~1300 LOC of ports + font fixtures (`hsk-studio-type/tests/support/mod.rs`); do NOT replicate that - define a narrow input type and a thin adapter.
1. `paragraph.rs`: `pub struct LayoutGlyph{cluster:SourceRange, advance_pt:f64}`, `pub struct ShapedParagraph{source:SourceRange, glyphs:Vec<LayoutGlyph> /*visual-or-logical order = as given*/, line_height_pt:f64, ascent_pt:f64, font_missing:bool}`; `impl ShapedParagraph{ pub fn from_type(p:&type::ParagraphResult<'_>, line_height_pt:f64, ascent_pt:f64) -> Self }` flattening `Run::glyphs()` (verify whether `SourceRange` offsets are bytes or scalar indices in `engine.rs`/`validate.rs` before use - UNVERIFIED); `pub fn break_opportunities(text:&str) -> Vec<Break{byte:usize, kind:Allowed|Mandatory}>` via UAX #14 (`unicode-linebreak`), mapped to glyph indices by cluster start.
2. `threaded_reflow.rs` core: `pub struct Frame{id:String, width_pt:f64, height_pt:f64, first_baseline_pt:f64, obstacles:Vec<Obstacle /*rect: x0,y0,x1,y1 in frame coords, pt*/>}`; `pub fn reflow(story:&[ShapedParagraph], text:&str, frames:&[Frame], thread:&[usize] /*ordered frame indices*/, expected_revision:u64, current_revision:u64, limits:&Limits, cancel:&CancellationToken) -> Result<LayoutProposal, LayoutError>`; greedy first-fit per line (document it as the single-line composer; NOT the paragraph composer): available width at each baseline = frame width minus blocked intervals (`trait`-free: compute from `obstacles` intersecting the line box), accumulate glyph advances, break at the last allowed opportunity before overflow (trailing spaces hang, not counted - convention to document), force break on `Mandatory`, a single unbreakable word wider than the line is placed alone and flagged `OverfullLine`; lines fill the frame top-down while `baseline + descent <= height`; remainder flows to the next frame in `thread`.
3. Output: `LayoutProposal{base_revision:u64, frames:Vec<FramePlacement{frame_id:String, lines:Vec<Line{baseline_y_pt:f64, x_start_pt:f64, width_pt:f64, source:SourceRange, glyph_range:Range<usize>, paragraph:usize, overfull:bool}>}>, overset:Option<SourceRange> /*exact first unplaced char .. story end; story is NEVER truncated*/}`. All lengths `_pt` (STU-DOC-003). Deterministic: same input -> byte-identical proposal (derive `PartialEq`, compare in tests).
4. Refusals (retain the story, no partial proposal): `CyclicThread` (thread revisits a frame), `InvalidGeometry` (non-finite/negative sizes), `MissingFont` (any `font_missing`), `UnsupportedComposition` (RTL/vertical/CJK runs: `Direction`/script not Latin-LTR in this slice -> typed refusal, never mis-laid), `BudgetExceeded{lines,glyphs}`, `StaleRevision{expected,current}`, `Canceled` (checked per line). 
5. Observability: `DESCRIPTOR` + `report(...)` mapping `Ok`->`Success` (overset is a finding inside the proposal, not a failure), `CyclicThread|InvalidGeometry|StaleRevision`->`Failure(Validation)`, `MissingFont`->`Failure(Unavailable)`, `UnsupportedComposition`->`Failure(Unsupported)`, `Canceled`->`Canceled`. Example `layout-consumer` takes a fixed-advance toy font (`--advance 6 --text "..." --frame 60x20 --frame 60x20`).
Hand-computed oracle (fixed advance 6 pt per glyph incl. space, frame width 60 pt = 10 glyphs, line height 12, frame height 24 = 2 lines): text "aaaa bbbb cccc dddd" -> line1 "aaaa bbbb " (hang) at baseline `first_baseline`, line2 "cccc dddd" fits 9 glyphs; with two frames of height 12 (1 line each) the second frame gets line 2 and `overset == None`; with a third word added and only two 1-line frames, `overset.start` == byte offset of the first unplaced word and glyph/source ranges tile the placed text exactly.
Oracle note (Operator; black-box only, no decompilation): later compare against InDesign-exported fixtures (InDesign installed): line starts, baselines, frame positions, overset boundary for the same story/font/settings; separate pinned cases for Single-line vs Paragraph Composer, RTL (Arabic/Hebrew with numbers), CJK kinsoku/ruby/vertical, hanging punctuation, justification, hyphenation. Latin first-fit parity does NOT cover those (MT-35537 text).

</topic>

<topic id="deps" status="active" summary="third-party deps">

## Deps
- `unicode-linebreak =0.1.5` (UAX #14; Apache-2.0; last release 2023-07-25, widely used 116M downloads; pure Rust; verified on crates.io 2026-10-11). Alternative `icu_segmenter` 2.3.0 (Unicode-3.0, ICU4X, 2026-08-13) is maintained but pulls ICU data providers - defer. Craft's designcraft uses `unicode-segmentation`/`unicode-bidi` (type already vendors bidi).
- No other crate (hand-formatted JSON in the example, or reuse serde_json only if the steward pins it for layout).
- Build/test: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-layout --tests` (then `clippy`, `test --test threaded_reflow`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/threaded_reflow.rs">

## Tests (<=5, `bounded_case`; MT case names)
1. `two_frame_flow_overset`: the fixed-advance oracle above incl. exact baselines/x/width/source/glyph ranges, flow into frame 2, then an overset variant; ranges tile the placed text with no gap/overlap.
2. `obstacle_wrap_and_overfull_word`: a rectangular obstacle narrows lines it intersects (line break moves earlier); a word wider than the frame sits alone with `overfull: true`.
3. `cyclic_missing_font_budget_cancel_stale`: thread `[0,1,0]` -> `CyclicThread`; `font_missing` -> `MissingFont`; `max_lines=1` -> `BudgetExceeded`; pre-cancelled -> `Canceled`; revision mismatch -> `StaleRevision`; each returns `Err` with no proposal and the input story value unchanged.
4. `unsupported_scripts_refused_not_mislaid`: an RTL (`Direction::RightToLeft`) or CJK-script paragraph -> `UnsupportedComposition` (guards "Latin proof never certifies RTL/CJK").
5. `determinism_and_break_opportunities`: two runs byte-equal; `break_opportunities("a-b c\nd")` yields the expected Allowed/Mandatory offsets (hyphen, space, newline).

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No shaping (Type owns it), no bidi/vertical/CJK/ruby, no hyphenation/justification/paragraph composer, no columns/span, no tables, no pages/spreads/master pages/sections, no nib-based contour wrap (add a `WrapProvider` over `nib::Network` next), no Folio frame contract (gap), no auto-resize, no preflight rules, no component constraints, no interop (IDML = `interop-layout`), no GUI/host wiring.
- Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
