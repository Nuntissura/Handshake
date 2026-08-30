[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[A-Za-z0-9_-]+$')]
    [string]$RunId,

    [ValidateRange(30, 600)]
    [int]$PerProcessTimeoutSeconds = 90,

    [string]$CargoTargetDir,

    [string]$ProofArtifactDir,

    [switch]$Headless
)

$ErrorActionPreference = 'Stop'
$crateRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$artifactSibling = [IO.Path]::GetFullPath((Join-Path $crateRoot '..\..\..\..\Handshake_Artifacts'))
$repoRoot = (& git -C $crateRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw 'Unable to resolve the product repository root for MT-108 source binding'
}
$sourceSha = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceSha -notmatch '^[0-9a-f]{40}$') {
    throw "Unable to resolve an exact committed source SHA; got '$sourceSha'"
}
$relevantStatus = @(& git -C $repoRoot status --porcelain --untracked-files=all -- `
        '.' `
        ':(exclude)AGENTS.md' `
        ':(exclude)CLAUDE.md')
if ($LASTEXITCODE -ne 0) {
    throw 'Unable to inspect MT-108 relevant source cleanliness'
}
if (@($relevantStatus | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }).Count -ne 0) {
    throw "MT-108 proof requires every compiled/configured repository input to match committed HEAD; dirty rows: $($relevantStatus -join '; ')"
}
$matrixPath = Join-Path $PSScriptRoot 'mt108_argus_matrix.json'
$matrix = Get-Content -LiteralPath $matrixPath -Raw | ConvertFrom-Json
if ($matrix.schema_id -ne 'hsk.native_gui.argus_surface_matrix@1' -or
    $matrix.wp_id -ne 'WP-KERNEL-012' -or $matrix.mt_id -ne 'MT-108' -or
    @($matrix.rows).Count -eq 0) {
    throw 'MT-108 Argus manifest schema/ownership is invalid'
}

function Resolve-ExternalPath {
    param(
        [string]$ConfiguredPath,
        [Parameter(Mandatory = $true)][string]$DefaultPath,
        [Parameter(Mandatory = $true)][string]$Label
    )

    $candidate = if ([string]::IsNullOrWhiteSpace($ConfiguredPath)) { $DefaultPath } else { $ConfiguredPath }
    $absolute = if ([IO.Path]::IsPathRooted($candidate)) {
        [IO.Path]::GetFullPath($candidate)
    } else {
        [IO.Path]::GetFullPath((Join-Path $crateRoot $candidate))
    }
    $root = $artifactSibling.TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar)
    $prefix = $root + [IO.Path]::DirectorySeparatorChar
    if (-not $absolute.Equals($root, [StringComparison]::OrdinalIgnoreCase) -and
        -not $absolute.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "$Label must resolve beneath the external Handshake_Artifacts root '$root'; got '$absolute'"
    }
    return $absolute
}

function Assert-NoReparsePointEscape {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Label
    )

    $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd('\')
    $cursor = [IO.Path]::GetFullPath($Path)
    while ($cursor.Length -ge $rootFull.Length) {
        if (Test-Path -LiteralPath $cursor) {
            $item = Get-Item -Force -LiteralPath $cursor
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "$Label must not traverse a junction, symlink, or other reparse point: '$cursor'"
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
    throw "$Label is not rooted beneath the canonical Handshake_Artifacts directory"
}

$CargoTargetDir = Resolve-ExternalPath -ConfiguredPath $CargoTargetDir `
    -DefaultPath (Join-Path $artifactSibling 'handshake-cargo-target\wp-kernel-012-mt-108') -Label 'CargoTargetDir'
$ProofArtifactDir = Resolve-ExternalPath -ConfiguredPath $ProofArtifactDir `
    -DefaultPath (Join-Path $artifactSibling 'handshake-test\wp-kernel-012-mt-108\integrated') -Label 'ProofArtifactDir'
$canonicalCargoTarget = [IO.Path]::GetFullPath((Join-Path $artifactSibling 'handshake-cargo-target\wp-kernel-012-mt-108'))
$canonicalProofRoot = [IO.Path]::GetFullPath((Join-Path $artifactSibling 'handshake-test\wp-kernel-012-mt-108\integrated'))
if (-not $CargoTargetDir.Equals($canonicalCargoTarget, [StringComparison]::OrdinalIgnoreCase)) {
    throw "CargoTargetDir must equal the allocated canonical target '$canonicalCargoTarget'; got '$CargoTargetDir'"
}
if (-not $ProofArtifactDir.Equals($canonicalProofRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw "ProofArtifactDir must equal the allocated MT-108 proof root '$canonicalProofRoot'; got '$ProofArtifactDir'"
}
Assert-NoReparsePointEscape -Path $CargoTargetDir -Root $artifactSibling -Label 'CargoTargetDir'
Assert-NoReparsePointEscape -Path $ProofArtifactDir -Root $artifactSibling -Label 'ProofArtifactDir'

# Managed native-surface scenarios start the real current-source backend. Bind that executable from the
# same canonical target used by every supervised Cargo child so the proof cannot inherit an arbitrary
# machine-local HSK_TEST_BACKEND_BIN. The backend fixture independently rejects a stale binary.
$backendBinary = [IO.Path]::GetFullPath((Join-Path $CargoTargetDir 'debug\handshake_core.exe'))
if (-not (Test-Path -LiteralPath $backendBinary -PathType Leaf)) {
    throw "MT-108 managed-surface proof requires the current-source backend at '$backendBinary'. Build it first with: cargo build --locked -j 2 --target-dir `"$CargoTargetDir`" --manifest-path `"$(Join-Path $repoRoot 'src\backend\handshake_core\Cargo.toml')`" --bin handshake_core --features app-runtime"
}

