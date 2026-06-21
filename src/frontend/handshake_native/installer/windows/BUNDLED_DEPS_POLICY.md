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

<topic id="managed-postgres-path-contract" wp="WP-KERNEL-011" status="DONE" summary="How bundled postgres binaries are auto-discovered at runtime">

# Managed-PostgreSQL Path Contract (RISK-031-01)

`handshake_core::managed_postgres` (`src/backend/handshake_core/src/managed_postgres.rs`) resolves
`pg_ctl`/`initdb`/`pg_isready`/`psql` via this `bin_dir` precedence, highest first:

1. `HANDSHAKE_MANAGED_PG_BIN` env var (operator override; the constant `managed_postgres::MANAGED_PG_BIN_ENV`);
2. the standard `PGBIN` env var;
3. **exe-relative auto-discovery** of `<exe_dir>/bundled/postgres` — used automatically when its `pg_ctl`
   (`pg_ctl.exe` on Windows) actually exists there;
4. empty `bin_dir`, which falls through inside `resolve_bin` to `PGBIN` / the fixed Windows default install
   path / `PATH`.

The single-installer guarantee therefore now requires only ONE installer-side piece:

1. The installer stages the postgres binaries at the exe-relative path `bundled/postgres/`
   (done by `build_installer.ps1` and verified by `installer::check_bundle_integrity`).

`handshake_core` startup **no longer needs to export `HANDSHAKE_MANAGED_PG_BIN`/`PGBIN`**: when Handshake
runs as an installed app, `ManagedPostgresConfig::from_env` auto-discovers `<exe_dir>/bundled/postgres`
(via `current_exe()` -> `bundled_bin_dir`) and uses it as `bin_dir`. The discovery is exe-relative and
disk-agnostic (no hardcoded absolute path), and is gated on `pg_ctl` actually existing there, so a
non-bundled / dev build silently falls through to `PATH`/default behavior. An operator may still override
with `HANDSHAKE_MANAGED_PG_BIN` (it wins over the bundle). An incomplete bundle (`pg_ctl` present but a
sibling like `initdb` missing) hard-errors via `resolve_bin` step 1 rather than silently using a
different-version system PostgreSQL.

**Status: DONE.** The backend wiring (the previous single follow-up) has landed: `bundled_bin_dir` /
`bundled_bin_dir_from_current_exe` plus the `from_env` precedence change, covered by unit tests in
`managed_postgres.rs`. `bundled/postgres/` remains the staged, verified location; the runtime self-check
still fails if it is missing (HBR-STOP).

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
