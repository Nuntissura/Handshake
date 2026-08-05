[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidatePattern('^MT046-RUN-[A-Za-z0-9_-]{1,118}$')]
    [string]$RunId,

    [ValidateRange(300, 1800)]
    [int]$CommandTimeoutSeconds = 1200,

    [ValidateRange(900, 7200)]
    [int]$WholeRunTimeoutSeconds = 5400,

    [string]$PostgresDsn = 'postgresql://postgres@127.0.0.1:5544/handshake_wp_kernel_012_mt_046',

    [ValidatePattern('^MT046-RUN-[A-Za-z0-9_-]{1,118}$')]
    [string]$BaselineRunId,

    [switch]$DiagnosticsSelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-FileSha256 {
    param([Parameter(Mandatory)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Cannot hash missing file '$Path'"
    }
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
    do {
        try {
            return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path -ErrorAction Stop).Hash.ToLowerInvariant()
        }
        catch [IO.IOException] {
            if ([DateTimeOffset]::UtcNow -ge $deadline) {
                throw "Cannot hash '$Path' after waiting 10s for the completed process to release it: $($_.Exception.Message)"
            }
            Start-Sleep -Milliseconds 50
        }
        catch [UnauthorizedAccessException] {
            if ([DateTimeOffset]::UtcNow -ge $deadline) {
                throw "Cannot hash '$Path' after waiting 10s for the completed process to release it: $($_.Exception.Message)"
            }
            Start-Sleep -Milliseconds 50
        }
    } while ($true)
}

function Get-TextSha256 {
    param([Parameter(Mandatory)][AllowEmptyString()][string]$Text)
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.UTF8Encoding]::new($false).GetBytes($Text)
        return ([BitConverter]::ToString($hasher.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    }
    finally { $hasher.Dispose() }
}

function Get-ComparableFullPath {
    param([Parameter(Mandatory)][string]$Path)
    $candidate = $Path
    if ($candidate.StartsWith('\\?\', [StringComparison]::Ordinal)) {
        $candidate = $candidate.Substring(4)
    }
    return [IO.Path]::GetFullPath($candidate).TrimEnd('\')
}

function Write-JsonAtomic {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)]$Value)
    [void][IO.Directory]::CreateDirectory((Split-Path $Path -Parent))
    $temporary = "$Path.$([guid]::NewGuid().ToString('N')).tmp"
    [IO.File]::WriteAllText(
        $temporary,
        (($Value | ConvertTo-Json -Depth 64) + [Environment]::NewLine),
        [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $Path -Force
}

function Write-ImmutableJson {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)]$Value)
    if (Test-Path -LiteralPath $Path) {
        throw "Immutable MT-046 artifact already exists: '$Path'"
    }
    [void][IO.Directory]::CreateDirectory((Split-Path $Path -Parent))
    $json = (($Value | ConvertTo-Json -Depth 64) + [Environment]::NewLine)
    $bytes = [Text.UTF8Encoding]::new($false).GetBytes($json)
    $stream = [IO.File]::Open($Path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
    try {
        $stream.Write($bytes, 0, $bytes.Length)
        $stream.Flush($true)
    }
    finally {
        $stream.Dispose()
    }
    $digest = Get-FileSha256 -Path $Path
    $sidecar = "$Path.sha256"
    [IO.File]::WriteAllText(
        $sidecar,
        "$digest  $(Split-Path $Path -Leaf)$([Environment]::NewLine)",
        [Text.UTF8Encoding]::new($false))
    return $digest
}

function Get-SanitizedPostgresIdentity {
    param([Parameter(Mandatory)][string]$Dsn)
    $uri = [Uri]$Dsn
    if ($uri.Scheme -notin @('postgres', 'postgresql')) {
        throw 'MT-046 requires a PostgreSQL DSN'
    }
    $database = $uri.AbsolutePath.TrimStart('/')
    if ([string]::IsNullOrWhiteSpace($database)) {
        throw 'MT-046 PostgreSQL DSN requires a database name'
    }
    return "$($uri.Scheme)://$($uri.Host):$($uri.Port)/$database"
}

function Assert-ExactScenarioSet {
    param([Parameter(Mandatory)]$State, [Parameter(Mandatory)][string[]]$ExpectedIds)
    $properties = @($State.scenarios.PSObject.Properties)
    $actualIds = @($properties.Name | Sort-Object)
    $expected = @($ExpectedIds | Sort-Object)
    if (($actualIds -join "`n") -cne ($expected -join "`n")) {
        throw "MT-046 scenario set mismatch: expected='$($expected -join ',')'; actual='$($actualIds -join ',')'"
    }
    foreach ($property in $properties) {
        $status = [string]$property.Value.status
        $reason = [string]$property.Value.terminal_reason
        if ($property.Name -ceq 'IC-13' -and $status -ceq 'SKIPPED' -and
            $reason -ceq 'HSK-409-LOOM-AI-NO-MODEL') { continue }
        if ($property.Name -cne 'IC-13' -and $status -ceq 'PASS') { continue }
        throw "MT-046 scenario $($property.Name) has forbidden terminal disposition '$status'/'$reason'"
    }
}

function Assert-Png {
    param([Parameter(Mandatory)][string]$Path)
    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 24 -or $bytes[0] -ne 137 -or $bytes[1] -ne 80 -or
        $bytes[2] -ne 78 -or $bytes[3] -ne 71) {
        throw "Argus screenshot is not a material PNG: '$Path'"
    }
    $width = [Net.IPAddress]::NetworkToHostOrder([BitConverter]::ToInt32($bytes, 16))
    $height = [Net.IPAddress]::NetworkToHostOrder([BitConverter]::ToInt32($bytes, 20))
    if ($width -lt 320 -or $height -lt 200) {
        throw "Argus screenshot dimensions are too small: ${width}x${height} '$Path'"
    }
    return [ordered]@{ path = $Path; width = $width; height = $height; sha256 = Get-FileSha256 $Path }
}

function Get-JsonPropertyValues {
    param(
        [Parameter(Mandatory)][AllowNull()]$Value,
        [Parameter(Mandatory)][string]$PropertyName
    )
    if ($null -eq $Value) { return }
    if ($Value -is [Collections.IDictionary]) {
        foreach ($key in $Value.Keys) {
            if ([string]$key -ceq $PropertyName) { Write-Output $Value[$key] }
            Get-JsonPropertyValues -Value $Value[$key] -PropertyName $PropertyName
        }
        return
    }
    if ($Value -is [Collections.IEnumerable] -and $Value -isnot [string]) {
        foreach ($item in $Value) { Get-JsonPropertyValues -Value $item -PropertyName $PropertyName }
        return
    }
    foreach ($property in @($Value.PSObject.Properties)) {
        if ($property.Name -ceq $PropertyName) { Write-Output $property.Value }
        Get-JsonPropertyValues -Value $property.Value -PropertyName $PropertyName
    }
}

function Get-RepoForbiddenDirectories {
    param([Parameter(Mandatory)][string]$Repository)
    return @(Get-ChildItem -LiteralPath $Repository -Directory -Recurse -Force -ErrorAction Stop |
        Where-Object {
            $_.Name -in @('target', 'test_output', 'screenshots') -and
            $_.FullName -notlike "*\.git\*"
        } | ForEach-Object { [IO.Path]::GetFullPath($_.FullName) } | Sort-Object -Unique)
}

$crateRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$repoRoot = (& git -C $crateRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw 'Unable to resolve the MT-046 product repository root'
}
$sourceSha = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceSha -notmatch '^[0-9a-f]{40}$') {
    throw "Unable to bind MT-046 to an exact source SHA; got '$sourceSha'"
}

$artifactRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot '..\Handshake_Artifacts'))
if (-not (Test-Path -LiteralPath $artifactRoot -PathType Container)) {
    throw "Canonical Handshake_Artifacts root is unavailable: '$artifactRoot'"
}
$targetRoot = Join-Path $artifactRoot 'handshake-cargo-target\wp-kernel-012-mt-046'
$testArtifactRoot = Join-Path $artifactRoot 'handshake-test'
$wpTestRoot = Join-Path $testArtifactRoot 'wp-kernel-012'
$measurementRoot = Join-Path $wpTestRoot 'mt-046\measurements'
$supervisorRoot = Join-Path $wpTestRoot 'mt-046\supervisor'
$legacyMeasurementRoot = Join-Path $artifactRoot 'wp-kernel-012\mt-046\measurements'
$runRoot = Join-Path $supervisorRoot "runs\$RunId"
$argusRoot = Join-Path $runRoot 'argus'
$argusRunRoot = Join-Path $argusRoot $RunId
$backendBindingRoot = Join-Path $wpTestRoot "mt-046\backend-bindings\$RunId"
$externalArgusWorkspaceRoot = Join-Path $wpTestRoot "mt-046\canonical-argus\$RunId"
$successRuntimeRoot = Join-Path $wpTestRoot "mt-045\success-runtime\$RunId"
$legacyRunIdPaths = @(
    (Join-Path $legacyMeasurementRoot "runs\$RunId.json"),
    (Join-Path $artifactRoot "wp-kernel-012\mt-046\supervisor\runs\$RunId"),
    (Join-Path $artifactRoot "wp-kernel-012\mt-046\backend-bindings\$RunId"),
    (Join-Path $artifactRoot "wp-kernel-012\mt-046\canonical-argus\$RunId")
)
$currentRunPath = Join-Path $measurementRoot 'current-run.json'
$latestRunPath = Join-Path $measurementRoot 'latest-run-summary.json'
$immutableRunPath = Join-Path $measurementRoot "runs\$RunId.json"
$supervisorSummaryPath = Join-Path $runRoot 'supervisor-summary.json'
$wholeRunDeadline = [DateTimeOffset]::UtcNow.AddSeconds($WholeRunTimeoutSeconds)
$baselineRunPath = $null
$baselineRunSha256 = $null
if (-not [string]::IsNullOrWhiteSpace($BaselineRunId)) {
    if ($BaselineRunId -ceq $RunId) { throw 'MT-046 baseline and current RunId must be distinct' }
    $baselineRunPath = Join-Path $measurementRoot "runs\$BaselineRunId.json"
    if (-not (Test-Path -LiteralPath $baselineRunPath -PathType Leaf)) {
        throw "MT-046 baseline immutable run is missing: '$baselineRunPath'"
    }
    $baselineRunSha256 = Get-FileSha256 $baselineRunPath
    $baselineRecordedSha = ((Get-Content -LiteralPath "$baselineRunPath.sha256" -Raw).Trim() -split '\s+')[0]
    if ($baselineRecordedSha -cne $baselineRunSha256) {
        throw 'MT-046 baseline immutable run digest is invalid'
    }
}

[void][IO.Directory]::CreateDirectory($supervisorRoot)
$supervisorLockPath = Join-Path $supervisorRoot 'supervisor.lock'
$supervisorLockStream = $null
for ($lockAttempt = 0; $lockAttempt -lt 2 -and $null -eq $supervisorLockStream; $lockAttempt++) {
    try {
        $supervisorLockStream = [IO.FileStream]::new(
            $supervisorLockPath,
            [IO.FileMode]::CreateNew,
            [IO.FileAccess]::ReadWrite,
            [IO.FileShare]::Read,
            4096,
            [IO.FileOptions]::DeleteOnClose)
        $lockOwner = [ordered]@{
            run_id = $RunId
            pid = $PID
            process_start_time_utc = ([DateTimeOffset](Get-Process -Id $PID).StartTime).ToUniversalTime().ToString('O')
            acquired_at = [DateTimeOffset]::UtcNow.ToString('O')
        }
        $lockBytes = [Text.UTF8Encoding]::new($false).GetBytes(
            (($lockOwner | ConvertTo-Json -Compress) + [Environment]::NewLine))
        $supervisorLockStream.Write($lockBytes, 0, $lockBytes.Length)
        $supervisorLockStream.Flush($true)
    }
    catch [IO.IOException] {
        $owner = try { Get-Content -LiteralPath $supervisorLockPath -Raw | ConvertFrom-Json } catch { $null }
        $liveOwner = if ($null -ne $owner -and $null -ne $owner.pid) {
            Get-Process -Id ([int]$owner.pid) -ErrorAction SilentlyContinue
        } else { $null }
        if ($null -ne $liveOwner) {
            throw "Another live MT-046 supervisor owns the run gate: run_id='$($owner.run_id)' pid=$($owner.pid)"
        }
        if ($lockAttempt -eq 0 -and (Test-Path -LiteralPath $supervisorLockPath)) {
            Remove-Item -LiteralPath $supervisorLockPath -Force
            continue
        }
        throw 'Unable to acquire the exclusive MT-046 supervisor ownership lock'
    }
}

if ((Test-Path -LiteralPath $runRoot) -or (Test-Path -LiteralPath $immutableRunPath) -or
    (Test-Path -LiteralPath $backendBindingRoot) -or
    (Test-Path -LiteralPath $externalArgusWorkspaceRoot) -or
    (Test-Path -LiteralPath $successRuntimeRoot)) {
    throw "RunId '$RunId' is not fresh; immutable MT-046 evidence already exists"
}
if (@($legacyRunIdPaths | Where-Object { Test-Path -LiteralPath $_ }).Count -ne 0) {
    throw "RunId '$RunId' already exists in the legacy MT-046 evidence generation"
}
[void][IO.Directory]::CreateDirectory($runRoot)
[void][IO.Directory]::CreateDirectory($argusRoot)
$forbiddenRepoDirectoriesBefore = @(Get-RepoForbiddenDirectories -Repository $repoRoot)

trap {
    $preflightFailure = $_
    try {
        $preflightFailed = [ordered]@{
            schema_id = 'hsk.wp_kernel_012.interconnection_run@2'
            work_packet_id = 'WP-KERNEL-012'
            micro_task_id = 'MT-046'
            run_id = $RunId
            source_sha = $sourceSha
            status = 'FAIL'
            phase = 'NEVER_STARTED_OR_PREFLIGHT'
            terminal_reason = $preflightFailure.Exception.Message
            completed_at = [DateTimeOffset]::UtcNow.ToString('O')
        }
        if (-not (Test-Path -LiteralPath $immutableRunPath)) {
            $preflightImmutableSha = Write-ImmutableJson -Path $immutableRunPath -Value $preflightFailed
            $preflightFailed['immutable_run_path'] = $immutableRunPath
            $preflightFailed['immutable_run_sha256'] = $preflightImmutableSha
        }
        Write-JsonAtomic -Path $currentRunPath -Value $preflightFailed
        Write-JsonAtomic -Path $latestRunPath -Value $preflightFailed
        if (-not (Test-Path -LiteralPath $supervisorSummaryPath)) {
            [void](Write-ImmutableJson -Path $supervisorSummaryPath -Value $preflightFailed)
        }
    }
    finally {
        if ($null -ne $supervisorLockStream) { $supervisorLockStream.Dispose() }
    }
    Write-Error $preflightFailure
    exit 1
}