$env:CARGO_TARGET_DIR = $CargoTargetDir
$env:HSK_TEST_BACKEND_BIN = $backendBinary
$env:HANDSHAKE_PROOF_ARTIFACT_DIR = $ProofArtifactDir
$env:HANDSHAKE_SCREENSHOT_RUN_ID = $RunId
$env:HANDSHAKE_ARGUS_MATRIX_RUN_ID = $RunId
$env:HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA = $sourceSha
if ($Headless) {
    [Environment]::SetEnvironmentVariable('HANDSHAKE_GPU_SCREENSHOT', $null, 'Process')
} else {
    $env:HANDSHAKE_GPU_SCREENSHOT = '1'
}

$runDir = [IO.Path]::GetFullPath((Join-Path $ProofArtifactDir $RunId))
if (Test-Path -LiteralPath $runDir) {
    throw "RunId '$RunId' is not fresh: proof directory already exists at '$runDir'"
}
New-Item -ItemType Directory -Force -Path $ProofArtifactDir | Out-Null
New-Item -ItemType Directory -Path $runDir | Out-Null
Copy-Item -LiteralPath $matrixPath -Destination (Join-Path $runDir 'mt108_argus_matrix.json')
$externalReceiptPath = Join-Path $runDir 'external_process_receipts.jsonl'

function Format-CanonicalUtc {
    param([Parameter(Mandatory = $true)]$Value)

    return ([DateTimeOffset]$Value).ToUniversalTime().ToString(
        "yyyy-MM-dd'T'HH:mm:ss.fffffff'Z'",
        [Globalization.CultureInfo]::InvariantCulture)
}

$supervisorStartedAtUtc = Format-CanonicalUtc ([DateTimeOffset]::UtcNow)

