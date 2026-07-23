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

$CargoTargetDir = Resolve-ExternalPath -ConfiguredPath $CargoTargetDir `
    -DefaultPath (Join-Path $artifactSibling 'handshake-cargo-target') -Label 'CargoTargetDir'
$ProofArtifactDir = Resolve-ExternalPath -ConfiguredPath $ProofArtifactDir `
    -DefaultPath (Join-Path $artifactSibling 'handshake-test\native_gui') -Label 'ProofArtifactDir'

$env:CARGO_TARGET_DIR = $CargoTargetDir
$env:HANDSHAKE_PROOF_ARTIFACT_DIR = $ProofArtifactDir
$env:HANDSHAKE_SCREENSHOT_RUN_ID = $RunId
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
        schema_id = 'hsk.native_gui.external_process_receipt@2'
        run_id = $RunId
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
    [void]$owned.Add($RootPid)
    do {
        $added = $false
        foreach ($row in $processRows) {
            if ($owned.Contains([int]$row.ParentProcessId) -and $owned.Add([int]$row.ProcessId)) {
                $added = $true
            }
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
        [Parameter(Mandatory = $true)][object[]]$Snapshot
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

function Get-LiveOwnedProcessInventory {
    param([Parameter(Mandatory = $true)][Collections.IDictionary]$ProcessContext)

    $live = @()
    $errors = @()
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
        [Parameter(Mandatory = $true)][string[]]$CargoArguments
    )

    $stdoutPath = Join-Path $runDir "$Scenario.stdout.log"
    $stderrPath = Join-Path $runDir "$Scenario.stderr.log"
    $correlationId = "cargo-$Scenario-$([guid]::NewGuid().ToString('N'))"
    $env:HANDSHAKE_PROOF_PROCESS_CORRELATION_ID = $correlationId
    $env:HANDSHAKE_PROOF_PROCESS_SCENARIO_ID = $Scenario
    $process = Start-Process -FilePath 'cargo' -ArgumentList $CargoArguments -WorkingDirectory $crateRoot `
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
        throw "${Scenario}: unable to read the supervised Cargo root identity for PID $($process.Id): $($_.Exception.Message)"
    }
    $rootExecutableName = if ($null -eq $rootIdentity) { '' } else {
        [IO.Path]::GetFileNameWithoutExtension([string]$rootIdentity.Executable)
    }
    if ($null -eq $rootIdentity -or [string]::IsNullOrWhiteSpace($rootIdentity.StartUtc) -or
        -not $rootExecutableName.Equals('cargo', [StringComparison]::OrdinalIgnoreCase)) {
        try {
            $process.Kill($true)
        } catch {
            # Best-effort termination before failing closed on an invalid root identity.
        }
        throw "${Scenario}: supervised Cargo root identity unavailable or non-Cargo for PID $($process.Id)"
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
        TestBinary = $testBinary
        TestProcessIdentity = $null
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
                -ExpectedRootStartUtc $context.StartedAtUtc)
        } catch {
            # A transient process-table read failure is recorded by the final lifecycle if it prevents proof.
        }
        if ($process.WaitForExit(25)) {
            $exited = $true
            try {
                # Capture one final process-table snapshot immediately after exit so
                # short-lived cargo test executables are attributable when possible.
                Add-ProcessTreeSnapshot -ProcessContext $context -Snapshot @(Get-OwnedProcessTreeSnapshot -RootPid $process.Id `
                    -ExpectedRootStartUtc $context.StartedAtUtc)
            } catch {
                # The root may already have disappeared; the prior snapshots remain
                # authoritative and the verifier must fail closed if identity is absent.
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

    $exitCode = $process.ExitCode
    if ($exitCode -ne 0) {
        Write-ExternalReceipt -ProcessContext $context -Status 'FAILED' -ReasonCode 'PROCESS_EXIT_NONZERO' `
            -Reason "Cargo exited $exitCode" -ExitCode $exitCode
        throw "$Scenario failed with exit code $exitCode; stderr=$stderrPath"
    }
    Write-ExternalReceipt -ProcessContext $context -Status 'COMPLETED' -ReasonCode 'PROCESS_EXIT_ZERO' `
        -Reason 'Cargo proof process exited successfully within the supervisor bound' -ExitCode $exitCode
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
            $completed[0].exit_code -ne 0) {
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

$surfaceTests = @(
    @{ Scenario = 'find_bar'; Binary = 'test_find_bar_accesskit'; Test = 'mt108_argus_find_bar_real_server_loop' },
    @{ Scenario = 'formatting_toolbar'; Binary = 'test_formatting_toolbar'; Test = 'mt108_argus_formatting_toolbar_real_server_loop' },
    @{ Scenario = 'slash_menu'; Binary = 'test_slash_commands'; Test = 'mt108_argus_slash_menu_real_server_loop' },
    @{ Scenario = 'outline_pane'; Binary = 'test_outline'; Test = 'mt108_argus_outline_real_server_loop' },
    @{ Scenario = 'rich_find_replace'; Binary = 'test_rich_find_replace'; Test = 'mt108_argus_rich_find_replace_real_server_loop' },
    @{ Scenario = 'runtime_chat'; Binary = 'test_runtime_chat_pane'; Test = 'mt108_argus_runtime_chat_real_server_loop' },
    @{ Scenario = 'diagnostics_panel'; Binary = 'test_diagnostics_panel'; Test = 'mt108_argus_diagnostics_panel_real_server_loop' }
)
$expectedCommands = [ordered]@{}

foreach ($surface in $surfaceTests) {
    $arguments = @('test', '--test', $surface.Binary, $surface.Test, '--', '--exact', '--nocapture')
    $expectedCommands[$surface.Scenario] = $arguments
    Invoke-BoundedCargoTest -Scenario $surface.Scenario -CargoArguments $arguments
}

$verifierArguments = @(
    'test', '--test', 'test_mt108_argus_aggregate', 'mt108_verify_argus_evidence_exact_seven', '--', '--ignored', '--exact', '--nocapture'
)
$expectedCommands['exact_seven_verifier'] = $verifierArguments
Invoke-BoundedCargoTest -Scenario 'exact_seven_verifier' -CargoArguments $verifierArguments
Assert-FinalProcessReceipts -ExpectedCommands $expectedCommands

Write-Output "MT-108 Argus proof closed for run_id=$RunId; artifacts=$runDir; successful_processes=8"
