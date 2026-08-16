#Requires -Version 7.0
<#
  WP-KERNEL-011 MT-031 — single-installer build + package script (PowerShell 7).

  WHAT IT DOES
    1. Builds the native shell, the handshake_core backend, and the Palmistry watcher:
         cargo build --profile release-native --bin handshake-native
         cargo build --release --features app-runtime --bin handshake_core
         cargo build --release --bin palmistry
       into the project-allocated external Handshake_Artifacts root.
    2. Stages the three product binaries + bundled assets (fonts and grammars) into
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
    SurrealDB is linked into handshake_core as an in-process embedded engine. No database executable,
    service, discovery variable, or placeholder binary is staged.
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

# --- 1. Build the product binaries -----------------------------------------------------------------
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

$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $CrateRoot '../../..'))
$CoreManifest = Join-Path $RepoRoot 'src/backend/handshake_core/Cargo.toml'
$PalmistryManifest = Join-Path $RepoRoot 'src/frontend/palmistry/Cargo.toml'

Write-Step "cargo build --release --features app-runtime --bin handshake_core"
& cargo build --manifest-path $CoreManifest --release --features app-runtime --bin handshake_core
if ($LASTEXITCODE -ne 0) { throw "handshake_core cargo build failed (exit $LASTEXITCODE)" }

Write-Step "cargo build --release --bin palmistry"
& cargo build --manifest-path $PalmistryManifest --release --bin palmistry
if ($LASTEXITCODE -ne 0) { throw "palmistry cargo build failed (exit $LASTEXITCODE)" }

$CoreExeName = 'handshake_core.exe'
$PalmistryExeName = 'palmistry.exe'
$CoreExePath = Join-Path (Join-Path $ShortTargetDir 'release') $CoreExeName
$PalmistryExePath = Join-Path (Join-Path $ShortTargetDir 'release') $PalmistryExeName
foreach ($binary in @($CoreExePath, $PalmistryExePath)) {
    if (-not (Test-Path $binary -PathType Leaf)) {
        throw "Built binary not found at $binary after cargo build"
    }
}

# --- 2. Stage the bundle (exe-relative layout) -----------------------------------------------------
$StagingDir = Join-Path (Join-Path $ShortTargetDir 'release-native') 'staging'
if (Test-Path $StagingDir) { Remove-Item -Recurse -Force $StagingDir }
New-Item -ItemType Directory -Force -Path $StagingDir | Out-Null

# 2a. product binaries. SurrealDB is embedded in handshake_core; no database executable is copied.
Copy-Item $ExePath (Join-Path $StagingDir $ExeName) -Force
Copy-Item $CoreExePath (Join-Path $StagingDir $CoreExeName) -Force
Copy-Item $PalmistryExePath (Join-Path $StagingDir $PalmistryExeName) -Force

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

# 2d. exact upstream SurrealDB license notices
$LicenseSrc = Join-Path (Join-Path $CrateRoot 'installer/windows') 'licenses'
$LicenseDst = Join-Path $StagingDir 'licenses'
New-Item -ItemType Directory -Force -Path $LicenseDst | Out-Null
foreach ($notice in @('SurrealDB-3.0-BUSL-1.1.txt', 'SurrealDB-Protocol-2.0-BUSL-1.1.txt')) {
    $source = Join-Path $LicenseSrc $notice
    if (-not (Test-Path $source -PathType Leaf)) {
        throw "Required SurrealDB license notice missing: $source"
    }
    Copy-Item $source (Join-Path $LicenseDst $notice) -Force
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