function Write-ExternalReceipt {
    param(
        [Parameter(Mandatory = $true)][Collections.IDictionary]$ProcessContext,
        [Parameter(Mandatory = $true)][string]$Status,
        [Parameter(Mandatory = $true)][string]$ReasonCode,
        [Parameter(Mandatory = $true)][string]$Reason,
        [Nullable[int]]$ExitCode,
        [string]$CleanupMethod
    )

    $receipt = [ordered]@{
        schema_id = 'hsk.native_gui.external_process_receipt@3'
        run_id = $RunId
        source_sha = $sourceSha
        outcome_id = "$($ProcessContext.CorrelationId)-$($Status.ToLowerInvariant())-$([guid]::NewGuid().ToString('N'))"
        process_correlation_id = $ProcessContext.CorrelationId
        mt_id = 'MT-108'
        owner_session = $RunId
        owner_wp = 'WP-KERNEL-012'
        owner_role = 'MT108_ARGUS_PROOF_SUPERVISOR'
        supervisor_pid = $PID
        supervisor_started_at_utc = $supervisorStartedAtUtc
        child_pid = $ProcessContext.ChildPid
        owned_process_tree_pids = @($ProcessContext.OwnedProcessTree | ForEach-Object { [int]$_.Pid })
        owned_process_tree = @($ProcessContext.OwnedProcessTree | ForEach-Object {
                [ordered]@{
                    pid = [int]$_.Pid
                    parent_pid = [int]$_.ParentPid
                    start_time_utc = [string]$_.StartUtc
                    executable = [string]$_.Executable
                }
            })
        test_process_pid = if ($null -eq $ProcessContext.TestProcessIdentity) { $null } else { [int]$ProcessContext.TestProcessIdentity.Pid }
        test_process_start_time_utc = if ($null -eq $ProcessContext.TestProcessIdentity) { $null } else { [string]$ProcessContext.TestProcessIdentity.StartUtc }
        test_process_executable = if ($null -eq $ProcessContext.TestProcessIdentity) { $null } else { [string]$ProcessContext.TestProcessIdentity.Executable }
        child_started_at_utc = $ProcessContext.StartedAtUtc
        deadline_at_utc = $ProcessContext.DeadlineAtUtc
        deadline_seconds = $PerProcessTimeoutSeconds
        command_executable = $ProcessContext.Executable
        command_arguments = @($ProcessContext.Arguments)
        command_display = $ProcessContext.CommandDisplay
        working_directory = $crateRoot
        scenario_id = $ProcessContext.Scenario
        status = $Status
        reason_code = $ReasonCode
        reason = $Reason
        exit_code = if ($null -eq $ExitCode) { $null } else { [int]$ExitCode }
        cleanup_method = if ([string]::IsNullOrWhiteSpace($CleanupMethod)) { $null } else { $CleanupMethod }
        cleanup_verified = [bool]$ProcessContext.CleanupVerified
        survivor_count_at_receipt = [int]$ProcessContext.SurvivorCountAtReceipt
        gpu_screenshot_enabled = -not $Headless.IsPresent
        timestamp_utc = Format-CanonicalUtc ([DateTimeOffset]::UtcNow)
    }
    $line = ($receipt | ConvertTo-Json -Compress -Depth 5) + [Environment]::NewLine
    $bytes = New-Object Text.UTF8Encoding($false)
    $payload = $bytes.GetBytes($line)
    $stream = [IO.File]::Open($externalReceiptPath, [IO.FileMode]::Append, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try {
        $stream.Write($payload, 0, $payload.Length)
        $stream.Flush($true)
    } finally {
        $stream.Dispose()
    }
}

function Get-ProcessStartTimeUtc {
    param([Parameter(Mandatory = $true)]$ProcessRow)

    if ($null -eq $ProcessRow.CreationDate) {
        return ''
    }
    try {
        if ($ProcessRow.CreationDate -is [DateTime]) {
            return Format-CanonicalUtc ([DateTime]$ProcessRow.CreationDate)
        }
        return Format-CanonicalUtc ([Management.ManagementDateTimeConverter]::ToDateTime([string]$ProcessRow.CreationDate))
    } catch {
        return ''
    }
}

function Get-OwnedProcessTreeSnapshot {
    param(
        [Parameter(Mandatory = $true)][int]$RootPid,
        [string]$ExpectedRootStartUtc,
        [switch]$AllowMissingRoot
    )

    $processRows = @(Get-CimInstance -ClassName Win32_Process -ErrorAction Stop)
    $rootRow = $processRows | Where-Object { [int]$_.ProcessId -eq $RootPid } | Select-Object -First 1
    if ($null -eq $rootRow) {
        if ($AllowMissingRoot.IsPresent) {
            return @()
        }
        throw "owned process-tree root PID $RootPid is no longer present"
    }
    $observedRootStartUtc = Get-ProcessStartTimeUtc $rootRow
    if (-not [string]::IsNullOrWhiteSpace($ExpectedRootStartUtc) -and
        $observedRootStartUtc -ne $ExpectedRootStartUtc) {
        throw "owned process-tree root PID $RootPid identity changed: expected start '$ExpectedRootStartUtc', observed '$observedRootStartUtc'"
    }
    $owned = New-Object 'System.Collections.Generic.HashSet[int]'
    $ownedStartUtc = @{}
    [void]$owned.Add($RootPid)
    $ownedStartUtc[$RootPid] = $observedRootStartUtc
    do {
        $added = $false
        foreach ($row in $processRows) {
            $candidatePid = [int]$row.ProcessId
            $candidateParentPid = [int]$row.ParentProcessId
            if (-not $owned.Contains($candidateParentPid) -or $owned.Contains($candidatePid)) {
                continue
            }
            $candidateStartUtc = Get-ProcessStartTimeUtc $row
            if ([string]::IsNullOrWhiteSpace($candidateStartUtc)) {
                throw "owned process-tree candidate PID $candidatePid has no verifiable start-time identity"
            }
            $parentStartedAt = [DateTimeOffset]::Parse(
                [string]$ownedStartUtc[$candidateParentPid],
                [Globalization.CultureInfo]::InvariantCulture,
                [Globalization.DateTimeStyles]::AssumeUniversal)
            $childStartedAt = [DateTimeOffset]::Parse(
                $candidateStartUtc,
                [Globalization.CultureInfo]::InvariantCulture,
                [Globalization.DateTimeStyles]::AssumeUniversal)
            if ($childStartedAt -lt $parentStartedAt) {
                # Win32_Process.ParentProcessId is only a PID number and may
                # point at a later reused PID. Chronologically impossible
                # ancestry must never claim or reclaim an unrelated process.
                continue
            }
            [void]$owned.Add($candidatePid)
            $ownedStartUtc[$candidatePid] = $candidateStartUtc
            $added = $true
        }
    } while ($added)
    return @($processRows | Where-Object { $owned.Contains([int]$_.ProcessId) } | ForEach-Object {
            [pscustomobject]@{
                Pid = [int]$_.ProcessId
                ParentPid = [int]$_.ParentProcessId
                StartUtc = Get-ProcessStartTimeUtc $_
                Executable = if ([string]::IsNullOrWhiteSpace([string]$_.ExecutablePath)) { [string]$_.Name } else { [string]$_.ExecutablePath }
            }
        } | Sort-Object Pid, StartUtc)
}

function Get-ProcessIdentityByPid {
    param([Parameter(Mandatory = $true)][int]$TargetPid)

    $row = Get-CimInstance -ClassName Win32_Process -Filter "ProcessId = $TargetPid" -ErrorAction Stop | Select-Object -First 1
    if ($null -eq $row) {
        return $null
    }
    [pscustomobject]@{
        Pid = [int]$row.ProcessId
        ParentPid = [int]$row.ParentProcessId
        StartUtc = Get-ProcessStartTimeUtc $row
        Executable = if ([string]::IsNullOrWhiteSpace([string]$row.ExecutablePath)) { [string]$row.Name } else { [string]$row.ExecutablePath }
    }
}

function Add-ProcessTreeSnapshot {
    param(
        [Parameter(Mandatory = $true)][Collections.IDictionary]$ProcessContext,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][object[]]$Snapshot
    )

    foreach ($identity in $Snapshot) {
        $existing = @($ProcessContext.OwnedProcessTree | Where-Object {
                $_.Pid -eq $identity.Pid -and $_.StartUtc -eq $identity.StartUtc
            })
        if ($existing.Count -eq 0) {
            $ProcessContext.OwnedProcessTree += $identity
        }
        if ($identity.Pid -ne $ProcessContext.ChildPid -and
            [IO.Path]::GetFileNameWithoutExtension($identity.Executable) -like "$($ProcessContext.TestBinary)*") {
            if ($null -eq $ProcessContext.TestProcessIdentity -or
                $ProcessContext.TestProcessIdentity.StartUtc -ne $identity.StartUtc) {
                $ProcessContext.TestProcessIdentity = $identity
            }
        }
    }
}

