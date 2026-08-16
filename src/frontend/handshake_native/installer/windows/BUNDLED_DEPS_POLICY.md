---
file_id: handshake-native-bundled-deps-policy
file_kind: operator_prose_installer_policy
updated_at: 2026-08-15
wp: WP-KERNEL-012
mt: MT-031
---

<topic id="purpose" wp="WP-KERNEL-012" summary="What the single-installer policy guarantees">

# Single-Installer Bundled-Deps Policy (CX-008-VIS)

Handshake ships as ONE installable artifact that bundles every runtime dependency. A clean Windows user
profile can install and launch the native shell with ZERO external prerequisites:

- no system WebView2 runtime (the native shell uses wgpu/egui, not a webview);
- no separately installed or bundled database server (SurrealDB runs in-process in `handshake_core.exe`);
- no CDN / network download at install time or first launch (fonts, grammars, runtime assets all bundled).

This file is the human-reviewable policy and build manual for that guarantee (HBR-MAN). It is referenced
by `Cargo.toml` (`workspace.metadata.handshake-native.installer`), `build.rs` (font fail-fast message),
and `tests/test_single_binary.rs` (font provenance + CRT-static + MAX_PATH build note).

</topic>

<topic id="bundle-layout" wp="WP-KERNEL-012" summary="Exe-relative asset layout the installer stages and the runtime verifies">

# Bundle Layout (exe-relative)

The installer stages all assets next to the native binary. Both the WiX MSI and the zip fallback install
this exact tree, and `installer::check_bundle_integrity` (runtime) + `bundled_deps_audit.rs` (build-time)
verify it. The canonical list lives in code at `src/installer/mod.rs::REQUIRED_ASSETS` — this table must
stay in sync with that constant (the constant is authoritative).

```text
<install_dir>/
  handshake-native.exe          # the single native shell binary (crt-static, no non-system DLLs)
  handshake_core.exe            # backend with in-process embedded SurrealDB
  palmistry.exe                 # external crash/freeze watcher
  fonts/
    Inter-Regular.ttf           # bundled UI fonts (MT-004); >= 1 .ttf/.otf required
    Inter-Bold.ttf
    OFL.txt
  grammars/                     # tree-sitter syntax grammars; may be empty on first pass (dir must exist)
  licenses/
    SurrealDB-3.0-BUSL-1.1.txt
    SurrealDB-Protocol-2.0-BUSL-1.1.txt
```

</topic>

<topic id="embedded-surrealdb-contract" wp="WP-KERNEL-012" status="active" summary="SurrealDB requires no external database payload or service">

# Embedded SurrealDB Contract

`handshake_core.exe` contains the SurrealDB engine as an in-process Rust dependency. Packaging therefore:

1. stages `handshake_core.exe` beside `handshake-native.exe`;
2. does not discover or consume database binary environment variables;
3. does not create a database bundle directory or placeholder executable;
4. does not install or start a database service; and
5. ships the exact SurrealDB BUSL notices in `installer/windows/licenses/`.

`installer::check_bundle_integrity` requires the backend binary itself. `bundled_deps_audit.rs` rejects
legacy PostgreSQL utilities, SQLite utilities, a standalone `surreal.exe`, and `bundled/postgres/`.

</topic>

<topic id="installer-tooling-decision" wp="WP-KERNEL-012" summary="WiX chosen; zip fallback; toolchain gating">

# Installer Tooling Decision

Windows installer: **WiX 4/5** (`handshake_native.wxs`). WiX is the field-standard MSI authoring toolkit,
ships a built-in `HarvestDirectory` that pulls the whole staging tree into the MSI (no per-file edits
when assets change), and produces a signed-installable `.msi`. NSIS is a documented alternative; the
`build_installer.ps1` zip fallback is always available so the smoke can complete on any host.

## Toolchain gating

The real MSI build step in `build_installer.ps1` is gated on `wix` being available. When it is absent,
the script produces `handshake-setup.zip` (a single self-contained artifact) instead of faking an `.msi`.

## Build the MSI on a WiX-equipped host

```powershell
dotnet tool install --global wix          # one-time: installs the `wix` build tool
wix extension add --global WixToolset.Util.wixext
pwsh -NoProfile -File scripts/build_installer.ps1   # detects wix and produces handshake-setup.msi
```

`build_installer.ps1` auto-detects the toolchain; the manual `wix build` command is documented at the top
of `handshake_native.wxs`.

</topic>

<topic id="build-path-max-path" wp="WP-KERNEL-012" summary="Where release-native build output is allocated">

# release-native artifact allocation

`build_installer.ps1` derives its canonical release target from the crate location and writes only to
`Handshake_Artifacts/handshake-release-target`. It rejects `HANDSHAKE_SHORT_TARGET_DIR` values that
resolve outside the allocated `Handshake_Artifacts` root. Installer builds must never fall back to a
TEMP directory, drive root, repo-local `target`, or another machine-global path.

</topic>

<topic id="font-provenance" wp="WP-KERNEL-012" summary="Bundled font identity and license">

# Font provenance

Bundled UI faces are the canonical **Inter 3.19** Desktop faces (SIL Open Font License, `OFL.txt`
shipped beside them). `tests/test_single_binary.rs` pins their SHA-256 checksums so a swapped/placeholder
font is caught:

- `Inter-Regular.ttf` = `529be850e06f62f8904f22bda77e45bde4834498fdbec4ff4201fa3177447a3a`
- `Inter-Bold.ttf`    = `e6c172fd8a2f957414a7a63ec8deb7f2aa239182394cfa5ee2ea6927c6194389`

</topic>

<topic id="ci-prerequisites" wp="WP-KERNEL-012" summary="What a CI runner needs to build the installer">

# CI / build prerequisites

- Repo-pinned Rust toolchain, `cargo` on PATH.
- PowerShell 7 (`pwsh`) for `build_installer.ps1`.
- A writable project-allocated `Handshake_Artifacts` root; the script derives the release target.
- OPTIONAL: WiX 4/5 (`dotnet tool install --global wix`) for the `.msi`. Absent it, the script emits a zip.
- No database server toolchain is a prerequisite. SurrealDB compiles into `handshake_core.exe`.

</topic>
