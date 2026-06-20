<#
.SYNOPSIS
  WP-KERNEL-011 MT-004 — build the Handshake native Windows MSI (WiX 4 scaffold).

.DESCRIPTION
  Builds the two release-native product binaries (handshake-native.exe + handshake_core.exe), then
  invokes the WiX 4 toolchain to produce handshake-native-setup.msi.

  CI GATING (CONTROL-4 / RISK-4): this is an OPTIONAL installer job. The default Cargo CI pipeline
  does NOT run it. It is intended to run only on release branches or when HANDSHAKE_BUILD_INSTALLER=1
  is set, so a missing WiX toolchain on ordinary CI never produces a silently-green-but-MSI-less
  build. If WiX 4 is not installed, this script exits non-zero with a clear message; it does NOT
  fail the Cargo build (the Cargo steps are independent and run first).

  DISK-AGNOSTIC ([GLOBAL-PORTABILITY-004]): no absolute paths are hardcoded. The cargo target dir is
  discovered via `cargo metadata` when -CargoTarget is not supplied; the WiX source is resolved
  relative to $PSScriptRoot.

.PARAMETER CargoTarget
  Cargo target directory root (the dir containing the `release-native` profile output). If omitted,
  it is discovered via `cargo metadata --format-version 1 --no-deps`.

.PARAMETER OutDir
  Output directory for handshake-native-setup.msi. Created if missing. Defaults to <repo>/dist.

.PARAMETER LtoFat
  When set, builds the binaries with HANDSHAKE_LTO=1 to enable fat LTO (slower, smaller). Off by
  default for faster iteration (CONTROL-2 / RISK-2) — note: the release-native profile pins fat LTO
  at the Cargo level, so this switch is reserved for a future thin/fat split; documented for parity
  with BUNDLED_DEPS_POLICY.md "Build pipeline".

.EXAMPLE
  pwsh installer/windows/build_installer.ps1 -OutDir ./dist
#>
[CmdletBinding()]
param(
    [string]$CargoTarget,
    [string]$OutDir,
    [switch]$LtoFat
)

$ErrorActionPreference = 'Stop'

# --- resolve paths (disk-agnostic) ---
$scriptDir = $PSScriptRoot
# repo layout: <repo>/installer/windows/build_installer.ps1 ; the native crate is the cargo root for
# the release-native profile and lives at <repo>/src/frontend/handshake_native.
$repoRoot  = (Resolve-Path (Join-Path $scriptDir '..\..')).Path
$nativeCrate = Join-Path $repoRoot 'src\frontend\handshake_native'
$wxs = Join-Path $scriptDir 'handshake_native.wxs'

if (-not $OutDir) { $OutDir = Join-Path $repoRoot 'dist' }
if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Path $OutDir -Force | Out-Null }

# --- choose cargo target dir ---
# MAX_PATH constraint (Windows 260-char limit): the `release-native` profile name lengthens the
# deepest build-script output paths past 260 chars when the crate's default external target-dir is
# used, and link.exe (which ignores the registry LongPathsEnabled opt-in) then fails with LNK1104.
# To keep the release-native build reliable, build into a SHORT target dir. Resolution order:
#   1. explicit -CargoTarget
#   2. HANDSHAKE_RN_TARGET env var (operator override)
#   3. a short default: <DriveRoot>\hsk-rn-target  (e.g. D:\hsk-rn-target)
# This is disk-agnostic: the drive is taken from the repo root, not hardcoded.
if (-not $CargoTarget) {
    if ($env:HANDSHAKE_RN_TARGET) {
        $CargoTarget = $env:HANDSHAKE_RN_TARGET
    } else {
        $driveRoot = [System.IO.Path]::GetPathRoot($repoRoot)   # e.g. "D:\"
        $CargoTarget = Join-Path $driveRoot 'hsk-rn-target'
    }
}
if (-not (Test-Path $CargoTarget)) { New-Item -ItemType Directory -Path $CargoTarget -Force | Out-Null }
$env:CARGO_TARGET_DIR = $CargoTarget
Write-Host "Cargo target dir (short, MAX_PATH-safe): $CargoTarget"

if ($LtoFat) { $env:HANDSHAKE_LTO = '1' }

# --- build the two release-native binaries ---
Write-Host 'Building handshake-native (release-native)...'
Push-Location $nativeCrate
try {
    cargo build --profile release-native -p handshake-native
    if ($LASTEXITCODE -ne 0) { throw "cargo build of handshake-native failed (exit $LASTEXITCODE)" }
} finally {
    Pop-Location
}

# handshake_core lives at src/backend/handshake_core and is built from its own crate dir. MT-004
# does not modify handshake_core; the installer only PACKAGES its release-native binary. If the core
# binary is not present, fail clearly rather than producing a half-populated MSI.
$nativeExe = Join-Path $CargoTarget 'release-native\handshake-native.exe'
$coreExe   = Join-Path $CargoTarget 'release-native\handshake_core.exe'

if (-not (Test-Path $nativeExe)) {
    throw "handshake-native.exe not found at $nativeExe after build. Aborting installer build."
}
if (-not (Test-Path $coreExe)) {
    throw @"
handshake_core.exe not found at $coreExe.
Build it first from its crate dir, e.g.:
  cargo build --profile release-native -p handshake_core --features app-runtime
(MT-004 does not build handshake_core itself; it only packages an existing release-native binary.)
"@
}

$env:HANDSHAKE_NATIVE_EXE = $nativeExe
$env:HANDSHAKE_CORE_EXE   = $coreExe

# --- WiX availability gate (CONTROL-4) ---
$wix = Get-Command wix -ErrorAction SilentlyContinue
if (-not $wix) {
    throw 'WiX 4 toolchain not found. Install from https://wixtoolset.org/docs/intro/ (dotnet tool install --global wix). Installer build skipped (Cargo build already succeeded).'
}

# --- build the MSI ---
$msi = Join-Path $OutDir 'handshake-native-setup.msi'
Write-Host "Building MSI -> $msi"
wix build $wxs -o $msi
if ($LASTEXITCODE -ne 0) { throw "wix build failed (exit $LASTEXITCODE)" }

if (-not (Test-Path $msi) -or (Get-Item $msi).Length -eq 0) {
    throw "MSI was not produced or is empty: $msi"
}
Write-Host "OK: produced $msi ($((Get-Item $msi).Length) bytes)"