function Write-ProcessObservationAck {
    param([Parameter(Mandatory = $true)][Collections.IDictionary]$ProcessContext)

    if ($null -eq $ProcessContext.TestProcessIdentity -or
        (Test-Path -LiteralPath $ProcessContext.ProcessObservationAckPath)) {
        return
    }
    $identity = $ProcessContext.TestProcessIdentity
    $ack = [ordered]@{
        schema_id = 'hsk.native_gui.process_observation_ack@1'
        process_correlation_id = $ProcessContext.CorrelationId
        process_id = [int]$identity.Pid
        process_start_time_utc = [string]$identity.StartUtc
        process_executable = [string]$identity.Executable
    }
    $payload = [Text.Encoding]::UTF8.GetBytes(
        (($ack | ConvertTo-Json -Compress) + [Environment]::NewLine))
    $temporaryAckPath = "$($ProcessContext.ProcessObservationAckPath).$([guid]::NewGuid().ToString('N')).tmp"
    $stream = [IO.File]::Open(
        $temporaryAckPath,
        [IO.FileMode]::CreateNew,
        [IO.FileAccess]::Write,
        [IO.FileShare]::None)
    try {
        $stream.Write($payload, 0, $payload.Length)
        $stream.Flush($true)
    } finally {
        $stream.Dispose()
    }
    try {
        [IO.File]::Move($temporaryAckPath, $ProcessContext.ProcessObservationAckPath)
    } catch {
        [IO.File]::Delete($temporaryAckPath)
        throw
    }
}

function Add-ProcessInventoryError {
    param(
        [Parameter(Mandatory = $true)][Collections.IDictionary]$ProcessContext,
        [Parameter(Mandatory = $true)][string]$Message
    )

    if (@($ProcessContext.ProcessInventoryErrors) -notcontains $Message) {
        $ProcessContext.ProcessInventoryErrors = @($ProcessContext.ProcessInventoryErrors) + $Message
    }
}

