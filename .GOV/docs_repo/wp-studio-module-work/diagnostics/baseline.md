---
file_id: WP-STUDIO-MODULE-WORK-BASELINE
file_kind: workspace_baseline_diagnostics
updated_at: 2026-10-11T02:00:00Z
owner: workspace-steward
baseline_candidate: 55c1096946b4dfb054331a8ae0fa6300f11b3570 (check/clippy/probe)
test_candidate: 47f212fe4adce5e96778de9c326ab3db8117a965 (clean tree; = 55c1096 + scaffold 3f6d5ab + Type e603db8 + Type pins 47f212f)
line_refs: tools/workspace-probe.rs and Cargo.toml at 96ad053 unless marked @55c1096
---

<topic id="summary" status="active" summary="Baseline verdict">

- `cargo check --workspace --tests` @55c1096: PASS, 8/8 crates. After scaffold: PASS, 35/35.
- `cargo clippy --workspace --tests` @55c1096: PASS (warnings only): type 1017 unique, observe 2, others 0.
- `cargo test --workspace --no-fail-fast` @47f212f: unit + integration all PASS (133 passed, 0 failed); FAIL only Type doctests (3/3, vendored ttf-parser examples). Exit 101.
- Probe @55c1096: 7/8 accepted, type rejected `registry_transitive_pin_missing` -> fixed 47f212f; @96ad053 all 35 accepted in one selection (closure 35, registry records 102).
- Enforcement: none automatic. Cargo ignores `[workspace.metadata.studio]`; no build.rs, no CI (.github absent), no test reads it. Only enforcer = `tools/workspace-probe.rs`, run manually by validators as MT proof commands (e.g. MT-35530 `.validation.proof_records[2].commands[8]`).
- Lead note: STUDIO_MODULES.json pigment `depends_on` = [accord, prism, observe]; root `allowed` (Cargo.toml:104-105) adds folio and pigment/Cargo.toml uses folio. Probe enforces root metadata, so module map is the stale side. Not edited (lead decision 6).

</topic>

<topic id="per-crate" status="active" summary="Per-crate test/check/clippy results">

| crate | unit | integration | doctest | clippy warnings |
|---|---|---|---|---|
| accord | 0 | 12/12 foundation_contract | 0 | 0 |
| observe | 0 | 19/19 foundation_contract | 0 | 2 `too_many_arguments` (src/lib.rs:919 `emit_inner`, :1019 `encode`) |
| folio | 0 | 15/15 foundation_contract | 0 | 0 |
| chronicle | 0 | 19/19 foundation_contract | 0 | 0 |
| prism | 0 | 20/20 foundation_contract | 0 | 0 |
| pigment | 0 | 4/4 masked_patch | 0 | 0 |
| nib | 3/3 | 2/2 path_boolean | 0 | 0 |
| type | 37/37 | 2/2 foundation_contract (4.19 s) | 0/3 FAIL | 1007 lib + 9 lib-test + 1 test |
| 27 scaffolded crates | 0 | - | 0 | 0 |

- Type doctest failures (src/provider/ttf/mod.rs:182 `Variation`, :205 `Tag::from_bytes`, :2080 `Face::outline_glyph`): E0432/E0433 `ttf_parser` unresolved. Root cause: vendored ttf-parser doc examples still name upstream crate `ttf_parser`; fix = mark them `ignore`/`text` or rewrite paths (Type lane owns).
- Type check warnings 666 lib (+5 lib-test unique); clippy: 291 `dead_code` constants, 246 `clippy::non_minimal_cfg` (`#[cfg(all())]`, e.g. src/provider/bidi/char_data/mod.rs:17), ~300 other `dead_code`, 27 `collapsible_if`, 17 `needless_borrow`, 12 `upper_case_acronyms`, 1 `needless_range_loop` (tests/support/mod.rs:652). Root cause: vendored subset keeps unused code and rewritten cfgs; no lint scoping on vendored modules. Warnings only.

</topic>

<topic id="probe" status="active" summary="workspace-probe use and fixes">

