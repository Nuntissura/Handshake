#Requires -Version 7.0
<#
  WP-KERNEL-011 MT-031 — single-installer build + package script (PowerShell 7).

  WHAT IT DOES
    1. Builds the single self-contained native binary:
         cargo build --profile release-native --bin handshake-native
       into the project-allocated external Handshake_Artifacts root.
    2. Stages the binary + all bundled assets (fonts, grammars dir, postgres binaries) into
         <target>/release-native/staging/   matching the exe-relative bundle layout that
         installer::check_bundle_integrity verifies.
    3. Produces ONE installer artifact at <out>/handshake-setup.{msi|zip}:
         - WiX 4/5 MSI  if the `wix` (or `cargo wix`) toolchain is on PATH  [GATED];
         - else a self-contained .zip fallback (always available via Compress-Archive).
       It NEVER fakes an .msi when WiX is absent — the zip is a real single artifact.
    4. Exports HANDSHAKE_INSTALLER_ARTIFACT (process + GITHUB_ENV when present) and prints a final
       line: "INSTALLER_ARTIFACT=<path> SIZE_BYTES=<n>".

  DISK-AGNOSTIC (AC-031-07 / GLOBAL-PORTABILITY-004): contains NO hardcoded absolute paths or drive
  letters. The canonical build target derives from $PSScriptRoot and remains inside the project-owned
  Handshake_Artifacts root after the worktree or project moves. An explicit override is accepted only
  when it also resolves inside that allocated root.

  PLACEMENT NOTE (DEVIATION): the MT-031 contract lists scripts/build_installer.ps1 at the repo root.
  This crate (src/frontend/handshake_native) is the build unit and the proof commands run from it, so
  the script lives at the crate's scripts/ dir — the same crate-relative placement decision MT-004/MT-029
  documented for tests and the .cargo config. $PSScriptRoot resolution keeps it disk-agnostic either way.

  PREREQUISITES (see installer/windows/BUNDLED_DEPS_POLICY.md):
    - Rust stable toolchain + cargo on PATH (required).
    - PowerShell 7 (required; this script).
    - WiX 4/5 (`dotnet tool install --global wix`) — OPTIONAL; absent => zip fallback.
    - PostgreSQL binaries to stage — OPTIONAL; absent => a documented placeholder is staged so the
      bundle layout is valid for the smoke (a real release MUST stage the real binaries).
#>

[CmdletBinding()]
param(
    # Override the release build target dir. It must remain inside Handshake_Artifacts.
    [string]$ShortTargetDir = $env:HANDSHAKE_SHORT_TARGET_DIR,
    # Force the zip fallback even if WiX is present (used by the smoke to stay deterministic).
    [switch]$ForceZip
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Write-Step([string]$msg) { Write-Host "[build_installer] $msg" }

# --- Resolve crate root (disk-agnostic: scripts/ is directly under the crate root) -----------------
$CrateRoot = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $CrateRoot 'Cargo.toml'))) {
    throw "Cannot locate crate Cargo.toml from PSScriptRoot=$PSScriptRoot (resolved CrateRoot=$CrateRoot)"
}
Write-Step "Crate root: $CrateRoot"

# --- Choose the project-allocated release CARGO_TARGET_DIR -----------------------------------------
$ArtifactRoot = [System.IO.Path]::GetFullPath(
    (Join-Path $CrateRoot '../../../../Handshake_Artifacts'))
$CanonicalReleaseTargetDir = Join-Path $ArtifactRoot 'handshake-release-target'
if ([string]::IsNullOrWhiteSpace($ShortTargetDir)) {
    $ShortTargetDir = $CanonicalReleaseTargetDir
}
$ShortTargetDir = [System.IO.Path]::GetFullPath($ShortTargetDir)
$artifactPrefix = $ArtifactRoot.TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
if (-not $ShortTargetDir.StartsWith($artifactPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "HANDSHAKE_SHORT_TARGET_DIR must stay inside the allocated artifact root $ArtifactRoot; got $ShortTargetDir"
}
New-Item -ItemType Directory -Force -Path $ShortTargetDir | Out-Null
Write-Step "Allocated release CARGO_TARGET_DIR: $ShortTargetDir"

# --- 1. Build the single self-contained binary -----------------------------------------------------
$env:CARGO_TARGET_DIR = $ShortTargetDir
Write-Step "cargo build --profile release-native --bin handshake-native"
Push-Location $CrateRoot
try {
    & cargo build --profile release-native --bin handshake-native
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }
}
finally {
    Pop-Location
}

$ExeName = 'handshake-native.exe'
$ExePath = Join-Path (Join-Path $ShortTargetDir 'release-native') $ExeName
if (-not (Test-Path $ExePath)) {
    throw "Built binary not found at $ExePath after cargo build"
}
$exeSize = (Get-Item $ExePath).Length
Write-Step "Built binary: $ExePath ($([math]::Round($exeSize/1MB,1)) MB)"

# --- 2. Stage the bundle (exe-relative layout) -----------------------------------------------------
$StagingDir = Join-Path (Join-Path $ShortTargetDir 'release-native') 'staging'
if (Test-Path $StagingDir) { Remove-Item -Recurse -Force $StagingDir }
New-Item -ItemType Directory -Force -Path $StagingDir | Out-Null

# 2a. the native binary
Copy-Item $ExePath (Join-Path $StagingDir $ExeName) -Force

