<!-- GOVARTIFACTS-019 exception: operator-facing prose surface; not a machine-readable authority artifact.
     CONTROL-5 / RISK-5: this is an explicit exception to the [no-default-md-files] JSON-authority
     rule. It documents the bundled-dependency + installer policy for no-context models and the
     operator. The crate's Cargo.toml, build script, installer integrity checker, and active WP-KERNEL-012
     contracts are the executable sources. If this file conflicts with those, they win. -->
---
file_id: wp-kernel-011-mt-004-bundled-deps-policy
file_kind: operator_prose_policy
updated_at: 2026-08-15
---

<topic id="overview" status="active" wp="WP-KERNEL-012" summary="What ships and how a user installs Handshake native">

# Bundled Dependencies & Installer Policy — Handshake Native

This document answers "how does a user install Handshake?" for the native (egui/wgpu) shell.

The native app ships as one Windows installer containing `handshake-native.exe`, `handshake_core.exe`,
and `palmistry.exe`. SurrealDB is compiled into `handshake_core.exe` and runs in process: the installer
does not discover, stage, install, or launch a database executable or service. The MSI supports clean
upgrade/uninstall. It is not yet code-signed; signing remains a later concern.

</topic>

<topic id="single-binary" status="active" wp="WP-KERNEL-011" summary="handshake-native.exe is a static single binary with zero non-system DLLs">

## 1. handshake-native.exe (the shell)

- **Statically linked single binary.** Built with `--profile release-native`, which statically
  links the MSVC C runtime via `target-feature=+crt-static` (set in the crate-local
  `src/frontend/handshake_native/.cargo/config.toml`, NOT the repo root, so handshake_core builds are
  not affected — RISK-1 / CONTROL-1).
- **Embeds the full toolkit** statically: egui, eframe, egui-wgpu, wgpu, accesskit, egui_tiles.
- **Embeds the Inter fonts** (Regular + Bold) via `include_bytes!` at compile time (feature
  `bundled-fonts`, ON by default).
- **Zero non-system DLL dependencies.** Verified by `tests/test_single_binary.rs`, which parses the
  PE import table and asserts every imported DLL is an OS/CRT/apiset DLL, and that VCRUNTIME140.dll /
  MSVCP140.dll are absent (the canonical proof that CRT static linking took effect).

</topic>

<topic id="core-binary" status="active" wp="WP-KERNEL-012" summary="handshake_core.exe hosts embedded SurrealDB in process">

## 2. handshake_core.exe (backend / embedded database host)

- A **separate binary** shipped alongside `handshake-native.exe` in the same install folder.
- Hosts the SurrealDB engine as an in-process Rust dependency.
- No database child process, Windows service, port-discovery step, or separately installed database is
  part of the installer contract.

</topic>

<topic id="embedded-database" status="active" wp="WP-KERNEL-012" summary="SurrealDB is linked in process and has no external payload">

## 3. Embedded SurrealDB

- SurrealDB is linked into `handshake_core.exe`; there is no `surreal.exe` payload.
- The installer does not inspect database environment variables or search `PATH` for database tools.
- The staging tree must not contain legacy `bundled/postgres/`, PostgreSQL utilities, SQLite utilities,
  or a standalone SurrealDB server binary.
- Exact SurrealDB BUSL notices ship from `installer/windows/licenses/`.

</topic>

<topic id="fonts" status="active" wp="WP-KERNEL-011" summary="Inter Regular+Bold embedded; no external font install">

## 4. Fonts

- `Inter-Regular.ttf` and `Inter-Bold.ttf` are **embedded** in `handshake-native.exe` at compile
  time via `include_bytes!` (`src/frontend/handshake_native/src/app.rs`, `install_fonts`).
- No external font installation is required on the user's machine.
- Inter is the canonical Handshake UI font (matches the React app's
  `app/src/App.css` `font-family: 'Inter', system-ui`).
- License: Inter is OFL-licensed. `assets/fonts/OFL.txt` ships the SIL Open Font License and is
  included in the install payload requirements (RISK-3 / CONTROL-3).

</topic>

<topic id="toolkit-dlls" status="active" wp="WP-KERNEL-011" summary="Zero toolkit DLLs; all statically linked">

## 5. Toolkit DLLs

- **Zero.** All toolkit code (egui, wgpu, accesskit, egui_tiles) is statically linked into
  `handshake-native.exe`. No `egui.dll`, `wgpu.dll`, etc. are shipped.

</topic>

<topic id="os-dlls" status="active" wp="WP-KERNEL-011" summary="Allowed system DLLs and wgpu GPU backend selection">

## 6. OS DLLs & wgpu GPU backends

Allowed (Windows system) DLLs the binary may import:
`KERNEL32.dll`, `USER32.dll`, `GDI32.dll`, `ADVAPI32.dll`, `SHELL32.dll`, `ntdll.dll`,
`d3d11.dll`, `d3d12.dll`, `dxgi.dll`, `dcomp.dll`, `dwmapi.dll`, `opengl32.dll`,
`vulkan-1.dll` (optional, via GPU driver), plus `api-ms-win-*` apisets and the static-CRT
support DLLs that resolve in `%WINDIR%\System32`.

