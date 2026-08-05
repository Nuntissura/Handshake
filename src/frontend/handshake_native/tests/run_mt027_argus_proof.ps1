[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[A-Za-z0-9_-]+$')]
    [string]$RunId,

    [ValidateRange(60, 1800)]
    [int]$TimeoutSeconds = 1200
)

$ErrorActionPreference = 'Stop'
$crateRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$repoRoot = (& git -C $crateRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw 'Unable to resolve the product repository root for MT-027 source binding'
}
$sourceSha = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceSha -notmatch '^[0-9a-f]{40}$') {
    throw "Unable to resolve an exact committed source SHA; got '$sourceSha'"
}
# ── MT-027 V5 cleanliness gate (validation_v4 remediation item 2) ─────────────────────────────────
#
# The previous gate treated the ENTIRE repository as MT-027 compiled/configured input and therefore
# rejected a fresh run over an unrelated pre-existing formatting diff in another test binary. It is now
# narrowed to the EXACT MT-027 input set, and the narrowing is conservative: everything is gated by
# DEFAULT, and the ONLY excludable rows are other test binaries' top-level `.rs` files under
# `src/frontend/handshake_native/tests/`. Each Rust integration test compiles into its OWN separate test
# binary, so such a file provably cannot enter the artifacts of `test_block_collection_view` — while the
# lib, this binary's own source, every shared `#[path]` helper module it declares, the manifests and
# lockfiles, and the whole managed backend crate all REMAIN gated and still hard-fail when dirty.
# Every excluded row is recorded with its HEAD and worktree blob hashes in the external process receipt
# so a reviewer can confirm exactly what was excluded and that none of it is a compiled input.
$mt027TestFileName = 'test_block_collection_view.rs'
$mt027TestsDirPrefix = 'src/frontend/handshake_native/tests/'