# 2b. fonts/  (from the crate's assets/fonts)
$FontsSrc = Join-Path (Join-Path $CrateRoot 'assets') 'fonts'
$FontsDst = Join-Path $StagingDir 'fonts'
New-Item -ItemType Directory -Force -Path $FontsDst | Out-Null
if (Test-Path $FontsSrc) {
    Copy-Item (Join-Path $FontsSrc '*') $FontsDst -Recurse -Force
}
# -Include only filters when the path ends in a wildcard; use a join-path wildcard so it actually applies.
$fontCount = @(Get-ChildItem (Join-Path $FontsDst '*') -File -Include '*.ttf', '*.otf' -ErrorAction SilentlyContinue).Count
if ($fontCount -lt 1) { throw "No bundled fonts staged into $FontsDst (need >= 1 .ttf/.otf)" }
Write-Step "Staged $fontCount font file(s)"

# 2c. grammars/  (directory must exist; may be empty on first pass)
New-Item -ItemType Directory -Force -Path (Join-Path $StagingDir 'grammars') | Out-Null

# 2d. bundled/postgres/  — stage real binaries if a postgres toolchain is discoverable, else a
#     documented placeholder so the bundle LAYOUT is valid for the smoke. A real release MUST stage
#     the actual managed-postgres binaries (BUNDLED_DEPS_POLICY.md managed-postgres-path-contract).
$PgDst = Join-Path (Join-Path $StagingDir 'bundled') 'postgres'
New-Item -ItemType Directory -Force -Path $PgDst | Out-Null
$pgBinDir = $null
# Discover via the SAME env vars handshake_core::managed_postgres reads, in its order:
# HANDSHAKE_MANAGED_PG_BIN (managed_postgres::MANAGED_PG_BIN_ENV) then the standard PGBIN.
foreach ($cand in @($env:HANDSHAKE_MANAGED_PG_BIN, $env:PGBIN)) {
    if (-not [string]::IsNullOrWhiteSpace($cand) -and (Test-Path (Join-Path $cand 'pg_ctl.exe'))) {
        $pgBinDir = $cand; break
    }
}
if (-not $pgBinDir) {
    $pgCtlCmd = Get-Command 'pg_ctl.exe' -ErrorAction SilentlyContinue
    if ($pgCtlCmd) { $pgBinDir = Split-Path -Parent $pgCtlCmd.Source }
}
if ($pgBinDir) {
    Write-Step "Staging real PostgreSQL binaries from $pgBinDir"
    Copy-Item (Join-Path $pgBinDir '*') $PgDst -Recurse -Force
}
else {
    Write-Step "WARNING: no PostgreSQL toolchain found (HANDSHAKE_MANAGED_PG_BIN/PGBIN/PATH). Staging placeholder pg_ctl.exe."
    Write-Step "         A production installer MUST stage the real managed-postgres binaries here."
    $placeholder = @(
        '@echo off',
        'REM MT-031 placeholder pg_ctl: bundle-layout placeholder only; replace with the real',
        'REM managed-postgres binaries for a production installer. See BUNDLED_DEPS_POLICY.md.',
        'echo handshake-bundled-postgres-placeholder & exit /b 0'
    ) -join "`r`n"
    Set-Content -Path (Join-Path $PgDst 'pg_ctl.exe') -Value $placeholder -Encoding ascii
}

# --- 3. Produce the single installer artifact ------------------------------------------------------
$OutDir = Join-Path (Join-Path $ShortTargetDir 'release-native') 'installer'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function Test-WixAvailable {
    if (Get-Command 'wix' -ErrorAction SilentlyContinue) { return $true }
    # cargo wix subcommand
    & cargo wix --version *> $null
    return ($LASTEXITCODE -eq 0)
}

$Artifact = $null
$wxs = Join-Path (Join-Path (Join-Path $CrateRoot 'installer') 'windows') 'handshake_native.wxs'

if (-not $ForceZip -and (Test-WixAvailable)) {
    Write-Step "WiX toolchain detected -> building MSI"
    $Artifact = Join-Path $OutDir 'handshake-setup.msi'
    $productVersion = '0.1.0'
    & wix build $wxs `
        -d "StagingDir=$StagingDir" `
        -d "ProductVersion=$productVersion" `
        -arch x64 `
        -ext WixToolset.Util.wixext `
        -o $Artifact
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path $Artifact)) {
        throw "wix build failed (exit $LASTEXITCODE); MSI not produced at $Artifact"
    }
}
else {
    if ($ForceZip) {
        Write-Step "ForceZip set -> producing zip fallback artifact"
    }
    else {
        Write-Step "WiX toolchain NOT available on this host -> producing zip fallback (single self-contained artifact)"
    }
    $Artifact = Join-Path $OutDir 'handshake-setup.zip'
    if (Test-Path $Artifact) { Remove-Item -Force $Artifact }
    Compress-Archive -Path (Join-Path $StagingDir '*') -DestinationPath $Artifact -CompressionLevel Optimal
    if (-not (Test-Path $Artifact)) { throw "Compress-Archive did not produce $Artifact" }
}

$size = (Get-Item $Artifact).Length

# --- 4. Export + final line ------------------------------------------------------------------------
$env:HANDSHAKE_INSTALLER_ARTIFACT = $Artifact
if ($env:GITHUB_ENV) { "HANDSHAKE_INSTALLER_ARTIFACT=$Artifact" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8 }

Write-Step "Staging dir: $StagingDir"
Write-Host "INSTALLER_ARTIFACT=$Artifact SIZE_BYTES=$size"
exit 0
