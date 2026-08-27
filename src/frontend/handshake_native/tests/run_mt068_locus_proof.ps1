<#
.SYNOPSIS
    WP-KERNEL-012 MT-068 canonical aggregate Locus proof supervisor.

.DESCRIPTION
    `validation_v4` remediation item 9: the critical Locus proof may stay runner-only for isolation, but
    ONE canonical command must run the ordinary suite PLUS the ignored live test and FAIL THE AGGREGATE
    when either component is skipped, fails, times out, produces stale artifacts, or leaves residue.

    A default `cargo test` that exits zero while reporting the material proof as `ignored` is exactly the
    failure mode V4 rejected, so this supervisor:

      * refuses to start unless every compiled input matches committed HEAD (the deliberate main-only
        `AGENTS.md`/`CLAUDE.md` surfaces excepted, which the in-test gate excludes identically),
      * builds the CURRENT-SOURCE backend into the MT-scoped external Cargo target,
      * DISCOVERS the expected scenario counts from the test source itself, so a test that silently
        disappears, or a mandatory scenario that becomes `#[ignore]`, fails the aggregate,
      * runs both scenarios under the reviewed MT-045 Windows Job Object containment (hard wall-clock
        bound + descendant reaping + leak detection) rather than a second containment core,
      * requires the live scenario to report exactly `1 passed; 0 failed`,
      * binds the canonical evidence JSON to THIS run (fresh mtime + exact source SHA) and re-verifies
        all four PNG digests and dimensions from disk,
      * independently verifies the product cleanup projection and that no embedded store directory
        survives below the supervisor-owned runtime root.

    Exit code 0 only when every one of those holds.

.EXAMPLE
    pwsh -NoProfile -File run_mt068_locus_proof.ps1 -RunId MT068-RUN-20260806A
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidatePattern('^MT068-RUN-[A-Za-z0-9_-]{1,118}$')]
    [string]$RunId,

    [ValidateRange(300, 3600)]
    [int]$PerScenarioTimeoutSeconds = 1800,

    [ValidateRange(600, 10800)]
    [int]$WholeRunTimeoutSeconds = 7200
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$TEST_BINARY = 'test_locus_interop'
$LIVE_SCENARIO = 'resolve_locus_ref_against_real_surrealdb_live'

function Get-FileSha256 {
    param([Parameter(Mandatory)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Cannot hash missing file '$Path'"
    }
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
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

function Assert-Png {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][string]$ExpectedSha256)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Declared canonical frame is missing from disk: '$Path'"
    }
    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 24 -or $bytes[0] -ne 137 -or $bytes[1] -ne 80 -or
        $bytes[2] -ne 78 -or $bytes[3] -ne 71) {
        throw "Declared canonical frame is not a material PNG: '$Path'"
    }
    $width = [Net.IPAddress]::NetworkToHostOrder([BitConverter]::ToInt32($bytes, 16))
    $height = [Net.IPAddress]::NetworkToHostOrder([BitConverter]::ToInt32($bytes, 20))
    if ($width -lt 1280 -or $height -lt 800) {
        throw "Canonical frame '$Path' is ${width}x${height}; the MT-068 proof renders 1440x900"
    }
    $actual = Get-FileSha256 $Path
    if ($actual -cne $ExpectedSha256.ToLowerInvariant()) {
        throw "Canonical frame '$Path' digest $actual does not match the declared $ExpectedSha256"
    }
    return [ordered]@{ path = $Path; width = $width; height = $height; sha256 = $actual }
}

function Get-TestResultCounts {
    param([Parameter(Mandatory)][string]$StdoutPath, [Parameter(Mandatory)][string]$Scenario)
    $text = Get-Content -LiteralPath $StdoutPath -Raw
    $match = [regex]::Match(
        $text,
        'test result:\s+(?<verdict>ok|FAILED)\.\s+(?<passed>\d+)\s+passed;\s+(?<failed>\d+)\s+failed;\s+(?<ignored>\d+)\s+ignored;\s+(?<measured>\d+)\s+measured;\s+(?<filtered>\d+)\s+filtered out')
    if (-not $match.Success) {
        throw "${Scenario}: no libtest result summary was produced; the scenario never reported a terminal verdict"
    }
    return [ordered]@{
        verdict = $match.Groups['verdict'].Value
        passed = [int]$match.Groups['passed'].Value
        failed = [int]$match.Groups['failed'].Value
        ignored = [int]$match.Groups['ignored'].Value
        measured = [int]$match.Groups['measured'].Value
        filtered_out = [int]$match.Groups['filtered'].Value
    }
}

