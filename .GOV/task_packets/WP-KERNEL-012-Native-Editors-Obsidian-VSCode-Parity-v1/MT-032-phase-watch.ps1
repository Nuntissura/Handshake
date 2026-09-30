param(
    [Parameter(Mandatory)][string]$LaneRoot,
    [Parameter(Mandatory)][string]$RuntimeRoot,
    [Parameter(Mandatory)][string]$StopSignal,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$CandidateSha,
    [ValidateRange(0, 86400)][int]$MaxSeconds = 21600,
    [ValidateSet('', 'continuation1', 'receipt-inner-v15', 'lookup-plan-v16', 'mt032-selection')][string]$CaptureSuffix = ''
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
$lookupPlanMode = $CaptureSuffix -eq 'lookup-plan-v16'
$scenarios = [ordered]@{}
foreach ($name in @(
    'live_surrealdb_owned_restart_preserves_document_backlink_and_content_hash',
    'live_surrealdb_self_seeded_loom_block_backlink_hash_and_ui_proof'
)) {
    if ($lookupPlanMode -and $name -ne 'live_surrealdb_owned_restart_preserves_document_backlink_and_content_hash') { continue }
    $scenarios[(Get-Component 's' $name)] = $name
}
$phases = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
@('create_request', 'save_request', 'create_body', 'middleware_authorize',
  'middleware_workspace_lookup', 'create_transaction', 'create_receipt', 'create_embeds',
  'create_index', 'save_crdt_lookup', 'save_test_pause', 'save_transaction', 'save_receipt',
  'save_backlinks', 'save_embeds', 'save_index') | ForEach-Object { [void]$phases.Add($_) }
@('storage_lease_wait', 'storage_namespace', 'storage_signin', 'storage_operation',
  'mutation_lock_wait', 'storage_query', 'storage_response_check', 'storage_response_decode',
  'storage_response_decode_pair', 'receipt_authorize', 'receipt_append', 'retry_attempt',
  'retry_sleep', 'index_source_lookup', 'index_source_stale', 'index_source_upsert',
  'index_entity_upsert', 'backlink_source_read', 'backlink_prior_read', 'backlink_target_reads',
  'backlink_write', 'index_source_prepare', 'index_source_transaction',
  'authority_lease_wait', 'authority_operation', 'authority_namespace',
  'authority_resource_query', 'authority_signin', 'authority_session_query',
  'authority_grant_query', 'authority_response_decode', 'authority_audit',
  'authority_audit_namespace', 'authority_audit_query', 'index_source_parent_authorize',
  'index_source_delete_authorize', 'create_owned_transaction', 'create_owned_result_read') |
    ForEach-Object { [void]$phases.Add($_) }

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
$receiptMode = $CaptureSuffix -in @('receipt-inner-v15', 'lookup-plan-v16', 'mt032-selection')
$prefix = Join-Path $logs $(if ($lookupPlanMode) { "mt032-v16-lookup-plan-watch-$CandidateSha" }
elseif ($receiptMode) { "mt032-v15-receipt-watch-$CandidateSha" + $(if ($CaptureSuffix -eq 'mt032-selection') { '-mt032-selection' } else { '' }) } else {
    "mt032-v14-statement-watch-$CandidateSha" + $(if ($CaptureSuffix) { "-$CaptureSuffix" } else { '' })
})
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
$beginPhases = @{}
$ends = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$issues = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$started = [DateTime]::UtcNow
$watch = [Diagnostics.Stopwatch]::StartNew()
$capture = [IO.File]::Open($capturePath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
$writer = [IO.StreamWriter]::new($capture, $utf8)
$writer.AutoFlush = $true
$state = @{ Records = 0; PhaseRecords = 0; StatementRecords = 0; UnavailableStatementTimings = 0; Polls = 0; StopObserved = $false; Expired = $false; Fatal = $false }
$statementQueries = @{}
$receiptObservations = @{}
$state.ReceiptRecords = 0; $state.InvalidReceiptTimings = 0
$lookupPlans = @{}
$state.LookupPlanRecords = 0; $state.UnsupportedLookupPlans = 0; $state.InvalidLookupPlanTimings = 0

function Accept-PhaseLine([string]$Line, $FileState) {
    if ($lookupPlanMode -and $Line.Contains('MT032_DOCUMENT_RECEIPT_PLAN')) { Accept-LookupPlanLine $Line $FileState; return }
    if ($receiptMode -and $Line.Contains('MT032_DOCUMENT_RECEIPT')) { Accept-ReceiptLine $Line $FileState; return }
    if ($Line.Contains('MT032_DOCUMENT_STATEMENT')) { Accept-StatementLine $Line $FileState; return }
    if (-not $Line.Contains('MT032_DOCUMENT_PHASE')) { return }
    try {
        $stamp = $null; $request = $null; $phase = $null; $event = $null; $elapsed = $null
        $observation = $null; $parentObservation = $null; $count = $null
        if ($FileState.Kind -eq 'json') {
            $entry = $Line | ConvertFrom-Json -AsHashtable
            if ($entry.target -ne 'handshake_core::knowledge_documents_api' -or
                $entry.fields.message -ne 'MT032_DOCUMENT_PHASE') { return }
            $stamp = $entry.timestamp
            $request = $entry.fields.request_id
            $phase = $entry.fields.phase
            $event = $entry.fields.event
            $elapsed = $entry.fields.elapsed_ms
            $observation = $entry.fields.observation_id
            $parentObservation = $entry.fields.parent_observation_id
            $count = $entry.fields.count
        } else {
            $plain = [regex]::Replace($Line, '\x1B\[[0-?]*[ -/]*[@-~]', '')
            if ($plain -notmatch '^\s*\S+\s+INFO\s+handshake_core::knowledge_documents_api:\s+MT032_DOCUMENT_PHASE(?:\s|$)') { return }
            $stamp = [regex]::Match($plain, '^\s*(\S+)').Groups[1].Value
            $request = [regex]::Match($plain, '\brequest_id="?([0-9a-fA-F-]{36})"?').Groups[1].Value
            $phase = [regex]::Match($plain, '\bphase="?([a-z_]+)"?').Groups[1].Value
            $event = [regex]::Match($plain, '\bevent="?(begin|end|error|dropped)"?').Groups[1].Value
            $elapsed = [regex]::Match($plain, '\belapsed_ms="?([0-9]+)"?').Groups[1].Value
            $observation = [regex]::Match($plain, '\bobservation_id="?([0-9]+)"?').Groups[1].Value
            $parentObservation = [regex]::Match($plain, '\bparent_observation_id="?([0-9]+)"?').Groups[1].Value
            $count = [regex]::Match($plain, '\bcount="?([0-9]+)"?').Groups[1].Value
        }
        # ConvertFrom-Json may materialize ISO timestamps as DateTime on newer PowerShell.
        # Preserve invariant ISO text before validation; culture formatting is not a timestamp source.
        if ($stamp -is [DateTime]) { $stamp = $stamp.ToUniversalTime().ToString('o', [Globalization.CultureInfo]::InvariantCulture) }
        elseif ($stamp -is [DateTimeOffset]) { $stamp = $stamp.ToUniversalTime().ToString('o', [Globalization.CultureInfo]::InvariantCulture) }
        $id = [Guid]::Empty; $milliseconds = [uint64]0; $timestamp = [DateTimeOffset]::MinValue
        $observationId = [uint64]0; $parentId = [uint64]0; $counter = [uint64]0
        if (-not [Guid]::TryParseExact([string]$request, 'D', [ref]$id) -or
            -not $phases.Contains([string]$phase) -or $event -notin @('begin', 'end', 'error', 'dropped') -or
            -not [uint64]::TryParse([string]$observation, [ref]$observationId) -or $observationId -eq 0 -or
            -not [uint64]::TryParse([string]$parentObservation, [ref]$parentId) -or
            -not [uint64]::TryParse([string]$count, [ref]$counter) -or
            -not [uint64]::TryParse([string]$elapsed, [ref]$milliseconds) -or
            -not [DateTimeOffset]::TryParse([string]$stamp, [ref]$timestamp)) {
            [void]$issues.Add('invalid_phase_record'); return
        }
        # Runtime UUID distinguishes replacement processes; observation IDs distinguish repeated phases.
        # stdout and JSON are two projections of the same event, not two operations.
        $key = "$($FileState.Root)|$id|$observationId"
        if (-not $seen.Add("$key|$event")) { return }
        if ($event -eq 'begin') { [void]$begins.Add($key); $beginPhases[$key] = [string]$phase } else {
            if (-not $ends.Add($key)) { [void]$issues.Add('observation_multiple_terminal_events') }
        }
        $record = [ordered]@{
            kind = 'phase'
            observed_utc = [DateTime]::UtcNow.ToString('o')
            timestamp = $timestamp.ToUniversalTime().ToString('o')
            request_id = $id.ToString('D'); phase = [string]$phase; event = [string]$event
            elapsed_ms = $milliseconds; scenario = $FileState.Scenario
            observation_id = $observationId; parent_observation_id = $parentId; count = $counter
            backend_pid = $roots[$FileState.Root].BackendPid
            runtime_root = [IO.Path]::GetRelativePath($runtime, $FileState.Root)
            source = $FileState.RelativePath
        }
        $writer.WriteLine(($record | ConvertTo-Json -Compress))
        $state.Records++
        $state.PhaseRecords++
        $roots[$FileState.Root].Records++
    } catch { [void]$issues.Add('phase_parse_or_capture_error') }
}

function Accept-StatementLine([string]$Line, $FileState) {
    try {
        if ($FileState.Kind -eq 'json') {
            $entry = $Line | ConvertFrom-Json -AsHashtable
            if ($entry.target -ne 'handshake_core::knowledge_documents_api' -or
                $entry.fields.message -ne 'MT032_DOCUMENT_STATEMENT') { return }
            $stamp = $entry.timestamp; $request = $entry.fields.request_id
            $query = $entry.fields.query_observation_id; $index = $entry.fields.statement_index
            $total = $entry.fields.statement_count; $available = $entry.fields.timing_available
            $micros = $entry.fields.execution_time_us
        } else {
            $plain = [regex]::Replace($Line, '\x1B\[[0-?]*[ -/]*[@-~]', '')
            if ($plain -notmatch '^\s*\S+\s+INFO\s+handshake_core::knowledge_documents_api:\s+MT032_DOCUMENT_STATEMENT(?:\s|$)') { return }
            $stamp = [regex]::Match($plain, '^\s*(\S+)').Groups[1].Value
            $request = [regex]::Match($plain, '\brequest_id="?([0-9a-fA-F-]{36})"?').Groups[1].Value
            $query = [regex]::Match($plain, '\bquery_observation_id="?([0-9]+)"?').Groups[1].Value
            $index = [regex]::Match($plain, '\bstatement_index="?([0-9]+)"?').Groups[1].Value
            $total = [regex]::Match($plain, '\bstatement_count="?([0-9]+)"?').Groups[1].Value
            $available = [regex]::Match($plain, '\btiming_available="?(true|false)"?').Groups[1].Value
            $micros = [regex]::Match($plain, '\bexecution_time_us="?([0-9]+)"?').Groups[1].Value
        }
        if ($stamp -is [DateTime]) { $stamp = $stamp.ToUniversalTime().ToString('o', [Globalization.CultureInfo]::InvariantCulture) }
        elseif ($stamp -is [DateTimeOffset]) { $stamp = $stamp.ToUniversalTime().ToString('o', [Globalization.CultureInfo]::InvariantCulture) }
        $id = [Guid]::Empty; $timestamp = [DateTimeOffset]::MinValue
        $queryId = [uint64]0; $statementIndex = [uint64]0; $statementCount = [uint64]0
        $executionMicros = [uint64]0; $timingAvailable = $false
        if (-not [Guid]::TryParseExact([string]$request, 'D', [ref]$id) -or
            -not [DateTimeOffset]::TryParse([string]$stamp, [ref]$timestamp) -or
            -not [uint64]::TryParse([string]$query, [ref]$queryId) -or $queryId -eq 0 -or
            -not [uint64]::TryParse([string]$index, [ref]$statementIndex) -or
            -not [uint64]::TryParse([string]$total, [ref]$statementCount) -or $statementCount -eq 0 -or
            $statementIndex -ge $statementCount -or
            -not [bool]::TryParse([string]$available, [ref]$timingAvailable) -or
            -not [uint64]::TryParse([string]$micros, [ref]$executionMicros) -or
            (-not $timingAvailable -and $executionMicros -ne 0)) {
            [void]$issues.Add('invalid_statement_record'); return
        }
        $queryKey = "$($FileState.Root)|$id|$queryId"
        if (-not $statementQueries.ContainsKey($queryKey)) {
            $statementQueries[$queryKey] = @{ Count = $statementCount; Indices = [Collections.Generic.HashSet[uint64]]::new() }
        }
        if ($statementQueries[$queryKey].Count -ne $statementCount) { [void]$issues.Add('statement_count_changed') }
        if (-not $seen.Add("statement|$queryKey|$statementIndex")) { return }
        [void]$statementQueries[$queryKey].Indices.Add($statementIndex)
        $record = [ordered]@{
            kind = 'statement'; observed_utc = [DateTime]::UtcNow.ToString('o')
            timestamp = $timestamp.ToUniversalTime().ToString('o'); request_id = $id.ToString('D')
            query_observation_id = $queryId; statement_index = $statementIndex; statement_count = $statementCount
            timing_available = $timingAvailable; execution_time_us = $executionMicros
            scenario = $FileState.Scenario; backend_pid = $roots[$FileState.Root].BackendPid
            runtime_root = [IO.Path]::GetRelativePath($runtime, $FileState.Root); source = $FileState.RelativePath
        }
        $writer.WriteLine(($record | ConvertTo-Json -Compress))
        $state.Records++; $state.StatementRecords++; $roots[$FileState.Root].Records++
        if (-not $timingAvailable) { $state.UnavailableStatementTimings++ }
    } catch { [void]$issues.Add('statement_parse_or_capture_error') }
}

function Accept-ReceiptLine([string]$Line, $FileState) {
    try {
        if ($FileState.Kind -eq 'json') {
            $entry = $Line | ConvertFrom-Json -AsHashtable
            if ($entry.target -ne 'handshake_core::knowledge_documents_api' -or
                $entry.fields.message -ne 'MT032_DOCUMENT_RECEIPT') { return }
            $stamp = $entry.timestamp; $request = $entry.fields.request_id
            $observation = $entry.fields.receipt_observation_id; $branch = $entry.fields.branch
            $clock = $entry.fields.clock; $valid = $entry.fields.timing_valid
            $lookup = $entry.fields.lookup_elapsed_us; $operation = $entry.fields.operation_elapsed_us
        } else {
            $plain = [regex]::Replace($Line, '\x1B\[[0-?]*[ -/]*[@-~]', '')
            if ($plain -notmatch '^\s*\S+\s+INFO\s+handshake_core::knowledge_documents_api:\s+MT032_DOCUMENT_RECEIPT(?:\s|$)') { return }
            $stamp = [regex]::Match($plain, '^\s*(\S+)').Groups[1].Value
            $request = [regex]::Match($plain, '\brequest_id="?([0-9a-fA-F-]{36})"?').Groups[1].Value
            $observation = [regex]::Match($plain, '\breceipt_observation_id="?([0-9]+)"?').Groups[1].Value
            $branch = [regex]::Match($plain, '\bbranch="?(create|replay)"?(?=\s|$)').Groups[1].Value
            $clock = [regex]::Match($plain, '\bclock="?(surreal_wall)"?(?=\s|$)').Groups[1].Value
            $valid = [regex]::Match($plain, '\btiming_valid="?(true|false)"?(?=\s|$)').Groups[1].Value
            $lookup = [regex]::Match($plain, '\blookup_elapsed_us="?(-?[0-9]+)"?(?=\s|$)').Groups[1].Value
            $operation = [regex]::Match($plain, '\boperation_elapsed_us="?(-?[0-9]+)"?(?=\s|$)').Groups[1].Value
        }
        if ($stamp -is [DateTime]) { $stamp = $stamp.ToUniversalTime().ToString('o', [Globalization.CultureInfo]::InvariantCulture) }
        elseif ($stamp -is [DateTimeOffset]) { $stamp = $stamp.ToUniversalTime().ToString('o', [Globalization.CultureInfo]::InvariantCulture) }
        $id = [Guid]::Empty; $timestamp = [DateTimeOffset]::MinValue; $observationId = [uint64]0
        $lookupMicros = [int64]0; $operationMicros = [int64]0; $timingValid = $false
        if (-not [Guid]::TryParseExact([string]$request, 'D', [ref]$id) -or
            -not [DateTimeOffset]::TryParse([string]$stamp, [ref]$timestamp) -or
            -not [uint64]::TryParse([string]$observation, [ref]$observationId) -or $observationId -eq 0 -or
            $branch -notin @('create', 'replay') -or $clock -ne 'surreal_wall' -or
            -not [bool]::TryParse([string]$valid, [ref]$timingValid) -or
            -not [int64]::TryParse([string]$lookup, [ref]$lookupMicros) -or
            -not [int64]::TryParse([string]$operation, [ref]$operationMicros) -or
            $timingValid -ne ($lookupMicros -ge 0 -and $operationMicros -ge 0)) {
            [void]$issues.Add('invalid_receipt_record'); return
        }
        $key = "$($FileState.Root)|$id|$observationId"
        # Each tracing layer formats its own timestamp for the same event.
        $fingerprint = "$branch|$clock|$timingValid|$lookupMicros|$operationMicros"
        if ($receiptObservations.ContainsKey($key)) {
            $previous = $receiptObservations[$key]
            if ($previous.Fingerprint -ne $fingerprint) { [void]$issues.Add('receipt_duplicate_payload_changed') }
            if (-not $previous.Sources.Add($FileState.RelativePath)) { [void]$issues.Add('receipt_duplicate_in_source') }
            return
        }
        $sources = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
        [void]$sources.Add($FileState.RelativePath)
        $receiptObservations[$key] = @{ Fingerprint = $fingerprint; Sources = $sources }
        $record = [ordered]@{
            kind = 'receipt'; observed_utc = [DateTime]::UtcNow.ToString('o')
            timestamp = $timestamp.ToUniversalTime().ToString('o'); request_id = $id.ToString('D')
            receipt_observation_id = $observationId; branch = $branch; clock = $clock
            timing_valid = $timingValid; lookup_elapsed_us = $lookupMicros; operation_elapsed_us = $operationMicros
            scenario = $FileState.Scenario; backend_pid = $roots[$FileState.Root].BackendPid
            runtime_root = [IO.Path]::GetRelativePath($runtime, $FileState.Root); source = $FileState.RelativePath
        }
        $writer.WriteLine(($record | ConvertTo-Json -Compress))
        $state.Records++; $state.ReceiptRecords++; $roots[$FileState.Root].Records++
        if (-not $timingValid) { $state.InvalidReceiptTimings++; [void]$issues.Add('invalid_receipt_timing') }
    } catch { [void]$issues.Add('receipt_parse_or_capture_error') }
}

function Accept-LookupPlanLine([string]$Line, $FileState) {
    try {
        if ($FileState.Kind -eq 'json') {
            $entry = $Line | ConvertFrom-Json -AsHashtable
            if ($entry.target -ne 'handshake_core::knowledge_documents_api' -or
                $entry.fields.message -ne 'MT032_DOCUMENT_RECEIPT_PLAN') { return }
            $stamp = $entry.timestamp; $fields = $entry.fields
        } else {
            $plain = [regex]::Replace($Line, '\x1B\[[0-?]*[ -/]*[@-~]', '')
            if ($plain -notmatch '^\s*\S+\s+INFO\s+handshake_core::knowledge_documents_api:\s+MT032_DOCUMENT_RECEIPT_PLAN(?:\s|$)') { return }
            $stamp = [regex]::Match($plain, '^\s*(\S+)').Groups[1].Value
            $fields = @{}
            foreach ($field in @('request_id', 'receipt_observation_id', 'plan_kind', 'expected_index_used',
                'supported_shape', 'entry_count', 'clock', 'explain_elapsed_us', 'timing_valid')) {
                $fields[$field] = [regex]::Match($plain, "\b$field=""?([^\s""]+)""?(?=\s|$)").Groups[1].Value
            }
        }
        if ($stamp -is [DateTime]) { $stamp = $stamp.ToUniversalTime().ToString('o', [Globalization.CultureInfo]::InvariantCulture) }
        elseif ($stamp -is [DateTimeOffset]) { $stamp = $stamp.ToUniversalTime().ToString('o', [Globalization.CultureInfo]::InvariantCulture) }
        $id = [Guid]::Empty; $timestamp = [DateTimeOffset]::MinValue; $observationId = [uint64]0
        $entryCount = [uint64]0; $micros = [int64]0
        $expectedIndex = $false; $supported = $false; $valid = $false
        if (-not [Guid]::TryParseExact([string]$fields.request_id, 'D', [ref]$id) -or
            -not [DateTimeOffset]::TryParse([string]$stamp, [ref]$timestamp) -or
            -not [uint64]::TryParse([string]$fields.receipt_observation_id, [ref]$observationId) -or $observationId -eq 0 -or
            $fields.plan_kind -notin @('index', 'table', 'none', 'other') -or $fields.clock -ne 'surreal_wall' -or
            -not [bool]::TryParse([string]$fields.expected_index_used, [ref]$expectedIndex) -or
            -not [bool]::TryParse([string]$fields.supported_shape, [ref]$supported) -or
            -not [bool]::TryParse([string]$fields.timing_valid, [ref]$valid) -or
            -not [uint64]::TryParse([string]$fields.entry_count, [ref]$entryCount) -or $entryCount -gt 16 -or
            -not [int64]::TryParse([string]$fields.explain_elapsed_us, [ref]$micros) -or
            $valid -ne ($micros -ge 0) -or
            ($expectedIndex -and (-not $supported -or $fields.plan_kind -ne 'index')) -or
            ($supported -ne ($fields.plan_kind -ne 'other'))) {
            [void]$issues.Add('invalid_lookup_plan_record'); return
        }
        $key = "$($FileState.Root)|$id|$observationId"
        $fingerprint = "$($fields.plan_kind)|$expectedIndex|$supported|$entryCount|$micros|$valid"
        if ($lookupPlans.ContainsKey($key)) {
            if ($lookupPlans[$key].Fingerprint -ne $fingerprint) { [void]$issues.Add('lookup_plan_duplicate_payload_changed') }
            if (-not $lookupPlans[$key].Sources.Add($FileState.RelativePath)) { [void]$issues.Add('lookup_plan_duplicate_in_source') }
            return
        }
        $sources = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
        [void]$sources.Add($FileState.RelativePath)
        $lookupPlans[$key] = @{ Fingerprint = $fingerprint; Sources = $sources }
        $record = [ordered]@{
            kind = 'lookup_plan'; observed_utc = [DateTime]::UtcNow.ToString('o')
            timestamp = $timestamp.ToUniversalTime().ToString('o'); request_id = $id.ToString('D')
            receipt_observation_id = $observationId; plan_kind = [string]$fields.plan_kind
            expected_index_used = $expectedIndex; supported_shape = $supported; entry_count = $entryCount
            clock = 'surreal_wall'; explain_elapsed_us = $micros; timing_valid = $valid
            scenario = $FileState.Scenario; backend_pid = $roots[$FileState.Root].BackendPid
            runtime_root = [IO.Path]::GetRelativePath($runtime, $FileState.Root); source = $FileState.RelativePath
        }
        $writer.WriteLine(($record | ConvertTo-Json -Compress))
        $state.Records++; $state.LookupPlanRecords++; $roots[$FileState.Root].Records++
        if (-not $supported) { $state.UnsupportedLookupPlans++ }
        if (-not $valid) { $state.InvalidLookupPlanTimings++; [void]$issues.Add('invalid_lookup_plan_timing') }
    } catch { [void]$issues.Add('lookup_plan_parse_or_capture_error') }
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
        $roots[$root.Path] = @{ Scenario = $root.Scenario; Records = 0; BackendPid = 0 }
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
    foreach ($rootPath in $roots.Keys) {
        if ($roots[$rootPath].BackendPid -ne 0) { continue }
        $reportPath = Join-Path $rootPath 'listen-report.json'
        if (-not [IO.File]::Exists($reportPath)) { continue }
        try {
            $reportItem = Get-Item -LiteralPath $reportPath
            if (($reportItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                [void]$issues.Add('reparse_listen_report_rejected'); continue
            }
            [void](Get-VerifiedDirectory $reportItem.DirectoryName)
            $report = Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
            if ($report.schema_id -eq 'handshake.backend-listen-report.v1' -and $report.pid -gt 0) {
                $roots[$rootPath].BackendPid = [uint64]$report.pid
            }
        } catch [System.Management.Automation.ItemNotFoundException] {
        } catch [IO.DirectoryNotFoundException] {
        } catch { [void]$issues.Add('listen_report_read_or_parse_error') }
    }
    foreach ($file in $files.Values) { Read-Available $file }
}

try {
    Write-NewJson $readyPath ([ordered]@{
        schema = $(if ($lookupPlanMode) { 'handshake.mt032.lookup-plan-watch.ready.v16.1' } elseif ($receiptMode) { 'handshake.mt032.receipt-watch.ready.v15.1' } else { 'handshake.mt032.statement-watch.ready.v14.1' }); candidate_sha = $CandidateSha
        watcher_pid = $PID; ready_utc = [DateTime]::UtcNow.ToString('o'); poll_ms = 100
        lane_root = $lane; runtime_root = $runtime; stop_signal = $stop
        capture_path = $capturePath; summary_path = $summaryPath; baseline_roots = $baseline.Count
    })
    while ($MaxSeconds -eq 0 -or $watch.Elapsed.TotalSeconds -lt $MaxSeconds) {
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
    foreach ($key in $statementQueries.Keys) {
        if (-not $begins.Contains($key)) { [void]$issues.Add('statement_query_without_begin') }
        elseif ($beginPhases[$key] -ne 'storage_query') { [void]$issues.Add('statement_parent_not_storage_query') }
        if (-not $ends.Contains($key)) { [void]$issues.Add('statement_query_without_terminal') }
        if ($statementQueries[$key].Indices.Count -ne $statementQueries[$key].Count) {
            [void]$issues.Add('statement_indexes_incomplete')
        }
    }
    if ($state.StatementRecords -eq 0) { [void]$issues.Add('no_statement_records') }
    if ($receiptMode) {
        foreach ($key in $receiptObservations.Keys) {
            if (-not $begins.Contains($key)) { [void]$issues.Add('receipt_without_begin') }
            elseif ($beginPhases[$key] -ne 'receipt_append') { [void]$issues.Add('receipt_parent_not_receipt_append') }
            if (-not $ends.Contains($key)) { [void]$issues.Add('receipt_without_terminal') }
        }
        if ($state.ReceiptRecords -eq 0) { [void]$issues.Add('no_receipt_records') }
    }
    if ($lookupPlanMode) {
        foreach ($key in $lookupPlans.Keys) {
            if (-not $receiptObservations.ContainsKey($key)) { [void]$issues.Add('lookup_plan_without_receipt') }
        }
        foreach ($key in $receiptObservations.Keys) {
            if (-not $lookupPlans.ContainsKey($key)) { [void]$issues.Add('receipt_without_lookup_plan') }
        }
        if ($state.LookupPlanRecords -eq 0) { [void]$issues.Add('no_lookup_plan_records') }
    }
    foreach ($rootState in $roots.Values) {
        if ($rootState.Records -gt 0 -and $rootState.BackendPid -eq 0) { [void]$issues.Add('backend_pid_unobserved') }
    }
    if ($state.Records -eq 0) { [void]$issues.Add('no_phase_records') }
    foreach ($name in $scenarios.Values) {
        if (@($roots.Values | Where-Object { $_.Scenario -eq $name -and $_.Records -gt 0 }).Count -eq 0) {
            [void]$issues.Add('scenario_without_phase_records')
        }
    }
    $writer.Dispose()
    $summary = [ordered]@{
        schema = $(if ($lookupPlanMode) { 'handshake.mt032.lookup-plan-watch.summary.v16.1' } elseif ($receiptMode) { 'handshake.mt032.receipt-watch.summary.v15.1' } else { 'handshake.mt032.statement-watch.summary.v14.1' }); candidate_sha = $CandidateSha
        started_utc = $started.ToString('o'); completed_utc = [DateTime]::UtcNow.ToString('o')
        watcher_pid = $PID; records = $state.Records; polls = $state.Polls
        phase_records = $state.PhaseRecords; statement_records = $state.StatementRecords
        unavailable_statement_timings = $state.UnavailableStatementTimings
        queries_with_statement_records = $statementQueries.Count
        new_roots = @($roots.Keys | Sort-Object | ForEach-Object { [IO.Path]::GetRelativePath($runtime, $_) })
        baseline_roots_excluded = $baseline.Count; stop_observed = $state.StopObserved
        incomplete_stream = ($issues.Count -gt 0); issues = @($issues | Sort-Object)
        capture_sha256 = (Get-FileHash -LiteralPath $capturePath -Algorithm SHA256).Hash
        completeness_limit = 'Polling cannot prove absence of events in a file deleted before first observation. dropped means no result observed, never engine cancellation or rollback proof. Statement execution_time is returned SDK response timing; a dropped SurrealQL query has no returned statement stats, and engine timing alone does not prove queue, permission or commit causation.'
    }
    if ($receiptMode) {
        $summary.receipt_records = $state.ReceiptRecords
        $summary.invalid_receipt_timings = $state.InvalidReceiptTimings
        $summary.completeness_limit += ' Receipt deltas use engine UTC wall time, not monotonic time; negative samples are invalid and forward clock jumps are undetected. Branch duration does not separate permissions, sequence allocation, indexes or commit.'
    }
    if ($lookupPlanMode) {
        $summary.lookup_plan_records = $state.LookupPlanRecords
        $summary.unsupported_lookup_plans = $state.UnsupportedLookupPlans
        $summary.invalid_lookup_plan_timings = $state.InvalidLookupPlanTimings
        $summary.completeness_limit += ' EXPLAIN follows the unchanged lookup under the same authenticated scope and skips iteration. Its second planning pass may be cache-warmed; duration does not measure row permissions and cannot be subtracted as an exact lookup-cost split. Added diagnostic latency is not acceptance evidence.'
    }
    Write-NewJson $summaryPath $summary
}
if ($state.Fatal -or $state.Expired) { exit 1 }