function Get-Mt027StatusRows {
    $rows = @(& git -C $repoRoot status --porcelain --untracked-files=all -- `
            '.' `
            ':(exclude)AGENTS.md' `
            ':(exclude)CLAUDE.md')
    if ($LASTEXITCODE -ne 0) {
        throw 'Unable to inspect MT-027 relevant source cleanliness'
    }
    return @($rows | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}

function Get-Mt027StatusPath {
    param([Parameter(Mandatory = $true)][string]$Row)

    # porcelain v1: 'XY <path>' or 'XY <old> -> <new>'. Quoted paths keep their quotes; we only need a
    # stable comparable path, and an unexpected shape must stay GATED rather than silently excluded.
    $path = $Row.Substring(3).Trim()
    $arrow = $path.IndexOf(' -> ', [StringComparison]::Ordinal)
    if ($arrow -ge 0) {
        $path = $path.Substring($arrow + 4).Trim()
    }
    return $path.Trim('"').Replace('\', '/')
}

function Test-Mt027ExcludableRow {
    param([Parameter(Mandatory = $true)][string]$Path)

    if ($Path.Contains('"')) {
        return $false
    }
    if (-not $Path.StartsWith($mt027TestsDirPrefix, [StringComparison]::Ordinal)) {
        return $false
    }
    $relative = $Path.Substring($mt027TestsDirPrefix.Length)
    if ($relative.Contains('/')) {
        # A shared helper module directory (native_gui_support/, interconnect_support/,
        # pg_proof_support/, fixtures/, ...) IS compiled/consumed by this binary. Stay gated.
        return $false
    }
    if (-not $relative.EndsWith('.rs', [StringComparison]::OrdinalIgnoreCase)) {
        return $false
    }
    return -not $relative.Equals($mt027TestFileName, [StringComparison]::OrdinalIgnoreCase)
}

function Get-Mt027BlobHash {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][ValidateSet('head', 'worktree')][string]$Source
    )

    if ($Source -eq 'head') {
        # An untracked exclusion has no HEAD blob; a non-zero `rev-parse` is expected there and must not
        # abort the gate (PS7 promotes native stderr to a terminating error under ErrorActionPreference
        # Stop, so the lookup is explicitly contained).
        $hash = $null
        try {
            $hash = (& git -C $repoRoot rev-parse --quiet --verify "HEAD:$Path" 2>$null)
        } catch {
            $hash = $null
        }
        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace([string]$hash)) {
            $global:LASTEXITCODE = 0
            return 'absent-at-head'
        }
        return ([string]$hash).Trim()
    }
    $full = Join-Path $repoRoot $Path
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
        return 'absent-in-worktree'
    }
    $hash = (& git -C $repoRoot hash-object -- $Path)
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to hash excluded MT-027 row '$Path'"
    }
    return ([string]$hash).Trim()
}

function Assert-Mt027Cleanliness {
    param([Parameter(Mandatory = $true)][string]$Phase)

    $gated = @()
    $excluded = @()
    foreach ($row in Get-Mt027StatusRows) {
        $path = Get-Mt027StatusPath -Row $row
        if (Test-Mt027ExcludableRow -Path $path) {
            $excluded += [ordered]@{
                path = $path
                status = $row.Substring(0, 2)
                reason = 'separate_test_binary_top_level_source_not_linked_into_test_block_collection_view'
                head_blob_sha1 = Get-Mt027BlobHash -Path $path -Source 'head'
                worktree_blob_sha1 = Get-Mt027BlobHash -Path $path -Source 'worktree'
            }
        } else {
            $gated += $row
        }
    }
    if ($gated.Count -ne 0) {
        throw "MT-027 proof requires every compiled/configured input to match committed HEAD ($Phase); dirty gated rows: $($gated -join '; ')"
    }
    return , @($excluded)
}

$excludedDirtyRows = Assert-Mt027Cleanliness -Phase 'preflight'

$artifactSibling = [IO.Path]::GetFullPath((Join-Path $crateRoot '..\..\..\..\Handshake_Artifacts'))
# HBR-SWARM-005 / CX-984: CARGO_TARGET_DIR MUST be a PER-OWNER subdirectory, never the shared root. The
# shared root previously let another worktree's `handshake_core.exe` be picked up as this proof's
# "current-source" backend, which is exactly the provenance failure the rule exists to prevent.
$cargoTarget = [IO.Path]::GetFullPath(
    (Join-Path $artifactSibling 'handshake-cargo-target\wp012-mt027'))
$proofRoot = [IO.Path]::GetFullPath(
    (Join-Path $artifactSibling 'handshake-test\wp-kernel-012-mt-027\integrated'))
$backendBinary = [IO.Path]::GetFullPath((Join-Path $cargoTarget 'debug\handshake_core.exe'))
# HBR-SWARM-005 / CX-984: a proof run needs a WP-SCOPED database. The shared `handshake` database
# carries another worktree's applied migration set, so a divergent migration in this worktree fails
# sqlx checksum validation there. The database identity is bound into the receipt below.
$postgresDatabase = 'handshake_wp_kernel_012_mt_027'
$postgresDsn = "postgresql://postgres@127.0.0.1:5544/$postgresDatabase"
$expectedArtifactRoot = [IO.Path]::GetFullPath(
    (Join-Path ([IO.Directory]::GetParent($repoRoot).FullName) 'Handshake_Artifacts'))
if (-not $artifactSibling.Equals($expectedArtifactRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw "MT-027 artifacts must use the existing sibling Handshake_Artifacts root; resolved '$artifactSibling'"
}
if (-not (Test-Path -LiteralPath $artifactSibling -PathType Container)) {
    throw "The existing sibling Handshake_Artifacts root is unavailable: '$artifactSibling'"
}
$postgresListener = Get-NetTCPConnection -LocalAddress '127.0.0.1' -LocalPort 5544 `
    -State Listen -ErrorAction Stop | Select-Object -First 1
$postgresProcess = Get-Process -Id $postgresListener.OwningProcess -ErrorAction Stop
if (-not $postgresProcess.ProcessName.Equals(
        'postgres', [StringComparison]::OrdinalIgnoreCase)) {
    throw "Port 5544 is not owned by PostgreSQL: pid=$($postgresListener.OwningProcess), process=$($postgresProcess.ProcessName)"
}
$initialPostgresPid = [int]$postgresProcess.Id
$initialPostgresStartTime = [DateTimeOffset]$postgresProcess.StartTime

function Assert-NoReparsePointEscape {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Root
    )

    $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd('\')
    $cursor = [IO.Path]::GetFullPath($Path)
    while ($cursor.Length -ge $rootFull.Length) {
        if (Test-Path -LiteralPath $cursor) {
            $item = Get-Item -Force -LiteralPath $cursor
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "MT-027 proof paths must not traverse a reparse point: '$cursor'"
            }
        }
        if ($cursor.Equals($rootFull, [StringComparison]::OrdinalIgnoreCase)) {
            return
        }
        $parent = [IO.Directory]::GetParent($cursor)
        if ($null -eq $parent) {
            break
        }
        $cursor = $parent.FullName
    }
    throw "MT-027 proof path escaped the existing sibling Handshake_Artifacts root: '$Path'"
}

function Get-ComparableWindowsPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    $fullPath = [IO.Path]::GetFullPath($Path)
    $extendedUncPrefix = '\\?\UNC\'
    if ($fullPath.StartsWith($extendedUncPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        return [IO.Path]::GetFullPath(
            '\\' + $fullPath.Substring($extendedUncPrefix.Length))
    }
    $extendedPrefix = '\\?\'
    if ($fullPath.StartsWith($extendedPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        return [IO.Path]::GetFullPath($fullPath.Substring($extendedPrefix.Length))
    }
    return $fullPath
}

Assert-NoReparsePointEscape -Path $cargoTarget -Root $artifactSibling
Assert-NoReparsePointEscape -Path $proofRoot -Root $artifactSibling
$runDir = Join-Path $proofRoot $RunId
if (Test-Path -LiteralPath $runDir) {
    throw "RunId '$RunId' is not fresh: '$runDir' already exists"
}
$fixedManagedPgReceipt = Join-Path (
    [IO.Directory]::GetParent($proofRoot).FullName) 'managed-pg-receipt.json'
if (Test-Path -LiteralPath $fixedManagedPgReceipt) {
    throw "Fresh MT-027 proof requires the prior fixed receipt to be cleaned first: '$fixedManagedPgReceipt'"
}
New-Item -ItemType Directory -Force -Path $proofRoot | Out-Null
New-Item -ItemType Directory -Path $runDir | Out-Null
$stageBindingRoot = Join-Path $runDir 'stage-binding'
$argusBindingRoot = Join-Path $runDir 'argus-binding'
# ── Owned-backend runtime root: deliberately SHALLOW (MAX_PATH), still run-scoped and cleaned ──────
#
# The fixture derives the owned backend's runtime directory from HANDSHAKE_ARTIFACTS_ROOT by appending
# a FIXED 105-character suffix (`\wp-kernel-012\backend-runtime\r-<16>\s-<16>\<36-char uuid>`), and the
# backend then opens `<runtime>\data\flight_recorder.db` through DuckDB. DuckDB is a C++ dependency: it
# does not use Windows extended-length `\\?\` paths, so that file name is hard-bound by MAX_PATH (260).
# Nesting the runtime root inside `integrated\<RunId>\fixture-artifacts` pushed the DuckDB file to 272
# characters and the backend exited 1 with `Cannot open file ... flight_recorder.db` before publishing
# its listen report — for ANY RunId, because the fixed overhead alone exceeds the budget.
#
# The runtime root therefore lives directly under the artifact sibling in a short run-scoped directory.
# Containment is unchanged in strength: it stays inside the approved Handshake_Artifacts root, its
# terminal component is exactly the RunId, it is reparse-point checked, and it is cleaned on both the
# success and failure paths exactly like the other transient roots.
$runtimeArtifactsParent = Join-Path $artifactSibling 'handshake-test\mt027-rt'
New-Item -ItemType Directory -Force -Path $runtimeArtifactsParent | Out-Null
$runtimeArtifactsRoot = Join-Path $runtimeArtifactsParent $RunId
if (Test-Path -LiteralPath $runtimeArtifactsRoot) {
    throw "RunId '$RunId' is not fresh: backend runtime root '$runtimeArtifactsRoot' already exists"
}
foreach ($containedRoot in @($stageBindingRoot, $runtimeArtifactsRoot, $argusBindingRoot)) {
    New-Item -ItemType Directory -Path $containedRoot | Out-Null
    Assert-NoReparsePointEscape -Path $containedRoot -Root $artifactSibling
}
# The DuckDB flight-recorder file is the deepest path the owned backend opens without `\\?\` support.
$backendRuntimeSuffixLength =
    '\wp-kernel-012\backend-runtime\r-'.Length + 16 + '\s-'.Length + 16 + 1 + 36
$duckDbSuffixLength = '\data\flight_recorder.db'.Length
$projectedRecorderPathLength =
    $runtimeArtifactsRoot.Length + $backendRuntimeSuffixLength + $duckDbSuffixLength
$maxNonExtendedPathLength = 259
if ($projectedRecorderPathLength -gt $maxNonExtendedPathLength) {
    $overflow = $projectedRecorderPathLength - $maxNonExtendedPathLength
    $maxRunId = $RunId.Length - $overflow
    throw ("MT-027 proof RunId '$RunId' makes the owned backend DuckDB flight-recorder path " +
        "$projectedRecorderPathLength characters, over the $maxNonExtendedPathLength-character " +
        "MAX_PATH budget DuckDB is bound by; use a RunId of at most $maxRunId characters")
}

# A transient proof root is cleanable only when it is EXACTLY one of the run-scoped directories this
# invocation created: inside the exact run directory, or the short backend runtime root whose terminal
# component is exactly this RunId under the approved Handshake_Artifacts sibling. Anything else is
# refused, so cleanup can never walk outside the paths this run owns.
function Test-CleanableTransientRoot {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$RunDirectory,
        [Parameter(Mandatory = $true)][string]$RuntimeParent,
        [Parameter(Mandatory = $true)][string]$RunIdentifier
    )

    $resolved = [IO.Path]::GetFullPath($Path)
    if ($resolved.StartsWith(
            [IO.Path]::GetFullPath($RunDirectory) + [IO.Path]::DirectorySeparatorChar,
            [StringComparison]::OrdinalIgnoreCase)) {
        return $true
    }
    $expectedRuntimeRoot = [IO.Path]::GetFullPath((Join-Path $RuntimeParent $RunIdentifier))
    return $resolved.Equals($expectedRuntimeRoot, [StringComparison]::OrdinalIgnoreCase) -and
        [IO.Path]::GetFileName($resolved).Equals($RunIdentifier, [StringComparison]::Ordinal)
}

function Format-CanonicalUtc {
    param([Parameter(Mandatory = $true)]$Value)

    return ([DateTimeOffset]$Value).ToUniversalTime().ToString(
        "yyyy-MM-dd'T'HH:mm:ss.fffffff'Z'",
        [Globalization.CultureInfo]::InvariantCulture)
}
$supervisorStartedAtUtc = Format-CanonicalUtc ([DateTimeOffset]::UtcNow)

function Get-ProcessStartUtc {
    param([Parameter(Mandatory = $true)]$Row)

    if ($null -eq $Row.CreationDate) {
        return ''
    }
    if ($Row.CreationDate -is [DateTime]) {
        return Format-CanonicalUtc ([DateTime]$Row.CreationDate)
    }
    return Format-CanonicalUtc (
        [Management.ManagementDateTimeConverter]::ToDateTime([string]$Row.CreationDate))
}

function Get-Identity {
    param([Parameter(Mandatory = $true)][int]$TargetPid)

    $row = Get-CimInstance Win32_Process -Filter "ProcessId = $TargetPid" |
        Select-Object -First 1
    if ($null -eq $row) {
        return $null
    }
    return [pscustomobject]@{
        pid = [int]$row.ProcessId
        parent_pid = [int]$row.ParentProcessId
        start_time_utc = Get-ProcessStartUtc $row
        executable = if ([string]::IsNullOrWhiteSpace([string]$row.ExecutablePath)) {
            [string]$row.Name
        } else {
            [string]$row.ExecutablePath
        }
    }
}

function Get-Descendants {
    param(
        [Parameter(Mandatory = $true)][int]$RootPid,
        [Parameter(Mandatory = $true)][string]$RootStartUtc
    )

    $rows = @(Get-CimInstance Win32_Process)
    $root = $rows | Where-Object { [int]$_.ProcessId -eq $RootPid } | Select-Object -First 1
    if ($null -eq $root) {
        return @()
    }
    if ((Get-ProcessStartUtc $root) -ne $RootStartUtc) {
        throw "Supervised root PID $RootPid changed identity"
    }
    $owned = New-Object 'System.Collections.Generic.HashSet[int]'
    $ownedStartUtc = @{}
    [void]$owned.Add($RootPid)
    $ownedStartUtc[$RootPid] = $RootStartUtc
    do {
        $added = $false
        foreach ($row in $rows) {
            $candidate = [int]$row.ProcessId
            $parentPid = [int]$row.ParentProcessId
            if ($owned.Contains($parentPid) -and -not $owned.Contains($candidate)) {
                $candidateStartUtc = Get-ProcessStartUtc $row
                if ([string]::IsNullOrWhiteSpace($candidateStartUtc)) {
                    throw "Owned process candidate PID $candidate has no verifiable start identity"
                }
                $parentStarted = [DateTimeOffset]::Parse(
                    [string]$ownedStartUtc[$parentPid],
                    [Globalization.CultureInfo]::InvariantCulture,
                    [Globalization.DateTimeStyles]::AssumeUniversal)
                $candidateStarted = [DateTimeOffset]::Parse(
                    $candidateStartUtc,
                    [Globalization.CultureInfo]::InvariantCulture,
                    [Globalization.DateTimeStyles]::AssumeUniversal)
                if ($candidateStarted -lt $parentStarted) {
                    continue
                }
                [void]$owned.Add($candidate)
                $ownedStartUtc[$candidate] = $candidateStartUtc
                $added = $true
            }
        }
    } while ($added)
    return @($rows | Where-Object { $owned.Contains([int]$_.ProcessId) } | ForEach-Object {
            [pscustomobject]@{
                pid = [int]$_.ProcessId
                parent_pid = [int]$_.ParentProcessId
                start_time_utc = Get-ProcessStartUtc $_
                executable = if ([string]::IsNullOrWhiteSpace([string]$_.ExecutablePath)) {
                    [string]$_.Name
                } else {
                    [string]$_.ExecutablePath
                }
            }
        })
}

$scenario = 'wp-kernel-012-mt-027-block-collections'
$correlationId = "cargo-$scenario-$([guid]::NewGuid().ToString('N'))"
$stdoutPath = Join-Path $runDir "$scenario.stdout.log"
$stderrPath = Join-Path $runDir "$scenario.stderr.log"
$exitCodePath = Join-Path $runDir "$correlationId.exit-code"
$tracePath = Join-Path $runDir 'canonical-argus-matrix.jsonl'
$receiptPath = Join-Path $runDir 'external-process-receipt.json'
$cargoArguments = @(
    'test',
    '--features', 'integration',
    '--test', 'test_block_collection_view',
    'block_collection_views_live_pg_self_seed_full_round_trip',
    '-j', '6',
    '--',
    '--exact',
    '--nocapture'
)
$backendBuildArguments = @(
    'build',
    '--manifest-path', (Join-Path $repoRoot 'src\backend\handshake_core\Cargo.toml'),
    '--features', 'app-runtime,duckdb-flight-recorder',
    '--bin', 'handshake_core',
    '-j', '6'
)

$environmentNames = @(
    'CARGO_TARGET_DIR',
    'HSK_TEST_BACKEND_BIN',
    'HANDSHAKE_TEST_PG_DSN',
    'HANDSHAKE_TEST_STAGE_BINDING_ROOT',
    'HANDSHAKE_ARTIFACTS_ROOT',
    'HANDSHAKE_ARGUS_BINDING_ROOT',
    'HSK_TEST_BASE',
    'HANDSHAKE_PROOF_ARTIFACT_DIR',
    'HANDSHAKE_ARGUS_MATRIX_RUN_ID',
    'HANDSHAKE_ARGUS_MATRIX_SCENARIO_ID',
    'HANDSHAKE_ARGUS_MATRIX_SURFACE',
    'HANDSHAKE_ARGUS_MATRIX_EDGE_STATE',
    'HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA',
    'HANDSHAKE_PROOF_PROCESS_CORRELATION_ID',
    'HANDSHAKE_PROOF_ACTION_RECEIPT_ID'
)
$environmentSnapshot = @{}
foreach ($environmentName in $environmentNames) {
    $environmentSnapshot[$environmentName] =
        [Environment]::GetEnvironmentVariable($environmentName, 'Process')
}
$proofAccepted = $false
$primaryFailure = $null
$process = $null
$rootIdentity = $null
$owned = @()
$managedReceiptDestination = Join-Path $runDir 'managed-pg-receipt.json'

try {
    $env:CARGO_TARGET_DIR = $cargoTarget
    $env:HSK_TEST_BACKEND_BIN = $backendBinary
    $env:HANDSHAKE_TEST_PG_DSN = $postgresDsn
    $env:HANDSHAKE_TEST_STAGE_BINDING_ROOT = $stageBindingRoot
    $env:HANDSHAKE_ARTIFACTS_ROOT = $runtimeArtifactsRoot
    $env:HANDSHAKE_ARGUS_BINDING_ROOT = $argusBindingRoot
    [Environment]::SetEnvironmentVariable('HSK_TEST_BASE', $null, 'Process')
    $env:HANDSHAKE_PROOF_ARTIFACT_DIR = $proofRoot
    $env:HANDSHAKE_ARGUS_MATRIX_RUN_ID = $RunId
    $env:HANDSHAKE_ARGUS_MATRIX_SCENARIO_ID = $scenario
    $env:HANDSHAKE_ARGUS_MATRIX_SURFACE = 'Block Collection Views'
    $env:HANDSHAKE_ARGUS_MATRIX_EDGE_STATE = 'create-mutate-switch-empty-error-retry'
    $env:HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA = $sourceSha
    $env:HANDSHAKE_PROOF_PROCESS_CORRELATION_ID = $correlationId

    $cargo = Get-Command cargo -CommandType Application -ErrorAction Stop
    $wrapperSpec = [ordered]@{
    cargo = $cargo.Source
    commands = @(
        [ordered]@{ cwd = $repoRoot; arguments = $backendBuildArguments },
        [ordered]@{ cwd = $crateRoot; arguments = $cargoArguments }
    )
    exit_code_path = $exitCodePath
    } | ConvertTo-Json -Compress -Depth 4
    $encodedSpec = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($wrapperSpec))
    $wrapper = @'
$spec = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('__SPEC__')) | ConvertFrom-Json
$code = 0
foreach ($command in @($spec.commands)) {
    Set-Location -LiteralPath ([string]$command.cwd)
    & ([string]$spec.cargo) @($command.arguments)
    $code = if ($null -eq $LASTEXITCODE) { 9009 } else { [int]$LASTEXITCODE }
    if ($code -ne 0) { break }
}
[IO.File]::WriteAllText([string]$spec.exit_code_path, [string]$code)
exit $code
'@ -replace '__SPEC__', $encodedSpec
    $encodedWrapper = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($wrapper))
    $process = Start-Process -FilePath 'powershell.exe' `
    -ArgumentList @('-NoProfile', '-NonInteractive', '-EncodedCommand', $encodedWrapper) `
    -WorkingDirectory $crateRoot -WindowStyle Hidden -PassThru `
    -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
    try {
        $rootIdentity = Get-Identity -TargetPid $process.Id
    } catch {
        try {
            if (-not $process.HasExited) {
                $process.Kill($true)
            }
            [void]$process.WaitForExit(10000)
        } catch {}
        throw "Unable to query the supervised process identity for PID $($process.Id): $($_.Exception.Message)"
    }
if ($null -eq $rootIdentity -or [string]::IsNullOrWhiteSpace($rootIdentity.start_time_utc)) {
    try {
        if (-not $process.HasExited) {
            $process.Kill($true)
        }
        if (-not $process.WaitForExit(10000)) {
            throw 'bounded wrapper reclamation did not complete'
        }
    } catch {
        throw "Unable to bind or reclaim the supervised process identity for PID $($process.Id): $($_.Exception.Message)"
    }
    throw "Unable to bind the supervised process identity for PID $($process.Id)"
}
$startedAt = [DateTimeOffset]::Parse($rootIdentity.start_time_utc)
$deadline = $startedAt.AddSeconds($TimeoutSeconds)
$owned = @($rootIdentity)
$testIdentities = @()
$backendIdentities = @()
$exited = $false
while ([DateTimeOffset]::UtcNow -lt $deadline) {
    foreach ($identity in @(Get-Descendants -RootPid $process.Id -RootStartUtc $rootIdentity.start_time_utc)) {
        if (@($owned | Where-Object {
                    $_.pid -eq $identity.pid -and $_.start_time_utc -eq $identity.start_time_utc
                }).Count -eq 0) {
            $owned += $identity
        }
        if ([IO.Path]::GetFileNameWithoutExtension([string]$identity.executable) -like
            'test_block_collection_view*') {
            if (@($testIdentities | Where-Object {
                        $_.pid -eq $identity.pid -and $_.start_time_utc -eq $identity.start_time_utc
                    }).Count -eq 0) {
                $testIdentities += $identity
            }
        }
        if ([IO.Path]::GetFileNameWithoutExtension([string]$identity.executable) -eq
            'handshake_core') {
            if (@($backendIdentities | Where-Object {
                        $_.pid -eq $identity.pid -and $_.start_time_utc -eq $identity.start_time_utc
                    }).Count -eq 0) {
                $backendIdentities += $identity
            }
        }
    }
    if ($process.WaitForExit(100)) {
        $exited = $true
        break
    }
}
if (-not $exited) {
    $killError = $null
    try {
        $process.Kill($true)
    } catch {
        $killError = $_.Exception.Message
    }
    $reclaimDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
    do {
        $process.WaitForExit(250) | Out-Null
        $survivors = @()
        foreach ($identity in $owned) {
            $current = Get-Identity -TargetPid $identity.pid
            if ($null -ne $current -and $current.start_time_utc -eq $identity.start_time_utc) {
                $survivors += $current
                try { (Get-Process -Id $identity.pid -ErrorAction Stop).Kill() } catch {}
            }
        }
        if ($survivors.Count -eq 0) { break }
        Start-Sleep -Milliseconds 100
    } while ([DateTimeOffset]::UtcNow -lt $reclaimDeadline)
    if ($survivors.Count -ne 0 -or -not $process.HasExited) {
        throw "MT-027 proof timed out and exact PID/start reclamation failed; kill_error='$killError'; survivors=$(($survivors | ForEach-Object { \"$($_.pid)/$($_.start_time_utc)\" }) -join ',')"
    }
    throw "MT-027 proof exceeded the hard ${TimeoutSeconds}s bound; exact observed PID/start identities were reclaimed"
}
if (-not (Test-Path -LiteralPath $exitCodePath -PathType Leaf)) {
    throw "The supervised Cargo wrapper exited without an exit-code sidecar: '$exitCodePath'"
}
$exitCode = [int](Get-Content -LiteralPath $exitCodePath -Raw).Trim()
if ($exitCode -ne 0) {
    $failureSurvivors = @($owned | ForEach-Object {
            $current = Get-Identity -TargetPid $_.pid
            if ($null -ne $current -and $current.start_time_utc -eq $_.start_time_utc) {
                $current
            }
        })
    $reclaimDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
    while ($failureSurvivors.Count -ne 0 -and [DateTimeOffset]::UtcNow -lt $reclaimDeadline) {
        foreach ($identity in $failureSurvivors) {
            try { (Get-Process -Id $identity.pid -ErrorAction Stop).Kill() } catch {}
        }
        Start-Sleep -Milliseconds 100
        $failureSurvivors = @($failureSurvivors | ForEach-Object {
                $current = Get-Identity -TargetPid $_.pid
                if ($null -ne $current -and $current.start_time_utc -eq $_.start_time_utc) {
                    $current
                }
            })
    }
    if ($failureSurvivors.Count -ne 0) {
        throw "MT-027 proof failed with exit code $exitCode and left exact PID/start survivors: $(($failureSurvivors.pid) -join ','); stderr='$stderrPath'"
    }
    throw "MT-027 canonical Argus proof failed with exit code $exitCode; exact observed PID/start identities absent; stderr='$stderrPath'"
}

$survivors = @()
foreach ($identity in $owned) {
    $current = Get-Identity -TargetPid $identity.pid
    if ($null -ne $current -and $current.start_time_utc -eq $identity.start_time_utc) {
        $survivors += $current
    }
}
if ($survivors.Count -ne 0) {
    $reclaimDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
    do {
        foreach ($identity in $survivors) {
            try { (Get-Process -Id $identity.pid -ErrorAction Stop).Kill() } catch {}
        }
        Start-Sleep -Milliseconds 100
        $survivors = @($owned | ForEach-Object {
                $current = Get-Identity -TargetPid $_.pid
                if ($null -ne $current -and $current.start_time_utc -eq $_.start_time_utc) {
                    $current
                }
            })
    } while ($survivors.Count -ne 0 -and [DateTimeOffset]::UtcNow -lt $reclaimDeadline)
    if ($survivors.Count -ne 0) {
        throw "MT-027 successful Cargo exit left exact owned PID/start identities alive: $(($survivors | ForEach-Object { \"$($_.pid)/$($_.start_time_utc)\" }) -join ',')"
    }
    throw 'MT-027 Cargo exited zero but required post-exit reclamation; proof rejected'
}
if ($testIdentities.Count -ne 1) {
    throw "MT-027 proof requires exactly one test-process identity; observed $($testIdentities.Count)"
}
if ($backendIdentities.Count -ne 1) {
    throw "MT-027 proof requires exactly one fixture-owned handshake_core identity; observed $($backendIdentities.Count)"
}
if (-not (Test-Path -LiteralPath $backendBinary -PathType Leaf)) {
    throw "MT-027 current-source backend build did not produce '$backendBinary'"
}
$testIdentity = $testIdentities[0]
$backendIdentity = $backendIdentities[0]
if (-not (Test-Path -LiteralPath $tracePath -PathType Leaf)) {
    throw "Canonical Argus trace is missing: '$tracePath'"
}
if (-not (Test-Path -LiteralPath $fixedManagedPgReceipt -PathType Leaf)) {
    throw "Managed-PostgreSQL receipt is missing: '$fixedManagedPgReceipt'"
}

$traceRows = @(Get-Content -LiteralPath $tracePath -Encoding UTF8 | Where-Object {
        -not [string]::IsNullOrWhiteSpace($_)
    } | ForEach-Object { $_ | ConvertFrom-Json })
if ($traceRows.Count -ne 24) {
    throw "Canonical Argus trace must contain the exact 24-action MT-027 manifest; got $($traceRows.Count) rows"
}
$invalidRows = @($traceRows | Where-Object {
        $_.schema_id -ne 'hsk.native_gui.canonical_argus_matrix_trace@1' -or
        $_.run_id -ne $RunId -or
        $_.source_sha -ne $sourceSha -or
        $_.process_correlation_id -ne $correlationId -or
        $_.scenario_id -ne $scenario -or
        $_.surface -ne 'Block Collection Views' -or
        $_.edge_state_tag -ne 'create-mutate-switch-empty-error-retry' -or
        $_.receipt_status -notin @('applied', 'rejected') -or
        [string]::IsNullOrWhiteSpace([string]$_.agent_id) -or
        -not ([string]$_.agent_id).EndsWith(':client:wp-kernel-012-mt-027-block-collections-agent') -or
        [int]$_.process_id -ne [int]$testIdentity.pid
    })
if ($invalidRows.Count -ne 0) {
    throw 'Canonical Argus trace is not bound to the committed source and observed test process'
}
# MT-027 V5 (validation_v4 remediation item 4): ZERO indeterminate receipts. This is strictly stronger
# than the previous allowlist, which permitted every row to be `indeterminate`.
$indeterminateRows = @($traceRows | Where-Object { $_.receipt_status -eq 'indeterminate' })
if ($indeterminateRows.Count -ne 0) {
    throw "Canonical Argus trace retains $($indeterminateRows.Count) indeterminate receipt(s): $(($indeterminateRows | ForEach-Object { \"$($_.receipt_id)/$($_.target)\" }) -join ',')"
}
$methods = @($traceRows.method | Sort-Object -Unique)
foreach ($requiredMethod in @('argus.click', 'argus.set_value')) {
    if ($methods -notcontains $requiredMethod) {
        throw "Canonical Argus trace is missing required method '$requiredMethod'"
    }
}
if (@($methods | Where-Object { $_ -notin @('argus.click', 'argus.set_value') }).Count -ne 0) {
    throw "Canonical Argus trace contains a non-action method: $($methods -join ', ')"
}
$receiptIds = @($traceRows.receipt_id)
if (@($receiptIds | Sort-Object -Unique).Count -ne $traceRows.Count) {
    throw 'Canonical Argus trace receipt ids must be unique'
}
$previousSequence = [UInt64]0
foreach ($row in $traceRows) {
    $sequence = [UInt64]$row.terminal_observed_sequence
    if ($sequence -le $previousSequence) {
        throw "Canonical Argus terminal sequences are not strictly increasing at receipt $($row.receipt_id)"
    }
    $previousSequence = $sequence
}

function Find-AuthorNode {
    param(
        [AllowNull()]$Node,
        [Parameter(Mandatory = $true)][string]$AuthorId
    )

    if ($null -eq $Node -or $Node -is [string] -or $Node -is [ValueType]) {
        return $null
    }
    $authorProperty = $Node.PSObject.Properties['author_id']
    if ($null -ne $authorProperty -and [string]$authorProperty.Value -eq $AuthorId) {
        return $Node
    }
    if ($Node -is [Collections.IEnumerable] -and
        $Node -isnot [Management.Automation.PSCustomObject]) {
        foreach ($child in $Node) {
            $found = Find-AuthorNode -Node $child -AuthorId $AuthorId
            if ($null -ne $found) {
                return $found
            }
        }
        return $null
    }
    foreach ($property in $Node.PSObject.Properties) {
        $found = Find-AuthorNode -Node $property.Value -AuthorId $AuthorId
        if ($null -ne $found) {
            return $found
        }
    }
    return $null
}

function Get-AuthorIds {
    param([AllowNull()]$Node)

    if ($null -eq $Node -or $Node -is [string] -or $Node -is [ValueType]) {
        return
    }
    $authorProperty = $Node.PSObject.Properties['author_id']
    if ($null -ne $authorProperty -and
        -not [string]::IsNullOrWhiteSpace([string]$authorProperty.Value)) {
        Write-Output ([string]$authorProperty.Value)
    }
    if ($Node -is [Collections.IEnumerable] -and
        $Node -isnot [Management.Automation.PSCustomObject]) {
        foreach ($child in $Node) {
            Get-AuthorIds -Node $child
        }
        return
    }
    foreach ($property in $Node.PSObject.Properties) {
        Get-AuthorIds -Node $property.Value
    }
}

function Find-AuthorNodes {
    param(
        [AllowNull()]$Node,
        [Parameter(Mandatory = $true)][string]$AuthorId
    )

    if ($null -eq $Node -or $Node -is [string] -or $Node -is [ValueType]) {
        return
    }
    $authorProperty = $Node.PSObject.Properties['author_id']
    if ($null -ne $authorProperty -and [string]$authorProperty.Value -eq $AuthorId) {
        Write-Output $Node
    }
    if ($Node -is [Collections.IEnumerable] -and
        $Node -isnot [Management.Automation.PSCustomObject]) {
        foreach ($child in $Node) {
            Find-AuthorNodes -Node $child -AuthorId $AuthorId
        }
        return
    }
    foreach ($property in $Node.PSObject.Properties) {
        Find-AuthorNodes -Node $property.Value -AuthorId $AuthorId
    }
}

function Test-AuthorValue {
    param(
        [Parameter(Mandatory = $true)]$Tree,
        [Parameter(Mandatory = $true)][string]$AuthorId,
        [Parameter(Mandatory = $true)][string]$Expected
    )

    $nodes = @(Find-AuthorNodes -Node $Tree -AuthorId $AuthorId)
    return $nodes.Count -eq 1 -and
        $null -ne $nodes[0].PSObject.Properties['value'] -and
        $null -ne $nodes[0].value -and
        [string]$nodes[0].value -eq $Expected
}

function Test-AuthorValues {
    param(
        [Parameter(Mandatory = $true)]$Tree,
        [Parameter(Mandatory = $true)][string]$AuthorId,
        [string[]]$RequiredValues = @(),
        [string[]]$ForbiddenValues = @(),
        [switch]$OnlyRequired
    )

    $nodes = @(Find-AuthorNodes -Node $Tree -AuthorId $AuthorId)
    if ($nodes.Count -eq 0 -or
        @($nodes | Where-Object {
                $null -eq $_.PSObject.Properties['value'] -or $null -eq $_.value
            }).Count -ne 0) {
        return $false
    }
    $values = @($nodes | ForEach-Object { [string]$_.value })
    foreach ($required in $RequiredValues) {
        if ($required -notin $values) {
            return $false
        }
    }
    foreach ($forbidden in $ForbiddenValues) {
        if ($forbidden -in $values) {
            return $false
        }
    }
    if ($OnlyRequired -and
        @($values | Where-Object { $_ -notin $RequiredValues }).Count -ne 0) {
        return $false
    }
    return $true
}

function Add-CanonicalTreeLines {
    param(
        [AllowNull()]$Node,
        [Parameter(Mandatory = $true)]
        [AllowEmptyCollection()]
        [Collections.Generic.List[string]]$Lines
    )

    if ($null -eq $Node -or $Node -is [string] -or $Node -is [ValueType]) {
        return
    }
    if ($Node -is [Collections.IEnumerable] -and
        $Node -isnot [Management.Automation.PSCustomObject]) {
        foreach ($child in $Node) {
            Add-CanonicalTreeLines -Node $child -Lines $Lines
        }
        return
    }
    $unit = [string][char]0x1F
    $absent = [string][char]0x00
    $authorProperty = $Node.PSObject.Properties['author_id']
    if ($null -ne $authorProperty -and $authorProperty.Value -is [string]) {
        $field = {
            param([string]$Name)
            $property = $Node.PSObject.Properties[$Name]
            if ($null -eq $property -or $property.Value -isnot [string]) {
                return $absent
            }
            return [string]$property.Value
        }
        $disabledProperty = $Node.PSObject.Properties['disabled']
        $disabled = if ($null -eq $disabledProperty -or $disabledProperty.Value -isnot [bool]) {
            $absent
        } elseif ([bool]$disabledProperty.Value) {
            '1'
        } else {
            '0'
        }
        $Lines.Add(
            [string]$authorProperty.Value + $unit + (& $field 'role') + $unit +
            (& $field 'label') + $unit + (& $field 'value') + $unit + $disabled)
    }
    foreach ($property in $Node.PSObject.Properties) {
        Add-CanonicalTreeLines -Node $property.Value -Lines $Lines
    }
}

# The exact digest `test_block_collection_view::canonical_tree_digest` records into every receipt:
# sorted, unit-separated `author_id / role / label / value / disabled` lines over the addressable nodes,
# newline-terminated, SHA-256. Recomputing it HERE is what makes the persisted receipt independently
# verifiable instead of a self-asserted claim.
function Get-CanonicalTreeDigest {
    param([AllowNull()]$Tree)

    $lines = New-Object 'System.Collections.Generic.List[string]'
    Add-CanonicalTreeLines -Node $Tree -Lines $lines
    $array = $lines.ToArray()
    [Array]::Sort($array, [StringComparer]::Ordinal)
    $builder = New-Object Text.StringBuilder
    foreach ($line in $array) {
        [void]$builder.Append($line)
        [void]$builder.Append("`n")
    }
    $bytes = (New-Object Text.UTF8Encoding($false)).GetBytes($builder.ToString())
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($hasher.ComputeHash($bytes))).
            Replace('-', '').ToLowerInvariant()
    } finally {
        $hasher.Dispose()
    }
}