# ── Source binding ───────────────────────────────────────────────────────────────────────────────
$crateRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$repoRoot = (& git -C $crateRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw 'Unable to resolve the MT-068 product repository root'
}
$sourceSha = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceSha -notmatch '^[0-9a-f]{40}$') {
    throw "Unable to bind MT-068 to an exact committed source SHA; got '$sourceSha'"
}
# The in-test `current_runtime_source_tree` gate excludes exactly these two deliberate main-only
# authority surfaces (CX-113A). The supervisor uses the identical exclusion so the two gates cannot
# disagree about what "clean" means.
$dirtyRows = @(& git -C $repoRoot status --porcelain=v1 --untracked-files=all -- `
        '.' ':(exclude)AGENTS.md' ':(exclude)CLAUDE.md')
if ($LASTEXITCODE -ne 0) { throw 'Unable to inspect MT-068 runtime source cleanliness' }
$dirtyRows = @($dirtyRows | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
if ($dirtyRows.Count -ne 0) {
    throw "MT-068 proof requires every compiled input to match committed HEAD; dirty rows: $($dirtyRows -join '; ')"
}

# ── Scenario declaration gate (source side) ──────────────────────────────────────────────────────
# The MT's own file must declare EXACTLY ONE runner-only scenario, and it must be the live proof. The
# expected COUNTS deliberately do NOT come from here: this binary also compiles `#[path]`-included
# support modules (`backend_proof_support` contributes its own unit tests), so a source regex over one file
# would silently undercount. The authoritative inventory is taken from the built binary below.
$testSourcePath = Join-Path $PSScriptRoot "$TEST_BINARY.rs"
$testSource = Get-Content -LiteralPath $testSourcePath -Raw
$declaredIgnored = @([regex]::Matches($testSource, '(?m)^#\[ignore'))
if ($declaredIgnored.Count -ne 1) {
    throw "MT-068 expects exactly one runner-only #[ignore] scenario in $TEST_BINARY.rs; found $($declaredIgnored.Count)"
}
if ($testSource -notmatch [regex]::Escape("fn $LIVE_SCENARIO()")) {
    throw "The mandatory live scenario $LIVE_SCENARIO is absent from $TEST_BINARY.rs"
}
$expectedIgnored = 1

# ── External artifact roots ──────────────────────────────────────────────────────────────────────
$artifactRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot '..\Handshake_Artifacts'))
if (-not (Test-Path -LiteralPath $artifactRoot -PathType Container)) {
    throw "Canonical Handshake_Artifacts root is unavailable: '$artifactRoot'"
}
$targetRoot = Join-Path $artifactRoot 'handshake-cargo-target\wp012-mt068'
$testArtifactRoot = Join-Path $artifactRoot 'handshake-test'
$canonicalArgusRoot = Join-Path $testArtifactRoot 'wp-kernel-012-mt-068\canonical-argus'
$supervisorRoot = Join-Path $testArtifactRoot 'wp-kernel-012-mt-068\supervisor'
$runRoot = Join-Path $supervisorRoot "runs\$RunId"
if (Test-Path -LiteralPath $runRoot) {
    throw "RunId '$RunId' is not fresh; supervisor evidence already exists at '$runRoot'"
}
[void][IO.Directory]::CreateDirectory($runRoot)
$summaryPath = Join-Path $runRoot 'mt068-aggregate-proof.json'
$runStartedAtUtc = [DateTimeOffset]::UtcNow
$wholeRunDeadline = $runStartedAtUtc.AddSeconds($WholeRunTimeoutSeconds)

# ── Embedded store identity (opened and closed by the backend, never by this script) ──────────────
# Handshake's database runs INSIDE the backend process. There is no listener to
# find, no owning process to identify, and no direct_db_client to locate: the store is a
# directory under this run's root, held under an exclusive RocksDB lock for as
# long as the backend lives.
$storeRoot = Join-Path $runRoot 'backend-runtime'
[void][IO.Directory]::CreateDirectory($storeRoot)
$storeIdentity = [IO.Path]::GetFullPath($storeRoot)

# Direct database access is deliberately absent. Product APIs prove exact record lifecycle while the
# supervisor independently verifies that the run-owned embedded store cannot survive cleanup.