function Get-LiveOwnedProcessInventory {
    param([Parameter(Mandatory = $true)][Collections.IDictionary]$ProcessContext)

    $live = @()
    $errors = @($ProcessContext.ProcessInventoryErrors)
    try {
        $live += @(Get-OwnedProcessTreeSnapshot -RootPid $ProcessContext.ChildPid `
            -ExpectedRootStartUtc $ProcessContext.StartedAtUtc -AllowMissingRoot)
    } catch {
        $errors += "owned process-tree query failed: $($_.Exception.Message)"
    }
    foreach ($identity in $ProcessContext.OwnedProcessTree) {
        $current = $null
        try {
            $current = Get-ProcessIdentityByPid -TargetPid $identity.Pid
        } catch {
            $errors += "PID $($identity.Pid) identity query failed: $($_.Exception.Message)"
            continue
        }
        if ($null -ne $current -and $current.StartUtc -eq $identity.StartUtc) {
            $live += $current
        }
    }
    [pscustomobject]@{
        Identities = @($live | Group-Object { "$($_.Pid)|$($_.StartUtc)" } | ForEach-Object { $_.Group[0] })
        InventoryHealthy = $errors.Count -eq 0
        InventoryError = if ($errors.Count -eq 0) { $null } else { $errors -join '; ' }
    }
}

function Stop-OwnedProcessIdentities {
    param([Parameter(Mandatory = $true)][Collections.IDictionary]$ProcessContext)

    $inventory = Get-LiveOwnedProcessInventory -ProcessContext $ProcessContext
    foreach ($identity in @($inventory.Identities)) {
        $current = Get-ProcessIdentityByPid -TargetPid $identity.Pid
        if ($null -eq $current -or $current.StartUtc -ne $identity.StartUtc) {
            continue
        }
        try {
            $ownedProcess = Get-Process -Id $identity.Pid -ErrorAction Stop
            $ownedProcess.Kill()
        } catch {
            # The reclamation poll records any exact identity that remains live.
        }
    }
}

function Invoke-BoundedCargoTest {
    param(
        [Parameter(Mandatory = $true)][string]$Scenario,
        [Parameter(Mandatory = $true)][string]$Surface,
        [Parameter(Mandatory = $true)][string]$EdgeStateTag,
        [Parameter(Mandatory = $true)][string[]]$CargoArguments
    )

    $stdoutPath = Join-Path $runDir "$Scenario.stdout.log"
    $stderrPath = Join-Path $runDir "$Scenario.stderr.log"
    $correlationId = "cargo-$Scenario-$([guid]::NewGuid().ToString('N'))"
    $exitCodePath = Join-Path $runDir "$correlationId.exit-code"
    $processObservationAckPath = Join-Path $runDir "$correlationId.process-observed.json"
    if (Test-Path -LiteralPath $exitCodePath) {
        throw "${Scenario}: Cargo exit-code sidecar is not fresh: $exitCodePath"
    }
    if (Test-Path -LiteralPath $processObservationAckPath) {
        throw "${Scenario}: process-observation acknowledgement is not fresh: $processObservationAckPath"
    }
    $env:HANDSHAKE_PROOF_PROCESS_CORRELATION_ID = $correlationId
    $env:HANDSHAKE_PROOF_PROCESS_SCENARIO_ID = $Scenario
    $env:HANDSHAKE_ARGUS_MATRIX_SCENARIO_ID = $Scenario
    $env:HANDSHAKE_ARGUS_MATRIX_SURFACE = $Surface
    $env:HANDSHAKE_ARGUS_MATRIX_EDGE_STATE = $EdgeStateTag
    $env:HANDSHAKE_PROOF_PROCESS_OBSERVATION_ACK = $processObservationAckPath
    $cargoCommand = Get-Command cargo -CommandType Application -ErrorAction Stop
    $wrapperSpecJson = [ordered]@{
        CargoPath = $cargoCommand.Source
        WorkingDirectory = $crateRoot
        Arguments = @($CargoArguments)
        ExitCodePath = $exitCodePath
    } | ConvertTo-Json -Compress -Depth 4
    $wrapperSpecBase64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($wrapperSpecJson))
    $wrapperCommand = @'
$specJson = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('__SPEC_BASE64__'))
$spec = $specJson | ConvertFrom-Json
Set-Location -LiteralPath ([string]$spec.WorkingDirectory)
& ([string]$spec.CargoPath) @($spec.Arguments)
$cargoExitCode = $LASTEXITCODE
if ($null -eq $cargoExitCode) {
    $cargoExitCode = 9009
}
[IO.File]::WriteAllText([string]$spec.ExitCodePath, [string][int]$cargoExitCode)
exit ([int]$cargoExitCode)
'@ -replace '__SPEC_BASE64__', $wrapperSpecBase64
    $wrapperCommandBase64 = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($wrapperCommand))
    $process = Start-Process -FilePath 'powershell.exe' `
        -ArgumentList @('-NoProfile', '-NonInteractive', '-EncodedCommand', $wrapperCommandBase64) `
        -WorkingDirectory $crateRoot `
        -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
    $rootIdentity = $null
    try {
        $rootIdentity = Get-ProcessIdentityByPid -TargetPid $process.Id
    } catch {
        try {
            $process.Kill($true)
        } catch {
            # Best-effort termination before failing closed on a missing root identity.
        }
        throw "${Scenario}: unable to read the supervised Cargo wrapper root identity for PID $($process.Id): $($_.Exception.Message)"
    }
    $rootExecutableName = if ($null -eq $rootIdentity) { '' } else {
        [IO.Path]::GetFileNameWithoutExtension([string]$rootIdentity.Executable)
    }
    if ($null -eq $rootIdentity -or [string]::IsNullOrWhiteSpace($rootIdentity.StartUtc) -or
        -not $rootExecutableName.Equals('powershell', [StringComparison]::OrdinalIgnoreCase)) {
        try {
            $process.Kill($true)
        } catch {
            # Best-effort termination before failing closed on an invalid root identity.
        }
        throw "${Scenario}: supervised Cargo wrapper root identity unavailable or non-PowerShell for PID $($process.Id)"
    }
    $startedAt = [DateTimeOffset]::Parse(
        $rootIdentity.StartUtc,
        [Globalization.CultureInfo]::InvariantCulture,
        [Globalization.DateTimeStyles]::AssumeUniversal)
    $testBinary = ''
    for ($argumentIndex = 0; $argumentIndex -lt ($CargoArguments.Count - 1); $argumentIndex++) {
        if ($CargoArguments[$argumentIndex] -eq '--test') {
            $testBinary = [string]$CargoArguments[$argumentIndex + 1]
            break
        }
    }
    $context = [ordered]@{
        Scenario = $Scenario
        CorrelationId = $correlationId
        ChildPid = $process.Id
        OwnedProcessTree = @($rootIdentity)
        ProcessInventoryErrors = @()
        TestBinary = $testBinary
        TestProcessIdentity = $null
        ProcessObservationAckPath = $processObservationAckPath
        CleanupVerified = $false
        SurvivorCountAtReceipt = 0
        StartedAtUtc = $rootIdentity.StartUtc
        DeadlineAtUtc = Format-CanonicalUtc ($startedAt.AddSeconds($PerProcessTimeoutSeconds))
        Executable = 'cargo'
        Arguments = @($CargoArguments)
        CommandDisplay = 'cargo ' + (($CargoArguments | ForEach-Object { '"' + ($_ -replace '"', '\"') + '"' }) -join ' ')
    }
    Write-ExternalReceipt -ProcessContext $context -Status 'STARTED' -ReasonCode 'PROCESS_STARTED' `
        -Reason 'Hidden Cargo proof child started under the bounded supervisor'

    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($PerProcessTimeoutSeconds)
    $exited = $false
    while ([DateTimeOffset]::UtcNow -lt $deadline) {
        try {
            Add-ProcessTreeSnapshot -ProcessContext $context -Snapshot @(Get-OwnedProcessTreeSnapshot -RootPid $process.Id `
                -ExpectedRootStartUtc $context.StartedAtUtc -AllowMissingRoot)
            Write-ProcessObservationAck -ProcessContext $context
        } catch {
            Add-ProcessInventoryError -ProcessContext $context `
                -Message "owned process-tree capture was indeterminate: $($_.Exception.Message)"
        }
        if ($process.WaitForExit(25)) {
            $exited = $true
            try {
                # Capture one final process-table snapshot immediately after exit so
                # short-lived cargo test executables are attributable when possible.
                Add-ProcessTreeSnapshot -ProcessContext $context -Snapshot @(Get-OwnedProcessTreeSnapshot -RootPid $process.Id `
                    -ExpectedRootStartUtc $context.StartedAtUtc -AllowMissingRoot)
                Write-ProcessObservationAck -ProcessContext $context
            } catch {
                Add-ProcessInventoryError -ProcessContext $context `
                    -Message "final owned process-tree capture was indeterminate: $($_.Exception.Message)"
            }
            break
        }
    }
    if (-not $exited) {
        $reason = "Cargo proof process exceeded the hard ${PerProcessTimeoutSeconds}s wall-clock bound"
        $inventoryError = $null
        try {
            Add-ProcessTreeSnapshot -ProcessContext $context -Snapshot @(Get-OwnedProcessTreeSnapshot -RootPid $process.Id `
                -ExpectedRootStartUtc $context.StartedAtUtc)
        } catch {
            $inventoryError = "unable to inventory the owned process tree before reclamation: $($_.Exception.Message)"
        }
        Write-ExternalReceipt -ProcessContext $context -Status 'BLOCKED' -ReasonCode 'EXTERNAL_PROCESS_TIMEOUT' -Reason $reason
        $cleanupMethod = 'Process.Kill(entireProcessTree=true)'
        $cleanupError = $inventoryError
        try {
            $process.Kill($true)
        } catch {
            $cleanupMethod = 'taskkill.exe /T /F'
            try {
                $taskkill = Start-Process -FilePath 'taskkill.exe' -ArgumentList @('/PID', $process.Id, '/T', '/F') `
                    -WindowStyle Hidden -Wait -PassThru
                if ($taskkill.ExitCode -ne 0) {
                    $cleanupError = "taskkill exited $($taskkill.ExitCode)"
                }
            } catch {
                $cleanupError = $_.Exception.Message
            }
        }
        Stop-OwnedProcessIdentities -ProcessContext $context
        $reclaimDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
        $survivors = @()
        $inventoryHealthError = $cleanupError
        do {
            $process.WaitForExit(250) | Out-Null
            $inventory = Get-LiveOwnedProcessInventory -ProcessContext $context
            $survivors = @($inventory.Identities)
            if (-not $inventory.InventoryHealthy -and [string]::IsNullOrWhiteSpace($inventoryHealthError)) {
                $inventoryHealthError = $inventory.InventoryError
            }
            if ($survivors.Count -eq 0) {
                break
            }
            Start-Sleep -Milliseconds 100
        } while ([DateTimeOffset]::UtcNow -lt $reclaimDeadline)
        $rootReclaimed = $process.HasExited
        if ($rootReclaimed -and $survivors.Count -eq 0 -and
            [string]::IsNullOrWhiteSpace($cleanupError) -and
            [string]::IsNullOrWhiteSpace($inventoryHealthError)) {
            Write-ExternalReceipt -ProcessContext $context -Status 'RECLAIMED' -ReasonCode 'PROCESS_TREE_RECLAIMED' `
                -Reason "$reason; owned child process tree is no longer running" -ExitCode $process.ExitCode -CleanupMethod $cleanupMethod
            throw "${Scenario}: $reason; process tree reclaimed; receipt=$externalReceiptPath"
        }
        $failure = if (-not [string]::IsNullOrWhiteSpace($inventoryHealthError)) {
            "$reason; process-tree reclamation inventory was indeterminate: $inventoryHealthError"
        } elseif ([string]::IsNullOrWhiteSpace($cleanupError)) {
            "$reason; owned process tree still has live PID/start identities after the 10s reclamation bound: $(($survivors | ForEach-Object { \"$($_.Pid)/$($_.StartUtc)\" }) -join ',')"
        } else {
            "$reason; cleanup failed: $cleanupError"
        }
        Write-ExternalReceipt -ProcessContext $context -Status 'RECLAIM_FAILED' -ReasonCode 'PROCESS_TREE_RECLAIM_FAILED' `
            -Reason $failure -CleanupMethod $cleanupMethod
        throw "${Scenario}: $failure; PID=$($process.Id); receipt=$externalReceiptPath"
    }

    if (-not (Test-Path -LiteralPath $exitCodePath -PathType Leaf)) {
        throw "$Scenario exited without the wrapper-owned Cargo exit-code sidecar: $exitCodePath"
    }
    $rawExitCode = (Get-Content -LiteralPath $exitCodePath -Raw).Trim()
    $exitCode = 0
    if (-not [int]::TryParse(
            $rawExitCode,
            [Globalization.NumberStyles]::Integer,
            [Globalization.CultureInfo]::InvariantCulture,
            [ref]$exitCode)) {
        throw "$Scenario wrote an invalid Cargo exit-code sidecar value '$rawExitCode'"
    }
    if ($exitCode -ne 0) {
        Write-ExternalReceipt -ProcessContext $context -Status 'FAILED' -ReasonCode 'PROCESS_EXIT_NONZERO' `
            -Reason "Cargo exited $exitCode" -ExitCode $exitCode
        throw "$Scenario failed with exit code $exitCode; stderr=$stderrPath"
    }
    $stdout = Get-Content -LiteralPath $stdoutPath -Raw
    if ($stdout -match '(?m)^running 0 tests\r?$') {
        Write-ExternalReceipt -ProcessContext $context -Status 'FAILED' -ReasonCode 'ZERO_TESTS' `
            -Reason 'Cargo exited zero without executing the selected proof test' -ExitCode $exitCode
        throw "$Scenario selected zero tests; stdout=$stdoutPath"
    }
    $closureDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
    $closureInventory = $null
    do {
        $closureInventory = Get-LiveOwnedProcessInventory -ProcessContext $context
        if (-not $closureInventory.InventoryHealthy -or @($closureInventory.Identities).Count -eq 0) {
            break
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTimeOffset]::UtcNow -lt $closureDeadline)
    $context.SurvivorCountAtReceipt = @($closureInventory.Identities).Count
    if (-not $closureInventory.InventoryHealthy -or $context.SurvivorCountAtReceipt -ne 0) {
        Stop-OwnedProcessIdentities -ProcessContext $context
        $reason = if (-not $closureInventory.InventoryHealthy) {
            "successful Cargo exit had an indeterminate owned-process inventory: $($closureInventory.InventoryError)"
        } else {
            "successful Cargo exit left $($context.SurvivorCountAtReceipt) owned PID/start identities alive"
        }
        Write-ExternalReceipt -ProcessContext $context -Status 'FAILED' -ReasonCode 'PROCESS_TREE_NOT_CLOSED' `
            -Reason $reason -ExitCode $exitCode -CleanupMethod 'identity-aware post-exit reclamation'
        $reclaimDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
        $survivors = @()
        $inventoryHealthError = $null
        do {
            $inventory = Get-LiveOwnedProcessInventory -ProcessContext $context
            $survivors = @($inventory.Identities)
            if (-not $inventory.InventoryHealthy) {
                $inventoryHealthError = $inventory.InventoryError
                break
            }
            if ($survivors.Count -eq 0) {
                break
            }
            Stop-OwnedProcessIdentities -ProcessContext $context
            Start-Sleep -Milliseconds 100
        } while ([DateTimeOffset]::UtcNow -lt $reclaimDeadline)
        if ($survivors.Count -eq 0 -and [string]::IsNullOrWhiteSpace($inventoryHealthError)) {
            $context.SurvivorCountAtReceipt = 0
            Write-ExternalReceipt -ProcessContext $context -Status 'RECLAIMED' -ReasonCode 'PROCESS_TREE_RECLAIMED' `
                -Reason "$reason; all owned PID/start identities were reclaimed after the zero exit" `
                -ExitCode $exitCode -CleanupMethod 'identity-aware post-exit reclamation'
            throw "${Scenario}: $reason; process tree reclaimed; receipt=$externalReceiptPath"
        }
        $reclaimFailure = if (-not [string]::IsNullOrWhiteSpace($inventoryHealthError)) {
            "$reason; reclamation inventory was indeterminate: $inventoryHealthError"
        } else {
            "$reason; owned PID/start identities survived the 10s reclamation bound: $(($survivors | ForEach-Object { \"$($_.Pid)/$($_.StartUtc)\" }) -join ',')"
        }
        Write-ExternalReceipt -ProcessContext $context -Status 'RECLAIM_FAILED' -ReasonCode 'PROCESS_TREE_RECLAIM_FAILED' `
            -Reason $reclaimFailure -ExitCode $exitCode -CleanupMethod 'identity-aware post-exit reclamation'
        throw "${Scenario}: $reclaimFailure; receipt=$externalReceiptPath"
    }
    $context.CleanupVerified = $true
    Write-ExternalReceipt -ProcessContext $context -Status 'COMPLETED' -ReasonCode 'PROCESS_EXIT_ZERO' `
        -Reason 'Cargo proof process exited successfully within the supervisor bound and no owned PID/start identity survived' -ExitCode $exitCode
}