function Test-TerminalTreePredicate {
    param(
        [Parameter(Mandatory = $true)][string]$PredicateId,
        [Parameter(Mandatory = $true)]$Tree,
        [Parameter(Mandatory = $true)]$MovePayload,
        [AllowNull()]$ActionValue,
        [AllowNull()]$PredicateEvidence
    )

    $has = {
        param([string]$Id)
        return $null -ne (Find-AuthorNode -Node $Tree -AuthorId $Id)
    }
    $text = $Tree | ConvertTo-Json -Depth 100 -Compress
    $cardId = "bcv.kanban.card.$($MovePayload.block_id)"
    $targetLaneId = "bcv.kanban.lane.$($MovePayload.to_lane)"
    switch ($PredicateId) {
        'initial-retry-recovered-projection' {
            return -not (& $has 'bcv.retry') -and (& $has 'bcv.kind.calendar')
        }
        'kind-table-selected' {
            return Test-AuthorValue $Tree 'bcv.kind.table.state' 'selected'
        }
        'kind-kanban-selected' {
            return Test-AuthorValue $Tree 'bcv.kind.kanban.state' 'selected'
        }
        'kind-calendar-selected' {
            return Test-AuthorValue $Tree 'bcv.kind.calendar.state' 'selected'
        }
        'kind-table-restored' {
            return Test-AuthorValue $Tree 'bcv.kind.table.state' 'selected'
        }
        'sort-title-ascending' {
            $node = Find-AuthorNode $Tree 'bcv.table.sort.title'
            return $null -ne $node -and
                [string]$node.label -eq ('Title ' + [char]0x25B2)
        }
        'kanban-retry-loaded-card' {
            return (& $has 'bcv.kanban.lane.untagged') -and
                (Test-AuthorValues $Tree $cardId -RequiredValues @('__untagged__') -OnlyRequired)
        }
        'kanban-card-moved-target-lane' {
            return (& $has $targetLaneId) -and
                (Test-AuthorValues $Tree $cardId `
                    -RequiredValues @([string]$MovePayload.to_lane) `
                    -ForbiddenValues @([string]$MovePayload.from_lane))
        }
        'calendar-retry-loaded-controls' {
            return (& $has 'bcv.calendar.date-from') -and
                (& $has 'bcv.calendar.date-to')
        }
        'calendar-from-value' {
            return Test-AuthorValue $Tree 'bcv.calendar.date-from' '2026-02-28'
        }
        'calendar-to-value' {
            return Test-AuthorValue $Tree 'bcv.calendar.date-to' '2026-04-30'
        }
        'calendar-range-terminal' {
            $requiredEntries = @($PredicateEvidence.required_entries)
            $expectedDays = @(
                'bcv.calendar.day.2026-03-01',
                'bcv.calendar.day.2026-03-02',
                'bcv.calendar.day.2026-04-01'
            )
            $actualAuthorIds = @(Get-AuthorIds -Node $Tree)
            $actualDays = @($actualAuthorIds | Where-Object {
                    ([string]$_).StartsWith(
                        'bcv.calendar.day.', [StringComparison]::Ordinal)
                })
            $actualEntries = @($actualAuthorIds | Where-Object {
                    ([string]$_).StartsWith(
                        'bcv.calendar.entry.', [StringComparison]::Ordinal)
                })
            $exactEntries = $requiredEntries.Count -eq 3 -and
                @($requiredEntries.entry_author_id | Sort-Object -Unique).Count -eq 3 -and
                @($requiredEntries.day_author_id | Sort-Object -Unique).Count -eq 3 -and
                $actualDays.Count -eq 3 -and
                $actualEntries.Count -eq 3 -and
                @($requiredEntries | Where-Object {
                        $_.day_author_id -notin $expectedDays -or
                        -not ([string]$_.entry_author_id).StartsWith(
                            'bcv.calendar.entry.', [StringComparison]::Ordinal) -or
                        -not (& $has ([string]$_.day_author_id)) -or
                        -not (Test-AuthorValue $Tree `
                            ([string]$_.entry_author_id) `
                            ([string]$_.day_author_id).Substring('bcv.calendar.day.'.Length))
                    }).Count -eq 0
            return (Test-AuthorValue $Tree 'bcv.calendar.date-from' '2026-02-28') -and
                (Test-AuthorValue $Tree 'bcv.calendar.date-to' '2026-04-30') -and
                $exactEntries
        }
        'unbound-create-form-open' {
            return (& $has 'bcv.new-view.title') -and
                (& $has 'bcv.new-view.confirm') -and
                -not (& $has 'bcv.kind.table')
        }
        'unbound-create-title-set' {
            $node = Find-AuthorNode $Tree 'bcv.new-view.title'
            return $null -ne $node -and [string]$node.value -eq [string]$ActionValue
        }
        'unbound-create-calendar-selected' {
            return Test-AuthorValue $Tree 'bcv.new-view.kind.calendar.state' 'selected'
        }
        'unbound-create-calendar-terminal' {
            return (Test-AuthorValue $Tree 'bcv.kind.calendar.state' 'selected') -and
                -not (& $has 'bcv.retry') -and
                -not (& $has 'bcv.new-view.title')
        }
        'retry-create-form-open' {
            return (& $has 'bcv.new-view.title') -and (& $has 'bcv.new-view.confirm')
        }
        'retry-create-title-set' {
            $node = Find-AuthorNode $Tree 'bcv.new-view.title'
            return $null -ne $node -and [string]$node.value -eq [string]$ActionValue
        }
        'retry-create-kanban-selected' {
            return Test-AuthorValue $Tree 'bcv.new-view.kind.kanban.state' 'selected'
        }
        'failed-create-retry-visible' {
            return & $has 'bcv.retry'
        }
        'retry-create-kanban-terminal' {
            return (Test-AuthorValue $Tree 'bcv.kind.kanban.state' 'selected') -and
                -not (& $has 'bcv.retry') -and
                -not (& $has 'bcv.new-view.title')
        }
        'empty-table-terminal' {
            return $text.Contains('No blocks match this view.') -and
                @((Get-AuthorIds $Tree) | Where-Object {
                        $_.StartsWith('bcv.table.row.', [StringComparison]::Ordinal)
                    }).Count -eq 0
        }
        'empty-kanban-terminal' {
            $authorIds = @(Get-AuthorIds $Tree)
            return $text.Contains('No lanes in this view.') -and
                @($authorIds | Where-Object {
                        $_.StartsWith('bcv.kanban.lane.', [StringComparison]::Ordinal) -or
                        $_.StartsWith('bcv.kanban.card.', [StringComparison]::Ordinal)
                    }).Count -eq 0
        }
        'empty-calendar-terminal' {
            $authorIds = @(Get-AuthorIds $Tree)
            return $text.Contains('No blocks in this date range.') -and
                @($authorIds | Where-Object {
                        $_.StartsWith('bcv.calendar.day.', [StringComparison]::Ordinal) -or
                        $_.StartsWith('bcv.calendar.entry.', [StringComparison]::Ordinal)
                    }).Count -eq 0
        }
        default {
            throw "No external terminal-tree verifier exists for '$PredicateId'"
        }
    }
}