# ── Reviewed MT-045 Windows Job Object containment (extracted, never duplicated) ──────────────────
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

$cargo = (Get-Command cargo -CommandType Application).Source
$scenarioReceipts = [Collections.Generic.List[object]]::new()

function Invoke-ContainedCommand {
    param(
        [Parameter(Mandatory)][string]$Label,
        [Parameter(Mandatory)][string]$Executable,
        [Parameter(Mandatory)][string[]]$Arguments,
        [Parameter(Mandatory)][string]$WorkingDirectory,
        [ValidateRange(500, 600000)][int]$DescendantExitGraceMilliseconds = 15000
    )
    $stdoutPath = Join-Path $runRoot "$Label.stdout.log"
    $stderrPath = Join-Path $runRoot "$Label.stderr.log"
    $remainingMs = [int][Math]::Floor(($wholeRunDeadline - [DateTimeOffset]::UtcNow).TotalMilliseconds)
    if ($remainingMs -le 0) { throw "MT-068 whole-run deadline exhausted before '$Label' started" }
    $budgetMs = [Math]::Min($PerScenarioTimeoutSeconds * 1000, $remainingMs)
    $startedAt = [DateTimeOffset]::UtcNow
    $result = [Mt045JobRunner]::Run(
        $Executable, $Arguments, $WorkingDirectory, $stdoutPath, $stderrPath,
        $budgetMs, $DescendantExitGraceMilliseconds)
    $receipt = [ordered]@{
        label = $Label
        executable = $Executable
        arguments = @($Arguments)
        working_directory = $WorkingDirectory
        root_process_id = [int]$result.RootProcessId
        started_at = $startedAt.ToString('O')
        completed_at = [DateTimeOffset]::UtcNow.ToString('O')
        budget_milliseconds = $budgetMs
        exit_code = [int]$result.ExitCode
        timed_out = [bool]$result.TimedOut
        leaked_process_count = [int]$result.LeakedProcessCount
        leaked_process_ids = @($result.LeakedProcessIds | Where-Object { $null -ne $_ })
        post_drain_descendant_process_ids = @($result.PostDrainDescendantProcessIds |
            Where-Object { $null -ne $_ })
        stdout_path = $stdoutPath
        stdout_sha256 = Get-FileSha256 $stdoutPath
        stderr_path = $stderrPath
        stderr_sha256 = Get-FileSha256 $stderrPath
    }
    return $receipt
}

function Assert-ContainedCommandPassed {
    param([Parameter(Mandatory)]$Receipt)
    if ($Receipt.timed_out) {
        throw "Scenario '$($Receipt.label)' exceeded its hard $($Receipt.budget_milliseconds)ms bound"
    }
    if ($Receipt.leaked_process_count -ne 0 -or @($Receipt.post_drain_descendant_process_ids).Count -ne 0) {
        throw "Scenario '$($Receipt.label)' left owned processes alive: $($Receipt.leaked_process_ids -join ',')"
    }
    if ($Receipt.exit_code -ne 0) {
        throw "Scenario '$($Receipt.label)' failed with exit code $($Receipt.exit_code); stderr=$($Receipt.stderr_path)"
    }
}

