---
file_id: WP-STUDIO-MODULE-WORK-WORKSPACE-RULES
file_kind: builder_workspace_rules
updated_at: 2026-10-11T01:30:00Z
owner: workspace-steward
---

<topic id="cargo-form" status="active" summary="Only allowed cargo invocation">

- Always: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo <cmd> -p <your-crate> ...`
- `<cmd>`: `check --tests`, `clippy --tests`, focused `test`. Never `--workspace` tests; no `timeout` wrapper; > 2 min -> background + poll. Nothing on C: (env sets target + TMP on D:).
- `Blocking waiting for file lock` = another agent building; wait, do not kill.

</topic>

<topic id="files" status="active" summary="What builders may edit">

- Edit only `crates/<your-crate>/**`. Never edit root `Cargo.toml`, `tools/workspace-probe.rs`, `rust-toolchain.toml`, other crates.
- Crate `Cargo.toml` (parsed by `tools/workspace-probe.rs`, bounded TOML subset): single-line inline tables only; no dotted keys (`x.workspace = true` -> write `x = { workspace = true }`); no `[dependencies.foo]` subtables; no git/registry/branch/rev fields; no `workspace = true` deps (no `[workspace.dependencies]` exists).
- Cargo.lock: commit it with your crate only if `git diff -- Cargo.lock` changes nothing outside your own `[[package]]` block; otherwise leave it for the steward.
- Commit with explicit paths; on `index.lock` wait 5 s and retry; never force push.

</topic>

<topic id="path-deps" status="active" summary="Allowed intra-Studio edges (root workspace.metadata.studio.dependencies)">

- Add allowed edges yourself in your crate: `hsk-studio-<dep> = { path = "../hsk-studio-<dep>", version = "=0.1.0", default-features = false }`. Any edge not listed rejects (`dependency_outside_declared_dag`); cycles reject.
- Every crate below may also use `accord` + `observe`, except: accord -> none; observe -> accord; folio -> accord only.
- chronicle: folio | pigment: prism, folio | prism, nib, type, score, reel, controls: (none extra)
- layout: folio, type, nib | develop: pigment, prism | components: folio, layout | pulse: folio
- motion: folio, nib, type | composite: folio, prism, pigment, nib, type
- render-cpu, render-gpu: prism, pigment, nib, type | package, interop-media, interop-lrcat, press, web: folio
- interop-psd: folio, pigment, prism, type | interop-vector: folio, nib, type, prism
- interop-fig: folio, components, type | interop-layout: folio, layout, type, prism
- session: folio, chronicle, package | harness: folio, session, controls | kernel-bindings: folio, session
- score-device, score-plugin: score | reel-native: reel | motion-js: motion
- Forbidden always: handshake_core, handshake_native, surrealdb, rocksdb (accord/folio also: egui, eframe, wgpu, image, ffmpeg-sys-next, libsqlite3-sys).

</topic>

<topic id="crates-io-deps" status="active" summary="Requesting a crates.io dependency">

- Do NOT add crates.io deps yourself. Append ONE JSON line to `builders/dep-requests.jsonl`:
  `{"crate":"hsk-studio-<x>","dep":"<name>","version":"<X.Y.Z>","features":["..."],"kind":"normal|dev","reason":"<why, research ref>","pure_rust":true}`
- `pure_rust:false` (FFI, `-sys`, wgpu, egui, codecs) is rejected by the probe policy (`pure_registry_name`); steward escalates to lead before any merge.
- Steward merges: inserts the exact line `<dep> = { version = "=X.Y.Z", default-features = false, features = [...] }` into your Cargo.toml, adds `direct` + every transitive `locked` pin (crates.io source, lock checksum, `kind = "pure-rust"`) under `workspace.metadata.studio.registry.<crate>`, extends the probe `approved` list + descriptor, refreshes Cargo.lock, runs the probe, commits, pushes, and reports back. Do not edit that line afterwards.
- Prefer deps already pinned in the workspace (serde, serde_json, schemars, sha2, kurbo, moxcms, bitflags, bytemuck, unicode-*) at the same version/features.

</topic>