$priorProjectionArchive = [Collections.Generic.List[object]]::new()
$priorProjectionSources = @(
    [ordered]@{ path = $currentRunPath; archive_name = 'current-run.json' },
    [ordered]@{ path = $latestRunPath; archive_name = 'latest-run-summary.json' },
    [ordered]@{ path = (Join-Path $legacyMeasurementRoot 'current-run.json'); archive_name = 'legacy-current-run.json' },
    [ordered]@{ path = (Join-Path $legacyMeasurementRoot 'latest-run-summary.json'); archive_name = 'legacy-latest-run-summary.json' }
)
foreach ($priorSource in $priorProjectionSources) {
    $priorPath = [string]$priorSource.path
    if (Test-Path -LiteralPath $priorPath -PathType Leaf) {
        $priorText = Get-Content -LiteralPath $priorPath -Raw
        $prior = $priorText | ConvertFrom-Json
        if ($prior.run_id -ceq $RunId) {
            throw "RunId '$RunId' already appears in projection '$priorPath'"
        }
        $abandonedReceipt = $null
        if ($prior.status -ceq 'RUNNING') {
            $priorSupervisorPid = $prior.provenance.supervisor_pid
            $livePriorSupervisor = if ($null -ne $priorSupervisorPid) {
                Get-Process -Id ([int]$priorSupervisorPid) -ErrorAction SilentlyContinue
            } else { $null }
            if ($null -ne $livePriorSupervisor) {
                throw "Prior RUNNING projection '$priorPath' still has a live supervisor pid=$priorSupervisorPid"
            }
            $abandonedPath = Join-Path $runRoot "prior-projections\abandoned-$($priorSource.archive_name)"
            $abandonedSha = Write-ImmutableJson -Path $abandonedPath -Value ([ordered]@{
                schema_id = 'hsk.wp_kernel_012.mt046_abandoned_run@1'
                run_id = [string]$prior.run_id
                status = 'FAIL'
                terminal_reason = 'ABANDONED_DEAD_SUPERVISOR'
                prior_projection_path = $priorPath
                prior_projection_sha256 = Get-FileSha256 $priorPath
                prior_supervisor_pid = $priorSupervisorPid
                recovered_by_run_id = $RunId
                completed_at = [DateTimeOffset]::UtcNow.ToString('O')
            })
            $abandonedReceipt = [ordered]@{ path = $abandonedPath; sha256 = $abandonedSha }
        }
        $archivePath = Join-Path $runRoot "prior-projections\$($priorSource.archive_name)"
        $archiveSha = Write-ImmutableJson -Path $archivePath -Value ([ordered]@{
            source_path = $priorPath
            source_sha256 = Get-FileSha256 $priorPath
            archived_at = [DateTimeOffset]::UtcNow.ToString('O')
            projection = $prior
        })
        $priorProjectionArchive.Add([ordered]@{
            path = $archivePath; sha256 = $archiveSha; abandonment = $abandonedReceipt
        })
    }
}
$attemptRoot = Join-Path $measurementRoot 'attempts'
$attemptRunMatches = if (Test-Path -LiteralPath $attemptRoot) {
    @(Get-ChildItem -LiteralPath $attemptRoot -File |
        Select-String -SimpleMatch $RunId -List)
} else { @() }
if (@($attemptRunMatches).Count -ne 0) {
    throw "RunId '$RunId' already appears in immutable attempt history"
}

# The existing MT-045 supervisor owns the repository's already-adversarially-tested CREATE_SUSPENDED ->
# Job Object -> resume -> bounded wait/reap implementation. Extract only that embedded C# type, bind its
# helper hash into this run, and reject any source-id drift rather than duplicating a second containment core.
$jobHelperPath = Join-Path $PSScriptRoot 'run_mt045_perf_proof.ps1'
$jobHelperSha256 = Get-FileSha256 $jobHelperPath
$jobHelperText = Get-Content -LiteralPath $jobHelperPath -Raw
$jobSourceMatch = [regex]::Match(
    $jobHelperText,
    '(?s)\$mt045JobRunnerSource\s*=\s*@''\r?\n(?<source>.*?)\r?\n''@')
if (-not $jobSourceMatch.Success -or
    $jobSourceMatch.Groups['source'].Value -notmatch
        'public const string SourceId = "mt045-job-runner-20260802-v7";') {
    throw 'The reviewed Windows Job Object helper source is missing or drifted'
}
if (-not ('Mt045JobRunner' -as [type])) {
    Add-Type -Language CSharp -TypeDefinition $jobSourceMatch.Groups['source'].Value
}
if ([Mt045JobRunner]::SourceId -cne 'mt045-job-runner-20260802-v7') {
    throw 'A stale Windows Job Object helper is loaded in this PowerShell host'
}

if ($DiagnosticsSelfTest) {
    function Assert-DiagnosticBinding {
        param([Parameter(Mandatory)]$Receipt)
        foreach ($required in @('run_id', 'source_sha', 'scenario_id', 'attempt_id', 'test_thread')) {
            if ([string]::IsNullOrWhiteSpace([string]$Receipt.$required)) {
                throw "missing binding '$required'"
            }
        }
        if ($Receipt.run_id -cne 'RUN-VALID' -or $Receipt.source_sha -cne ('a' * 40) -or
            $Receipt.scenario_id -cne 'IC-01' -or
            $Receipt.test_thread -cne 'interconnect_ic01_ckc_image_into_note') {
            throw 'stale/cross-run/wrong-source receipt'
        }
    }
    function Assert-DiagnosticUniqueIds {
        param([Parameter(Mandatory)][object[]]$Receipts)
        if (@($Receipts.scenario_id | Sort-Object -Unique).Count -ne $Receipts.Count -or
            @($Receipts.attempt_id | Sort-Object -Unique).Count -ne $Receipts.Count) {
            throw 'duplicate scenario/attempt ids'
        }
    }
    function Expect-DiagnosticRejection {
        param([Parameter(Mandatory)][scriptblock]$Action, [Parameter(Mandatory)][string]$Label)
        try { & $Action; throw "$Label unexpectedly accepted" }
        catch { if ($_.Exception.Message -eq "$Label unexpectedly accepted") { throw } }
        return $Label
    }

    $valid = [pscustomobject]@{
        run_id = 'RUN-VALID'; source_sha = ('a' * 40); scenario_id = 'IC-01';
        attempt_id = 'ATTEMPT-1'; test_thread = 'interconnect_ic01_ckc_image_into_note'
    }
    Assert-DiagnosticBinding $valid
    $negativeResults = [Collections.Generic.List[string]]::new()
    foreach ($case in @(
        @{ label = 'stale_current_running'; mutate = { param($r) $r.run_id = 'RUN-OLD' } },
        @{ label = 'cross_run_receipt'; mutate = { param($r) $r.run_id = 'RUN-OTHER' } },
        @{ label = 'wrong_source_binding'; mutate = { param($r) $r.source_sha = ('b' * 40) } },
        @{ label = 'missing_source_binding'; mutate = { param($r) $r.source_sha = $null } }
    )) {
        $tampered = $valid | Select-Object *
        & $case.mutate $tampered
        $negativeResults.Add((Expect-DiagnosticRejection -Label $case.label -Action {
            Assert-DiagnosticBinding $tampered
        }))
    }
    $duplicate = @($valid, ($valid | Select-Object *))
    $negativeResults.Add((Expect-DiagnosticRejection -Label 'duplicate_scenario_attempt_ids' -Action {
        Assert-DiagnosticUniqueIds $duplicate
    }))

    $diagnosticsRoot = Join-Path $runRoot 'diagnostics-self-test'
    [void][IO.Directory]::CreateDirectory($diagnosticsRoot)
    $cmd = (Get-Command cmd.exe -CommandType Application).Source
    $failed = [Mt045JobRunner]::Run(
        $cmd, [string[]]@('/d', '/c', 'exit 7'), $diagnosticsRoot,
        (Join-Path $diagnosticsRoot 'failed.stdout.log'),
        (Join-Path $diagnosticsRoot 'failed.stderr.log'), 5000, 500)
    if ($failed.ExitCode -ne 7 -or $failed.TimedOut -or $failed.LeakedProcessCount -ne 0) {
        throw 'failed-command diagnostic did not retain exact terminal failure'
    }
    $negativeResults.Add('failed_command_terminal_receipt')

    $powershell = (Get-Command powershell.exe -CommandType Application).Source
    $timeoutScript = 'Start-Process powershell.exe -WindowStyle Hidden -ArgumentList ''-NoProfile'',''-NonInteractive'',''-Command'',''Start-Sleep 30''; Start-Sleep 30'
    $timedOut = [Mt045JobRunner]::Run(
        $powershell, [string[]]@('-NoProfile', '-NonInteractive', '-Command', $timeoutScript),
        $diagnosticsRoot, (Join-Path $diagnosticsRoot 'timeout.stdout.log'),
        (Join-Path $diagnosticsRoot 'timeout.stderr.log'), 1000, 500)
    if (-not $timedOut.TimedOut -or @($timedOut.PreCleanupDescendantProcessIds).Count -lt 1 -or
        @($timedOut.PostDrainDescendantProcessIds).Count -ne 0) {
        throw 'timeout diagnostic did not capture and reap the descendant tree'
    }
    $negativeResults.Add('timeout_descendant_captured_and_reaped')

    $fake = [pscustomobject]@{ scenarios = [pscustomobject]@{} }
    foreach ($id in 1..18 | ForEach-Object { 'IC-{0:D2}' -f $_ }) {
        $fake.scenarios | Add-Member -NotePropertyName $id -NotePropertyValue (
            [pscustomobject]@{ status = 'PASS'; terminal_reason = $null })
    }
    $fake.scenarios.'IC-13'.status = 'SKIPPED'
    $fake.scenarios.'IC-13'.terminal_reason = 'HSK-409-LOOM-AI-NO-MODEL'
    Assert-ExactScenarioSet $fake @(1..18 | ForEach-Object { 'IC-{0:D2}' -f $_ })
    $fake.scenarios.'IC-13'.terminal_reason = 'UNAUTHORIZED'
    $negativeResults.Add((Expect-DiagnosticRejection -Label 'unauthorized_skip' -Action {
        Assert-ExactScenarioSet $fake @(1..18 | ForEach-Object { 'IC-{0:D2}' -f $_ })
    }))

    $diagnosticPath = Join-Path $diagnosticsRoot 'diagnostics-summary.json'
    $diagnosticSha = Write-ImmutableJson -Path $diagnosticPath -Value ([ordered]@{
        run_id = $RunId; status = 'PASS'; negative_cases = $negativeResults;
        failed_command = $failed; timeout_reap = $timedOut;
        completed_at = [DateTimeOffset]::UtcNow.ToString('O')
    })
    Write-Output ([ordered]@{
        status = 'PASS'; diagnostics = $diagnosticPath; diagnostics_sha256 = $diagnosticSha;
        negative_cases = $negativeResults
    } | ConvertTo-Json -Depth 8)
    $supervisorLockStream.Dispose()
    exit 0
}

