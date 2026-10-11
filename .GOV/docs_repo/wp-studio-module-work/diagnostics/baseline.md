---
file_id: WP-STUDIO-MODULE-WORK-BASELINE
file_kind: workspace_baseline_diagnostics
updated_at: 2026-10-11T01:35:00Z
owner: workspace-steward
candidate: 55c1096946b4dfb054331a8ae0fa6300f11b3570 (tree e69febc5a2a4588645287686c6005ca8acebe463)
---

<topic id="summary" status="active" summary="Baseline verdict at 55c1096">

- `cargo check --workspace --tests`: PASS (exit 0), 8/8 crates.
- `cargo clippy --workspace --tests`: PASS (exit 0, warnings only). Warnings: type 1017 unique, observe 2, all others 0.
- `cargo test --workspace --no-fail-fast`: PENDING (blocked on one-build-per-disk wait for lead tool install).
- `tools/workspace-probe.rs` (rustc 1.97.1, per-package selection): accord, observe, folio, chronicle, prism, pigment, nib ACCEPTED; type REJECTED `registry_transitive_pin_missing`.
- In-repo enforcement: none automatic. Cargo ignores `[workspace.metadata.studio]`; no build.rs, no CI, no test reads it. Only enforcer = probe, run manually by validators as MT proof commands.

</topic>

<topic id="check-clippy" status="active" summary="Per-crate check/clippy results">

| crate | check | check warnings | clippy warnings |
|---|---|---|---|
| accord | pass | 0 | 0 |
| observe | pass | 0 | 2 (`too_many_arguments` src/lib.rs:919 `emit_inner`, :1019 `encode`) |
| folio | pass | 0 | 0 |
| chronicle | pass | 0 | 0 |
| prism | pass | 0 | 0 |
| pigment | pass | 0 | 0 |
| nib | pass | 0 | 0 |
| type | pass | 666 lib (+5 lib-test unique) | 1007 lib + 9 lib-test + 1 test `foundation_contract` (`needless_range_loop` tests/support/mod.rs:652) |

- Type warnings are vendored rustybuzz/ttf-parser subset: 291 `dead_code` constant never used, 246 `clippy::non_minimal_cfg` (`#[cfg(all())]`, e.g. src/provider/bidi/char_data/mod.rs:17), ~300 other `dead_code` (method/struct/fn/field never used), 27 `collapsible_if`, 17 `needless_borrow`, 12 `upper_case_acronyms`. Root cause: vendored subset keeps unused table/shaper code and rewritten `cfg`s; no lint scoping on vendored modules. Not a failure (warnings only); owned by Type lane.

</topic>

<topic id="probe" status="active" summary="workspace-probe results and Type pin gap">

- Build: `rustc --edition=2024 -O tools/workspace-probe.rs -o <artifact>/studio-team-dev/probe/workspace-probe.exe` (13 s). Provenance TOML = [source] repo/subtree/revision=HEAD/tree=HEAD^{tree} + [files] sha256 of Cargo.toml, Cargo.lock, rust-toolchain.toml, each selected crate Cargo.toml (same shape validators used: MT-35530 `.validation.proof_records[2].commands[8]`).
- Type rejection root cause: `[workspace.metadata.studio.registry.hsk-studio-type].locked` lacks 6 transitive pins present in Cargo.lock closure: `cfg-if@1.0.5` (via sha2 0.10.9), `proc-macro2@1.0.107`, `quote@1.0.47`, `serde_derive@1.0.229`, `syn@3.0.7`, `unicode-ident@1.0.26` (via serde_json -> serde lock edges). Probe check: workspace-probe.rs:599 (`registry_transitive_pin_missing`). All other owners: closure fully pinned, no unused pins.
- Model drift: STUDIO_MODULES.json pigment `depends_on` = [accord, prism, observe]; root `allowed` adds folio and pigment/Cargo.toml uses folio. Probe enforces root metadata, so build is consistent; module map is stale for pigment.

</topic>

<topic id="enforcement" status="active" summary="What workspace-probe enforces (file:line)">

