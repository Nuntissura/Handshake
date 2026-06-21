# native_gui proof artifacts + installer smoke prerequisites

This directory holds machine-readable proof artifacts for WP-KERNEL-011 native-GUI tests.
`artifacts/` is written by the integration tests; do not hand-edit it.

## MT-031 single-installer build smoke

The MT-031 tests build and inspect the single-installer bundle:

- `installer_build_smoke.rs` (crate `tests/`) — runs `scripts/build_installer.ps1`, asserts a single
  installer artifact is produced, runs the staged binary with `--version` and `--self-check` under a
  clean-profile env, checks the PE import table for a system WebView2 dependency, and writes
  `artifacts/installer_smoke_report.json`.
- `bundled_deps_audit.rs` (crate `tests/`) — walks the staged bundle, asserts every required runtime
  asset is present and no system WebView2 DLL ships, and writes `artifacts/bundle_deps_audit_report.json`.

### Prerequisites

- Rust stable toolchain (rustc 1.91.1 pin) + `cargo` on PATH.
- PowerShell 7 (`pwsh`) on PATH (drives `build_installer.ps1`).
- A short writable `CARGO_TARGET_DIR` for the `release-native` profile (Windows MAX_PATH; the script
  picks one automatically — see `installer/windows/BUNDLED_DEPS_POLICY.md`).
- OPTIONAL: WiX 4/5 (`dotnet tool install --global wix`) to build the real `.msi`. Absent it, the script
  emits a `handshake-setup.zip` (still a single self-contained artifact) and the smoke runs `-ForceZip`.
- OPTIONAL: a PostgreSQL toolchain on PATH / `HANDSHAKE_MANAGED_PG_BIN` / `PGBIN` to stage the real managed
  binaries. Absent it, the script stages a documented placeholder so the bundle layout is valid; a real
  release MUST stage the real binaries.

### Clean-profile limitation (documented approximation)

A true clean Windows profile test needs a VM snapshot or Windows Sandbox. The smoke approximates a clean
profile by running the staged binary from a temp staging tree with `HANDSHAKE_WORKSPACE_ROOT` and
`HANDSHAKE_RUNTIME_ROOT` removed from the environment, and `--self-check` (which does not start the GUI,
open a window, or connect to postgres). This is the best practical automated approximation without a full
VM; a real clean-profile VM install pass is a later hardening item.

### Run

```bash
# from src/frontend/handshake_native
cargo test -p handshake-native --test installer_build_smoke -- --nocapture
cargo test -p handshake-native --test bundled_deps_audit -- --nocapture
```
