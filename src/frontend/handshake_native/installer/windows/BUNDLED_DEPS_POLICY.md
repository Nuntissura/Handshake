---
file_id: handshake-native-bundled-deps-policy
file_kind: operator_prose_installer_policy
updated_at: 2026-06-21
wp: WP-KERNEL-011
mt: MT-031
---

<topic id="purpose" wp="WP-KERNEL-011" summary="What the single-installer policy guarantees">

# Single-Installer Bundled-Deps Policy (CX-008-VIS)

Handshake ships as ONE installable artifact that bundles every runtime dependency. A clean Windows user
profile can install and launch the native shell with ZERO external prerequisites:

- no system WebView2 runtime (the native shell uses wgpu/egui, not a webview);
- no separately-installed PostgreSQL (the managed cluster binaries ship inside the bundle);
- no CDN / network download at install time or first launch (fonts, grammars, runtime assets all bundled).

This file is the human-reviewable policy and build manual for that guarantee (HBR-MAN). It is referenced
by `Cargo.toml` (`workspace.metadata.handshake-native.installer`), `build.rs` (font fail-fast message),
and `tests/test_single_binary.rs` (font provenance + CRT-static + MAX_PATH build note).

</topic>

<topic id="bundle-layout" wp="WP-KERNEL-011" summary="Exe-relative asset layout the installer stages and the runtime verifies">

# Bundle Layout (exe-relative)

The installer stages all assets next to the native binary. Both the WiX MSI and the zip fallback install
this exact tree, and `installer::check_bundle_integrity` (runtime) + `bundled_deps_audit.rs` (build-time)
verify it. The canonical list lives in code at `src/installer/mod.rs::REQUIRED_ASSETS` — this table must
stay in sync with that constant (the constant is authoritative).

```text
<install_dir>/
  handshake-native.exe          # the single native shell binary (crt-static, no non-system DLLs)
  bundled/
    postgres/
      pg_ctl.exe                # managed-postgres anchor binary (+ initdb/pg_isready/psql/postgres.exe + lib/share)
  fonts/
    Inter-Regular.ttf           # bundled UI fonts (MT-004); >= 1 .ttf/.otf required
    Inter-Bold.ttf
    OFL.txt
  grammars/                     # tree-sitter syntax grammars; may be empty on first pass (dir must exist)
```

</topic>

<topic id="managed-postgres-path-contract" wp="WP-KERNEL-011" summary="How bundled postgres binaries are discovered at runtime">

# Managed-PostgreSQL Path Contract (RISK-031-01)

`handshake_core::managed_postgres` (read-only for MT-031: `src/backend/handshake_core/src/managed_postgres.rs`)
resolves `pg_ctl`/`initdb`/`pg_isready`/`psql` via, in order: an explicit `bin_dir` config field, the
`HANDSHAKE_MANAGED_PG_BIN` env var (the constant `managed_postgres::MANAGED_PG_BIN_ENV`), the standard
`PGBIN` env var, a fixed Windows default install path, then `PATH`. It does **not** today derive a path
from `current_exe()`.

The single-installer guarantee therefore requires two cooperating pieces:

1. The installer stages the postgres binaries at the exe-relative path `bundled/postgres/`
   (done by `build_installer.ps1` and verified by `installer::check_bundle_integrity`).
2. `handshake_core` startup exports `HANDSHAKE_MANAGED_PG_BIN` (or `PGBIN`) = `<exe_dir>/bundled/postgres`
   before `ManagedPostgres::ensure_running`, so the managed cluster finds the bundled binaries instead of
   falling through to `PATH` / the default install path.

Piece (2) is a backend change (`src/backend/handshake_core`, a `forbidden_path` for MT-031) and is
recorded here as the single follow-up needed to wire the bundle to the managed cluster. Until it lands,
`bundled/postgres/` is the staged, verified location both halves target; the runtime self-check already
fails if it is missing (HBR-STOP).

</topic>

<topic id="installer-tooling-decision" wp="WP-KERNEL-011" summary="WiX chosen; zip fallback; toolchain gating on this host">