$sourceBoundPaths = @(
    'Cargo.lock',
    '.cargo/config.toml',
    'src/frontend/handshake_native/Cargo.toml',
    'src/frontend/handshake_native/Cargo.lock',
    'src/frontend/handshake_native/build.rs',
    'src/frontend/handshake_native/src',
    'src/frontend/handshake_native/tests/interconnect_support',
    'src/frontend/handshake_native/tests/test_interconnect_ckc_to_note.rs',
    'src/frontend/handshake_native/tests/test_interconnect_note_code_crossref.rs',
    'src/frontend/handshake_native/tests/test_interconnect_loom_backlink_search.rs',
    'src/frontend/handshake_native/tests/test_interconnect_shared_undo_ledger.rs',
    'src/frontend/handshake_native/tests/test_interconnect_manifest.json',
    'src/frontend/handshake_native/tests/run_mt046_interconnect_proof.ps1',
    'src/frontend/handshake_native/tests/run_mt045_perf_proof.ps1',
    'src/frontend/handshake_native/tests/native_gui_support/canonical_argus_driver.rs',
    'src/frontend/handshake_native/tests/native_gui_support/screenshot_harness.rs',
    'src/frontend/handshake_native/tests/native_gui_support/screenshot_marker.rs',
    'src/frontend/handshake_native/tests/pg_proof_support/mod.rs',
    'src/backend',
    'src/shared'
)
$dirtyRows = @(& git -C $repoRoot status --porcelain=v1 --untracked-files=all -- $sourceBoundPaths)
if ($LASTEXITCODE -ne 0) { throw 'Unable to inspect MT-046 controlled candidate state' }
$controlledPatch = ((& git -C $repoRoot diff --binary HEAD -- $sourceBoundPaths) | Out-String)
if ($LASTEXITCODE -ne 0) { throw 'Unable to capture MT-046 controlled candidate patch' }
$controlledPatchPath = Join-Path $runRoot 'candidate-source.patch'
$controlledPatchBytes = [Text.UTF8Encoding]::new($false).GetBytes($controlledPatch)
$controlledPatchStream = [IO.File]::Open(
    $controlledPatchPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
try {
    $controlledPatchStream.Write($controlledPatchBytes, 0, $controlledPatchBytes.Length)
    $controlledPatchStream.Flush($true)
}
finally { $controlledPatchStream.Dispose() }
$controlledPatchSha256 = Get-FileSha256 $controlledPatchPath
$untrackedBindings = @($dirtyRows | Where-Object { $_ -match '^\?\? ' } | ForEach-Object {
    $relative = $_.Substring(3)
    $absolute = Join-Path $repoRoot $relative
    if (-not (Test-Path -LiteralPath $absolute -PathType Leaf)) {
        throw "Controlled untracked input is not a hashable file: '$relative'"
    }
    [ordered]@{ path = $relative; sha256 = Get-FileSha256 $absolute }
})
$candidateIdentityMaterial = [ordered]@{
    head_sha = $sourceSha
    controlled_paths = $sourceBoundPaths
    status_rows = @($dirtyRows)
    binary_patch_sha256 = $controlledPatchSha256
    untracked_files = $untrackedBindings
}
$candidateDescriptor = [ordered]@{
    identity_material = $candidateIdentityMaterial
    binary_patch_path = $controlledPatchPath
}
$candidateSourceId = "$sourceSha-$((Get-TextSha256 ($candidateIdentityMaterial | ConvertTo-Json -Compress -Depth 16)).Substring(0, 16))"
$candidateBindingPath = Join-Path $runRoot 'candidate-source-binding.json'
$candidateBindingSha256 = Write-ImmutableJson -Path $candidateBindingPath -Value $candidateDescriptor
$sourceDirtyResultSha256 = Get-TextSha256 (@($dirtyRows) -join "`n")

$manifestPath = Join-Path $PSScriptRoot 'test_interconnect_manifest.json'
$manifestSha256 = Get-FileSha256 $manifestPath
$manifest = @(Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json)
$expectedScenarioIds = @(1..18 | ForEach-Object { 'IC-{0:D2}' -f $_ })
$manifestIds = @($manifest | ForEach-Object { [string]$_.scenario_id } | Sort-Object)
if ($manifest.Count -ne 18 -or ($manifestIds -join "`n") -cne (($expectedScenarioIds | Sort-Object) -join "`n")) {
    throw 'MT-046 manifest must contain the exact unique IC-01..IC-18 scenario set'
}
$scenarioSourceFiles = @(
    'test_interconnect_ckc_to_note.rs',
    'test_interconnect_note_code_crossref.rs',
    'test_interconnect_loom_backlink_search.rs',
    'test_interconnect_shared_undo_ledger.rs'
)
$discoveredProofFunctions = @($scenarioSourceFiles | ForEach-Object {
    $sourcePath = Join-Path $PSScriptRoot $_
    [regex]::Matches((Get-Content -LiteralPath $sourcePath -Raw), '(?m)^fn (interconnect_[A-Za-z0-9_]+)\(\)') |
        ForEach-Object { $_.Groups[1].Value }
})
$expectedProofsByBinary = @{}
foreach ($sourceFile in $scenarioSourceFiles) {
    $binary = [IO.Path]::GetFileNameWithoutExtension($sourceFile)
    $expectedProofsByBinary[$binary] = @([regex]::Matches(
        (Get-Content -LiteralPath (Join-Path $PSScriptRoot $sourceFile) -Raw),
        '(?m)^fn (interconnect_[A-Za-z0-9_]+)\(\)') | ForEach-Object { $_.Groups[1].Value })
}
$proofToBinary = @{}
foreach ($binary in $expectedProofsByBinary.Keys) {
    foreach ($proofFunction in @($expectedProofsByBinary[$binary])) {
        $proofToBinary[$proofFunction] = $binary
    }
}
$manifestProofFunctions = @($manifest | ForEach-Object { [string]$_.proof_fn })
if ($discoveredProofFunctions.Count -ne 18 -or
    @($discoveredProofFunctions | Sort-Object -Unique).Count -ne 18 -or
    (($discoveredProofFunctions | Sort-Object) -join "`n") -cne
        (($manifestProofFunctions | Sort-Object) -join "`n")) {
    throw 'MT-046 source discovery must find exactly the manifest one-to-one set of 18 interconnect_ proof functions'
}
$discoveryPath = Join-Path $runRoot 'canonical-test-discovery.json'
$discoverySha256 = Write-ImmutableJson -Path $discoveryPath -Value ([ordered]@{
    schema_id = 'hsk.wp_kernel_012.mt046_test_discovery@1'
    run_id = $RunId
    source_sha = $sourceSha
    source_files = $scenarioSourceFiles
    manifest_sha256 = $manifestSha256
    proof_functions = @($discoveredProofFunctions | Sort-Object)
    exact_count = 18
    status = 'PASS'
})

$postgresIdentity = Get-SanitizedPostgresIdentity $PostgresDsn
$pgUri = [Uri]$PostgresDsn
$listener = Get-NetTCPConnection -LocalAddress $pgUri.Host -LocalPort $pgUri.Port -State Listen |
    Select-Object -First 1
if ($null -eq $listener) { throw "PostgreSQL is not listening at $postgresIdentity" }
$postgresProcess = Get-Process -Id $listener.OwningProcess
if (-not $postgresProcess.ProcessName.Equals('postgres', [StringComparison]::OrdinalIgnoreCase)) {
    throw "The PostgreSQL endpoint is owned by '$($postgresProcess.ProcessName)', not postgres"
}
$postgresExecutable = [IO.Path]::GetFullPath([string]$postgresProcess.Path)
$psql = Join-Path (Split-Path $postgresExecutable -Parent) 'psql.exe'
if (-not (Test-Path -LiteralPath $psql -PathType Leaf)) {
    throw "The verified PostgreSQL runtime has no sibling psql executable: '$psql'"
}
$psql = [IO.Path]::GetFullPath($psql)
$psqlSha256 = Get-FileSha256 $psql
$postgresReceipt = [ordered]@{
    dsn = $postgresIdentity
    pid = [int]$postgresProcess.Id
    process_name = $postgresProcess.ProcessName
    start_time_utc = ([DateTimeOffset]$postgresProcess.StartTime).ToUniversalTime().ToString('O')
    executable = $postgresExecutable
    psql_path = $psql
    psql_sha256 = $psqlSha256
    lifecycle = 'existing_internal_postgresql_preserved'
}

$cargo = (Get-Command cargo -CommandType Application).Source
$commands = [Collections.Generic.List[object]]::new()
function Invoke-ContainedCommand {
    param(
        [Parameter(Mandatory)][string]$Label,
        [Parameter(Mandatory)][string]$Executable,
        [Parameter(Mandatory)][string[]]$Arguments,
        [Parameter(Mandatory)][string]$WorkingDirectory,
        [Parameter(Mandatory)][string]$LogRoot,
        [ValidateRange(500, 300000)][int]$DescendantExitGraceMilliseconds = 5000
    )
    [void][IO.Directory]::CreateDirectory($LogRoot)
    $stdoutPath = Join-Path $LogRoot "$Label.stdout.log"
    $stderrPath = Join-Path $LogRoot "$Label.stderr.log"
    $startedAt = [DateTimeOffset]::UtcNow
    $remainingMilliseconds = [int][Math]::Floor(($wholeRunDeadline - [DateTimeOffset]::UtcNow).TotalMilliseconds)
    if ($remainingMilliseconds -le 0) {
        throw "MT-046 whole-run deadline exhausted before '$Label' started"
    }
    $commandBudgetMilliseconds = [Math]::Min($CommandTimeoutSeconds * 1000, $remainingMilliseconds)
    $result = [Mt045JobRunner]::Run(
        $Executable, $Arguments, $WorkingDirectory, $stdoutPath, $stderrPath,
        $commandBudgetMilliseconds, $DescendantExitGraceMilliseconds)
    $receipt = [ordered]@{
        label = $Label
        executable = $Executable
        arguments = $Arguments
        working_directory = $WorkingDirectory
        root_process_id = [int]$result.RootProcessId
        started_at = $startedAt.ToString('O')
        completed_at = [DateTimeOffset]::UtcNow.ToString('O')
        exit_code = [int]$result.ExitCode
        timed_out = [bool]$result.TimedOut
        leaked_process_count = [int]$result.LeakedProcessCount
        leaked_process_ids = @($result.LeakedProcessIds)
        pre_cleanup_descendant_process_ids = @($result.PreCleanupDescendantProcessIds)
        post_drain_descendant_process_ids = @($result.PostDrainDescendantProcessIds)
        stdout_path = $stdoutPath
        stdout_sha256 = Get-FileSha256 $stdoutPath
        stderr_path = $stderrPath
        stderr_sha256 = Get-FileSha256 $stderrPath
    }
    return $receipt
}

function Assert-ContainedCommandPassed {
    param([Parameter(Mandatory)]$Receipt)
    if ($Receipt.timed_out -or $Receipt.leaked_process_count -ne 0 -or
        @($Receipt.post_drain_descendant_process_ids).Count -ne 0 -or $Receipt.exit_code -ne 0) {
        throw "Contained command '$($Receipt.label)' failed: exit=$($Receipt.exit_code); timeout=$($Receipt.timed_out); leaks=$($Receipt.leaked_process_count)"
    }
}

function Start-ProcessObservationJob {
    param(
        [Parameter(Mandatory)][string]$MarkerPath,
        [Parameter(Mandatory)][string]$AckPath,
        [Parameter(Mandatory)][string]$CorrelationId,
        [Parameter(Mandatory)][string]$ExpectedExecutable,
        [Parameter(Mandatory)][string]$ObservationRunId,
        [Parameter(Mandatory)][string]$ObservationSourceSha,
        [Parameter(Mandatory)][string]$ScenarioId
    )
    if (Test-Path -LiteralPath $AckPath) {
        throw "Refusing pre-existing Argus process-observation ACK: '$AckPath'"
    }
    return Start-Job -ScriptBlock {
        param($MarkerPath, $AckPath, $CorrelationId, $ExpectedExecutable, $ObservationRunId,
            $ObservationSourceSha, $ScenarioId)
        $deadline = [DateTimeOffset]::UtcNow.AddMinutes(30)
        while ([DateTimeOffset]::UtcNow -lt $deadline) {
            if (Test-Path -LiteralPath $MarkerPath -PathType Leaf) {
                foreach ($line in @(Get-Content -LiteralPath $MarkerPath -ErrorAction SilentlyContinue)) {
                    $marker = try { $line | ConvertFrom-Json } catch { $null }
                    if ($null -eq $marker -or $marker.process_correlation_id -cne $CorrelationId -or
                        $marker.run_id -cne $ObservationRunId -or
                        $marker.source_sha -cne $ObservationSourceSha -or
                        $marker.process_scenario_id -cne $ScenarioId) { continue }
                    $process = Get-Process -Id ([int]$marker.process_id) -ErrorAction Stop
                    $actualExecutable = [IO.Path]::GetFullPath($process.Path)
                    if (-not $actualExecutable.Equals(
                        [IO.Path]::GetFullPath($ExpectedExecutable),
                        [StringComparison]::OrdinalIgnoreCase)) {
                        throw "Observed Argus process executable mismatch: '$actualExecutable'"
                    }
                    $ack = [ordered]@{
                        schema_id = 'hsk.native_gui.process_observation_ack@1'
                        run_id = $ObservationRunId
                        source_sha = $ObservationSourceSha
                        scenario_id = $ScenarioId
                        process_correlation_id = $CorrelationId
                        process_id = [int]$process.Id
                        process_start_time_utc = ([DateTimeOffset]$process.StartTime).ToUniversalTime().ToString('O')
                        process_executable = $actualExecutable
                        observed_at = [DateTimeOffset]::UtcNow.ToString('O')
                    }
                    $temporary = "$AckPath.tmp.$PID.$([guid]::NewGuid().ToString('N'))"
                    $bytes = [Text.UTF8Encoding]::new($false).GetBytes(
                        (($ack | ConvertTo-Json -Depth 8) + [Environment]::NewLine))
                    $stream = [IO.FileStream]::new($temporary, [IO.FileMode]::CreateNew,
                        [IO.FileAccess]::Write, [IO.FileShare]::None)
                    try {
                        $stream.Write($bytes, 0, $bytes.Length)
                        $stream.Flush($true)
                    } finally { $stream.Dispose() }
                    [IO.File]::Move($temporary, $AckPath)
                    return $ack
                }
            }
            Start-Sleep -Milliseconds 10
        }
        throw 'Timed out observing the exact Argus test process'
    } -ArgumentList $MarkerPath, $AckPath, $CorrelationId, $ExpectedExecutable,
        $ObservationRunId, $ObservationSourceSha, $ScenarioId
}

function Complete-ProcessObservationJob {
    param([Parameter(Mandatory)]$Job, [Parameter(Mandatory)][string]$ScenarioId)
    try {
        if ($null -eq (Wait-Job -Job $Job -Timeout 15)) {
            throw "Argus process observer did not complete for $ScenarioId"
        }
        $result = Receive-Job -Job $Job -ErrorAction Stop
        if ($Job.State -cne 'Completed' -or $null -eq $result) {
            throw "Argus process observer failed for $ScenarioId"
        }
        return $result
    }
    finally {
        Stop-Job -Job $Job -ErrorAction SilentlyContinue
        Remove-Job -Job $Job -Force -ErrorAction SilentlyContinue
    }
}

$runSucceeded = $false
$processObservationJobs = [Collections.Generic.List[object]]::new()
try {
    $build = Invoke-ContainedCommand -Label 'build-current-backend-debug' -Executable $cargo `
        -Arguments @('build', '--locked', '--target-dir', $targetRoot, '--manifest-path',
            (Join-Path $repoRoot 'src\backend\handshake_core\Cargo.toml'), '--bin',
            'handshake_core', '--features', 'app-runtime') `
        -WorkingDirectory $repoRoot -LogRoot (Join-Path $runRoot 'commands') `
        -DescendantExitGraceMilliseconds 120000
    $commands.Add($build)
    [void](Write-ImmutableJson -Path (Join-Path $runRoot 'command-receipts\build-current-backend-debug.json') -Value $build)
    Assert-ContainedCommandPassed -Receipt $build
    $backendPath = Join-Path $targetRoot 'debug\handshake_core.exe'
    $backendSha256 = Get-FileSha256 $backendPath

    $provenance = [ordered]@{
        supervisor_run_id = $RunId
        source_sha = $sourceSha
        candidate_source_id = $candidateSourceId
        source_dirty_policy = 'controlled_candidate_status_patch_and_untracked_hashes_bound'
        source_dirty_result = @($dirtyRows)
        source_dirty_result_sha256 = $sourceDirtyResultSha256
        candidate_source_binding = $candidateBindingPath
        candidate_source_binding_sha256 = $candidateBindingSha256
        cargo_profile = 'debug'
        cargo_locked = $true
        canonical_target_root = $targetRoot
        backend_path = $backendPath
        backend_sha256 = $backendSha256
        postgres = $postgresReceipt
        manifest_path = $manifestPath
        manifest_sha256 = $manifestSha256
        job_helper_path = $jobHelperPath
        job_helper_sha256 = $jobHelperSha256
        supervisor_pid = $PID
        canonical_test_discovery = $discoveryPath
        canonical_test_discovery_sha256 = $discoverySha256
    }
    $state = [ordered]@{
        schema_id = 'hsk.wp_kernel_012.interconnection_run@2'
        work_packet_id = 'WP-KERNEL-012'
        micro_task_id = 'MT-046'
        run_id = $RunId
        status = 'RUNNING'
        started_at = [DateTimeOffset]::UtcNow.ToString('O')
        expected_scenario_count = 18
        catalog_reference = 'tests/test_interconnect_manifest.json'
        provenance = $provenance
        prior_projection_archive = $priorProjectionArchive
        completed_at = $null
        terminal_scenario_count = 0
        exact_scenario_set = $false
        all_statuses_accepted = $false
        all_attempts_sealed = $false
        sealed_attempts = @()
        workspace_cleanup = @()
        argus_terminal_proof = $null
        commands = @()
        idempotency_comparison = $null
        scenarios = [ordered]@{}
    }
    Write-JsonAtomic -Path $currentRunPath -Value $state
    Write-JsonAtomic -Path $latestRunPath -Value $state

    $env:HANDSHAKE_ARTIFACTS_ROOT = $artifactRoot
    $env:HANDSHAKE_TEST_ARTIFACTS_ROOT = $testArtifactRoot
    $env:CARGO_TARGET_DIR = $targetRoot
    $env:HSK_TEST_BACKEND_BIN = $backendPath
    $env:HANDSHAKE_TEST_PG_DSN = $PostgresDsn
    $env:HSK_PSQL_BIN = $psql
    $env:HANDSHAKE_TEST_STAGE_BINDING_ROOT = Join-Path $runRoot 'stage-binding'
    $env:HSK_MT045_RUN_ID = $RunId # pg_proof_support's current generic managed-backend receipt key
    Remove-Item Env:HSK_TEST_BASE -ErrorAction SilentlyContinue
    $env:HSK_MT046_RUN_ID = $RunId
    $env:HSK_MT046_CANONICAL = '1'
    $env:HSK_MT046_SOURCE_SHA = $sourceSha
    $env:HSK_MT046_CANDIDATE_SOURCE_ID = $candidateSourceId
    $env:HSK_MT046_SOURCE_DIRTY_POLICY = 'controlled_candidate_status_patch_and_untracked_hashes_bound'
    $env:HSK_MT046_SOURCE_DIRTY_RESULT_SHA256 = $sourceDirtyResultSha256
    $env:HSK_MT046_CANDIDATE_BINDING_PATH = $candidateBindingPath
    $env:HSK_MT046_CANDIDATE_BINDING_SHA256 = $candidateBindingSha256
    $env:HSK_MT046_CARGO_PROFILE = 'debug'
    $env:HSK_MT046_CARGO_LOCKED = 'true'
    $env:HSK_MT046_BACKEND_PATH = $backendPath
    $env:HSK_MT046_BACKEND_SHA256 = $backendSha256
    $env:HSK_MT046_POSTGRES_IDENTITY = $postgresIdentity
    $env:HSK_MT046_MANIFEST_SHA256 = $manifestSha256
    $env:HSK_MT046_SUPERVISOR_PID = [string]$PID
    $env:HANDSHAKE_PROOF_ARTIFACT_DIR = $argusRoot
    $env:HANDSHAKE_SCREENSHOT_RUN_ID = $RunId
    $env:HANDSHAKE_ARGUS_MATRIX_RUN_ID = $RunId
    $env:HANDSHAKE_ARGUS_MATRIX_SCENARIO_ID = 'IC-05'
    $env:HANDSHAKE_ARGUS_MATRIX_SURFACE = 'Editors-to-Stage interconnection'
    $env:HANDSHAKE_ARGUS_MATRIX_EDGE_STATE = 'initial-menu-open-routed-terminal'
    $env:HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA = $sourceSha
    $env:HANDSHAKE_ARGUS_BINDING_ROOT = Join-Path $runRoot 'argus-binding'
    $env:HANDSHAKE_GPU_SCREENSHOT = '1'
    $env:HANDSHAKE_PROOF_MT_ID = 'MT-046'

    $argusMatrix = @(
        [ordered]@{
            scenario_id = 'IC-05'
            surface = 'Editors-to-Stage interconnection'
            edge_state = 'initial-menu-open-routed-terminal'
            test_binary = 'test_interconnect_ckc_to_note'
            test_name = 'interconnect_ic05_route_selection_to_stage'
            invocation = 'canonical-suite'
            targets = @('menu-editors', 'menu.editors.route-to-stage')
            statuses = @('applied', 'applied')
            target_count = 2
            frame_count = 3
        },
        [ordered]@{
            scenario_id = 'IC-14'
            surface = 'Loom Search Quick Switcher'
            edge_state = 'initial-view-menu-dialog-query-terminal'
            test_binary = 'test_interconnect_loom_backlink_search'
            test_name = 'supplemental_mt046_argus_ic14_quick_switcher_search'
            invocation = 'ignored-exact'
            targets = @('menu-view', 'menu.view.open-quick-switcher', 'quick-switcher.search-submit', 'quick-switcher.search-submit')
            statuses = @('applied', 'applied', 'rejected', 'applied')
            target_count = 4
            frame_count = 3
        },
        [ordered]@{
            scenario_id = 'IC-04'; surface = 'CKC Module'; edge_state = 'initial-module-selected-terminal';
            test_binary = 'test_interconnect_ckc_to_note';
            test_name = 'supplemental_mt046_argus_ic04_ckc_module'; invocation = 'ignored-exact';
            targets = @('module-ckc'); target_patterns = @(); statuses = @('applied');
            target_count = 1; frame_count = 2
        },
        [ordered]@{
            scenario_id = 'IC-06'; surface = 'Note to Code'; edge_state = 'chip-open-terminal';
            test_binary = 'test_interconnect_note_code_crossref';
            test_name = 'supplemental_mt046_argus_ic06_note_to_code'; invocation = 'ignored-exact';
            targets = @('code-ref-chip-ic06_fixture_src/lib.rs#my_function'); target_patterns = @(); statuses = @('applied'); target_count = 1; frame_count = 1
        },
        [ordered]@{
            scenario_id = 'IC-07'; surface = 'Code to Note Reference'; edge_state = 'disabled-rejected-recovered-applied';
            test_binary = 'test_interconnect_note_code_crossref';
            test_name = 'supplemental_mt046_argus_ic07_copy_note_reference'; invocation = 'ignored-exact';
            targets = @('code_editor_ctx_rename_symbol', 'code_editor_ctx_rename_symbol', 'code_editor_ctx_rename_symbol', 'ctx-menu.code_editor_ctx_copy_note_ref'); statuses = @('applied', 'applied', 'applied', 'applied'); target_count = 4; frame_count = 1
        },
        [ordered]@{
            scenario_id = 'IC-08'; surface = 'Shared Find'; edge_state = 'query-search-terminal';
            test_binary = 'test_interconnect_note_code_crossref';
            test_name = 'supplemental_mt046_argus_ic08_shared_find'; invocation = 'ignored-exact';
            targets = @('find-in-files.query', 'find-in-files.search'); statuses = @('applied', 'applied'); target_count = 2; frame_count = 1
        },
        [ordered]@{
            scenario_id = 'IC-09'; surface = 'Diagnostic to Note'; edge_state = 'diagnostic-chip-note-terminal';
            test_binary = 'test_interconnect_note_code_crossref';
            test_name = 'supplemental_mt046_argus_ic09_diagnostic_to_note'; invocation = 'ignored-exact';
            targets = @(); target_patterns = @('^code_editor_diagnostic_note_ref_0(?:#.+)?$'); statuses = @('applied'); target_count = 1; frame_count = 1
        },
        [ordered]@{
            scenario_id = 'IC-15'; surface = 'Rich Undo'; edge_state = 'edit-menu-undo-terminal';
            test_binary = 'test_interconnect_shared_undo_ledger';
            test_name = 'supplemental_mt046_argus_ic15_rich_undo'; invocation = 'ignored-exact';
            targets = @('menu-edit', 'menu.edit.undo'); statuses = @('applied', 'applied'); target_count = 2; frame_count = 1
        },
        [ordered]@{
            scenario_id = 'IC-16'; surface = 'Code Undo'; edge_state = 'edit-menu-undo-terminal';
            test_binary = 'test_interconnect_shared_undo_ledger';
            test_name = 'supplemental_mt046_argus_ic16_code_undo'; invocation = 'ignored-exact';
            targets = @('menu-edit', 'menu.edit.undo'); statuses = @('applied', 'applied'); target_count = 2; frame_count = 1
        },
        [ordered]@{
            scenario_id = 'IC-18'; surface = 'Scoped Undo'; edge_state = 'dual-edit-menu-local-undo-terminal';
            test_binary = 'test_interconnect_shared_undo_ledger';
            test_name = 'supplemental_mt046_argus_ic18_scoped_undo'; invocation = 'ignored-exact';
            targets = @('menu-edit', 'menu.edit.undo'); statuses = @('applied', 'applied'); target_count = 2; frame_count = 1
        }
    )

    $testBinaries = @(
        'test_interconnect_ckc_to_note',
        'test_interconnect_note_code_crossref',
        'test_interconnect_loom_backlink_search',
        'test_interconnect_shared_undo_ledger'
    )
    $guardTests = @(
        [ordered]@{
            test_binary = 'test_interconnect_note_code_crossref'
            test_name = 'supplemental_ic06_code_panel_focus_boundary_probe'
        },
        [ordered]@{
            test_binary = 'test_interconnect_loom_backlink_search'
            test_name = 'typed_no_model_skip_classifier_is_fail_closed'
        }
    )
    $commandByBinary = @{}
    $testExecutableByBinary = @{}
    $argusCommandByScenario = @{}
    foreach ($binary in $testBinaries) {
        $commandPath = Join-Path $runRoot "command-receipts\$binary.json"
        $stdoutPath = Join-Path $runRoot "commands\$binary.stdout.log"
        $stderrPath = Join-Path $runRoot "commands\$binary.stderr.log"
        $env:HSK_MT046_TEST_BINARY = $binary
        $env:HSK_MT046_COMMAND_RECEIPT_PATH = $commandPath
        $env:HSK_MT046_STDOUT_PATH = $stdoutPath
        $env:HSK_MT046_STDERR_PATH = $stderrPath
        $processCorrelationId = "$RunId-$binary-$([guid]::NewGuid().ToString('N'))"
        $env:HANDSHAKE_PROOF_PROCESS_CORRELATION_ID = $processCorrelationId
        if ($binary -ceq 'test_interconnect_ckc_to_note') {
            $env:HANDSHAKE_PROOF_PROCESS_SCENARIO_ID = 'IC-05'
        } else {
            Remove-Item Env:HANDSHAKE_PROOF_PROCESS_SCENARIO_ID -ErrorAction SilentlyContinue
        }
        Write-JsonAtomic -Path (Join-Path $runRoot "command-current\$binary.json") -Value ([ordered]@{
            run_id = $RunId; test_binary = $binary; status = 'RUNNING';
            started_at = [DateTimeOffset]::UtcNow.ToString('O')
        })
        $buildTest = Invoke-ContainedCommand -Label "discover-build-$binary" -Executable $cargo `
            -Arguments @('test', '--locked', '--target-dir', $targetRoot, '--test', $binary,
                '--no-run', '--message-format=json-render-diagnostics') `
            -WorkingDirectory $crateRoot -LogRoot (Join-Path $runRoot 'commands') `
            -DescendantExitGraceMilliseconds 120000
        $commands.Add($buildTest)
        [void](Write-ImmutableJson -Path (Join-Path $runRoot "command-receipts\discover-build-$binary.json") -Value $buildTest)
        Assert-ContainedCommandPassed -Receipt $buildTest
        $artifactRows = @(Get-Content -LiteralPath $buildTest.stdout_path | ForEach-Object {
            try { $_ | ConvertFrom-Json } catch { $null }
        } | Where-Object {
            $null -ne $_ -and $_.reason -ceq 'compiler-artifact' -and
            $_.target.name -ceq $binary -and -not [string]::IsNullOrWhiteSpace([string]$_.executable)
        })
        if ($artifactRows.Count -ne 1) {
            throw "MT-046 could not resolve one exact built executable for $binary"
        }
        $testExecutable = [IO.Path]::GetFullPath([string]$artifactRows[0].executable)
        $testExecutableSha256 = Get-FileSha256 $testExecutable
        $listCommand = Invoke-ContainedCommand -Label "discover-list-$binary" -Executable $testExecutable `
            -Arguments @('--list', '--format=terse') -WorkingDirectory $crateRoot `
            -LogRoot (Join-Path $runRoot 'commands')
        $commands.Add($listCommand)
        [void](Write-ImmutableJson -Path (Join-Path $runRoot "command-receipts\discover-list-$binary.json") -Value $listCommand)
        Assert-ContainedCommandPassed -Receipt $listCommand
        $listedProofs = @(Get-Content -LiteralPath $listCommand.stdout_path | ForEach-Object {
            if ($_ -match '^(interconnect_[A-Za-z0-9_]+): test$') { $Matches[1] }
        })
        if ((($listedProofs | Sort-Object) -join "`n") -cne
            ((@($expectedProofsByBinary[$binary]) | Sort-Object) -join "`n")) {
            throw "MT-046 built-binary discovery drifted for $binary"
        }
        $binaryDiscoveryPath = Join-Path $runRoot "binary-discovery\$binary.json"
        $binaryDiscoverySha = Write-ImmutableJson -Path $binaryDiscoveryPath -Value ([ordered]@{
            run_id = $RunId; test_binary = $binary; executable = $testExecutable;
            executable_sha256 = $testExecutableSha256; proof_functions = $listedProofs;
            build_command = $buildTest; list_command = $listCommand; status = 'PASS'
        })
        $testExecutableByBinary[$binary] = [ordered]@{
            path = $testExecutable; sha256 = $testExecutableSha256;
            discovery_path = $binaryDiscoveryPath; discovery_sha256 = $binaryDiscoverySha
        }
        $processObservationJob = $null
        if ($binary -ceq 'test_interconnect_ckc_to_note') {
            $processObservationAckPath = Join-Path $argusRunRoot 'process-observation-IC-05.json'
            $env:HANDSHAKE_PROOF_PROCESS_OBSERVATION_ACK = $processObservationAckPath
            $processObservationJob = Start-ProcessObservationJob `
                -MarkerPath (Join-Path $argusRunRoot 'screenshot_marker.jsonl') `
                -AckPath $processObservationAckPath -CorrelationId $processCorrelationId `
                -ExpectedExecutable $testExecutable -ObservationRunId $RunId `
                -ObservationSourceSha $sourceSha -ScenarioId 'IC-05'
            $processObservationJobs.Add($processObservationJob)
        } else {
            Remove-Item Env:HANDSHAKE_PROOF_PROCESS_OBSERVATION_ACK -ErrorAction SilentlyContinue
        }
        $command = Invoke-ContainedCommand -Label $binary -Executable $testExecutable `
            -Arguments @('interconnect_', '--nocapture', '--test-threads=1') `
            -WorkingDirectory $crateRoot -LogRoot (Join-Path $runRoot 'commands')
        $commandPassed = -not $command.timed_out -and $command.leaked_process_count -eq 0 -and
            @($command.post_drain_descendant_process_ids).Count -eq 0 -and $command.exit_code -eq 0
        $terminalCommand = [ordered]@{
            schema_id = 'hsk.wp_kernel_012.mt046_command_attempt@1'
            run_id = $RunId
            source_sha = $sourceSha
            test_binary = $binary
            cargo_profile = 'debug'
            cargo_locked = $true
            backend_path = $backendPath
            backend_sha256 = $backendSha256
            postgres_identity = $postgresIdentity
            manifest_sha256 = $manifestSha256
            test_executable_path = $testExecutable
            test_executable_sha256 = $testExecutableSha256
            process_correlation_id = $processCorrelationId
            status = if ($commandPassed) { 'PASS' } else { 'FAIL' }
            containment = $command
        }
        $processObservationFailure = $null
        if ($null -ne $processObservationJob) {
            try {
                $terminalCommand['process_observation'] = Complete-ProcessObservationJob `
                    -Job $processObservationJob -ScenarioId 'IC-05'
            }
            catch {
                $processObservationFailure = $_
                $terminalCommand['process_observation'] = [ordered]@{
                    status = 'FAIL'; error = $_.Exception.Message
                }
            }
        }
        $terminalCommandSha = Write-ImmutableJson -Path $commandPath -Value $terminalCommand
        $terminalCommand.command_receipt_sha256 = $terminalCommandSha
        Write-JsonAtomic -Path (Join-Path $runRoot "command-current\$binary.json") -Value $terminalCommand
        $commands.Add($terminalCommand)
        $commandByBinary[$binary] = $terminalCommand
        if ($binary -ceq 'test_interconnect_ckc_to_note') {
            $argusCommandByScenario['IC-05'] = $terminalCommand
        }
        if ($null -ne $processObservationFailure) { throw $processObservationFailure }
        Assert-ContainedCommandPassed -Receipt $command
    }

    foreach ($guard in $guardTests) {
        $label = "guard-$($guard.test_name)"
        $guardExecutable = $testExecutableByBinary[[string]$guard.test_binary]
        $guardCommand = Invoke-ContainedCommand -Label $label -Executable ([string]$guardExecutable.path) `
            -Arguments @([string]$guard.test_name, '--exact', '--nocapture', '--test-threads=1') `
            -WorkingDirectory $crateRoot -LogRoot (Join-Path $runRoot 'commands')
        $commands.Add($guardCommand)
        [void](Write-ImmutableJson -Path (Join-Path $runRoot "command-receipts\$label.json") `
            -Value ([ordered]@{
                schema_id = 'hsk.wp_kernel_012.mt046_guard_command@1'
                run_id = $RunId
                source_sha = $sourceSha
                test_binary = $guard.test_binary
                exact_test = $guard.test_name
                status = if (-not $guardCommand.timed_out -and $guardCommand.exit_code -eq 0 -and
                    $guardCommand.leaked_process_count -eq 0 -and
                    @($guardCommand.post_drain_descendant_process_ids).Count -eq 0) { 'PASS' } else { 'FAIL' }
                containment = $guardCommand
            }))
        Assert-ContainedCommandPassed -Receipt $guardCommand
    }

    foreach ($matrixRow in @($argusMatrix | Where-Object { $_.invocation -ceq 'ignored-exact' })) {
        $label = "argus-$($matrixRow.scenario_id)-$($matrixRow.test_name)"
        $commandPath = Join-Path $runRoot "command-receipts\$label.json"
        $env:HSK_MT046_TEST_BINARY = [string]$matrixRow.test_binary
        $env:HSK_MT046_COMMAND_RECEIPT_PATH = $commandPath
        $env:HSK_MT046_STDOUT_PATH = Join-Path $runRoot "commands\$label.stdout.log"
        $env:HSK_MT046_STDERR_PATH = Join-Path $runRoot "commands\$label.stderr.log"
        $env:HANDSHAKE_ARGUS_MATRIX_SCENARIO_ID = [string]$matrixRow.scenario_id
        $env:HANDSHAKE_ARGUS_MATRIX_SURFACE = [string]$matrixRow.surface
        $env:HANDSHAKE_ARGUS_MATRIX_EDGE_STATE = [string]$matrixRow.edge_state
        $env:HANDSHAKE_PROOF_PROCESS_SCENARIO_ID = [string]$matrixRow.scenario_id
        $processCorrelationId = "$RunId-$label-$([guid]::NewGuid().ToString('N'))"
        $env:HANDSHAKE_PROOF_PROCESS_CORRELATION_ID = $processCorrelationId
        $argusExecutable = $testExecutableByBinary[[string]$matrixRow.test_binary]
        $processObservationAckPath = Join-Path $argusRunRoot "process-observation-$($matrixRow.scenario_id).json"
        $env:HANDSHAKE_PROOF_PROCESS_OBSERVATION_ACK = $processObservationAckPath
        $processObservationJob = Start-ProcessObservationJob `
            -MarkerPath (Join-Path $argusRunRoot 'screenshot_marker.jsonl') `
            -AckPath $processObservationAckPath -CorrelationId $processCorrelationId `
            -ExpectedExecutable ([string]$argusExecutable.path) -ObservationRunId $RunId `
            -ObservationSourceSha $sourceSha -ScenarioId ([string]$matrixRow.scenario_id)
        $processObservationJobs.Add($processObservationJob)
        $command = Invoke-ContainedCommand -Label $label -Executable ([string]$argusExecutable.path) `
            -Arguments @([string]$matrixRow.test_name, '--ignored', '--exact', '--nocapture',
                '--test-threads=1') `
            -WorkingDirectory $crateRoot -LogRoot (Join-Path $runRoot 'commands')
        $terminalCommand = [ordered]@{
            schema_id = 'hsk.wp_kernel_012.mt046_argus_command_attempt@1'
            run_id = $RunId
            source_sha = $sourceSha
            candidate_source_id = $candidateSourceId
            scenario_id = $matrixRow.scenario_id
            surface = $matrixRow.surface
            test_binary = $matrixRow.test_binary
            exact_test = $matrixRow.test_name
            test_executable_path = $argusExecutable.path
            test_executable_sha256 = $argusExecutable.sha256
            process_correlation_id = $processCorrelationId
            status = if (-not $command.timed_out -and $command.exit_code -eq 0 -and
                $command.leaked_process_count -eq 0 -and
                @($command.post_drain_descendant_process_ids).Count -eq 0) { 'PASS' } else { 'FAIL' }
            containment = $command
        }
        $processObservationFailure = $null
        try {
            $terminalCommand['process_observation'] = Complete-ProcessObservationJob `
                -Job $processObservationJob -ScenarioId ([string]$matrixRow.scenario_id)
        }
        catch {
            $processObservationFailure = $_
            $terminalCommand['process_observation'] = [ordered]@{
                status = 'FAIL'; error = $_.Exception.Message
            }
        }
        [void](Write-ImmutableJson -Path $commandPath -Value $terminalCommand)
        $commands.Add($terminalCommand)
        $argusCommandByScenario[[string]$matrixRow.scenario_id] = $terminalCommand
        if ($null -ne $processObservationFailure) { throw $processObservationFailure }
        Assert-ContainedCommandPassed -Receipt $command
    }

    $current = Get-Content -LiteralPath $currentRunPath -Raw | ConvertFrom-Json
    Assert-ExactScenarioSet -State $current -ExpectedIds $expectedScenarioIds

    $sealedAttempts = [Collections.Generic.List[object]]::new()
    $workspaceIds = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $backendRuntimeLogPaths = [Collections.Generic.List[string]]::new()
    $liveBackendScenarioIds = @(
        'IC-01', 'IC-02', 'IC-03', 'IC-04', 'IC-06', 'IC-07', 'IC-08', 'IC-09',
        'IC-10', 'IC-11', 'IC-12', 'IC-13', 'IC-14', 'IC-15', 'IC-17'
    )
    foreach ($scenarioProperty in @($current.scenarios.PSObject.Properties)) {
        $scenarioId = [string]$scenarioProperty.Name
        $scenarioState = $scenarioProperty.Value
        $manifestRow = $manifest | Where-Object { $_.scenario_id -ceq $scenarioId } |
            Select-Object -First 1
        if ($null -eq $manifestRow) { throw "No manifest row for current scenario $scenarioId" }
        $attemptPath = [IO.Path]::GetFullPath((Join-Path $measurementRoot ([string]$scenarioState.attempt_receipt_path)))
        $attemptRootPrefix = [IO.Path]::GetFullPath((Join-Path $measurementRoot 'attempts')).TrimEnd('\') + '\'
        if (-not $attemptPath.StartsWith($attemptRootPrefix, [StringComparison]::OrdinalIgnoreCase) -or
            -not (Test-Path -LiteralPath $attemptPath -PathType Leaf)) {
            throw "Scenario $scenarioId points outside current immutable attempts: '$attemptPath'"
        }
        $attempt = Get-Content -LiteralPath $attemptPath -Raw | ConvertFrom-Json
        $attemptDigest = Get-FileSha256 $attemptPath
        $attemptDigestPath = "$attemptPath.sha256"
        $recordedAttemptDigest = ((Get-Content -LiteralPath $attemptDigestPath -Raw).Trim() -split '\s+')[0]
        if ($recordedAttemptDigest -cne $attemptDigest) {
            throw "Scenario $scenarioId immutable attempt digest sidecar is invalid"
        }
        $expectedBinary = [string]$proofToBinary[[string]$manifestRow.proof_fn]
        $binary = [string]$attempt.provenance.test_binary
        if ($attempt.run_id -cne $RunId -or $attempt.scenario_id -cne $scenarioId -or
            $attempt.attempt_id -cne [string]$scenarioState.attempt_id -or
            $attempt.status -cne [string]$scenarioState.status -or
            $attempt.test_thread -cne [string]$manifestRow.proof_fn -or
            $binary -cne $expectedBinary) {
            throw "Scenario $scenarioId attempt identity/proof_fn/binary binding is invalid"
        }
        if ([int]$attempt.process_id -ne [int]$commandByBinary[$binary].containment.root_process_id -or
            [string]::IsNullOrWhiteSpace([string]$attempt.started_at) -or
            [string]::IsNullOrWhiteSpace([string]$attempt.completed_at)) {
            throw "Scenario $scenarioId attempt PID/timestamp binding is invalid"
        }
        if (($scenarioId -ceq 'IC-13' -and $attempt.status -ceq 'SKIPPED' -and
                $attempt.terminal_reason -cne 'HSK-409-LOOM-AI-NO-MODEL') -or
            ($scenarioId -cne 'IC-13' -and $attempt.status -cne 'PASS')) {
            throw "Scenario $scenarioId attempt has an unauthorized disposition"
        }
        $command = $commandByBinary[$binary]
        $binaryIdentity = $testExecutableByBinary[$binary]
        $expectedCommandReceiptPath = Join-Path $runRoot "command-receipts\$binary.json"
        $provenanceInvalid =
            $attempt.provenance.supervisor_run_id -cne $RunId -or
            $attempt.provenance.source_sha -cne $sourceSha -or
            $attempt.provenance.candidate_source_id -cne $candidateSourceId -or
            $attempt.provenance.source_dirty_policy -cne $provenance.source_dirty_policy -or
            $attempt.provenance.source_dirty_result_sha256 -cne $sourceDirtyResultSha256 -or
            $attempt.provenance.candidate_source_binding_path -cne $candidateBindingPath -or
            $attempt.provenance.candidate_source_binding_sha256 -cne $candidateBindingSha256 -or
            $attempt.provenance.cargo_profile -cne 'debug' -or
            $attempt.provenance.cargo_locked -ne $true -or
            $attempt.provenance.backend_path -cne $backendPath -or
            $attempt.provenance.backend_sha256 -cne $backendSha256 -or
            $attempt.provenance.postgres_identity -cne $postgresIdentity -or
            $attempt.provenance.manifest_sha256 -cne $manifestSha256 -or
            $attempt.provenance.supervisor_pid -cne [string]$PID -or
            $attempt.provenance.command_receipt_path -cne $expectedCommandReceiptPath -or
            $attempt.provenance.stdout_path -cne [string]$command.containment.stdout_path -or
            $attempt.provenance.stderr_path -cne [string]$command.containment.stderr_path -or
            $attempt.provenance.test_executable_path -cne [string]$binaryIdentity.path -or
            $attempt.provenance.test_executable_sha256 -cne [string]$binaryIdentity.sha256
        if ($provenanceInvalid) {
            throw "Scenario $scenarioId full supervisor/source/runtime provenance binding is invalid"
        }
        if ($scenarioId -in $liveBackendScenarioIds) {
            $binding = $attempt.evidence.backend_binding
            $runtimeReceipt = $attempt.evidence.backend_runtime_receipt
            if ($null -eq $binding -or $binding.owned -ne $true -or
                (Get-ComparableFullPath ([string]$binding.backend_binary)) -cne
                    (Get-ComparableFullPath $backendPath) -or
                [string]$binding.backend_binary_sha256 -cne $backendSha256 -or
                [uint64]$binding.backend_pid -le 0 -or
                [string]$binding.database_host -cne $pgUri.Host -or
                [int]$binding.database_port -ne $pgUri.Port -or
                [string]$binding.database_name -cne $pgUri.AbsolutePath.TrimStart('/') -or
                $null -eq $runtimeReceipt -or $runtimeReceipt.status -cne 'complete' -or
                [uint64]$runtimeReceipt.process.pid -ne [uint64]$binding.backend_pid -or
                $runtimeReceipt.process.termination -cne 'terminated_and_reaped') {
                throw "Scenario $scenarioId lacks an exact owned backend/runtime binding"
            }
        } elseif ($attempt.evidence.backend_not_used -ne $true) {
            throw "Scenario $scenarioId must explicitly classify the backend as not used"
        }
        foreach ($workspaceId in @(Get-JsonPropertyValues -Value $attempt.evidence -PropertyName 'workspace_id')) {
            if (-not [string]::IsNullOrWhiteSpace([string]$workspaceId)) {
                [void]$workspaceIds.Add([string]$workspaceId)
            }
        }
        $commandReceiptPath = [string]$attempt.provenance.command_receipt_path
        $commandReceiptDigest = Get-FileSha256 $commandReceiptPath
        $recordedCommandDigest = ((Get-Content -LiteralPath "$commandReceiptPath.sha256" -Raw).Trim() -split '\s+')[0]
        if ($recordedCommandDigest -cne $commandReceiptDigest) {
            throw "Scenario $scenarioId command receipt digest sidecar is invalid"
        }
        $sealPath = Join-Path $runRoot "sealed-attempts\$($attempt.attempt_id).json"
        $seal = [ordered]@{
            schema_id = 'hsk.wp_kernel_012.mt046_sealed_scenario_attempt@1'
            run_id = $RunId
            scenario_id = $attempt.scenario_id
            attempt_id = $attempt.attempt_id
            status = $attempt.status
            terminal_reason = $attempt.terminal_reason
            source_receipt_path = $attemptPath
            source_receipt_sha256 = $attemptDigest
            source_receipt_digest_sidecar = $attemptDigestPath
            command_receipt_path = [string]$attempt.provenance.command_receipt_path
            command_receipt_sha256 = [string]$command.command_receipt_sha256
            stdout_path = [string]$command.containment.stdout_path
            stdout_sha256 = [string]$command.containment.stdout_sha256
            stderr_path = [string]$command.containment.stderr_path
            stderr_sha256 = [string]$command.containment.stderr_sha256
            root_process_id = [int]$command.containment.root_process_id
            exit_code = [int]$command.containment.exit_code
            timed_out = [bool]$command.containment.timed_out
            leaked_process_count = [int]$command.containment.leaked_process_count
            completed_at = [string]$command.containment.completed_at
            seal_status = 'BOUND'
            sealed_at = [DateTimeOffset]::UtcNow.ToString('O')
        }
        $sealSha = Write-ImmutableJson -Path $sealPath -Value $seal
        $sealedAttempts.Add([ordered]@{ path = $sealPath; sha256 = $sealSha; scenario_id = $attempt.scenario_id })
    }

    $allSameRunAttempts = @(Get-ChildItem -LiteralPath $attemptRoot -File -Filter '*.json' |
        ForEach-Object {
            $candidateAttempt = try { Get-Content -LiteralPath $_.FullName -Raw | ConvertFrom-Json } catch { $null }
            if ($null -ne $candidateAttempt -and $candidateAttempt.run_id -ceq $RunId) {
                [ordered]@{
                    path = [IO.Path]::GetFullPath($_.FullName)
                    scenario_id = [string]$candidateAttempt.scenario_id
                    attempt_id = [string]$candidateAttempt.attempt_id
                }
            }
        })
    $projectedAttemptPaths = @($sealedAttempts | ForEach-Object {
        $seal = Get-Content -LiteralPath $_.path -Raw | ConvertFrom-Json
        [IO.Path]::GetFullPath([string]$seal.source_receipt_path)
    } | Sort-Object)
    $allSameRunAttemptPaths = @($allSameRunAttempts.path | Sort-Object)
    if ($allSameRunAttempts.Count -ne 18 -or
        @($allSameRunAttempts.scenario_id | Sort-Object -Unique).Count -ne 18 -or
        @($allSameRunAttempts.attempt_id | Sort-Object -Unique).Count -ne 18 -or
        ($allSameRunAttemptPaths -join "`n") -cne ($projectedAttemptPaths -join "`n")) {
        throw 'MT-046 same-run attempt history contains an orphan, duplicate, or unsealed receipt'
    }

    $backendBindingPaths = @(Get-ChildItem -LiteralPath $backendBindingRoot -File -Filter '*.json' `
        -ErrorAction Stop | Where-Object { $_.Name -notlike '*.sha256' })
    if ($backendBindingPaths.Count -ne 19) {
        throw "MT-046 expected 19 fresh owned-backend bindings, found $($backendBindingPaths.Count)"
    }
    $backendBindings = [Collections.Generic.List[object]]::new()
    foreach ($bindingPath in $backendBindingPaths) {
        $bindingReceipt = Get-Content -LiteralPath $bindingPath.FullName -Raw | ConvertFrom-Json
        $bindingDigest = Get-FileSha256 $bindingPath.FullName
        $bindingRecordedDigest = ((Get-Content -LiteralPath "$($bindingPath.FullName).sha256" -Raw).Trim() -split '\s+')[0]
        $bindingCommand = if (-not [string]::IsNullOrWhiteSpace([string]$bindingReceipt.process_scenario_id) -and
            $argusCommandByScenario.ContainsKey([string]$bindingReceipt.process_scenario_id)) {
            $argusCommandByScenario[[string]$bindingReceipt.process_scenario_id]
        } else {
            $commandByBinary[[string]$bindingReceipt.test_binary]
        }
        if ($bindingDigest -cne $bindingRecordedDigest -or
            $bindingReceipt.schema_id -cne 'hsk.wp_kernel_012.mt046_owned_backend_binding@1' -or
            $bindingReceipt.run_id -cne $RunId -or $bindingReceipt.source_sha -cne $sourceSha -or
            $bindingReceipt.candidate_source_id -cne $candidateSourceId -or $null -eq $bindingCommand -or
            [int]$bindingReceipt.test_process_id -ne [int]$bindingCommand.containment.root_process_id -or
            [int]$bindingReceipt.backend_parent_process_id -ne [int]$bindingReceipt.test_process_id -or
            $bindingReceipt.process_correlation_id -cne $bindingCommand.process_correlation_id -or
            [string]$bindingReceipt.test_executable_path -cne [string]$bindingCommand.test_executable_path -or
            [string]$bindingReceipt.test_executable_sha256 -cne [string]$bindingCommand.test_executable_sha256 -or
            $bindingReceipt.backend.owned -ne $true -or [uint64]$bindingReceipt.backend.backend_pid -le 0 -or
            (Get-ComparableFullPath ([string]$bindingReceipt.backend.backend_binary)) -cne
                (Get-ComparableFullPath $backendPath) -or
            [string]$bindingReceipt.backend.backend_binary_sha256 -cne $backendSha256 -or
            [string]$bindingReceipt.backend.database_host -cne $pgUri.Host -or
            [int]$bindingReceipt.backend.database_port -ne $pgUri.Port -or
            [string]$bindingReceipt.backend.database_name -cne $pgUri.AbsolutePath.TrimStart('/')) {
            throw "MT-046 owned-backend binding is stale, cross-process, or runtime-invalid: '$($bindingPath.FullName)'"
        }
        $backendBindings.Add([ordered]@{ path = $bindingPath.FullName; sha256 = $bindingDigest; receipt = $bindingReceipt })
    }

    $runtimeReceiptPaths = @(Get-ChildItem -LiteralPath $successRuntimeRoot -Recurse -File `
        -Filter 'runtime-diagnostics.json' -ErrorAction Stop)
    if ($runtimeReceiptPaths.Count -ne 19) {
        throw "MT-046 expected 19 immutable owned-backend runtime receipts, found $($runtimeReceiptPaths.Count)"
    }
    $boundBackendPids = @($backendBindings | ForEach-Object { [uint64]$_.receipt.backend.backend_pid } | Sort-Object)
    $runtimeBackendPids = [Collections.Generic.List[uint64]]::new()
    foreach ($runtimeReceiptPath in $runtimeReceiptPaths) {
        $runtimeReceipt = Get-Content -LiteralPath $runtimeReceiptPath.FullName -Raw | ConvertFrom-Json
        $runtimeDigest = Get-FileSha256 $runtimeReceiptPath.FullName
        $runtimeRecordedDigest = ((Get-Content -LiteralPath "$($runtimeReceiptPath.FullName).sha256" -Raw).Trim() -split '\s+')[0]
        if ($runtimeDigest -cne $runtimeRecordedDigest -or $runtimeReceipt.run_id -cne $RunId -or
            $runtimeReceipt.status -cne 'complete' -or $runtimeReceipt.process.owned -ne $true -or
            $runtimeReceipt.process.termination -cne 'terminated_and_reaped' -or
            @($runtimeReceipt.files).Count -ne 3 -or
            @($runtimeReceipt.files | Where-Object { $_.status -cne 'retained' }).Count -ne 0) {
            throw "MT-046 owned-backend runtime receipt is incomplete: '$($runtimeReceiptPath.FullName)'"
        }
        $runtimeBackendPids.Add([uint64]$runtimeReceipt.process.pid)
        foreach ($file in @($runtimeReceipt.files)) {
            if (-not (Test-Path -LiteralPath ([string]$file.path) -PathType Leaf) -or
                (Get-FileSha256 ([string]$file.path)) -cne [string]$file.sha256) {
                throw "MT-046 retained owned-backend runtime file is missing or hash-invalid: '$($file.path)'"
            }
            if ([string]$file.name -in @('backend.stdout.log', 'backend.stderr.log')) {
                $backendRuntimeLogPaths.Add([string]$file.path)
            }
        }
    }
    if ((@($runtimeBackendPids | Sort-Object) -join "`n") -cne ($boundBackendPids -join "`n")) {
        throw 'MT-046 owned-backend binding/runtime PID sets differ; stale or missing backend evidence detected'
    }

    $supplementalWorkspaceRoot = Join-Path $wpTestRoot "mt-046\canonical-argus\$RunId"
    $supplementalWorkspaceScenarios = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    if (Test-Path -LiteralPath $supplementalWorkspaceRoot -PathType Container) {
        foreach ($workspaceReceiptPath in @(Get-ChildItem -LiteralPath $supplementalWorkspaceRoot `
            -Recurse -File -Filter 'workspace.json')) {
            $workspaceReceipt = Get-Content -LiteralPath $workspaceReceiptPath.FullName -Raw |
                ConvertFrom-Json
            $scenarioCommand = $argusCommandByScenario[[string]$workspaceReceipt.scenario_id]
            $workspaceDigest = Get-FileSha256 $workspaceReceiptPath.FullName
            $workspaceRecordedDigest = ((Get-Content -LiteralPath "$($workspaceReceiptPath.FullName).sha256" -Raw).Trim() -split '\s+')[0]
            if ($workspaceDigest -cne $workspaceRecordedDigest -or
                $workspaceReceipt.schema_id -cne 'hsk.mt046.workspace-binding@1' -or
                $workspaceReceipt.run_id -cne $RunId -or $workspaceReceipt.source_sha -cne $sourceSha -or
                $null -eq $scenarioCommand -or
                [int]$workspaceReceipt.process_id -ne
                    [int]$scenarioCommand.containment.root_process_id -or
                $workspaceReceipt.process_correlation_id -cne $scenarioCommand.process_correlation_id -or
                [string]::IsNullOrWhiteSpace([string]$workspaceReceipt.workspace_id)) {
                throw "Supplemental workspace receipt is not run/process bound: '$($workspaceReceiptPath.FullName)'"
            }
            [void]$workspaceIds.Add([string]$workspaceReceipt.workspace_id)
            [void]$supplementalWorkspaceScenarios.Add([string]$workspaceReceipt.scenario_id)
        }
    }
    foreach ($requiredWorkspaceScenario in @('IC-06', 'IC-08', 'IC-09', 'IC-14')) {
        if (-not $supplementalWorkspaceScenarios.Contains($requiredWorkspaceScenario)) {
            throw "Supplemental $requiredWorkspaceScenario omitted its run-bound workspace receipt"
        }
    }

    $workspaceCleanup = [Collections.Generic.List[object]]::new()
    foreach ($workspaceId in $workspaceIds) {
        if ($workspaceId -notmatch '^[A-Za-z0-9_-]{1,128}$') {
            throw "MT-046 refuses unsafe workspace cleanup identity '$workspaceId'"
        }
        $cleanupLabel = "workspace-absent-$workspaceId"
        $cleanupSql = "DO `$mt046`$ DECLARE r record; remaining bigint; BEGIN " +
            "FOR r IN SELECT table_schema, table_name FROM information_schema.columns " +
            "WHERE column_name = 'workspace_id' AND table_schema = current_schema() LOOP " +
            "EXECUTE format('SELECT count(*) FROM %I.%I WHERE workspace_id::text = %L', " +
            "r.table_schema, r.table_name, '$workspaceId') INTO remaining; " +
            "IF remaining <> 0 THEN RAISE EXCEPTION 'workspace residue %.% rows=%', " +
            "r.table_schema, r.table_name, remaining; END IF; END LOOP; END `$mt046`$; " +
            "SELECT COUNT(*) FROM workspaces WHERE id::text = '$workspaceId';"
        $cleanup = Invoke-ContainedCommand -Label $cleanupLabel -Executable $psql `
            -Arguments @('--no-psqlrc', '--no-password', '--set', 'ON_ERROR_STOP=1', '--quiet',
                '--tuples-only', '--no-align', '--dbname', $PostgresDsn, '--set',
                "workspace_id=$workspaceId", '--command', $cleanupSql) `
            -WorkingDirectory $repoRoot -LogRoot (Join-Path $runRoot 'cleanup')
        $commands.Add($cleanup)
        Assert-ContainedCommandPassed -Receipt $cleanup
        $remaining = (Get-Content -LiteralPath $cleanup.stdout_path -Raw).Trim()
        if ($remaining -cne '0') {
            throw "MT-046 exact workspace cleanup failed for '$workspaceId': remaining='$remaining'"
        }
        $cleanupReceiptPath = Join-Path $runRoot "cleanup-receipts\$workspaceId.json"
        $cleanupSha = Write-ImmutableJson -Path $cleanupReceiptPath -Value ([ordered]@{
            run_id = $RunId; workspace_id = $workspaceId; remaining_workspace_rows = 0;
            command = $cleanup; status = 'PASS'
        })
        $workspaceCleanup.Add([ordered]@{ path = $cleanupReceiptPath; sha256 = $cleanupSha })
    }

    $tracePath = Join-Path $argusRunRoot 'canonical-argus-matrix.jsonl'
    $markerPath = Join-Path $argusRunRoot 'screenshot_marker.jsonl'
    if (-not (Test-Path -LiteralPath $tracePath) -or -not (Test-Path -LiteralPath $markerPath)) {
        throw 'MT-046 canonical Argus trace or screenshot marker is missing'
    }
    $traceRows = @(Get-Content -LiteralPath $tracePath | ForEach-Object { $_ | ConvertFrom-Json })
    $expectedTraceCount = @(
        $argusMatrix | ForEach-Object { [int]$_['target_count'] } | Measure-Object -Sum
    ).Sum
    if ($traceRows.Count -ne $expectedTraceCount -or @($traceRows | Where-Object {
        $_.run_id -cne $RunId -or $_.source_sha -cne $sourceSha -or
        $_.terminal_refreshed -ne $true -or $_.receipt_status -notin @('applied', 'rejected') -or
        @($_.terminal_predicates | Where-Object { $_.passed -ne $true }).Count -ne 0
    }).Count -ne 0) {
        throw 'MT-046 Argus trace lacks exact source-bound initial/action/fresh-terminal/finish evidence'
    }
    foreach ($matrixRow in $argusMatrix) {
        $scenarioRows = @($traceRows | Where-Object { $_.scenario_id -ceq $matrixRow.scenario_id })
        $targetPatterns = @()
        if ($matrixRow.Contains('target_patterns')) {
            $targetPatterns = @($matrixRow['target_patterns'])
        }
        if ($scenarioRows.Count -ne [int]$matrixRow.target_count -or
            (@($matrixRow.targets).Count -ne 0 -and
                ($scenarioRows.target -join "`n") -cne (@($matrixRow.targets) -join "`n")) -or
            ($targetPatterns.Count -ne 0 -and
                @($scenarioRows | Where-Object { $_.target -notmatch $targetPatterns[0] }).Count -ne 0) -or
            ($scenarioRows.receipt_status -join "`n") -cne (@($matrixRow.statuses) -join "`n") -or
            @($scenarioRows | Where-Object {
                $scenarioCommand = $argusCommandByScenario[[string]$matrixRow.scenario_id]
                $traceProcessId = $_.PSObject.Properties['process_id']
                $_.surface -cne $matrixRow.surface -or $_.edge_state_tag -cne $matrixRow.edge_state -or
                $null -eq $scenarioCommand -or
                $_.process_correlation_id -cne $scenarioCommand.process_correlation_id -or
                ($null -ne $traceProcessId -and $null -ne $traceProcessId.Value -and
                    [int]$traceProcessId.Value -ne [int]$scenarioCommand.containment.root_process_id)
            }).Count -ne 0) {
            throw "MT-046 Argus matrix drifted for $($matrixRow.scenario_id)"
        }
    }
    $markers = @(Get-Content -LiteralPath $markerPath | ForEach-Object { $_ | ConvertFrom-Json })
    $expectedFrameCount = @(
        $argusMatrix | ForEach-Object { [int]$_['frame_count'] } | Measure-Object -Sum
    ).Sum
    if ($markers.Count -ne $expectedFrameCount -or @($markers | Where-Object {
        $_.mt_id -cne 'MT-046' -or $_.run_id -cne $RunId -or $_.status -cne 'CAPTURED'
    }).Count -ne 0) {
        throw 'MT-046 requires exactly three material CAPTURED Argus frames (initial, menu, terminal)'
    }
    $argusRunPrefix = [IO.Path]::GetFullPath($argusRunRoot).TrimEnd('\') + '\'
    foreach ($matrixRow in $argusMatrix) {
        $scenarioMarkers = @($markers | Where-Object {
            $_.process_scenario_id -ceq $matrixRow.scenario_id
        } | Sort-Object proof_event_sequence)
        $scenarioTrace = @($traceRows | Where-Object {
            $_.scenario_id -ceq $matrixRow.scenario_id
        })
        $scenarioCommand = $argusCommandByScenario[[string]$matrixRow.scenario_id]
        if ($scenarioMarkers.Count -ne [int]$matrixRow.frame_count -or $null -eq $scenarioCommand) {
            throw "MT-046 Argus screenshot/process binding is incomplete for $($matrixRow.scenario_id)"
        }
        $observation = $scenarioCommand.process_observation
        if ($null -eq $observation -or $observation.run_id -cne $RunId -or
            $observation.source_sha -cne $sourceSha -or
            $observation.scenario_id -cne $matrixRow.scenario_id -or
            $observation.process_correlation_id -cne $scenarioCommand.process_correlation_id -or
            [int]$observation.process_id -ne [int]$scenarioCommand.containment.root_process_id -or
            [string]$observation.process_executable -cne [string]$scenarioCommand.test_executable_path) {
            throw "MT-046 external live-process observation is incomplete for $($matrixRow.scenario_id)"
        }
        foreach ($marker in $scenarioMarkers) {
            $framePath = [IO.Path]::GetFullPath([string]$marker.frame_path)
            if ($marker.source_sha -cne $sourceSha -or
                $marker.process_correlation_id -cne $scenarioCommand.process_correlation_id -or
                [int]$marker.process_id -ne [int]$scenarioCommand.containment.root_process_id -or
                -not $framePath.StartsWith($argusRunPrefix, [StringComparison]::OrdinalIgnoreCase)) {
                throw "MT-046 Argus frame escaped source/process/run binding for $($matrixRow.scenario_id)"
            }
        }
        $terminalMarker = $scenarioMarkers[-1]
        $terminalTrace = $scenarioTrace[-1]
        if ([uint64]$terminalMarker.action_receipt_id -ne [uint64]$terminalTrace.receipt_id -or
            [uint64]$terminalMarker.proof_event_sequence -le
                [uint64]$terminalTrace.terminal_observed_sequence) {
            throw "MT-046 terminal frame is not causally after the final terminal receipt for $($matrixRow.scenario_id)"
        }
    }
    $neverStartedPath = Join-Path $argusRunRoot 'trees\IC-07\never-started.json'
    if (-not (Test-Path -LiteralPath $neverStartedPath -PathType Leaf)) {
        throw 'MT-046 IC-07 stale-hidden never-started proof is missing'
    }
    $neverStarted = Get-Content -LiteralPath $neverStartedPath -Raw | ConvertFrom-Json
    $ic07Command = $argusCommandByScenario['IC-07']
    $neverStartedDigest = Get-FileSha256 $neverStartedPath
    $neverStartedRecordedDigest = ((Get-Content -LiteralPath "$neverStartedPath.sha256" -Raw).Trim() -split '\s+')[0]
    if ($neverStartedDigest -cne $neverStartedRecordedDigest -or
        $neverStarted.schema_id -cne 'hsk.mt046.argus-never-started@1' -or
        $neverStarted.run_id -cne $RunId -or $neverStarted.source_sha -cne $sourceSha -or
        $neverStarted.scenario_id -cne 'IC-07' -or $null -eq $ic07Command -or
        [int]$neverStarted.process_id -ne [int]$ic07Command.containment.root_process_id -or
        $neverStarted.process_correlation_id -cne $ic07Command.process_correlation_id -or
        -not ([string]$neverStarted.correlation_id).StartsWith(
            "$($ic07Command.process_correlation_id):never-started:", [StringComparison]::Ordinal) -or
        $neverStarted.target -cne 'ctx-menu.code_editor_ctx_copy_note_ref' -or
        $neverStarted.error.code -ne -32000 -or $neverStarted.state_unchanged.equal -ne $true -or
        $neverStarted.state_unchanged.clipboard_before -cne $neverStarted.state_unchanged.clipboard_after -or
        $neverStarted.canonical_stale_snapshot.response.error.code -ne -32000 -or
        @($neverStarted.canonical_stale_snapshot.after.action_receipts).Count -ne
            @($neverStarted.canonical_stale_snapshot.before.action_receipts).Count) {
        throw 'MT-046 stale-hidden action did not prove pre-dispatch rejection with zero receipt creation'
    }
    [void](Write-ImmutableJson -Path (Join-Path $runRoot 'sealed-artifacts\ic07-never-started.json') `
        -Value ([ordered]@{ run_id = $RunId; source_sha = $sourceSha; scenario_id = 'IC-07';
            source_path = $neverStartedPath; source_sha256 = $neverStartedDigest; status = 'BOUND' }))
    $disabledNeverStartedPath = Join-Path $argusRunRoot 'trees\IC-07\disabled-never-started.json'
    if (-not (Test-Path -LiteralPath $disabledNeverStartedPath -PathType Leaf)) {
        throw 'MT-046 IC-07 disabled never-started proof is missing'
    }
    $disabledNeverStarted = Get-Content -LiteralPath $disabledNeverStartedPath -Raw | ConvertFrom-Json
    $disabledNeverStartedDigest = Get-FileSha256 $disabledNeverStartedPath
    $disabledNeverStartedRecordedDigest = ((Get-Content -LiteralPath "$disabledNeverStartedPath.sha256" -Raw).Trim() -split '\s+')[0]
    if ($disabledNeverStartedDigest -cne $disabledNeverStartedRecordedDigest -or
        $disabledNeverStarted.schema_id -cne 'hsk.mt046.argus-disabled-never-started@1' -or
        $disabledNeverStarted.run_id -cne $RunId -or
        $disabledNeverStarted.source_sha -cne $sourceSha -or
        $disabledNeverStarted.scenario_id -cne 'IC-07' -or $null -eq $ic07Command -or
        [int]$disabledNeverStarted.process_id -ne [int]$ic07Command.containment.root_process_id -or
        $disabledNeverStarted.process_correlation_id -cne $ic07Command.process_correlation_id -or
        -not ([string]$disabledNeverStarted.correlation_id).StartsWith(
            "$($ic07Command.process_correlation_id):disabled-never-started:", [StringComparison]::Ordinal) -or
        $disabledNeverStarted.target -cne 'ctx-menu.code_editor_ctx_copy_note_ref' -or
        $disabledNeverStarted.error.code -ne -32000 -or
        -not ([string]$disabledNeverStarted.error.message).Contains('disabled') -or
        $disabledNeverStarted.state_unchanged.equal -ne $true -or
        $disabledNeverStarted.state_unchanged.receipt_count_before -ne
            $disabledNeverStarted.state_unchanged.receipt_count_after -or
        $disabledNeverStarted.state_unchanged.clipboard_before -cne
            $disabledNeverStarted.state_unchanged.clipboard_after) {
        throw 'MT-046 disabled action did not prove pre-dispatch rejection with zero receipt/clipboard mutation'
    }
    [void](Write-ImmutableJson -Path (Join-Path $runRoot 'sealed-artifacts\ic07-disabled-never-started.json') `
        -Value ([ordered]@{ run_id = $RunId; source_sha = $sourceSha; scenario_id = 'IC-07';
            source_path = $disabledNeverStartedPath; source_sha256 = $disabledNeverStartedDigest; status = 'BOUND' }))
    $frames = @($markers | ForEach-Object { Assert-Png -Path ([string]$_.frame_path) })
    $errorTerms = @('panic', 'fatal', 'unhandled', 'stack trace')
    $treeText = ($traceRows | ConvertTo-Json -Depth 64)
    $errorHits = @($errorTerms | Where-Object { $treeText -match [regex]::Escape($_) })
    if ($errorHits.Count -ne 0) {
        throw "MT-046 Argus error scan found terminal-tree error terms: $($errorHits -join ',')"
    }
    $runtimeErrorPatterns = @(
        "thread '.+' panicked at",
        'fatal runtime error',
        'Unhandled exception',
        'stack backtrace:',
        'wgpu[^\r\n]*(validation|device lost|out of memory)'
    )
    $runtimeErrorHits = [Collections.Generic.List[object]]::new()
    $logPaths = @(
        (Get-JsonPropertyValues -Value $commands -PropertyName 'stdout_path')
        (Get-JsonPropertyValues -Value $commands -PropertyName 'stderr_path')
        @($backendRuntimeLogPaths)
        @(if (Test-Path -LiteralPath (Join-Path $wpTestRoot "backend-runtime\$RunId")) {
            Get-ChildItem -LiteralPath (Join-Path $wpTestRoot "backend-runtime\$RunId") `
                -Recurse -File -Filter '*.log' | ForEach-Object FullName
        })
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Sort-Object -Unique
    foreach ($logPath in $logPaths) {
        if (-not (Test-Path -LiteralPath $logPath -PathType Leaf)) { continue }
        $logText = Get-Content -LiteralPath $logPath -Raw
        foreach ($pattern in $runtimeErrorPatterns) {
            if ($logText -match $pattern) {
                $runtimeErrorHits.Add([ordered]@{ path = $logPath; pattern = $pattern })
            }
        }
    }
    if ($runtimeErrorHits.Count -ne 0) {
        throw "MT-046 renderer/runtime error scan failed: $($runtimeErrorHits | ConvertTo-Json -Compress)"
    }
    $argusReceiptPath = Join-Path $runRoot 'argus-terminal-proof.json'
    $argusReceiptSha = Write-ImmutableJson -Path $argusReceiptPath -Value ([ordered]@{
        schema_id = 'hsk.wp_kernel_012.mt046_argus_terminal_proof@1'
        run_id = $RunId
        source_sha = $sourceSha
        initial_tree = $traceRows[0].before
        actions = $traceRows
        fresh_terminal_tree = $traceRows[-1].after
        finish_gate = 'finish_require_no_indeterminate'
        screenshots = $frames
        error_scan = [ordered]@{
            tree_terms = $errorTerms; runtime_patterns = $runtimeErrorPatterns;
            tree_hits = @(); runtime_hits = @(); scanned_logs = $logPaths; status = 'PASS'
        }
        status = 'PASS'
        completed_at = [DateTimeOffset]::UtcNow.ToString('O')
    })

    if ((Get-FileSha256 $backendPath) -cne $backendSha256) {
        throw 'MT-046 backend executable changed during the supervised run'
    }
    foreach ($binary in $testExecutableByBinary.Keys) {
        if ((Get-FileSha256 ([string]$testExecutableByBinary[$binary].path)) -cne
            [string]$testExecutableByBinary[$binary].sha256) {
            throw "MT-046 test executable changed during the run: $binary"
        }
    }
    foreach ($forbiddenLocal in @(
        (Join-Path $crateRoot 'test_output'),
        (Join-Path $crateRoot 'tests\screenshots'),
        (Join-Path $crateRoot 'target')
    )) {
        if (Test-Path -LiteralPath $forbiddenLocal) {
            throw "MT-046 repo-local artifact/target hygiene failed: '$forbiddenLocal'"
        }
    }
    $forbiddenRepoDirectoriesAfter = @(Get-RepoForbiddenDirectories -Repository $repoRoot)
    $newForbiddenRepoDirectories = @($forbiddenRepoDirectoriesAfter | Where-Object {
        $_ -notin $forbiddenRepoDirectoriesBefore
    })
    if ($newForbiddenRepoDirectories.Count -ne 0) {
        throw "MT-046 created repo-local forbidden directories: $($newForbiddenRepoDirectories -join ', ')"
    }
    $finalDirtyRows = @(& git -C $repoRoot status --porcelain=v1 --untracked-files=all -- $sourceBoundPaths)
    $finalControlledPatch = ((& git -C $repoRoot diff --binary HEAD -- $sourceBoundPaths) | Out-String)
    $finalUntrackedBindings = @($finalDirtyRows | Where-Object { $_ -match '^\?\? ' } | ForEach-Object {
        $relative = $_.Substring(3)
        [ordered]@{ path = $relative; sha256 = Get-FileSha256 (Join-Path $repoRoot $relative) }
    })
    $finalIdentityMaterial = [ordered]@{
        head_sha = (& git -C $repoRoot rev-parse HEAD).Trim()
        controlled_paths = $sourceBoundPaths
        status_rows = @($finalDirtyRows)
        binary_patch_sha256 = Get-TextSha256 $finalControlledPatch
        untracked_files = $finalUntrackedBindings
    }
    $finalCandidateSourceId = "$($finalIdentityMaterial.head_sha)-$((Get-TextSha256 ($finalIdentityMaterial | ConvertTo-Json -Compress -Depth 16)).Substring(0, 16))"
    if ($finalCandidateSourceId -cne $candidateSourceId) {
        throw "MT-046 controlled candidate source changed during the run: '$candidateSourceId' -> '$finalCandidateSourceId'"
    }
    $currentPostgres = Get-Process -Id $postgresReceipt.pid
    if ($currentPostgres.ProcessName -cne $postgresReceipt.process_name -or
        ([DateTimeOffset]$currentPostgres.StartTime).ToUniversalTime().ToString('O') -cne
            $postgresReceipt.start_time_utc) {
        throw 'MT-046 internal PostgreSQL identity changed during the proof'
    }

    $current.status = 'PASS'
    $current.completed_at = [DateTimeOffset]::UtcNow.ToString('O')
    $current.terminal_scenario_count = 18
    $current.exact_scenario_set = $true
    $current.all_statuses_accepted = $true
    $current.all_attempts_sealed = $true
    $current.sealed_attempts = $sealedAttempts
    $current.workspace_cleanup = $workspaceCleanup
    $current.argus_terminal_proof = [ordered]@{ path = $argusReceiptPath; sha256 = $argusReceiptSha }
    $current.commands = $commands
    $idempotencyComparison = $null
    if ($null -ne $baselineRunPath) {
        if ((Get-FileSha256 $baselineRunPath) -cne $baselineRunSha256) {
            throw 'MT-046 baseline immutable run changed during the second run'
        }
        $baselineRun = Get-Content -LiteralPath $baselineRunPath -Raw | ConvertFrom-Json
        $baselineRunRootPrefix = [IO.Path]::GetFullPath(
            (Join-Path $supervisorRoot "runs\$BaselineRunId")).TrimEnd('\') + '\'
        $baselineAttemptRows = [Collections.Generic.List[object]]::new()
        foreach ($baselineSealRef in @($baselineRun.sealed_attempts)) {
            $baselineSealPath = [IO.Path]::GetFullPath([string]$baselineSealRef.path)
            if (-not $baselineSealPath.StartsWith($baselineRunRootPrefix, [StringComparison]::OrdinalIgnoreCase) -or
                -not (Test-Path -LiteralPath $baselineSealPath -PathType Leaf) -or
                (Get-FileSha256 $baselineSealPath) -cne [string]$baselineSealRef.sha256) {
                throw 'MT-046 baseline sealed-attempt reference escaped its run or failed its digest'
            }
            $baselineSeal = Get-Content -LiteralPath $baselineSealPath -Raw | ConvertFrom-Json
            $baselineSourcePath = [IO.Path]::GetFullPath([string]$baselineSeal.source_receipt_path)
            if ($baselineSeal.run_id -cne $BaselineRunId -or
                $baselineSeal.scenario_id -cne [string]$baselineSealRef.scenario_id -or
                $baselineSeal.seal_status -cne 'BOUND' -or
                -not (Test-Path -LiteralPath $baselineSourcePath -PathType Leaf) -or
                (Get-FileSha256 $baselineSourcePath) -cne [string]$baselineSeal.source_receipt_sha256) {
                throw 'MT-046 baseline sealed-attempt content is stale, cross-run, or hash-invalid'
            }
            $baselineAttempt = Get-Content -LiteralPath $baselineSourcePath -Raw | ConvertFrom-Json
            if ($baselineAttempt.run_id -cne $BaselineRunId -or
                $baselineAttempt.scenario_id -cne $baselineSeal.scenario_id -or
                $baselineAttempt.attempt_id -cne $baselineSeal.attempt_id) {
                throw 'MT-046 baseline source attempt does not match its seal'
            }
            $baselineAttemptRows.Add($baselineAttempt)
        }
        if ($baselineAttemptRows.Count -ne 18 -or
            @($baselineAttemptRows.scenario_id | Sort-Object -Unique).Count -ne 18 -or
            @($baselineAttemptRows.attempt_id | Sort-Object -Unique).Count -ne 18 -or
            @($baselineRun.workspace_cleanup).Count -eq 0 -or
            @($baselineRun.workspace_cleanup | Where-Object {
                -not (Test-Path -LiteralPath ([string]$_.path) -PathType Leaf) -or
                (Get-FileSha256 ([string]$_.path)) -cne [string]$_.sha256
            }).Count -ne 0 -or
            -not (Test-Path -LiteralPath ([string]$baselineRun.argus_terminal_proof.path) -PathType Leaf) -or
            (Get-FileSha256 ([string]$baselineRun.argus_terminal_proof.path)) -cne
                [string]$baselineRun.argus_terminal_proof.sha256) {
            throw 'MT-046 baseline terminal evidence is incomplete or hash-invalid'
        }
        $baselineAttempts = @($baselineAttemptRows.attempt_id)
        $currentAttempts = @($sealedAttempts | ForEach-Object {
            (Get-Content -LiteralPath $_.path -Raw | ConvertFrom-Json).attempt_id
        })
        if ($baselineRun.provenance.candidate_source_id -cne $candidateSourceId -or
            $baselineRun.provenance.backend_sha256 -cne $backendSha256 -or
            $baselineRun.provenance.manifest_sha256 -cne $manifestSha256 -or
            @($currentAttempts | Where-Object { $_ -in $baselineAttempts }).Count -ne 0 -or
            $baselineRun.status -cne 'PASS' -or $baselineRun.terminal_scenario_count -ne 18 -or
            $baselineRun.exact_scenario_set -ne $true -or $baselineRun.all_statuses_accepted -ne $true) {
            throw 'MT-046 second run reused stale attempts or changed candidate source identity'
        }
        $baselineScenarioProjection = @($baselineRun.scenarios.PSObject.Properties | ForEach-Object {
            "$($_.Name)=$($_.Value.status)"
        } | Sort-Object)
        $currentScenarioProjection = @($current.scenarios.PSObject.Properties | ForEach-Object {
            "$($_.Name)=$($_.Value.status)"
        } | Sort-Object)
        if (($baselineScenarioProjection -join "`n") -cne ($currentScenarioProjection -join "`n")) {
            throw 'MT-046 second run accepted scenario projection differs from the baseline'
        }
        $idempotencyComparison = [ordered]@{
            baseline_run_id = $BaselineRunId; baseline_sha256 = $baselineRunSha256;
            current_run_id = $RunId; candidate_source_id = $candidateSourceId;
            distinct_attempts = $true; accepted_scenario_projection_equal = $true;
            baseline_immutable_unchanged = $true; status = 'PASS'
        }
        $current.idempotency_comparison = $idempotencyComparison
    }
    $immutableRunSha = Write-ImmutableJson -Path $immutableRunPath -Value $current
    Write-JsonAtomic -Path $currentRunPath -Value $current
    Write-JsonAtomic -Path $latestRunPath -Value $current

    $summarySha = Write-ImmutableJson -Path $supervisorSummaryPath -Value ([ordered]@{
        schema_id = 'hsk.wp_kernel_012.mt046_supervisor_summary@1'
        run_id = $RunId
        status = 'PASS'
        provenance = $provenance
        immutable_run_path = $immutableRunPath
        immutable_run_sha256 = $immutableRunSha
        sealed_attempt_count = $sealedAttempts.Count
        exact_workspace_cleanup_count = $workspaceCleanup.Count
        command_count = $commands.Count
        argus_terminal_proof = $argusReceiptPath
        argus_terminal_proof_sha256 = $argusReceiptSha
        idempotency_comparison = $idempotencyComparison
        completed_at = [DateTimeOffset]::UtcNow.ToString('O')
    })
    $runSucceeded = $true
    Write-Output ([ordered]@{
        run_id = $RunId; status = 'PASS'; source_sha = $sourceSha;
        supervisor_summary = $supervisorSummaryPath; supervisor_summary_sha256 = $summarySha
    } | ConvertTo-Json -Depth 8)
}
catch {
    $failure = $_
    $partialState = if (Test-Path -LiteralPath $currentRunPath -PathType Leaf) {
        try { Get-Content -LiteralPath $currentRunPath -Raw | ConvertFrom-Json } catch { $null }
    } else { $null }
    $failureProvenance = Get-Variable -Name provenance -ValueOnly -ErrorAction SilentlyContinue
    if ($null -eq $failureProvenance) {
        $failureProvenance = [ordered]@{
            supervisor_run_id = $RunId
            source_sha = $sourceSha
            candidate_source_id = $candidateSourceId
            source_dirty_policy = 'controlled_candidate_status_patch_and_untracked_hashes_bound'
            source_dirty_result = @($dirtyRows)
            source_dirty_result_sha256 = $sourceDirtyResultSha256
            candidate_source_binding = $candidateBindingPath
            candidate_source_binding_sha256 = $candidateBindingSha256
            cargo_profile = 'debug'
            cargo_locked = $true
            canonical_target_root = $targetRoot
            backend_path = Get-Variable -Name backendPath -ValueOnly -ErrorAction SilentlyContinue
            backend_sha256 = Get-Variable -Name backendSha256 -ValueOnly -ErrorAction SilentlyContinue
            postgres = $postgresReceipt
            manifest_path = $manifestPath
            manifest_sha256 = $manifestSha256
            job_helper_path = $jobHelperPath
            job_helper_sha256 = $jobHelperSha256
            supervisor_pid = $PID
            canonical_test_discovery = $discoveryPath
            canonical_test_discovery_sha256 = $discoverySha256
        }
    }
    $failed = [ordered]@{
        schema_id = 'hsk.wp_kernel_012.interconnection_run@2'
        work_packet_id = 'WP-KERNEL-012'
        micro_task_id = 'MT-046'
        run_id = $RunId
        source_sha = $sourceSha
        candidate_source_id = $candidateSourceId
        status = 'FAIL'
        terminal_reason = $failure.Exception.Message
        provenance = $failureProvenance
        partial_state = $partialState
        completed_commands = $commands
        completed_at = [DateTimeOffset]::UtcNow.ToString('O')
    }
    if (-not (Test-Path -LiteralPath $immutableRunPath)) {
        $failedImmutableSha = Write-ImmutableJson -Path $immutableRunPath -Value $failed
        $failed['immutable_run_path'] = $immutableRunPath
        $failed['immutable_run_sha256'] = $failedImmutableSha
    } else {
        $invalidationPath = Join-Path $runRoot 'pass-publication-invalidated.json'
        $invalidationSha = Write-ImmutableJson -Path $invalidationPath -Value ([ordered]@{
            schema_id = 'hsk.wp_kernel_012.mt046_pass_publication_invalidation@1'
            run_id = $RunId
            status = 'FAIL'
            terminal_reason = $failure.Exception.Message
            preexisting_immutable_run_path = $immutableRunPath
            preexisting_immutable_run_sha256 = Get-FileSha256 $immutableRunPath
            completed_at = [DateTimeOffset]::UtcNow.ToString('O')
        })
        $failed['pass_publication_invalidation'] = [ordered]@{
            path = $invalidationPath; sha256 = $invalidationSha
        }
    }
    Write-JsonAtomic -Path $currentRunPath -Value $failed
    Write-JsonAtomic -Path $latestRunPath -Value $failed
    if (-not (Test-Path -LiteralPath $supervisorSummaryPath)) {
        [void](Write-ImmutableJson -Path $supervisorSummaryPath -Value $failed)
    }
    throw
}
finally {
    foreach ($observerJob in @($processObservationJobs)) {
        Stop-Job -Job $observerJob -ErrorAction SilentlyContinue
        Remove-Job -Job $observerJob -Force -ErrorAction SilentlyContinue
    }
    if ($null -ne $supervisorLockStream) {
        $supervisorLockStream.Dispose()
    }
    if (-not $runSucceeded) {
        Write-Verbose "MT-046 terminal FAIL projection published for $RunId"
    }
}
