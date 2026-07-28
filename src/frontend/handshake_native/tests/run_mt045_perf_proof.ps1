[CmdletBinding()]
param(
    [string]$RunId = ("MT045-RUN-" + [guid]::NewGuid().ToString("N")),
    [int]$CommandTimeoutSeconds = 1800,
    [string]$PostgresDsn = "postgresql://postgres@127.0.0.1:5544/handshake"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$crateRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$repoRoot = (Resolve-Path (Join-Path $crateRoot "..\..\..")).Path
$artifactRootPath = Join-Path (Split-Path $repoRoot -Parent) "Handshake_Artifacts"
if (-not (Test-Path -LiteralPath $artifactRootPath -PathType Container)) {
    throw "The existing sibling Handshake_Artifacts root is required; this supervisor will not create it: $artifactRootPath"
}
$artifactRoot = (Resolve-Path -LiteralPath $artifactRootPath).Path
if ((Split-Path $artifactRoot -Leaf) -cne "Handshake_Artifacts") {
    throw "Resolved artifact root is not the canonical Handshake_Artifacts directory: $artifactRoot"
}
$targetRootPath = Join-Path $artifactRoot "handshake-cargo-target"
if (-not (Test-Path -LiteralPath $targetRootPath -PathType Container)) {
    throw "The configured canonical Cargo target must already exist: $targetRootPath"
}
$targetRoot = (Resolve-Path -LiteralPath $targetRootPath).Path
$runRoot = Join-Path $artifactRoot "wp-kernel-012\mt-045\supervisor\$RunId"
if (Test-Path -LiteralPath $runRoot) {
    throw "Supervisor run id already exists: $RunId"
}
[void][IO.Directory]::CreateDirectory($runRoot)

$sourcePaths = @(
    "src/frontend/handshake_native/build.rs",
    "src/frontend/handshake_native/Cargo.toml",
    "src/frontend/handshake_native/Cargo.lock",
    "src/frontend/handshake_native/src",
    "src/frontend/handshake_native/tests/perf_proof_support/mod.rs",
    "src/frontend/handshake_native/tests/pg_proof_support/mod.rs",
    "src/frontend/handshake_native/tests/test_perf_large_code.rs",
    "src/frontend/handshake_native/tests/test_perf_large_rich.rs",
    "src/frontend/handshake_native/tests/test_perf_large_knowledge.rs",
    "src/frontend/handshake_native/tests/run_mt045_perf_proof.ps1"
)

function Invoke-GitText {
    param([Parameter(Mandatory)][string[]]$Arguments)
    $text = & git -C $repoRoot @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed: $text"
    }
    return (($text | Out-String).Trim())
}

$sourceSha = Invoke-GitText @("rev-parse", "HEAD")
$sourceStatus = Invoke-GitText (@("status", "--porcelain=v1", "--") + $sourcePaths)
if (-not [string]::IsNullOrWhiteSpace($sourceStatus)) {
    throw "Canonical MT-045 source-binding paths must match committed HEAD:`n$sourceStatus"
}

$forbiddenBudgetOverrides = Get-ChildItem Env: |
    Where-Object { $_.Name -like "PERF_BUDGET_*" -and -not [string]::IsNullOrWhiteSpace($_.Value) }
if ($forbiddenBudgetOverrides) {
    throw "Canonical MT-045 proof forbids PERF_BUDGET_* overrides: $($forbiddenBudgetOverrides.Name -join ', ')"
}
if (-not [string]::IsNullOrWhiteSpace($env:SKIP_PERF_TESTS)) {
    throw "Canonical MT-045 proof forbids SKIP_PERF_TESTS"
}

$pgListener = Get-NetTCPConnection -LocalAddress "127.0.0.1" -LocalPort 5544 -State Listen -ErrorAction Stop |
    Select-Object -First 1
if (-not $pgListener) {
    throw "Handshake internal PostgreSQL is not listening at 127.0.0.1:5544"
}
$pgProcess = Get-Process -Id $pgListener.OwningProcess -ErrorAction Stop
if ($pgProcess.ProcessName -cne "postgres") {
    throw "Port 5544 is not owned by PostgreSQL (pid=$($pgListener.OwningProcess), process=$($pgProcess.ProcessName))"
}