# Installer Tooling Decision

Windows installer: **WiX 4/5** (`handshake_native.wxs`). WiX is the field-standard MSI authoring toolkit,
ships a built-in `HarvestDirectory` that pulls the whole staging tree into the MSI (no per-file edits
when assets change), and produces a signed-installable `.msi`. NSIS is a documented alternative; the
`build_installer.ps1` zip fallback is always available so the smoke can complete on any host.

## Toolchain gating on this build host (verified 2026-06-21)

The WiX toolchain is **NOT installed** on this host: `wix --version`, `cargo wix --version`, legacy
`candle.exe`/`light.exe`, and `makensis` are all absent (`dotnet` IS present). The real MSI build step in
`build_installer.ps1` is therefore GATED to run only when `wix` (or `cargo wix`) is on PATH — mirroring
the GPU / live-desktop `#[ignore]` gating used elsewhere in WP-KERNEL-011. On this host the script
produces `handshake-setup.zip` (a single self-contained artifact) instead of faking an `.msi`.

## Build the MSI on a WiX-equipped host

```powershell
dotnet tool install --global wix          # one-time: installs the `wix` build tool
wix extension add --global WixToolset.Util.wixext
pwsh -NoProfile -File scripts/build_installer.ps1   # detects wix and produces handshake-setup.msi
```

`build_installer.ps1` auto-detects the toolchain; the manual `wix build` command is documented at the top
of `handshake_native.wxs`.

</topic>

<topic id="build-path-max-path" wp="WP-KERNEL-011" summary="Why release-native builds into a short target dir">

# release-native + Windows MAX_PATH (build constraint, not a defect)

The `release-native` profile name is longer than `release`, which pushes the deepest build-script output
paths (`icu_*`, `parking_lot_core`, `windows_x86_64_msvc`) past the Windows 260-char MAX_PATH limit when
the crate's external target-dir (`.cargo/config.toml`) is used, causing `link.exe` LNK1104 ("cannot open
file build_script_build.exe"). `link.exe` does not honor the registry `LongPathsEnabled` opt-in.

Fix (applied by `build_installer.ps1`): build `release-native` into a SHORT `CARGO_TARGET_DIR` (e.g.
`C:\hsk-rn`). The single-binary proof test resolves the binary from `CARGO_TARGET_DIR`. The profile
settings themselves (fat LTO, codegen-units=1, panic=abort, strip=symbols) build cleanly once the path is
short — this is an environment/path constraint, not a profile defect.

</topic>

<topic id="font-provenance" wp="WP-KERNEL-011" summary="Bundled font identity and license">

# Font provenance

Bundled UI faces are the canonical **Inter 3.19** Desktop faces (SIL Open Font License, `OFL.txt`
shipped beside them). `tests/test_single_binary.rs` pins their SHA-256 checksums so a swapped/placeholder
font is caught:

- `Inter-Regular.ttf` = `529be850e06f62f8904f22bda77e45bde4834498fdbec4ff4201fa3177447a3a`
- `Inter-Bold.ttf`    = `e6c172fd8a2f957414a7a63ec8deb7f2aa239182394cfa5ee2ea6927c6194389`

</topic>

<topic id="ci-prerequisites" wp="WP-KERNEL-011" summary="What a CI runner needs to build the installer">

# CI / build prerequisites

- Rust stable toolchain (rustc 1.91.1 pin), `cargo` on PATH.
- PowerShell 7 (`pwsh`) for `build_installer.ps1`.
- A short writable drive path for `CARGO_TARGET_DIR` (MAX_PATH; the script picks one).
- OPTIONAL: WiX 4/5 (`dotnet tool install --global wix`) for the `.msi`. Absent it, the script emits a zip.
- OPTIONAL: PostgreSQL binaries to stage under `bundled/postgres/`. If the host has no postgres toolchain,
  `build_installer.ps1` stages a minimal placeholder so the bundle layout is correct and the smoke can
  run; a real release MUST stage the actual managed-postgres binaries (see managed-postgres-path-contract).

</topic>