function Assert-FinalProcessReceipts {
    param([Parameter(Mandatory = $true)][Collections.IDictionary]$ExpectedCommands)

    $rows = @(Get-Content -LiteralPath $externalReceiptPath | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
    if ($rows.Count -ne ($ExpectedCommands.Count * 2)) {
        throw "Process closure requires exactly $($ExpectedCommands.Count * 2) lifecycle receipts; got $($rows.Count)"
    }
    $invalid = @($rows | Where-Object { $_.status -notin @('STARTED', 'COMPLETED') })
    if ($invalid.Count -ne 0) {
        throw "Process closure rejects failed, blocked, or reclamation lifecycles: $($invalid.status -join ', ')"
    }
    foreach ($scenario in $ExpectedCommands.Keys) {
        $lifecycle = @($rows | Where-Object { $_.scenario_id -eq $scenario })
        $started = @($lifecycle | Where-Object { $_.status -eq 'STARTED' })
        $completed = @($lifecycle | Where-Object { $_.status -eq 'COMPLETED' })
        if ($lifecycle.Count -ne 2 -or $started.Count -ne 1 -or $completed.Count -ne 1) {
            throw "$scenario requires exactly one STARTED and one COMPLETED receipt"
        }
        if ($started[0].process_correlation_id -ne $completed[0].process_correlation_id -or
            $started[0].child_pid -ne $completed[0].child_pid -or
            $completed[0].exit_code -ne 0 -or
            -not [bool]$completed[0].cleanup_verified -or
            [int]$completed[0].survivor_count_at_receipt -ne 0) {
            throw "$scenario process lifecycle correlation/PID/exit proof is invalid"
        }
        $expected = @($ExpectedCommands[$scenario])
        $actual = @($started[0].command_arguments)
        if ($started[0].command_executable -ne 'cargo' -or
            ($actual -join [char]31) -ne ($expected -join [char]31) -or
            [string]::IsNullOrWhiteSpace($started[0].child_started_at_utc) -or
            [string]::IsNullOrWhiteSpace($started[0].deadline_at_utc)) {
            throw "$scenario exact command/start/deadline proof is invalid"
        }
    }
}

$expectedCommands = [ordered]@{}

foreach ($surface in @($matrix.rows)) {
    $testBinary = [string]$surface.test_binary
    $testName = [string]$surface.test_name
    $testIgnored = [bool]$surface.ignored
    if ($Headless -and
        -not [string]::IsNullOrWhiteSpace([string]$surface.headless_test_binary) -and
        -not [string]::IsNullOrWhiteSpace([string]$surface.headless_test_name)) {
        $testBinary = [string]$surface.headless_test_binary
        $testName = [string]$surface.headless_test_name
        $testIgnored = [bool]$surface.headless_ignored
    }
    $arguments = @(
        'test', '--features', 'integration,wgpu_screenshots', '--no-fail-fast', '-j', '2',
        '--test', $testBinary, $testName, '--'
    )
    if ($testIgnored) {
        $arguments += '--ignored'
    }
    $arguments += @('--exact', '--nocapture')
    $expectedCommands[[string]$surface.scenario_id] = $arguments
    Invoke-BoundedCargoTest -Scenario ([string]$surface.scenario_id) `
        -Surface ([string]$surface.surface) -EdgeStateTag ([string]$surface.edge_state_tag) `
        -CargoArguments $arguments
}

$verifierArguments = @(
    'test', '--features', 'integration,wgpu_screenshots', '--no-fail-fast', '-j', '2',
    '--test', 'test_mt108_argus_aggregate', 'mt108_verify_argus_evidence_manifest', '--', '--ignored', '--exact', '--nocapture'
)
$expectedCommands['manifest_verifier'] = $verifierArguments
Invoke-BoundedCargoTest -Scenario 'manifest_verifier' -Surface 'matrix verifier' `
    -EdgeStateTag 'closure' -CargoArguments $verifierArguments
Assert-FinalProcessReceipts -ExpectedCommands $expectedCommands

if ($Headless) {
    Write-Output "MT-108 headless typed-marker proof complete (NOT pixel closure) for run_id=$RunId; source_sha=$sourceSha; artifacts=$runDir; successful_processes=$($expectedCommands.Count)"
} else {
    Write-Output "MT-108 Argus pixel proof closed for run_id=$RunId; source_sha=$sourceSha; artifacts=$runDir; successful_processes=$($expectedCommands.Count)"
}