function Stop-OwnedProcessTree {
    param([Parameter(Mandatory)][int]$RootPid)
    $owned = [Collections.Generic.HashSet[int]]::new()
    [void]$owned.Add($RootPid)
    do {
        $added = $false
        foreach ($process in Get-CimInstance Win32_Process) {
            if ($owned.Contains([int]$process.ParentProcessId) -and $owned.Add([int]$process.ProcessId)) {
                $added = $true
            }
        }
    } while ($added)
    $descendants = @($owned | Where-Object { $_ -ne $RootPid } | Sort-Object -Descending)
    foreach ($pid in $descendants) {
        Stop-Process -Id $pid -Force -ErrorAction SilentlyContinue
    }
    Stop-Process -Id $RootPid -Force -ErrorAction SilentlyContinue
}

function Invoke-BoundedCargo {
    param(
        [Parameter(Mandatory)][string]$Label,
        [Parameter(Mandatory)][string[]]$Arguments,
        [int]$TimeoutSeconds = $CommandTimeoutSeconds
    )
    $safeLabel = $Label -replace "[^A-Za-z0-9_.-]", "-"
    $stdoutPath = Join-Path $runRoot "$safeLabel.stdout.log"
    $stderrPath = Join-Path $runRoot "$safeLabel.stderr.log"
    $startedAt = [DateTimeOffset]::UtcNow
    $process = Start-Process -FilePath "cargo" -ArgumentList $Arguments -WorkingDirectory $crateRoot `
        -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath `
        -WindowStyle Hidden -PassThru
    try {
        Wait-Process -Id $process.Id -Timeout $TimeoutSeconds -ErrorAction Stop
    }
    catch {
        Stop-OwnedProcessTree -RootPid $process.Id
        throw "$Label exceeded ${TimeoutSeconds}s; only its owned process tree was terminated. Logs: $stdoutPath, $stderrPath"
    }
    $process.WaitForExit()
    $process.Refresh()
    if ($process.ExitCode -ne 0) {
        throw "$Label failed with exit code $($process.ExitCode). Logs: $stdoutPath, $stderrPath"
    }
    return [ordered]@{
        label = $Label
        command = "cargo " + ($Arguments -join " ")
        started_at = $startedAt.ToString("O")
        completed_at = [DateTimeOffset]::UtcNow.ToString("O")
        exit_code = $process.ExitCode
        stdout = $stdoutPath
        stdout_sha256 = (Get-FileHash -LiteralPath $stdoutPath -Algorithm SHA256).Hash.ToLowerInvariant()
        stderr = $stderrPath
        stderr_sha256 = (Get-FileHash -LiteralPath $stderrPath -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

function Write-ImmutableJson {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)]$Value
    )
    $json = ($Value | ConvertTo-Json -Depth 20) + [Environment]::NewLine
    $encoding = [Text.UTF8Encoding]::new($false)
    $stream = [IO.File]::Open($Path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
    try {
        $bytes = $encoding.GetBytes($json)
        $stream.Write($bytes, 0, $bytes.Length)
        $stream.Flush($true)
    }
    finally {
        $stream.Dispose()
    }
    $digest = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    $digestPath = "$Path.sha256"
    [IO.File]::WriteAllText(
        $digestPath,
        "$digest  $(Split-Path $Path -Leaf)$([Environment]::NewLine)",
        $encoding
    )
    (Get-Item -LiteralPath $Path).IsReadOnly = $true
    (Get-Item -LiteralPath $digestPath).IsReadOnly = $true
    return $digest
}

$env:HSK_MT045_CANONICAL_RUN = "1"
$env:HSK_MT045_RUN_ID = $RunId
$env:HSK_MT045_SOURCE_SHA = $sourceSha
$env:HANDSHAKE_ARTIFACTS_ROOT = $artifactRoot
$env:HANDSHAKE_TEST_PG_DSN = $PostgresDsn
$env:HANDSHAKE_TEST_STAGE_BINDING_ROOT = (Join-Path $runRoot "binding")
Remove-Item Env:HSK_TEST_BASE -ErrorAction SilentlyContinue

$commands = [Collections.Generic.List[object]]::new()
$commands.Add((Invoke-BoundedCargo -Label "build-handshake-core-release" -Arguments @(
    "build", "--release", "--manifest-path", "..\..\backend\handshake_core\Cargo.toml",
    "--bin", "handshake_core", "--features", "app-runtime"
)))
$backendBinary = Join-Path $targetRoot "release\handshake_core.exe"
if (-not (Test-Path -LiteralPath $backendBinary -PathType Leaf)) {
    throw "Release backend build did not produce $backendBinary"
}
$env:HSK_TEST_BACKEND_BIN = (Resolve-Path -LiteralPath $backendBinary).Path
$backendSha256 = (Get-FileHash -LiteralPath $env:HSK_TEST_BACKEND_BIN -Algorithm SHA256).Hash.ToLowerInvariant()

$diagnosticCommands = @(
    @("test_heartbeat", "heartbeat_advances_by_n_over_n_frames"),
    @("test_heartbeat", "idle_repaint_cadence_is_bounded"),
    @("test_diagnostics_panel", "panel_projects_live_heartbeat_frame_and_events")
)
$diagnosticResults = [Collections.Generic.List[object]]::new()
foreach ($entry in $diagnosticCommands) {
    $bin = $entry[0]
    $test = $entry[1]
    $result = Invoke-BoundedCargo -Label "diagnostic-$test" -Arguments @(
        "test", "--release", "--test", $bin, $test, "--", "--exact", "--nocapture", "--test-threads=1"
    )
    $commands.Add($result)
    $diagnosticResults.Add($result)
}
$diagnosticReceiptPath = Join-Path $runRoot "diagnostics-preflight.json"
$diagnosticReceipt = [ordered]@{
    schema_id = "hsk.wp_kernel_012.mt045_diagnostics_preflight@1"
    work_packet_id = "WP-KERNEL-012"
    micro_task_id = "MT-045"
    run_id = $RunId
    source_sha = $sourceSha
    status = "PASS"
    tests = $diagnosticResults
    completed_at = [DateTimeOffset]::UtcNow.ToString("O")
}
$diagnosticSha256 = Write-ImmutableJson -Path $diagnosticReceiptPath -Value $diagnosticReceipt
$env:HSK_MT045_DIAGNOSTIC_RECEIPT = $diagnosticReceiptPath

$perfCommands = @(
    @("test_perf_large_code", "perf_proof_perf_lc01_initial_render"),
    @("test_perf_large_code", "perf_proof_perf_lc02_scroll_to_bottom"),
    @("test_perf_large_code", "perf_proof_perf_lc03_find_replace"),
    @("test_perf_large_code", "perf_proof_perf_lc04_multi_cursor"),
    @("test_perf_large_code", "perf_proof_perf_lc05_memory"),
    @("test_perf_large_code", "perf_proof_perf_lc06_codebase_index"),
    @("test_perf_large_code", "perf_proof_perf_lc07_minimap"),
    @("test_perf_large_code", "perf_proof_perf_lc08_diagnostics_overlay"),
    @("test_perf_large_rich", "perf_proof_perf_lr01_load_large_doc"),
    @("test_perf_large_rich", "perf_proof_perf_lr02_scroll_large_doc"),
    @("test_perf_large_rich", "perf_proof_perf_lr03_find_in_doc"),
    @("test_perf_large_rich", "perf_proof_perf_lr04_save_large_doc"),
    @("test_perf_large_rich", "perf_proof_perf_lr05_transclusion_chain_live"),
    @("test_perf_large_rich", "perf_proof_perf_lr06_memory"),
    @("test_perf_large_rich", "perf_proof_perf_lr07_html_projection"),
    @("test_perf_large_knowledge", "perf_proof_perf_lk01_graph_load"),
    @("test_perf_large_knowledge", "perf_proof_perf_lk02_graph_layout"),
    @("test_perf_large_knowledge", "perf_proof_perf_lk03_tag_hub"),
    @("test_perf_large_knowledge", "perf_proof_perf_lk04_search_index"),
    @("test_perf_large_knowledge", "perf_proof_perf_lk05_folder_tree")
)
foreach ($entry in $perfCommands) {
    $bin = $entry[0]
    $test = $entry[1]
    $commands.Add((Invoke-BoundedCargo -Label $test -Arguments @(
        "test", "--release", "--test", $bin, $test, "--", "--exact", "--nocapture", "--test-threads=1"
    )))
}

$measurementRoot = Join-Path $artifactRoot "wp-kernel-012\mt-045\measurements"
$currentRunPath = Join-Path $measurementRoot "current-run.json"
$manifestPath = Join-Path $crateRoot "tests\perf_proof\perf_manifest.json"
$immutableRunPath = Join-Path $measurementRoot "runs\$RunId.json"
$immutableRunDigestPath = "$immutableRunPath.sha256"
foreach ($required in @($currentRunPath, $manifestPath, $immutableRunPath, $immutableRunDigestPath)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Canonical MT-045 completion artifact is missing: $required"
    }
}
$currentRun = Get-Content -LiteralPath $currentRunPath -Raw | ConvertFrom-Json
if ($currentRun.run_id -cne $RunId -or $currentRun.status -cne "PASS") {
    throw "MT-045 current run is not PASS for $RunId"
}
$manifest = @(Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json)
if ($manifest.Count -ne 20) {
    throw "MT-045 manifest must contain exactly 20 rows, found $($manifest.Count)"
}
foreach ($row in $manifest) {
    $contractBudget = if ($null -ne $row.budget_ms) { $row.budget_ms } else { $row.budget_mb }
    if (
        $row.status -cne "PASS" -or
        $row.measured_profile -cne "release" -or
        $row.suite_run_id -cne $RunId -or
        $row.override_applied -ne $false -or
        $row.gated -ne $false -or
        $null -eq $row.measured_value -or
        $row.effective_budget -ne $contractBudget
    ) {
        throw "MT-045 manifest row $($row.scenario_id) is not an exact canonical PASS"
    }
}

