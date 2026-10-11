---
file_id: BUILDER-BRIEF-HSK-STUDIO-HARNESS
file_kind: builder_brief
updated_at: 2026-10-11T03:40:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-harness
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-HARNESS` / key `harness` / crate `hsk-studio-harness`; foundation MT `MT-35555` (read `implementation_steps` 1-3, `proof.checks`; depends_on MT-35554 session and MT-35553 controls; verified titles). Write scope `crates/hsk-studio-harness/**` only.
- Responsibility (verbatim): "Standalone asset desktop/headless development consumers with explicit selected providers; no unconditional all-domain facade, no separately shipped suite app."
- Foundation capability: "Real headless consumer of all selected foundation interfaces plus controlled reusable UI asset route; no mandatory all-engine facade."
- Allowed deps (STUDIO_MODULES depends_on == root Cargo.toml `allowed`, verified identical): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-session`, `hsk-studio-controls`, `hsk-studio-observe`. MT build closure additionally lists chronicle and package (reached transitively through `session`; they are NOT direct edges - if the harness must call `package::write_package` / `chronicle::prepare` directly, ask the lead whether to widen `allowed` instead of routing through session; UNVERIFIED decision).
- Hard prerequisites: `session` (new) and `controls` (new, no brief by this reader; owned by another lane) => wave 2, LAST. Capabilities 1-3 need only session (+package via session); capability 4 needs controls.
- Forbidden: handshake_core/native/SurrealDB; render-cpu/render-gpu/raster/vector/type engines as unconditional deps (no all-engine facade - renderer is an injected `RenderPort`); egui/eframe/wgpu directly (Controls owns the egui mechanic); opening windows, foreground focus, OS input injection (STU-QUIET-001/002); a `[[bin]]` suite app (examples only: "no separately shipped suite app").
- Conventions (verified): edition 2024, rust-version 1.97.1, single-line inline `Cargo.toml` tables (`builders/WORKSPACE_RULES.md`), `[lib] path="src/lib.rs"`, `[[example]] name="harness-headless"`, `DESCRIPTOR`, `tests/foundation_contract.rs` + `bounded_case`.
- Prep refs: `docs_repo/studio-collaboration-2026-10-09/modular-authoring/shared-team/preparation.json` -> `shared_contract_proposals` (`descriptors`: ONE owned command/control schema feeds model, AccessKit, UserManual and Argus descriptors; `visual_receipt`: bind command/actor, revision, renderer/version, region/frame, profile, original/proxy, fresh grant - stale capture or skipped provider is NOT_PROVEN; `closure`: no default all-domain facade; `proof_roles`); `repass-2026-10-09/argus-diagnostics-root-drill.json` (Argus/visual observation constraints - grep before use); `filmcraft-root-drill.json` FILM-ROOT-06/11 (render returns image-not-error => strict delivery needs typed loss; UI requests can raise occluded windows => quiet observation).

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-QUIET-001..005 (l.22364-22381): agent-driven work is headless by law (no foreground, focus steal, OS input); every automation surface reachable through the command API and Argus/AccessKit; headless visual proof via a render-to-buffer harness, with AccessKit/state-assertion fallback where headless-GPU readback is unreliable (documented crash class `0xc0000005`). STU-ARC-014 (l.223): Controls = one reusable panel/control mechanic; same descriptors feed structured tools, AccessKit, UserManual and Argus; agent-local view/selection must not change operator focus. STU-ARC-015 (l.225): visual receipts bind Argus observation to exact revision, command/actor, frame/region, renderer/version, colour profile, original/proxy resolution and current grant; optional early-return test paths are NOT_PROVEN; every render/import error and unavailable provider returns typed loss/failure.
- STU-ARC-016 (l.227): all 35 foundations precede features; hollow registries, canned responses, no-op sinks, empty tests do NOT satisfy a foundation. STU-UNI-*, STU-MDL-* (model/backend visibility: every capability reachable through a typed command), STU-MAN-* (dual-audience manual) shape the descriptor route.
- Later: STU-VAL (validation catalog), full Argus capture, native/real-GPU visual proof (explicitly separate), GUI desktop consumer, MT corpus harness feature MTs (UNVERIFIED location).

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust; ~700-900 LOC; library + one example; real crates only, no canned registry)
Reuse (verified): `hsk_studio_folio::{inspect_bytes, Inspection::{Editable,ReadOnly}, Snapshot, Budget, Unresolved, DESCRIPTOR}`; `hsk_studio_accord::{ActorContext, CancellationToken, DomainId, DESCRIPTOR}`; `hsk_studio_observe::{Observe, Observation, Outcome, FailureCode, SinkPort, DESCRIPTOR}`; `hsk_studio_chronicle::DESCRIPTOR`; from the new crates: `session::{Session, SubmitRequest, ProviderRegistry, ChronicleProvider, ...}` and `package::{write_package, read_package, Limits}` (use only what those crates actually export - read them first), `controls` descriptors once present.
1. Explicit selection, no facade: `pub struct Selection{providers:Vec<ProviderSel>, descriptors:Vec<DescriptorSource>}`; `Harness::new(selection)` fails with `UnknownProvider{id}` for anything not registered (no default "everything" build). `pub struct DescriptorSource{owner:&'static str, json:&'static str}` filled from each selected crate's REAL `DESCRIPTOR` const; `pub fn collect_descriptors(&self) -> Result<Vec<DescriptorEntry{owner, version, sha256?}>, HarnessError>` parses each as JSON (`owner`/`version` required), rejects a mismatched `owner`/unsupported `version` (`DescriptorMismatch`) - never a hard-coded success list.
2. Headless document workflow runner: `Harness::run(&self, spec:&WorkflowSpec{document_bytes:Vec<u8>, actor:ActorContext, provider:String, params:Vec<u8>, expected_revision:u64}, cancel:&CancellationToken, obs:&mut Observe, sink:&mut impl SinkPort) -> HarnessReport` executing typed steps in order: `Load` (package read or `inspect_bytes`, `ReadOnly` => stop with `ReadOnlyDocument`) -> `ProposeEdit` (session submit with the selected provider) -> `Validate` (`Snapshot::require_available` / folio budget validation of the result) -> `Serialize` (package write) -> `Reopen` (package read of the serialized bytes) -> `Compare` (reopened revision == committed revision, `encoded_bytes()` equality). `HarnessReport{steps:Vec<StepRecord{step, outcome:StepOutcome, revision:Option<u64>}>, descriptors:Vec<DescriptorEntry>, verdict}` where `StepOutcome::{Ok{receipt_id}|Failed{code}|NotProven{reason}|Skipped{because}|Canceled}` and `verdict` is `Pass` ONLY if every required step is `Ok` (any `NotProven` => `NotProven`, never silently `Pass`). Deterministic: two runs on the same inputs produce byte-identical report JSON.
3. Render/visual honesty: `pub trait RenderPort{ fn render(&self, snap:&Snapshot, region:Region, cancel:&CancellationToken) -> Result<RenderCapture, RenderError> }` injected; `capture` step is required only when `spec.capture` is set; with no port => `NotProven{reason:"renderer_unavailable"}`; a capture whose `revision` != current committed revision => `NotProven{reason:"stale_capture"}`; captured output carries `{revision, region, renderer, version, profile}` per STU-ARC-015. No GPU/pixel dependency in this crate.
4. Reusable UI asset route (needs `controls`): `pub fn ui_route(&self) -> Vec<ControlDescriptor>` returns the selected Controls descriptors headlessly (no window, no egui types in the public API), keyed by stable element ids; plus `fn command_catalog(&self)` listing the headless document operations (`load`, `propose_edit`, `validate`, `serialize`, `reopen`) with the same command names used by the report so a model can drive the harness without screen reading. Until `controls` lands, gate this behind the descriptor-source trait and mark the capability `NOT_PROVEN` in `DESCRIPTOR`.
5. `DESCRIPTOR` + example `harness-headless` (`--descriptor`, `--run DOC.json --rename NAME [--expect-rev N] [--cancel] [--capture]`): prints the `HarnessReport` JSON; exit code non-zero unless verdict `Pass`; opens no window, spawns no process.
Fixture: build a valid folio `StudioDocument` JSON from `crates/hsk-studio-folio/examples` / `tests/foundation_contract.rs`, and a chronicle rename patch from `crates/hsk-studio-chronicle/examples/chronicle-consumer.rs`.

</topic>

<topic id="deps" status="active" summary="third-party deps">

## Deps
- `serde_json 1.0.151` (`std`, pure-rust, workspace-pinned for folio/chronicle) for descriptor parsing and report JSON (request via dep-request; probe `approved` extension). `serde 1.0.229` only if report structs derive. No other crates; no `egui`/`eframe` (probe-banned names; Controls owns egui through the host-inspected 0.33 family per shared-team `provider_selection`).
- Build/test (never on C:): `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-harness --tests` (then clippy, focused test; example run via `cargo run -p hsk-studio-harness --example harness-headless -- --descriptor`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests">

## Tests (<=5, `bounded_case`, no windows, no network)
1. `headless_workflow_end_to_end`: valid fixture -> all six steps `Ok`; reopened snapshot has the new name and revision N+1; `verdict == Pass`; descriptors list contains accord, folio, chronicle, observe, session, package entries parsed from their real consts.
2. `unknown_provider_and_missing_renderer`: unknown provider => `UnknownProvider` before any step; `capture` requested without `RenderPort` => `NotProven{renderer_unavailable}` and verdict `NotProven` (not `Pass`).
3. `stale_and_canceled`: `expected_revision` != current => `Failed{stale_revision}`; pre-canceled / mid-run cancel => `Canceled` with later steps `Skipped` and NO serialized output produced.
4. `descriptor_mismatch_and_stale_capture`: a descriptor source whose `owner`/`version` disagree => `DescriptorMismatch`; a `RenderPort` returning a capture for an older revision => `NotProven{stale_capture}`; `ReadOnly` inspection input => `ReadOnlyDocument`.
5. `deterministic_and_quiet`: two runs give byte-identical report JSON; the run performs no process spawn and no env/file access outside the supplied bytes (assert by running with only in-memory inputs and no `Command` use in the crate - grep check in test).

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No real rendering/GPU/pixel proof (native visual proof is separately scheduled, MT-35555 says so), no GUI/desktop window, no Argus implementation (descriptors only), no multi-document/multi-actor swarm harness, no persistence beyond the in-memory package bytes, no all-domain facade or default feature set that pulls engines.
- Wave: 2, last (after session and controls). Report READY_FOR_VALIDATION only; never self-PASS; unavailable pieces stay `NOT_PROVEN`.

</topic>
