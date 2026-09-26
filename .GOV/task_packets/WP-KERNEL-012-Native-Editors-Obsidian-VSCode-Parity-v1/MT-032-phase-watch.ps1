param(
    [Parameter(Mandatory)][string]$LaneRoot,
    [Parameter(Mandatory)][string]$RuntimeRoot,
    [Parameter(Mandatory)][string]$StopSignal,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$CandidateSha,
    [ValidateRange(60, 86400)][int]$MaxSeconds = 21600
)

# Observation only: no process control, fixture mutation, or retention override.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$utf8 = [Text.UTF8Encoding]::new($false)
$comparison = [StringComparison]::OrdinalIgnoreCase
$share = [IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete

function Get-VerifiedDirectory([string]$Path) {
    $resolved = (Resolve-Path -LiteralPath $Path).ProviderPath.TrimEnd('\', '/')
    $item = Get-Item -LiteralPath $resolved
    if (-not $item.PSIsContainer) { throw 'expected_directory' }
    # Reject junction/symlink ancestors, including the caller-supplied lane itself.
    for ($cursor = [IO.DirectoryInfo]::new($resolved); $null -ne $cursor; $cursor = $cursor.Parent) {
        $attributes = $cursor.Attributes
        if ([int]$attributes -eq -1) { throw [IO.DirectoryNotFoundException]::new('directory_disappeared') }
        if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw 'reparse_directory_rejected'
        }
    }
    return $resolved
}

function Write-NewJson([string]$Path, $Value) {
    $stream = [IO.File]::Open($Path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
    try {
        $bytes = $utf8.GetBytes(($Value | ConvertTo-Json -Depth 8 -Compress) + "`n")
        $stream.Write($bytes, 0, $bytes.Length)
        $stream.Flush($true)
    } finally { $stream.Dispose() }
}

function Get-Component([string]$Prefix, [string]$Value) {
    $hash = [Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Value))
    return $Prefix + '-' + [Convert]::ToHexString($hash).ToLowerInvariant().Substring(0, 16)
}

$lane = Get-VerifiedDirectory $LaneRoot
$runtime = Get-VerifiedDirectory $RuntimeRoot
$expected = [IO.Path]::GetFullPath((Join-Path $lane 'e/wp-kernel-012/backend-runtime')).TrimEnd('\', '/')
if (-not $runtime.Equals($expected, $comparison)) { throw 'runtime_root_not_assigned_lane' }
$logs = Get-VerifiedDirectory (Join-Path $lane 'logs')
$stop = [IO.Path]::GetFullPath($StopSignal)
$stopParent = Get-VerifiedDirectory ([IO.Path]::GetDirectoryName($stop))
if (-not $stopParent.StartsWith($lane + [IO.Path]::DirectorySeparatorChar, $comparison)) {
    throw 'stop_signal_outside_lane'
}
if (Test-Path -LiteralPath $stop) { throw 'stop_signal_already_exists' }

# The approved entrypoint uses the fixture's default run id. Do not silently watch a different run.
if ($env:HSK_MT045_RUN_ID -and $env:HSK_MT045_RUN_ID -ne 'standalone-run') {
    throw 'nondefault_fixture_run_id_requires_declared_watcher_scope'
}
$runRoot = Join-Path $runtime (Get-Component 'r' 'standalone-run')
$scenarios = [ordered]@{}
foreach ($name in @(
    'live_surrealdb_owned_restart_preserves_document_backlink_and_content_hash',
    'live_surrealdb_self_seeded_loom_block_backlink_hash_and_ui_proof'
)) {
    $scenarios[(Get-Component 's' $name)] = $name
}
$phases = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
@('create_request', 'save_request', 'create_body', 'middleware_authorize',
  'middleware_workspace_lookup', 'create_transaction', 'create_receipt', 'create_embeds',
  'create_index', 'save_crdt_lookup', 'save_test_pause', 'save_transaction', 'save_receipt',
  'save_backlinks', 'save_embeds', 'save_index') | ForEach-Object { [void]$phases.Add($_) }

function Get-ScenarioRoots {
    try {
        if (-not (Test-Path -LiteralPath $runRoot -PathType Container)) { return }
        [void](Get-VerifiedDirectory $runRoot)
    } catch [System.Management.Automation.ItemNotFoundException] { return
    } catch [IO.DirectoryNotFoundException] { return }
    foreach ($scenario in $scenarios.Keys) {
        try {
            $parent = Join-Path $runRoot $scenario
            if (-not (Test-Path -LiteralPath $parent -PathType Container)) { continue }
            [void](Get-VerifiedDirectory $parent)
            foreach ($directory in Get-ChildItem -LiteralPath $parent -Directory -Force) {
                $uuid = [Guid]::Empty
                if (-not [Guid]::TryParseExact($directory.Name, 'D', [ref]$uuid)) { continue }
                $attributes = $directory.Attributes
                if ([int]$attributes -eq -1) { continue }
                if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                    throw 'reparse_runtime_root_rejected'
                }
                [pscustomobject]@{ Path = $directory.FullName; Scenario = $scenarios[$scenario] }
            }
        } catch [System.Management.Automation.ItemNotFoundException] {
            # Normal cleanup race; still inspect the other scenario, including during baseline.
        } catch [IO.DirectoryNotFoundException] {
            # Retry next poll; containment/reparse failures still propagate.
        }
    }
}

$baseline = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($root in @(Get-ScenarioRoots)) { [void]$baseline.Add($root.Path) }
$prefix = Join-Path $logs "mt032-phase-watch-$CandidateSha"
$capturePath = "$prefix.jsonl"
$readyPath = "$prefix.ready.json"
$summaryPath = "$prefix.summary.json"
foreach ($path in @($capturePath, $readyPath, $summaryPath)) {
    if (Test-Path -LiteralPath $path) { throw 'watcher_output_already_exists' }
}
$roots = @{}
$files = @{}
$seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$begins = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$ends = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$issues = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$started = [DateTime]::UtcNow
$watch = [Diagnostics.Stopwatch]::StartNew()
$capture = [IO.File]::Open($capturePath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
$writer = [IO.StreamWriter]::new($capture, $utf8)
$writer.AutoFlush = $true
$state = @{ Records = 0; Polls = 0; StopObserved = $false; Expired = $false; Fatal = $false }

function Accept-PhaseLine([string]$Line, $FileState) {
    if (-not $Line.Contains('MT032_DOCUMENT_PHASE')) { return }
    try {
        $stamp = $null; $request = $null; $phase = $null; $event = $null; $elapsed = $null
        if ($FileState.Kind -eq 'json') {
            $entry = $Line | ConvertFrom-Json -AsHashtable
            if ($entry.target -ne 'handshake_core::knowledge_documents_api' -or
                $entry.fields.message -ne 'MT032_DOCUMENT_PHASE') { return }
            $stamp = $entry.timestamp
            $request = $entry.fields.request_id
            $phase = $entry.fields.phase
            $event = $entry.fields.event
            $elapsed = $entry.fields.elapsed_ms
        } else {
            $plain = [regex]::Replace($Line, '\x1B\[[0-?]*[ -/]*[@-~]', '')
            if ($plain -notmatch '^\s*\S+\s+INFO\s+handshake_core::knowledge_documents_api:\s+MT032_DOCUMENT_PHASE(?:\s|$)') { return }
            $stamp = [regex]::Match($plain, '^\s*(\S+)').Groups[1].Value
            $request = [regex]::Match($plain, '\brequest_id="?([0-9a-fA-F-]{36})"?').Groups[1].Value
            $phase = [regex]::Match($plain, '\bphase="?([a-z_]+)"?').Groups[1].Value
            $event = [regex]::Match($plain, '\bevent="?(begin|end|error)"?').Groups[1].Value
            $elapsed = [regex]::Match($plain, '\belapsed_ms="?([0-9]+)"?').Groups[1].Value
        }
        $id = [Guid]::Empty; $milliseconds = [uint64]0; $timestamp = [DateTimeOffset]::MinValue
        if (-not [Guid]::TryParseExact([string]$request, 'D', [ref]$id) -or
            -not $phases.Contains([string]$phase) -or $event -notin @('begin', 'end', 'error') -or
            -not [uint64]::TryParse([string]$elapsed, [ref]$milliseconds) -or
            -not [DateTimeOffset]::TryParse([string]$stamp, [ref]$timestamp)) {
            [void]$issues.Add('invalid_phase_record'); return
        }
        # stdout and JSON are two projections of the same event. Each phase executes once per request.
        $key = "$id|$phase"
        if (-not $seen.Add("$key|$event")) { return }
        if ($event -eq 'begin') { [void]$begins.Add($key) } else { [void]$ends.Add($key) }
        $record = [ordered]@{
            observed_utc = [DateTime]::UtcNow.ToString('o')
            timestamp = $timestamp.ToUniversalTime().ToString('o')
            request_id = $id.ToString('D'); phase = [string]$phase; event = [string]$event
            elapsed_ms = $milliseconds; scenario = $FileState.Scenario
            runtime_root = [IO.Path]::GetRelativePath($runtime, $FileState.Root)
            source = $FileState.RelativePath
        }
        $writer.WriteLine(($record | ConvertTo-Json -Compress))
        $state.Records++
        $roots[$FileState.Root].Records++
    } catch { [void]$issues.Add('phase_parse_or_capture_error') }
}

function Read-Available($FileState) {
    if (-not [IO.File]::Exists($FileState.Path)) { return }
    $inputStream = $null
    try {
        $item = Get-Item -LiteralPath $FileState.Path
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            [void]$issues.Add('reparse_log_rejected'); return
        }
        [void](Get-VerifiedDirectory $item.DirectoryName)
        $inputStream = [IO.File]::Open($FileState.Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, $share)
        if ($inputStream.Length -lt $FileState.Offset) {
            [void]$issues.Add('log_truncated'); return
        }
        [void]$inputStream.Seek($FileState.Offset, [IO.SeekOrigin]::Begin)
        $buffer = [byte[]]::new(65536)
        $count = $inputStream.Read($buffer, 0, $buffer.Length)
        $FileState.Backlog = ($inputStream.Length - $FileState.Offset - $count) -gt 0
        if ($count -eq 0) { return }
        $FileState.Offset += $count
        $chars = [char[]]::new($utf8.GetMaxCharCount($count))
        $charCount = $FileState.Decoder.GetChars($buffer, 0, $count, $chars, 0, $false)
        for ($index = 0; $index -lt $charCount; $index++) {
            if ($chars[$index] -eq "`n") {
                if (-not $FileState.Discard) { Accept-PhaseLine $FileState.Pending.ToString().TrimEnd("`r") $FileState }
                [void]$FileState.Pending.Clear(); $FileState.Discard = $false
            } elseif (-not $FileState.Discard) {
                if ($FileState.Pending.Length -ge 16384) {
                    [void]$FileState.Pending.Clear(); $FileState.Discard = $true
                    [void]$issues.Add('oversize_log_line_not_captured')
                } else { [void]$FileState.Pending.Append($chars[$index]) }
            }
        }
    } catch [IO.FileNotFoundException] {
        # Expected when successful fixture cleanup wins the open/read race.
    } catch [IO.DirectoryNotFoundException] {
    } catch { [void]$issues.Add('log_read_or_containment_error') }
    finally { if ($null -ne $inputStream) { $inputStream.Dispose() } }
}

function Poll-Phases {
    $state.Polls++
    foreach ($root in @(Get-ScenarioRoots)) {
        if ($baseline.Contains($root.Path) -or $roots.ContainsKey($root.Path)) { continue }
        $roots[$root.Path] = @{ Scenario = $root.Scenario; Records = 0 }
        foreach ($relative in @('backend.stdout.log', 'data/logs/handshake_core.log')) {
            $path = Join-Path $root.Path $relative
            $files[$path] = @{
                Path = $path; Root = $root.Path; Scenario = $root.Scenario; RelativePath = $relative
                Kind = $(if ($relative -eq 'backend.stdout.log') { 'stdout' } else { 'json' })
                Offset = [int64]0; Decoder = $utf8.GetDecoder()
                Pending = [Text.StringBuilder]::new(); Discard = $false; Backlog = $false
            }
        }
    }
    foreach ($file in $files.Values) { Read-Available $file }
}

try {
    Write-NewJson $readyPath ([ordered]@{
        schema = 'handshake.mt032.phase-watch.ready.v1'; candidate_sha = $CandidateSha
        watcher_pid = $PID; ready_utc = [DateTime]::UtcNow.ToString('o'); poll_ms = 100
        lane_root = $lane; runtime_root = $runtime; stop_signal = $stop
        capture_path = $capturePath; summary_path = $summaryPath; baseline_roots = $baseline.Count
    })
    while ($watch.Elapsed.TotalSeconds -lt $MaxSeconds) {
        Poll-Phases
        if ([IO.File]::Exists($stop)) { $state.StopObserved = $true; break }
        Start-Sleep -Milliseconds 100
    }
    if (-not $state.StopObserved) { $state.Expired = $true; [void]$issues.Add('watcher_deadline') }
    # One final bounded drain; never delete or preserve fixture roots ourselves.
    Poll-Phases
} catch {
    $state.Fatal = $true; [void]$issues.Add('watcher_fatal_error')
} finally {
    foreach ($file in $files.Values) {
        if ($file.Pending.Length -gt 0 -or $file.Discard) { [void]$issues.Add('partial_final_log_line') }
        if ($file.Backlog) { [void]$issues.Add('final_read_budget_exceeded') }
    }
    foreach ($key in $begins) { if (-not $ends.Contains($key)) { [void]$issues.Add('phase_begin_without_terminal') } }
    foreach ($key in $ends) { if (-not $begins.Contains($key)) { [void]$issues.Add('phase_terminal_without_begin') } }
    if ($state.Records -eq 0) { [void]$issues.Add('no_phase_records') }
    foreach ($name in $scenarios.Values) {
        if (@($roots.Values | Where-Object { $_.Scenario -eq $name -and $_.Records -gt 0 }).Count -eq 0) {
            [void]$issues.Add('scenario_without_phase_records')
        }
    }
    $writer.Dispose()
    Write-NewJson $summaryPath ([ordered]@{
        schema = 'handshake.mt032.phase-watch.summary.v1'; candidate_sha = $CandidateSha
        started_utc = $started.ToString('o'); completed_utc = [DateTime]::UtcNow.ToString('o')
        watcher_pid = $PID; records = $state.Records; polls = $state.Polls
        new_roots = @($roots.Keys | Sort-Object | ForEach-Object { [IO.Path]::GetRelativePath($runtime, $_) })
        baseline_roots_excluded = $baseline.Count; stop_observed = $state.StopObserved
        incomplete_stream = ($issues.Count -gt 0); issues = @($issues | Sort-Object)
        capture_sha256 = (Get-FileHash -LiteralPath $capturePath -Algorithm SHA256).Hash
        completeness_limit = 'Polling cannot prove absence of events in a file deleted before first observation; missing terminal may be a stalled request or capture loss.'
    })
}
if ($state.Fatal -or $state.Expired) { exit 1 }