$supervisorSummaryPath = Join-Path $runRoot "supervisor-summary.json"
$supervisorSummary = [ordered]@{
    schema_id = "hsk.wp_kernel_012.mt045_supervisor_summary@1"
    work_packet_id = "WP-KERNEL-012"
    micro_task_id = "MT-045"
    run_id = $RunId
    status = "PASS"
    source_sha = $sourceSha
    cargo_profile = "release"
    budget_overrides = @()
    postgres = [ordered]@{
        endpoint = "127.0.0.1:5544"
        owning_pid = $pgListener.OwningProcess
        process_name = $pgProcess.ProcessName
        lifecycle = "existing_internal_postgresql_never_stopped"
    }
    backend = [ordered]@{
        path = $env:HSK_TEST_BACKEND_BIN
        sha256 = $backendSha256
        managed_postgres = $false
    }
    diagnostics_receipt = $diagnosticReceiptPath
    diagnostics_receipt_sha256 = $diagnosticSha256
    immutable_run_summary = $immutableRunPath
    immutable_run_summary_sha256 = (Get-FileHash -LiteralPath $immutableRunPath -Algorithm SHA256).Hash.ToLowerInvariant()
    manifest = $manifestPath
    commands = $commands
    completed_at = [DateTimeOffset]::UtcNow.ToString("O")
}
$supervisorSha256 = Write-ImmutableJson -Path $supervisorSummaryPath -Value $supervisorSummary
Write-Output ([ordered]@{
    run_id = $RunId
    status = "PASS"
    source_sha = $sourceSha
    supervisor_summary = $supervisorSummaryPath
    supervisor_summary_sha256 = $supervisorSha256
} | ConvertTo-Json -Depth 5)
