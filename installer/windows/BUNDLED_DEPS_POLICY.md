<!-- GOVARTIFACTS-019 exception: operator-facing prose surface; not a machine-readable authority artifact.
     CONTROL-5 / RISK-5: this is an explicit exception to the [no-default-md-files] JSON-authority
     rule. It documents the bundled-dependency + installer policy for the WP-KERNEL-011 native shell
     for no-context models and the operator. The machine-readable authority is the MT-004 contract
     (.GOV/task_packets/WP-KERNEL-011-Native-WorkSurface-Shell-v1/MT-004.json) and the crate's
     Cargo.toml / .cargo/config.toml. If this file conflicts with those, they win. -->
---
file_id: wp-kernel-011-mt-004-bundled-deps-policy
file_kind: operator_prose_policy
updated_at: 2026-06-20
---

<topic id="overview" status="active" wp="WP-KERNEL-011" summary="What ships and how a user installs Handshake native">

# Bundled Dependencies & Installer Policy — Handshake Native (WP-KERNEL-011 MT-004)

This document answers "how does a user install Handshake?" for the native (egui/wgpu) shell.

The native app ships as **two binaries in one Windows MSI** plus an externally-managed PostgreSQL
cluster that is **not** bundled as a binary. The MSI in MT-004 is a functional scaffold: it installs
the binaries, creates a Start Menu shortcut, and supports clean upgrade/uninstall. It is **not yet
code-signed** — signing is a later concern.

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

<topic id="core-binary" status="active" wp="WP-KERNEL-011" summary="handshake_core.exe is a separate binary that manages PostgreSQL">

## 2. handshake_core.exe (the backend / managed-postgres host)

- A **separate binary** shipped alongside `handshake-native.exe` in the same install folder.
- Manages the PostgreSQL cluster lifecycle via
  `handshake_core::managed_postgres::ManagedPostgres::ensure_running()`
  (`src/backend/handshake_core/src/managed_postgres.rs`).
- MT-004 **packages** this binary in the MSI but does **not** modify handshake_core (it lives under
  `src/backend/`, which is reuse-via-API only for this MT). The native shell's sidecar launch of
  handshake_core is a **later MT** — for MT-004 the two-binary handoff is documented, not wired.

</topic>

<topic id="postgres" status="active" wp="WP-KERNEL-011" summary="PostgreSQL cluster is managed, not bundled as a binary">

## 3. PostgreSQL cluster (NOT bundled)

- The PostgreSQL server is **not** bundled as a binary in the installer.
- `handshake_core` discovers or provisions a cluster through
  `ManagedPostgres` + `ManagedPostgresConfig::from_env()`
  (`src/backend/handshake_core/src/managed_postgres.rs`).
- This keeps the MSI small and avoids shipping/patching a full PostgreSQL distribution.
- No Docker, no Docker Compose, no third-party daemons (CX-503S): PostgreSQL is a
  Handshake-managed component, not an outside app the operator must start.

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

<topic id="build-pipeline" status="active" wp="WP-KERNEL-011" summary="Three-step build pipeline + LTO and CI gating">

## Build pipeline

The full pipeline to produce a user-installable Handshake native app. **Build release-native into a
SHORT target dir** (see the MAX_PATH note below) — `build_installer.ps1` does this automatically:

```
# from src/frontend/handshake_native, with a short CARGO_TARGET_DIR (e.g. D:\hsk-rn-target):
set CARGO_TARGET_DIR=D:\hsk-rn-target
cargo build --profile release-native -p handshake-native
cargo build --profile release-native -p handshake_core --features app-runtime  # from its crate dir
# then, from the repo root:
pwsh installer/windows/build_installer.ps1 -OutDir ./dist
```

`build_installer.ps1` sets a short `CARGO_TARGET_DIR` (`<DriveRoot>\hsk-rn-target` by default, or
`-CargoTarget` / `$env:HANDSHAKE_RN_TARGET`) so the MAX_PATH issue below cannot recur.

**MAX_PATH (Windows 260-char) constraint — important.** The `release-native` profile name is longer
than `release`, which pushes the deepest build-script output paths (`icu_*`, `parking_lot_core`,
`windows_x86_64_msvc`) past the Windows 260-char `MAX_PATH` limit when the crate's default external
target dir (`../Handshake_Artifacts/handshake-native-target`, set in `.cargo/config.toml`) is used.
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
  tests use `--profile release`. The profile pins `lto = "fat"` + `codegen-units = 1`, which is slow
  (RISK-2): a clean release-native build takes ~3-4 min here. For faster iteration, prefer the
  dev/release profiles; the `build_installer.ps1 -LtoFat` switch (and the `HANDSHAKE_LTO` env var)
  is reserved for a future thin/fat LTO split (CONTROL-2).
- **strip = "symbols"** on `release-native` reduces binary size and links fine on this toolchain
  (MSVC link.exe 14.44). Debug info lives in a side `.pdb`, not the `.exe`. If a future toolchain
  rejects strip, fall back to `strip = "none"` and note the limitation.
- **Installer CI is optional (CONTROL-4 / RISK-4).** The default Cargo CI pipeline runs only the
  Cargo steps. `build_installer.ps1` should be run only on release branches or when
  `HANDSHAKE_BUILD_INSTALLER=1`. If WiX 4 is absent, the script exits non-zero with a clear message
  and does **not** fail the Cargo build.
- **Disk-agnostic ([GLOBAL-PORTABILITY-004]):** `build_installer.ps1` hardcodes no absolute paths.
  The cargo target dir is discovered via `cargo metadata` when `-CargoTarget` is omitted, and the
  WiX source + repo root are resolved relative to `$PSScriptRoot`. Note the native crate's
  `.cargo/config.toml` redirects the cargo target dir to the external artifacts root
  (`../Handshake_Artifacts/handshake-native-target`, CX-212E), so the `release-native` binary lives
  there, not under the repo `target/`.

</topic>

<topic id="manual-validation-gaps" status="active" wp="WP-KERNEL-011" summary="What still needs manual proof in this environment">

## Manual validation gaps (this environment)

- **WiX 4 toolchain is not installed** on this build host (`wix` not on PATH), so
  `wix build --validate handshake_native.wxs` and the MSI production step could not be exercised
  here. The `.wxs` is authored to the WiX 4 schema and is gated behind the availability check in
  `build_installer.ps1`. Validation/MSI production must be confirmed on a host with WiX 4 installed
  (`dotnet tool install --global wix`). This is a documented proof gap, not a fabricated pass.
- **release-native single-binary test** skips cleanly until the release-native binary is built;
  building it requires the slow fat-LTO profile. See the handoff for which proofs were exercised.

</topic>