**wgpu backend selection at runtime:** DX12 > DX11 > Vulkan > WARP (software), overridable via the
`WGPU_BACKEND` env var. DX12/DX11 are system DLLs; Vulkan needs the driver-provided `vulkan-1.dll`
(a system component); WARP is a built-in Windows 10+ software rasterizer. **No non-system DLLs are
required for any wgpu backend.**

</topic>

<topic id="font-provenance" status="active" wp="WP-KERNEL-011" summary="Canonical Inter 3.19 release + SHA-256 checksums">

## Font provenance

- Source: Inter **v3.19** release — <https://github.com/rsms/inter/releases/tag/v3.19>
  (`Inter-3.19.zip`), faces taken from `Inter Hinted for Windows/Desktop/`.
- SHA-256 (verified in `tests/test_single_binary.rs::fonts_present_sized_and_provenance_matches`):
  - `Inter-Regular.ttf` = `529be850e06f62f8904f22bda77e45bde4834498fdbec4ff4201fa3177447a3a`
  - `Inter-Bold.ttf`    = `e6c172fd8a2f957414a7a63ec8deb7f2aa239182394cfa5ee2ea6927c6194389`
- License file: `assets/fonts/OFL.txt` (SIL Open Font License, from the same release).

</topic>

<topic id="installer-tooling" status="active" wp="WP-KERNEL-011" summary="WiX 4 chosen over cargo-bundle">

## Installer tooling decision

**WiX 4** is used directly for the Windows installer (`installer/windows/handshake_native.wxs`).

`cargo-bundle` was evaluated but **rejected** in favor of WiX 4 because WiX provides full control
over install layout, uninstall, and upgrade (MajorUpgrade) behavior required for bundling
`handshake_core.exe` alongside `handshake-native.exe` and managing the Start Menu shortcut. No
Docker or outside-app dependency is introduced (CX-503S).

UpgradeCode GUID: `609E7B1F-D861-4353-A0D6-85B79B459614` — **do not change** after first release.

</topic>

<topic id="build-pipeline" status="active" wp="WP-KERNEL-012" summary="Installer build pipeline and artifact allocation">

## Build pipeline

The crate-local script builds and stages all three product binaries, then produces an MSI when WiX is
available or a real zip fallback otherwise:

```powershell
pwsh -NoProfile -File src/frontend/handshake_native/scripts/build_installer.ps1
```

The script derives `Handshake_Artifacts/handshake-release-target` from its own location and rejects a
`-ShortTargetDir` / `HANDSHAKE_SHORT_TARGET_DIR` override outside that project-allocated artifact root.
It builds `handshake-native.exe` with `release-native`, builds `handshake_core.exe` with `app-runtime`,
builds `palmistry.exe`, and stages the three siblings plus fonts, grammars, and license notices.

**MAX_PATH (Windows 260-char) constraint — important.** The `release-native` profile name is longer
than `release`, which pushes the deepest build-script output paths (`icu_*`, `parking_lot_core`,
`windows_x86_64_msvc`) past the Windows 260-char `MAX_PATH` limit in a deeply nested target directory.
`link.exe` does **not** honor the registry `LongPathsEnabled` opt-in, so it fails with
`LNK1104: cannot open file build_script_build.exe`. The fix is purely environmental: build
release-native into a short target dir (the `dev`/`release` profiles fit under 260 and are
unaffected). This is **not** a profile defect — the full contract profile (fat LTO, codegen-units=1,
panic=abort, strip=symbols) builds and links cleanly once the path is short.

Single-binary proof (run with the same short `CARGO_TARGET_DIR`):
```
cargo test --test test_single_binary -- --nocapture
# -> PASS: no non-system DLLs found in handshake-native.exe (20 system DLLs imported)
```

Notes:

- **release-native profile is for installer builds only.** Dev builds use `--profile dev`; CI smoke
  tests use `--profile release`. The profile pins `lto = "fat"` + `codegen-units = 1`, so use the
  installer script only for release proof rather than ordinary iteration.
- **strip = "symbols"** on `release-native` reduces binary size and links fine on this toolchain
  (MSVC link.exe 14.44). Debug info lives in a side `.pdb`, not the `.exe`. If a future toolchain
  rejects strip, fall back to `strip = "none"` and note the limitation.
- **Installer CI is optional (CONTROL-4 / RISK-4).** If WiX 4/5 is absent, the script emits the
  self-contained zip fallback and never fabricates an MSI.
- **Disk-agnostic ([GLOBAL-PORTABILITY-004]):** `build_installer.ps1` hardcodes no absolute paths.
  The artifact root, manifests, WiX source, and repo root resolve relative to `$PSScriptRoot`.

</topic>

<topic id="manual-validation-gaps" status="active" wp="WP-KERNEL-012" summary="Focused proof still required after concurrent implementation settles">

## Validation still required

- Run the focused installer tests after concurrent Cargo-editing lanes settle.
- Run `build_installer.ps1 -ForceZip`, inspect the archive, and confirm all three product binaries plus
  both SurrealDB notices are present while no external database executable/directory is present.
- On a WiX-equipped host, build and validate the MSI and inspect its installed payload. XML parsing alone
  is structural evidence, not MSI production proof.

</topic>