$aggregateStatus = 'FAIL'
$terminalReason = 'NEVER_REACHED_TERMINAL_STATE'
try {
    # ── Build the CURRENT-SOURCE backend ─────────────────────────────────────────────────────────
    $build = Invoke-ContainedCommand -Label 'build-current-source-backend' -Executable $cargo `
        -Arguments @('build', '--locked', '-j', '2', '--target-dir', $targetRoot, '--manifest-path',
            (Join-Path $repoRoot 'src\backend\handshake_core\Cargo.toml'), '--bin', 'handshake_core',
            '--features', 'app-runtime') `
        -WorkingDirectory $repoRoot -DescendantExitGraceMilliseconds 180000
    $scenarioReceipts.Add($build)
    Assert-ContainedCommandPassed -Receipt $build
    $backendPath = Join-Path $targetRoot 'debug\handshake_core.exe'
    $backendSha256 = Get-FileSha256 $backendPath

    # ── Proof environment (external artifacts only; real SurrealDB; GPU frames) ──────────────────
    $env:CARGO_TARGET_DIR = $targetRoot
    $env:HANDSHAKE_ARTIFACTS_ROOT = $artifactRoot
    $env:HANDSHAKE_TEST_ARTIFACTS_ROOT = $testArtifactRoot
    $env:HSK_TEST_BACKEND_BIN = $backendPath
    $env:HANDSHAKE_DATA_DIR = $storeIdentity
    $env:HANDSHAKE_GPU_SCREENSHOT = '1'
    $env:HSK_MT045_RUN_ID = $RunId
    Remove-Item Env:HSK_TEST_BASE -ErrorAction SilentlyContinue
    Remove-Item Env:HANDSHAKE_ARGUS_MATRIX_RUN_ID -ErrorAction SilentlyContinue

    # ── Authoritative scenario inventory, taken from the BUILT binary ─────────────────────────────
    $listArgs = @('test', '--locked', '-j', '2', '--test', $TEST_BINARY, '--', '--list')
    $list = Invoke-ContainedCommand -Label 'scenario-inventory' -Executable $cargo `
        -Arguments $listArgs -WorkingDirectory $crateRoot -DescendantExitGraceMilliseconds 180000
    $scenarioReceipts.Add($list)
    Assert-ContainedCommandPassed -Receipt $list
    $listText = Get-Content -LiteralPath $list.stdout_path -Raw
    $listedTests = @([regex]::Matches($listText, '(?m)^(?<name>[A-Za-z0-9_:]+): test$') |
        ForEach-Object { $_.Groups['name'].Value })
    $listSummary = [regex]::Match($listText, '(?m)^(?<total>\d+) tests, \d+ benchmarks$')
    if (-not $listSummary.Success -or [int]$listSummary.Groups['total'].Value -ne $listedTests.Count) {
        throw "scenario-inventory could not bind an exact test inventory for $TEST_BINARY"
    }
    $totalScenarios = $listedTests.Count
    if (@($listedTests | Where-Object { $_ -ceq $LIVE_SCENARIO }).Count -ne 1) {
        throw "The mandatory live scenario $LIVE_SCENARIO is not present exactly once in the built binary"
    }
    $expectedOrdinaryPassed = $totalScenarios - $expectedIgnored

    # ── Scenario 1: the ordinary suite ───────────────────────────────────────────────────────────
    $ordinaryArgs = @('test', '--locked', '-j', '2', '--test', $TEST_BINARY, '--',
        '--test-threads=1', '--nocapture')
    $ordinary = Invoke-ContainedCommand -Label 'ordinary-suite' -Executable $cargo `
        -Arguments $ordinaryArgs -WorkingDirectory $crateRoot
    $scenarioReceipts.Add($ordinary)
    Assert-ContainedCommandPassed -Receipt $ordinary
    $ordinaryCounts = Get-TestResultCounts -StdoutPath $ordinary.stdout_path -Scenario 'ordinary-suite'
    if ($ordinaryCounts.verdict -cne 'ok' -or $ordinaryCounts.failed -ne 0) {
        throw "ordinary-suite reported $($ordinaryCounts.failed) failures"
    }
    if ($ordinaryCounts.passed -ne $expectedOrdinaryPassed -or
        $ordinaryCounts.ignored -ne $expectedIgnored -or $ordinaryCounts.filtered_out -ne 0) {
        throw "ordinary-suite reported $($ordinaryCounts.passed) passed / $($ordinaryCounts.ignored) ignored / $($ordinaryCounts.filtered_out) filtered; the built binary declares $expectedOrdinaryPassed / $expectedIgnored / 0"
    }
    # The mandatory scenario must be visibly SKIPPED here, never counted as ordinary acceptance.
    $ordinaryStdout = Get-Content -LiteralPath $ordinary.stdout_path -Raw
    if ($ordinaryStdout -notmatch "(?m)^test $([regex]::Escape($LIVE_SCENARIO)) \.\.\. ignored") {
        throw "ordinary-suite did not record $LIVE_SCENARIO as ignored; a mandatory scenario must never be silently absorbed into the default result"
    }

    # ── Scenario 2: the mandatory runner-only live proof ──────────────────────────────────────────
    $liveArgs = @('test', '--locked', '-j', '2', '--test', $TEST_BINARY, $LIVE_SCENARIO, '--',
        '--ignored', '--exact', '--test-threads=1', '--nocapture')
    $live = Invoke-ContainedCommand -Label 'runner-only-live-proof' -Executable $cargo `
        -Arguments $liveArgs -WorkingDirectory $crateRoot
    $scenarioReceipts.Add($live)
    Assert-ContainedCommandPassed -Receipt $live
    $liveCounts = Get-TestResultCounts -StdoutPath $live.stdout_path -Scenario 'runner-only-live-proof'
    if ($liveCounts.verdict -cne 'ok' -or $liveCounts.passed -ne 1 -or
        $liveCounts.failed -ne 0 -or $liveCounts.ignored -ne 0 -or
        $liveCounts.filtered_out -ne ($totalScenarios - 1)) {
        throw "runner-only-live-proof must report exactly 1 passed / 0 failed / 0 ignored / $($totalScenarios - 1) filtered; got $($liveCounts.passed)/$($liveCounts.failed)/$($liveCounts.ignored)/$($liveCounts.filtered_out)"
    }
    # Under `--nocapture` libtest interleaves the test's own stdout between the `test <name> ... `
    # prefix and its terminal `ok`, so an anchored `... ok` match is not available here. The exact
    # counts above already carry the verdict; these two checks additionally prove the scenario really
    # STARTED (rather than being filtered to nothing) and that no failures section was emitted.
    $liveStdout = Get-Content -LiteralPath $live.stdout_path -Raw
    if ($liveStdout -notmatch "(?m)^test $([regex]::Escape($LIVE_SCENARIO)) \.\.\.") {
        throw "runner-only-live-proof never started $LIVE_SCENARIO"
    }
    if ($liveStdout -match '(?m)^failures:') {
        throw "runner-only-live-proof emitted a failures section"
    }

    # ── Canonical evidence must be FRESH and bound to THIS source ─────────────────────────────────
    $shortSha = $sourceSha.Substring(0, 12)
    $candidates = @(Get-ChildItem -LiteralPath $canonicalArgusRoot -Directory -Filter "run-$shortSha-*" |
        ForEach-Object { Join-Path $_.FullName 'mt068-locus-canonical-argus.json' } |
        Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
        Where-Object { (Get-Item -LiteralPath $_).LastWriteTimeUtc -ge $runStartedAtUtc.UtcDateTime })
    if ($candidates.Count -ne 1) {
        throw "Expected exactly one FRESH canonical evidence file for source $shortSha written during this run; found $($candidates.Count)"
    }
    $evidencePath = $candidates[0]
    $evidence = Get-Content -LiteralPath $evidencePath -Raw | ConvertFrom-Json
    if ($evidence.status -cne 'PASS') { throw "Canonical evidence status is '$($evidence.status)'" }
    if ($evidence.source.source_sha -cne $sourceSha) {
        throw "Canonical evidence binds source $($evidence.source.source_sha), not $sourceSha"
    }
    if ($evidence.test -cne $LIVE_SCENARIO) {
        throw "Canonical evidence names test '$($evidence.test)'"
    }

    # ── All four canonical frames must exist, be material, and match their declared digests ───────
    $frames = [Collections.Generic.List[object]]::new()
    foreach ($state in @($evidence.canonical_argus_state_matrix)) {
        foreach ($phase in @('before', 'after')) {
            $shot = $state.screenshots.$phase
            if ($shot.status -cne 'CAPTURED') {
                throw "Canonical frame $($state.state)/$phase is '$($shot.status)', not CAPTURED"
            }
            $frames.Add((Assert-Png -Path ([string]$shot.path) -ExpectedSha256 ([string]$shot.sha256)))
        }
        # Independently of what the run asserted about itself: a before/after pair with identical
        # bytes reproduced the pre-action frame and cannot evidence the navigation destination.
        if ([string]$state.screenshots.before.sha256 -ceq [string]$state.screenshots.after.sha256) {
            throw "Canonical frames for $($state.state) are byte-identical; the after frame is a stale pre-navigation capture"
        }
        if ([string]$state.observed_navigation_content_id -cne [string]$state.expected_navigation_content_id) {
            throw "Canonical navigation identity mismatch for $($state.state)"
        }
        if ([string]$state.observation.receipt_status -cne 'applied') {
            throw "Canonical action for $($state.state) terminalized '$($state.observation.receipt_status)', not applied"
        }
        $predicates = @($state.observation.terminal_predicates)
        if ($predicates.Count -lt 1 -or @($predicates | Where-Object { -not $_.passed }).Count -ne 0) {
            throw "Canonical action for $($state.state) carries no passing terminal predicate"
        }
        if (-not [bool]$state.observation.terminal_refreshed) {
            throw "Canonical action for $($state.state) was not rebound to an authoritative terminal snapshot"
        }
    }
    if ($frames.Count -ne 4) {
        throw "MT-068 requires exactly four canonical frames (WP before/after, MT before/after); got $($frames.Count)"
    }

    # ── Independent product-cleanup + embedded-store containment verification ───────────────────
    foreach ($field in @('workspace_absent', 'persisted_document_absent', 'work_packet_rows_zero',
            'microtask_rows_zero', 'eventledger_mirror_rows_zero', 'flight_recorder_rows_zero')) {
        if (-not [bool]$evidence.cleanup.$field) {
            throw "Canonical product cleanup projection did not prove '$field'"
        }
    }
    $survivingStoreDirectories = if (Test-Path -LiteralPath $storeIdentity -PathType Container) {
        @(Get-ChildItem -LiteralPath $storeIdentity -Recurse -Directory -Filter 'handshake-surreal' |
            ForEach-Object { $_.FullName })
    } else {
        @()
    }
    if ($survivingStoreDirectories.Count -ne 0) {
        throw "Fixture-owned embedded store survived cleanup: $($survivingStoreDirectories -join ', ')"
    }
    $residue = 'product_absence_proven_and_run_owned_store_removed'

    $aggregateStatus = 'PASS'
    $terminalReason = 'ORDINARY_AND_RUNNER_ONLY_SCENARIOS_BOTH_TERMINALIZED_WITH_ZERO_RESIDUE'
    $summary = [ordered]@{
        schema_id = 'hsk.wp_kernel_012.mt068_aggregate_locus_proof@1'
        work_packet_id = 'WP-KERNEL-012'
        micro_task_id = 'MT-068'
        run_id = $RunId
        status = $aggregateStatus
        terminal_reason = $terminalReason
        source_sha = $sourceSha
        started_at = $runStartedAtUtc.ToString('O')
        completed_at = [DateTimeOffset]::UtcNow.ToString('O')
        supervisor_pid = $PID
        job_helper_path = $jobHelperPath
        job_helper_sha256 = $jobHelperSha256
        backend_path = $backendPath
        backend_sha256 = $backendSha256
        store = [ordered]@{
            kind = 'embedded_surrealdb'
            identity = $storeIdentity
            store_directory_name = 'handshake-surreal'
            namespace = 'handshake'
            database = 'primary'
            lifecycle = 'opened_and_closed_by_the_backend_process'
            external_server = $false
        }
        discovered_scenarios = [ordered]@{
            inventory_source = 'built binary --list'
            total_scenarios = $totalScenarios
            listed_scenarios = @($listedTests)
            declared_ignored = $expectedIgnored
            expected_ordinary_passed = $expectedOrdinaryPassed
            mandatory_runner_only_scenario = $LIVE_SCENARIO
        }
        ordinary_result = $ordinaryCounts
        live_result = $liveCounts
        canonical_evidence_path = $evidencePath
        canonical_evidence_sha256 = Get-FileSha256 $evidencePath
        canonical_frames = @($frames)
        store_residue_probe = $residue
        canonical_cleanup_projection = $evidence.cleanup
        surviving_store_directories = @($survivingStoreDirectories)
        commands = @($scenarioReceipts)
    }
    Write-JsonAtomic -Path $summaryPath -Value $summary
    Write-Output "MT-068 AGGREGATE PASS run_id=$RunId source_sha=$sourceSha ordinary=$($ordinaryCounts.passed)/$($ordinaryCounts.failed)/$($ordinaryCounts.ignored) live=$($liveCounts.passed)/$($liveCounts.failed)/$($liveCounts.ignored) frames=$($frames.Count) residue='$residue' summary=$summaryPath"
    exit 0
}
catch {
    $terminalReason = $_.Exception.Message
    Write-JsonAtomic -Path $summaryPath -Value ([ordered]@{
        schema_id = 'hsk.wp_kernel_012.mt068_aggregate_locus_proof@1'
        work_packet_id = 'WP-KERNEL-012'
        micro_task_id = 'MT-068'
        run_id = $RunId
        status = 'FAIL'
        terminal_reason = $terminalReason
        source_sha = $sourceSha
        started_at = $runStartedAtUtc.ToString('O')
        completed_at = [DateTimeOffset]::UtcNow.ToString('O')
        commands = @($scenarioReceipts)
    })
    Write-Error "MT-068 AGGREGATE FAIL run_id=$RunId : $terminalReason (summary=$summaryPath)"
    exit 1
}