- Build: `rustc --edition=2024 -O tools/workspace-probe.rs -o Handshake_Artifacts/WP-KERNEL-STUDIO/studio-team-dev/probe/workspace-probe.exe` (13 s). Provenance TOML: [source] repository, subtree, revision=HEAD, tree=HEAD^{tree} (validator convention, MT-35530); [files] sha256 of Cargo.toml, Cargo.lock, rust-toolchain.toml and every selected crate Cargo.toml.
- Type gap (fixed 47f212f): `registry.hsk-studio-type.locked` lacked `cfg-if@1.0.5` (via sha2 0.10.9) and `proc-macro2@1.0.107`, `quote@1.0.47`, `serde_derive@1.0.229`, `syn@3.0.7`, `unicode-ident@1.0.26` (serde_json -> serde lock edges). Other owners: fully pinned, no unused pins.
- Policy (96ad053, lead decision): APPROVED table generated (probe:460-469); `-sys` only for NATIVE_OWNERS (471-477) with `kind = "native-ffi"` (628-633); wgpu->render-gpu, egui->controls, ffmpeg-sys-next->reel-native (479-483); ALWAYS_FORBIDDEN (484-492). Ad-hoc cases on a disposable copy: 6/6 expected (native+sys accepted; sys mislabelled pure-rust -> `registry_native_kind_required`; pure owner+sys -> `registry_unapproved_native_or_identity`; pure owner native-ffi label -> `registry_nonpure_policy_forbidden`; wgpu in render-gpu accepted; wgpu in render-cpu rejected).
- Risk: `registry_observed` limit 256 is shared across the whole selection (probe:645); a full-workspace selection with wgpu+egui closures may hit `registry_record_limit`; per-package selections unaffected.

</topic>

<topic id="enforcement" status="active" summary="What the probe enforces (file:line @96ad053)">

- Toolchain 1.97.1 in rust-toolchain.toml + `workspace.metadata.studio.toolchain` + `workspace.package.rust-version` (770); resolver "3", repository, subtree (777); Cargo.lock version 4 (780).
- `declared-members` match `crates/hsk-studio-*`, unique (794); every `workspace.members` entry declared (803) -> explicit member list, no globs.
- Selected package: member (842), `package.name` == dir (846), version literal or inline `{ workspace = true }` (441) locked in Cargo.lock (850); closure <= 35 packages (838).
- Manifest parser = bounded TOML subset (key 130, parse 222): single-line strings, single-line inline tables, NO dotted keys (`version.workspace = true` fails `syntax_expected:=`), array-of-tables only package/example (254).
- Dependencies (normal/dev/build/target.*) inline only (980); no git/branch/tag/rev/registry (906); `workspace = true` deps need `[workspace.dependencies]` (897; absent).
- Forbidden edges: handshake_core, handshake_native, surrealdb, rocksdb anywhere; egui/eframe/wgpu/image/ffmpeg-sys-next/libsqlite3-sys for accord/folio (938).
- Path deps: must be the declared member dir AND in `dependencies.<pkg>.allowed` (957); acyclic (1016).
- Registry deps: owner/lib in APPROVED (540-542); `version = "=X.Y.Z"` (576); explicit `default-features = false` (560); features == pin (598); alias needs `package` (602); every Cargo.lock closure record pinned `name@ver = {source, checksum, kind}` (625) with crates.io source and lock checksum (641); <= 256 records (606/617/645).
- Provenance [files] must hash every loaded manifest (986); inputs re-read for stability (1033).

</topic>

<topic id="recipes" status="active" summary="Add a member crate / a crates.io dependency so nothing fails">

Add member crate `hsk-studio-<x>` (all 35 already members since 3f6d5ab):
1. Name must be in `declared-members` (Cargo.toml:51) and have `[workspace.metadata.studio.dependencies.hsk-studio-<x>] allowed = [...]`.
2. Add `"crates/hsk-studio-<x>"` to `[workspace] members` (Cargo.toml:3).
3. Crate Cargo.toml: `version = { workspace = true }` inline (or literal "0.1.0"), same for edition/rust-version; inline deps only.
4. Any cargo command refreshes Cargo.lock (probe needs the package locked); commit Cargo.toml + Cargo.lock + crate dir.

Add crates.io dependency `<dep>` to `hsk-studio-<x>`:
1. Crate Cargo.toml: `<dep> = { version = "=X.Y.Z", default-features = false, features = [..] }`.
2. Cargo command (lock update), then `python -X utf8 -I .GOV/docs_repo/wp-studio-module-work/tools/registry_sync.py <workspace> --requests .../builders/dep-requests.jsonl --write`: regenerates `[workspace.metadata.studio.registry.*]` (direct + full lock closure, `kind` pure-rust|native-ffi) and the probe APPROVED block; refuses unrequested new libs; check mode (no `--write`) exits 1 when out of sync. Verified idempotent and semantically equal on current metadata.
3. Rebuild probe, run with `--package hsk-studio-<x>` -> `accepted`; commit Cargo.toml, Cargo.lock, tools/workspace-probe.rs.
Cargo never checks any of this; skipping it only fails the validator probe step.

</topic>

<topic id="timings" status="active" summary="Build timings (shared target on D:)">

- Cold `cargo check --workspace --tests` (empty target incl. crates.io download): 3m59s.
- `cargo clippy --workspace --tests` right after: 16.3 s.
- Warm `cargo check --workspace --tests` after adding 27 crates: 11.8 s.
- `cargo test --workspace --no-fail-fast` (test profile cold for workspace crates, deps warm; incl. lock wait): 51 s.

</topic>