$movePayloadForPredicates = $traceRows[7].action_value | ConvertFrom-Json
$expectedActions = @(
    @{ method = 'argus.click'; target = 'bcv.retry'; predicate = 'initial-retry-recovered-projection'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.kind.table'; predicate = 'kind-table-selected'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.kind.kanban'; predicate = 'kind-kanban-selected'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.kind.calendar'; predicate = 'kind-calendar-selected'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.kind.table'; predicate = 'kind-table-restored'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.table.sort.title'; predicate = 'sort-title-ascending'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.retry'; predicate = 'kanban-retry-loaded-card'; status = 'applied' },
    @{ method = 'argus.click'; target = 'collection.kanban-move'; predicate = 'kanban-card-moved-target-lane'; payload = 'kanban-move'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.retry'; predicate = 'calendar-retry-loaded-controls'; status = 'applied' },
    @{ method = 'argus.set_value'; target = 'bcv.calendar.date-from'; predicate = 'calendar-from-value'; value = '2026-02-28'; status = 'applied' },
    @{ method = 'argus.set_value'; target = 'bcv.calendar.date-to'; predicate = 'calendar-to-value'; value = '2026-04-30'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.calendar.apply-range'; predicate = 'calendar-range-terminal'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.new-view'; predicate = 'unbound-create-form-open'; status = 'applied' },
    @{ method = 'argus.set_value'; target = 'bcv.new-view.title'; predicate = 'unbound-create-title-set'; value_suffix = '-host-created'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.new-view.kind.calendar'; predicate = 'unbound-create-calendar-selected'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.new-view.confirm'; predicate = 'unbound-create-calendar-terminal'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.new-view'; predicate = 'retry-create-form-open'; status = 'applied' },
    @{ method = 'argus.set_value'; target = 'bcv.new-view.title'; predicate = 'retry-create-title-set'; value_suffix = '-retry-created'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.new-view.kind.kanban'; predicate = 'retry-create-kanban-selected'; status = 'applied' },
    # The create against the dead backend is a CAUSALLY OWNED typed terminal failure, not an
    # unprovable outcome: the observer binds the exact target/context/generation/semantic tuple.
    @{ method = 'argus.click'; target = 'bcv.new-view.confirm'; predicate = 'failed-create-retry-visible'; status = 'rejected' },
    @{ method = 'argus.click'; target = 'bcv.retry'; predicate = 'retry-create-kanban-terminal'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.retry'; predicate = 'empty-table-terminal'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.kind.kanban'; predicate = 'empty-kanban-terminal'; status = 'applied' },
    @{ method = 'argus.click'; target = 'bcv.kind.calendar'; predicate = 'empty-calendar-terminal'; status = 'applied' }
)
for ($index = 0; $index -lt $expectedActions.Count; $index++) {
    $expectedAction = $expectedActions[$index]
    $row = $traceRows[$index]
    if ($row.method -ne $expectedAction.method -or $row.target -ne $expectedAction.target) {
        throw "Canonical Argus action $($index + 1) is out of contract order: expected $($expectedAction.method) $($expectedAction.target), got $($row.method) $($row.target)"
    }
    if ($row.terminal_refreshed -isnot [bool] -or
        $row.terminal_refreshed -ne $true) {
        throw "Canonical Argus action $($index + 1) lacks post-authority terminal inspection"
    }
    $predicates = @($row.terminal_predicates)
    if ($predicates.Count -ne 1 -or
        $predicates[0].predicate_id -ne $expectedAction.predicate -or
        $predicates[0].passed -isnot [bool] -or
        $predicates[0].passed -ne $true) {
        throw "Canonical Argus action $($index + 1) lacks its exact passing terminal predicate '$($expectedAction.predicate)'"
    }
    if (-not (Test-TerminalTreePredicate -PredicateId $expectedAction.predicate `
                -Tree $row.after -MovePayload $movePayloadForPredicates `
                -ActionValue $row.action_value `
                -PredicateEvidence $predicates[0].evidence)) {
        throw "Canonical Argus action $($index + 1) terminal tree contradicts predicate '$($expectedAction.predicate)'"
    }
    # MT-027 V5 (validation_v4 remediation items 1 + 4): the receipt itself must be TERMINAL and must
    # carry its own verifiable evidence — the expected causal status, the target binding, the
    # workspace/view identity, an independent authoritative backend readback, and a digest of the exact
    # terminal tree that is recomputed HERE from the persisted row.
    if ([string]$row.receipt_status -ne [string]$expectedAction.status) {
        throw "Canonical Argus action $($index + 1) receipt status is '$($row.receipt_status)', expected the causal '$($expectedAction.status)'"
    }
    $evidence = $predicates[0].evidence
    if ($null -eq $evidence -or
        $evidence.schema_id -ne 'hsk.mt027.terminal_receipt_evidence@1') {
        throw "Canonical Argus action $($index + 1) carries no MT-027 terminal receipt evidence"
    }
    if ([string]$evidence.receipt_binding.target -ne [string]$row.target -or
        [string]$evidence.receipt_binding.expected_receipt_status -ne [string]$row.receipt_status) {
        throw "Canonical Argus action $($index + 1) receipt evidence is not bound to its own row: evidence_target='$($evidence.receipt_binding.target)'; row_target='$($row.target)'; evidence_status='$($evidence.receipt_binding.expected_receipt_status)'; row_status='$($row.receipt_status)'"
    }
    if ([string]::IsNullOrWhiteSpace([string]$evidence.workspace_id)) {
        throw "Canonical Argus action $($index + 1) receipt evidence has no workspace identity"
    }
    if ($null -eq $evidence.backend_readback) {
        throw "Canonical Argus action $($index + 1) receipt evidence has no authoritative backend readback"
    }
    if ([bool]$evidence.backend_readback.bound) {
        if ([string]$evidence.backend_readback.view_block_id -ne [string]$evidence.view_block_id) {
            throw "Canonical Argus action $($index + 1) backend readback view id does not match the bound view identity"
        }
    } elseif (-not [string]::IsNullOrWhiteSpace([string]$evidence.view_block_id)) {
        throw "Canonical Argus action $($index + 1) declares an unbound readback while naming a view id"
    }
    $recomputedDigest = Get-CanonicalTreeDigest -Tree $row.after
    if ([string]$evidence.terminal_tree_sha256 -ne $recomputedDigest) {
        throw "Canonical Argus action $($index + 1) terminal tree digest mismatch: receipt='$($evidence.terminal_tree_sha256)'; recomputed='$recomputedDigest'"
    }
    $observerNode = Find-AuthorNode -Node $row.after -AuthorId 'bcv.action-completion'
    if ($null -eq $observerNode -or [string]::IsNullOrWhiteSpace([string]$observerNode.value)) {
        throw "Canonical Argus action $($index + 1) terminal tree has no published bcv.action-completion observer"
    }
    if ($expectedAction.ContainsKey('value') -and
        [string]$row.action_value -ne [string]$expectedAction.value) {
        throw "Canonical Argus action $($index + 1) has the wrong value: '$($row.action_value)'"
    }
    if ($expectedAction.ContainsKey('value_suffix') -and
        -not ([string]$row.action_value).EndsWith([string]$expectedAction.value_suffix)) {
        throw "Canonical Argus action $($index + 1) has the wrong value suffix: '$($row.action_value)'"
    }
    if (-not $expectedAction.ContainsKey('value') -and
        -not $expectedAction.ContainsKey('value_suffix') -and
        -not $expectedAction.ContainsKey('payload') -and
        $null -ne $row.action_value) {
        throw "Canonical Argus action $($index + 1) unexpectedly carries action_value"
    }
}
$movePayload = $traceRows[7].action_value | ConvertFrom-Json
if ([string]::IsNullOrWhiteSpace([string]$movePayload.block_id) -or
    $movePayload.from_lane -ne '__untagged__' -or
    [string]::IsNullOrWhiteSpace([string]$movePayload.to_lane)) {
    throw 'Canonical Argus Kanban move payload is not the required untagged-to-tag authority mutation'
}

$managedReceipt = Get-Content -LiteralPath $fixedManagedPgReceipt -Raw | ConvertFrom-Json
$receiptBackendBinary = Get-ComparableWindowsPath (
    [string]$managedReceipt.backend_binding.backend_binary)
$observedBackendExecutable = Get-ComparableWindowsPath (
    [string]$backendIdentity.executable)
$expectedBackendBinary = Get-ComparableWindowsPath $backendBinary
if ($managedReceipt.schema_id -ne 'hsk.mt027_managed_pg_proof@1' -or
    -not [bool]$managedReceipt.backend_binding.owned -or
    [int]$managedReceipt.backend_binding.backend_pid -ne [int]$backendIdentity.pid -or
    -not $observedBackendExecutable.Equals(
        $expectedBackendBinary, [StringComparison]::OrdinalIgnoreCase) -or
    -not $receiptBackendBinary.Equals(
        $expectedBackendBinary, [StringComparison]::OrdinalIgnoreCase) -or
    $managedReceipt.backend_binding.database_host -ne '127.0.0.1' -or
    [int]$managedReceipt.backend_binding.database_port -ne 5544 -or
    $managedReceipt.backend_binding.database_name -ne $postgresDatabase) {
    throw "Managed proof receipt binding mismatch: schema='$($managedReceipt.schema_id)'; owned='$($managedReceipt.backend_binding.owned)'; receipt_pid='$($managedReceipt.backend_binding.backend_pid)'; observed_pid='$($backendIdentity.pid)'; receipt_binary='$receiptBackendBinary'; observed_executable='$observedBackendExecutable'; expected_binary='$expectedBackendBinary'; database_host='$($managedReceipt.backend_binding.database_host)'; database_port='$($managedReceipt.backend_binding.database_port)'; database_name='$($managedReceipt.backend_binding.database_name)'"
}
$backendSha256 = (Get-FileHash -LiteralPath $backendBinary -Algorithm SHA256).Hash.ToLowerInvariant()
if ($managedReceipt.backend_binding.backend_binary_sha256 -ne $backendSha256) {
    throw 'Managed proof backend binary hash does not match the supervised current-source build'
}
$managedRuntimeData = Get-ComparableWindowsPath (
    [string]$managedReceipt.backend_binding.runtime_data_dir)
$expectedRuntimeArtifactsRoot = Get-ComparableWindowsPath $runtimeArtifactsRoot
if (-not $managedRuntimeData.StartsWith(
        $expectedRuntimeArtifactsRoot + [IO.Path]::DirectorySeparatorChar,
        [StringComparison]::OrdinalIgnoreCase) -or
    (Test-Path -LiteralPath $managedRuntimeData)) {
    throw 'Fixture-owned backend runtime was not contained and cleaned before receipt publication'
}

$postSourceSha = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $postSourceSha -ne $sourceSha) {
    throw "Repository HEAD changed during MT-027 proof: before='$sourceSha', after='$postSourceSha'"
}
# Re-assert the GATED set after the run: every compiled/configured input must still match committed
# HEAD, so nothing this proof actually consumed can have moved underneath it.
#
# The EXCLUDED rows are deliberately NOT required to be byte-stable across the run. The exclusion holds
# because each is another test binary's top-level source, compiled into its OWN separate binary, which
# provably cannot enter this proof's artifacts — and that remains true whether or not a parallel lane
# edits it mid-run. Requiring them frozen would fail this proof for an edit it can never observe. Both
# the preflight and post-run excluded sets are recorded with their blob hashes in the receipt, so a
# reviewer can still see exactly what was excluded and confirm none of it was a compiled input.
$postExcludedDirtyRows = Assert-Mt027Cleanliness -Phase 'post-run'

$receiptPgPort = [int]$managedReceipt.backend_binding.database_port
$currentPostgresListener = Get-NetTCPConnection -LocalAddress '127.0.0.1' `
    -LocalPort $receiptPgPort -State Listen -ErrorAction Stop | Select-Object -First 1
$currentPostgresProcess = Get-Process `
    -Id ([int]$currentPostgresListener.OwningProcess) -ErrorAction Stop
$initialPostgresStartUtc = Format-CanonicalUtc $initialPostgresStartTime
$currentPostgresStartUtc = Format-CanonicalUtc (
    [DateTimeOffset]$currentPostgresProcess.StartTime)
if ([int]$currentPostgresProcess.Id -ne $initialPostgresPid -or
    $currentPostgresStartUtc -ne $initialPostgresStartUtc -or
    -not $currentPostgresProcess.ProcessName.Equals(
        'postgres', [StringComparison]::OrdinalIgnoreCase)) {
    throw "The exact PostgreSQL listener identity changed before receipt acceptance: initial_pid='$initialPostgresPid'; current_pid='$($currentPostgresProcess.Id)'; initial_start='$initialPostgresStartUtc'; current_start='$currentPostgresStartUtc'; current_process='$($currentPostgresProcess.ProcessName)'"
}
$currentPostgresIdentity = [pscustomobject]@{
    pid = [int]$currentPostgresProcess.Id
    start_time_utc = $currentPostgresStartUtc
    executable = if ([string]::IsNullOrWhiteSpace([string]$currentPostgresProcess.Path)) {
        "$($currentPostgresProcess.ProcessName).exe"
    } else {
        [string]$currentPostgresProcess.Path
    }
}

Move-Item -LiteralPath $fixedManagedPgReceipt `
    -Destination $managedReceiptDestination
$traceArtifact = [ordered]@{
    path = $tracePath
    size_bytes = [int64](Get-Item -LiteralPath $tracePath).Length
    sha256 = (Get-FileHash -LiteralPath $tracePath -Algorithm SHA256).Hash.ToLowerInvariant()
}
$managedReceiptArtifact = [ordered]@{
    path = $managedReceiptDestination
    size_bytes = [int64](Get-Item -LiteralPath $managedReceiptDestination).Length
    sha256 = (Get-FileHash -LiteralPath $managedReceiptDestination -Algorithm SHA256).Hash.ToLowerInvariant()
}
foreach ($transientRoot in @($stageBindingRoot, $runtimeArtifactsRoot, $argusBindingRoot)) {
    $resolvedTransient = [IO.Path]::GetFullPath($transientRoot)
    if (-not (Test-CleanableTransientRoot -Path $resolvedTransient -RunDirectory $runDir `
                -RuntimeParent $runtimeArtifactsParent -RunIdentifier $RunId)) {
        throw "Refusing to clean transient proof path outside this run's owned directories: '$resolvedTransient'"
    }
    if (Test-Path -LiteralPath $resolvedTransient) {
        [IO.Directory]::Delete($resolvedTransient, $true)
    }
    if (Test-Path -LiteralPath $resolvedTransient) {
        throw "Transient proof path survived exact cleanup: '$resolvedTransient'"
    }
}
$wrapperHasher = [Security.Cryptography.SHA256]::Create()
try {
    $wrapperSpecSha256 = ([BitConverter]::ToString(
            $wrapperHasher.ComputeHash([Text.Encoding]::UTF8.GetBytes($wrapperSpec)))).
        Replace('-', '').ToLowerInvariant()
} finally {
    $wrapperHasher.Dispose()
}

$receipt = [ordered]@{
    schema_id = 'hsk.mt027.canonical_argus_process_receipt@2'
    wp_id = 'WP-KERNEL-012'
    mt_id = 'MT-027'
    run_id = $RunId
    source_sha = $sourceSha
    process_correlation_id = $correlationId
    supervisor_pid = $PID
    supervisor_started_at_utc = $supervisorStartedAtUtc
    deadline_at_utc = Format-CanonicalUtc $deadline
    timeout_seconds = $TimeoutSeconds
    wrapper_pid = [int]$rootIdentity.pid
    wrapper_start_time_utc = $rootIdentity.start_time_utc
    test_process_pid = [int]$testIdentity.pid
    test_process_start_time_utc = $testIdentity.start_time_utc
    test_process_executable = $testIdentity.executable
    backend_process_pid = [int]$backendIdentity.pid
    backend_process_start_time_utc = $backendIdentity.start_time_utc
    backend_process_executable = $backendIdentity.executable
    backend_binary_sha256 = $backendSha256
    postgres_pid = [int]$currentPostgresIdentity.pid
    postgres_start_time_utc = $currentPostgresIdentity.start_time_utc
    postgres_database = $postgresDatabase
    postgres_host = '127.0.0.1'
    postgres_port = 5544
    owned_process_tree = $owned
    wrapper_executable = 'powershell.exe'
    cargo_executable = $cargo.Source
    wrapper_spec_sha256 = $wrapperSpecSha256
    supervised_commands = @(
        [ordered]@{ cwd = $repoRoot; arguments = $backendBuildArguments },
        [ordered]@{ cwd = $crateRoot; arguments = $cargoArguments }
    )
    exit_code = $exitCode
    survivor_count = 0
    trace_row_count = $traceRows.Count
    trace_methods = $methods
    trace_receipt_count = @($receiptIds | Sort-Object -Unique).Count
    trace_artifact = $traceArtifact
    managed_pg_receipt_artifact = $managedReceiptArtifact
    artifact_root = $artifactSibling
    cargo_target_dir = $cargoTarget
    trace_receipt_statuses = @($traceRows | ForEach-Object {
            [ordered]@{
                receipt_id = $_.receipt_id
                target = $_.target
                method = $_.method
                receipt_status = $_.receipt_status
                predicate_id = @($_.terminal_predicates)[0].predicate_id
                predicate_passed = @($_.terminal_predicates)[0].passed
                terminal_tree_sha256 = @($_.terminal_predicates)[0].evidence.terminal_tree_sha256
                workspace_id = @($_.terminal_predicates)[0].evidence.workspace_id
                view_block_id = @($_.terminal_predicates)[0].evidence.view_block_id
            }
        })
    indeterminate_receipt_count = 0
    cleanliness_gate = [ordered]@{
        policy = 'gated_by_default_only_other_test_binary_top_level_sources_are_excludable'
        gated_inputs = @(
            'src/frontend/handshake_native/src/**',
            'src/frontend/handshake_native/tests/test_block_collection_view.rs',
            'src/frontend/handshake_native/tests/native_gui_support/**',
            'src/frontend/handshake_native/tests/interconnect_support/**',
            'src/frontend/handshake_native/tests/pg_proof_support/**',
            'src/frontend/handshake_native/tests/fixtures/**',
            'src/frontend/handshake_native/Cargo.toml',
            'src/frontend/handshake_native/Cargo.lock',
            'src/frontend/handshake_native/diag_ring/**',
            'src/frontend/palmistry/**',
            'src/backend/handshake_core/**',
            'every other repository path'
        )
        excluded_unrelated_dirty_rows = @($excludedDirtyRows)
        excluded_unrelated_dirty_row_count = @($excludedDirtyRows).Count
        excluded_unrelated_dirty_rows_post_run = @($postExcludedDirtyRows)
        excluded_unrelated_dirty_row_count_post_run = @($postExcludedDirtyRows).Count
    }
    transient_roots_cleaned = @($stageBindingRoot, $runtimeArtifactsRoot, $argusBindingRoot)
    status = 'COMPLETED'
    completed_at_utc = Format-CanonicalUtc ([DateTimeOffset]::UtcNow)
}
$receiptJson = $receipt | ConvertTo-Json -Depth 6
[IO.File]::WriteAllText(
    $receiptPath, $receiptJson, [Text.UTF8Encoding]::new($false))
$proofAccepted = $true
Write-Output "MT-027 canonical Argus proof complete; run_id=$RunId; source_sha=$sourceSha; artifacts=$runDir"
} catch {
    $primaryFailure = $_
    throw
} finally {
    $cleanupFailures = @()

    # The concrete Process handle is the unconditional primary cleanup path.
    # CIM PID/start inventory adds attribution and catches detached children,
    # but a WMI failure must never prevent killing the exact tree we launched.
    if ($null -ne $process) {
        try {
            if (-not $process.HasExited) {
                $process.Kill($true)
            }
            if (-not $process.WaitForExit(10000)) {
                $cleanupFailures += "wrapper-handle-timeout: pid=$($process.Id)"
            }
        } catch {
            $cleanupFailures += "wrapper-handle: $($_.Exception.Message)"
        }
    }

    if ($null -ne $rootIdentity -and
        -not [string]::IsNullOrWhiteSpace([string]$rootIdentity.start_time_utc)) {
        try {
            $latestOwned = @(Get-Descendants -RootPid ([int]$rootIdentity.pid) `
                    -RootStartUtc ([string]$rootIdentity.start_time_utc))
            foreach ($identity in $latestOwned) {
                if (@($owned | Where-Object {
                            $_.pid -eq $identity.pid -and
                            $_.start_time_utc -eq $identity.start_time_utc
                        }).Count -eq 0) {
                    $owned += $identity
                }
            }
        } catch {
            $cleanupFailures += "descendant-discovery: $($_.Exception.Message)"
        }
        $reclaimDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
        do {
            $liveOwned = @()
            foreach ($identity in $owned) {
                $current = $null
                try {
                    $current = Get-Identity -TargetPid ([int]$identity.pid)
                } catch {
                    $cleanupFailures += "identity-$($identity.pid): $($_.Exception.Message)"
                    continue
                }
                if ($null -ne $current -and
                    $current.start_time_utc -eq $identity.start_time_utc) {
                    $liveOwned += $current
                    try {
                        (Get-Process -Id ([int]$identity.pid) -ErrorAction Stop).Kill()
                    } catch {
                        $cleanupFailures += "pid-$($identity.pid): $($_.Exception.Message)"
                    }
                }
            }
            if ($liveOwned.Count -eq 0) {
                break
            }
            Start-Sleep -Milliseconds 100
        } while ([DateTimeOffset]::UtcNow -lt $reclaimDeadline)
        $remainingOwned = @()
        foreach ($identity in $owned) {
            $current = $null
            try {
                $current = Get-Identity -TargetPid ([int]$identity.pid)
            } catch {
                $cleanupFailures += "final-identity-$($identity.pid): $($_.Exception.Message)"
                continue
            }
            if ($null -ne $current -and
                $current.start_time_utc -eq $identity.start_time_utc) {
                $remainingOwned += $current
            }
        }
        if ($remainingOwned.Count -ne 0) {
            $cleanupFailures += "surviving-owned-identities: $((
                    $remainingOwned | ForEach-Object {
                        "$($_.pid)/$($_.start_time_utc)"
                    }) -join ',')"
        }
    } elseif ($null -ne $process) {
        try {
            if (-not $process.HasExited) {
                $cleanupFailures += "wrapper-identity-unavailable-and-process-live: pid=$($process.Id)"
            }
        } catch {
            $cleanupFailures += "wrapper-state-after-identity-failure: $($_.Exception.Message)"
        }
    }

    foreach ($transientRoot in @($stageBindingRoot, $runtimeArtifactsRoot, $argusBindingRoot)) {
        try {
            $resolvedTransient = [IO.Path]::GetFullPath($transientRoot)
            if (-not (Test-CleanableTransientRoot -Path $resolvedTransient -RunDirectory $runDir `
                        -RuntimeParent $runtimeArtifactsParent -RunIdentifier $RunId)) {
                throw "path escaped this run's owned directories: '$resolvedTransient'"
            }
            if (Test-Path -LiteralPath $resolvedTransient) {
                [IO.Directory]::Delete($resolvedTransient, $true)
            }
        } catch {
            $cleanupFailures += "transient-${transientRoot}: $($_.Exception.Message)"
        }
    }
    if (-not $proofAccepted) {
        try {
            if (-not (Test-Path -LiteralPath $fixedManagedPgReceipt -PathType Leaf)) {
                throw [IO.FileNotFoundException]::new(
                    "No fixed managed receipt exists at '$fixedManagedPgReceipt'")
            }
            Remove-Item -LiteralPath $fixedManagedPgReceipt -Force
        } catch [IO.FileNotFoundException] {
            # No receipt was published before failure; nothing to remove.
        } catch {
            $cleanupFailures += "fixed-receipt: $($_.Exception.Message)"
        }
    }
    foreach ($environmentName in $environmentNames) {
        try {
            [Environment]::SetEnvironmentVariable(
                $environmentName,
                $environmentSnapshot[$environmentName],
                'Process')
        } catch {
            $cleanupFailures += "environment-${environmentName}: $($_.Exception.Message)"
        }
    }
    if ($cleanupFailures.Count -ne 0) {
        $cleanupMessage = $cleanupFailures -join '; '
        if ($null -ne $primaryFailure) {
            throw "MT-027 primary failure: $($primaryFailure.Exception.Message); mandatory cleanup failures: $cleanupMessage"
        }
        if ($proofAccepted) {
            throw "MT-027 proof completed but mandatory cleanup failed: $cleanupMessage"
        }
        throw "MT-027 mandatory cleanup failed: $cleanupMessage"
    }
}