Probe = std-only bounded inspector, read-only, exit 0 accepted / 2 rejected. Checks:
- Toolchain 1.97.1 in rust-toolchain.toml + `workspace.metadata.studio.toolchain` + `workspace.package.rust-version` (probe:736-741); resolver "3", repository, subtree (742-748); Cargo.lock version 4 (749).
- `declared-members` entries match `crates/hsk-studio-*`, unique (752-770); every `workspace.members` entry must be declared (771-775). Members must be listed explicitly (no globs).
- Selected package: must be a member (811), manifest `package.name` == dir name (815), version (literal or `{ workspace = true }` inline, 441-448) locked in Cargo.lock (818-821). Closure <= 35 packages (807).
- Manifest parser = bounded TOML subset (62-282): single-line strings, inline tables single-line, no dotted keys (`version.workspace = true` fails at key()/expect('=') 130-146, 267-269), array-of-tables only `package`/`example` (253).
- Dependencies (all of `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, `[target.*.…]`) must be inline (943-951); no git/branch/tag/rev/registry (871-878); `workspace = true` needs `[workspace.dependencies]` (864-870, absent today).
- Forbidden: handshake_core, handshake_native, surrealdb, rocksdb everywhere; egui/eframe/wgpu/image/ffmpeg-sys-next/libsqlite3-sys for accord/folio (894-909).
- Path deps: must resolve to the declared member dir AND be in `workspace.metadata.studio.dependencies.<pkg>.allowed` (913-930); graph acyclic (976-1000).
- Registry deps (registry_closure 497-654): owner+library must be in the HARDCODED `approved` match (507-514); spec `version = "=X.Y.Z"` exact (548-551); `default-features = false` explicit (529-535); features == pinned features (559-573); alias needs `package` (575-577); every transitive lock package needs `locked` pin `name@ver = {source, checksum, kind="pure-rust"}` with crates.io source and checksum == Cargo.lock (578-612); names ending `-sys`/`_sys` and egui/eframe/wgpu/image/... rejected for ANY owner (458-479); <= 256 records.
- Provenance [files] must hash every loaded manifest (954-959); inputs re-read for stability (1001-1004).

</topic>

<topic id="recipes" status="active" summary="Add a member crate / add a crates.io dependency so nothing fails">

Add member crate `hsk-studio-<x>`:
1. Must already be in `declared-members` and have a `workspace.metadata.studio.dependencies.hsk-studio-<x>` table (all 35 present). New name -> add both first.
2. Add `"crates/hsk-studio-<x>"` to `[workspace] members` (explicit path).
3. `crates/hsk-studio-<x>/Cargo.toml`: `name`, `version = { workspace = true }` (inline, not dotted) or literal `"0.1.0"`; same for edition/rust-version; inline deps only.
4. Run any cargo command (Cargo.lock gains the package; probe needs it locked). Commit Cargo.toml + Cargo.lock + crate dir.

Add crates.io dependency `<dep>` to `hsk-studio-<x>`:
1. Crate Cargo.toml: `<dep> = { version = "=X.Y.Z", default-features = false, features = [..] }` (inline, exact pin, explicit no-defaults).
2. Root: `[workspace.metadata.studio.registry.hsk-studio-<x>]` with `direct = { <dep> = { version = "X.Y.Z", features = [..], default-features = false } }` and `locked = { "<name>@<ver>" = { source = "registry+https://github.com/rust-lang/crates.io-index", checksum = "<Cargo.lock checksum>", kind = "pure-rust" }, ... }` covering the WHOLE Cargo.lock closure of every direct dep (incl. dev-deps) - single line.
3. `tools/workspace-probe.rs`: add owner/lib to `approved` (507-514) and to the HELP descriptor text (1025; validators compare descriptor, cf. MT-35535 descriptor repair).
4. Cargo refresh lock; rebuild probe; run probe with `--package hsk-studio-<x>` -> `accepted`. Native/`-sys`/wgpu/egui deps cannot pass without changing `pure_registry_name` (458-479): lead/operator decision.
Note: none of this is checked by cargo; skipping it fails only the validator probe step.

</topic>

<topic id="timings" status="active" summary="Build timings">

- Cold `cargo check --workspace --tests` (empty target, crates.io download): 3m59s.
- `cargo clippy --workspace --tests` right after: 16.3 s.

</topic>
