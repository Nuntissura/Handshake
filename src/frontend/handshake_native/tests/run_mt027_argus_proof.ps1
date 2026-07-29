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
$relevantStatus = @(& git -C $repoRoot status --porcelain --untracked-files=all -- `
        '.' `
        ':(exclude)AGENTS.md' `
        ':(exclude)CLAUDE.md')
if ($LASTEXITCODE -ne 0) {
    throw 'Unable to inspect MT-027 relevant source cleanliness'
}
if (@($relevantStatus | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }).Count -ne 0) {
    throw "MT-027 proof requires every compiled/configured repository input to match committed HEAD; dirty rows: $($relevantStatus -join '; ')"
}

$artifactSibling = [IO.Path]::GetFullPath((Join-Path $crateRoot '..\..\..\..\Handshake_Artifacts'))
$cargoTarget = [IO.Path]::GetFullPath((Join-Path $artifactSibling 'handshake-cargo-target'))
$proofRoot = [IO.Path]::GetFullPath(
    (Join-Path $artifactSibling 'handshake-test\wp-kernel-012-mt-027\integrated'))
$backendBinary = [IO.Path]::GetFullPath((Join-Path $cargoTarget 'debug\handshake_core.exe'))
$postgresDsn = 'postgresql://postgres@127.0.0.1:5544/handshake'
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
$runtimeArtifactsRoot = Join-Path $runDir 'fixture-artifacts'
$argusBindingRoot = Join-Path $runDir 'argus-binding'
foreach ($containedRoot in @($stageBindingRoot, $runtimeArtifactsRoot, $argusBindingRoot)) {
    New-Item -ItemType Directory -Path $containedRoot | Out-Null
    Assert-NoReparsePointEscape -Path $containedRoot -Root $artifactSibling
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
    '-j', '1',
    '--',
    '--exact',
    '--nocapture'
)
$backendBuildArguments = @(
    'build',
    '--manifest-path', (Join-Path $repoRoot 'src\backend\handshake_core\Cargo.toml'),
    '--features', 'app-runtime,duckdb-flight-recorder',
    '--bin', 'handshake_core',
    '-j', '1'
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

$traceRows = @(Get-Content -LiteralPath $tracePath | Where-Object {
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
        $_.receipt_status -notin @('applied', 'indeterminate') -or
        [string]::IsNullOrWhiteSpace([string]$_.agent_id) -or
        -not ([string]$_.agent_id).EndsWith(':client:wp-kernel-012-mt-027-block-collections-agent') -or
        [int]$_.process_id -ne [int]$testIdentity.pid
    })
if ($invalidRows.Count -ne 0) {
    throw 'Canonical Argus trace is not bound to the committed source and observed test process'
}
$methods = @($traceRows.method | Sort-Object -Unique)
foreach ($requiredMethod in @('input.click', 'input.set_value')) {
    if ($methods -notcontains $requiredMethod) {
        throw "Canonical Argus trace is missing required method '$requiredMethod'"
    }
}
if (@($methods | Where-Object { $_ -notin @('input.click', 'input.set_value') }).Count -ne 0) {
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
            return Test-AuthorValue $Tree 'bcv.kind.table' 'selected'
        }
        'kind-kanban-selected' {
            return Test-AuthorValue $Tree 'bcv.kind.kanban' 'selected'
        }
        'kind-calendar-selected' {
            return Test-AuthorValue $Tree 'bcv.kind.calendar' 'selected'
        }
        'kind-table-restored' {
            return Test-AuthorValue $Tree 'bcv.kind.table' 'selected'
        }
        'sort-title-ascending' {
            $node = Find-AuthorNode $Tree 'bcv.table.sort.title'
            return $null -ne $node -and
                [string]$node.label -eq ('Title ' + [char]0x25B2)
        }
        'kanban-retry-loaded-card' {
            return (& $has 'bcv.kanban.lane.untagged') -and
                (Test-AuthorValues $Tree $cardId -RequiredValues @('untagged') -OnlyRequired)
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
            return Test-AuthorValue $Tree 'bcv.new-view.kind.calendar' 'selected'
        }
        'unbound-create-calendar-terminal' {
            return (Test-AuthorValue $Tree 'bcv.kind.calendar' 'selected') -and
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
            return Test-AuthorValue $Tree 'bcv.new-view.kind.kanban' 'selected'
        }
        'failed-create-retry-visible' {
            return & $has 'bcv.retry'
        }
        'retry-create-kanban-terminal' {
            return (Test-AuthorValue $Tree 'bcv.kind.kanban' 'selected') -and
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
            return $text.Contains('No Kanban lanes.')
        }
        'empty-calendar-terminal' {
            return $text.Contains('No blocks in this date range.')
        }
        default {
            throw "No external terminal-tree verifier exists for '$PredicateId'"
        }
    }
}

$movePayloadForPredicates = $traceRows[7].action_value | ConvertFrom-Json
$expectedActions = @(
    @{ method = 'input.click'; target = 'bcv.retry'; predicate = 'initial-retry-recovered-projection' },
    @{ method = 'input.click'; target = 'bcv.kind.table'; predicate = 'kind-table-selected' },
    @{ method = 'input.click'; target = 'bcv.kind.kanban'; predicate = 'kind-kanban-selected' },
    @{ method = 'input.click'; target = 'bcv.kind.calendar'; predicate = 'kind-calendar-selected' },
    @{ method = 'input.click'; target = 'bcv.kind.table'; predicate = 'kind-table-restored' },
    @{ method = 'input.click'; target = 'bcv.table.sort.title'; predicate = 'sort-title-ascending' },
    @{ method = 'input.click'; target = 'bcv.retry'; predicate = 'kanban-retry-loaded-card' },
    @{ method = 'input.click'; target = 'collection.kanban-move'; predicate = 'kanban-card-moved-target-lane'; payload = 'kanban-move' },
    @{ method = 'input.click'; target = 'bcv.retry'; predicate = 'calendar-retry-loaded-controls' },
    @{ method = 'input.set_value'; target = 'bcv.calendar.date-from'; predicate = 'calendar-from-value'; value = '2026-02-28' },
    @{ method = 'input.set_value'; target = 'bcv.calendar.date-to'; predicate = 'calendar-to-value'; value = '2026-04-30' },
    @{ method = 'input.click'; target = 'bcv.calendar.apply-range'; predicate = 'calendar-range-terminal' },
    @{ method = 'input.click'; target = 'bcv.new-view'; predicate = 'unbound-create-form-open' },
    @{ method = 'input.set_value'; target = 'bcv.new-view.title'; predicate = 'unbound-create-title-set'; value_suffix = '-host-created' },
    @{ method = 'input.click'; target = 'bcv.new-view.kind.calendar'; predicate = 'unbound-create-calendar-selected' },
    @{ method = 'input.click'; target = 'bcv.new-view.confirm'; predicate = 'unbound-create-calendar-terminal' },
    @{ method = 'input.click'; target = 'bcv.new-view'; predicate = 'retry-create-form-open' },
    @{ method = 'input.set_value'; target = 'bcv.new-view.title'; predicate = 'retry-create-title-set'; value_suffix = '-retry-created' },
    @{ method = 'input.click'; target = 'bcv.new-view.kind.kanban'; predicate = 'retry-create-kanban-selected' },
    @{ method = 'input.click'; target = 'bcv.new-view.confirm'; predicate = 'failed-create-retry-visible' },
    @{ method = 'input.click'; target = 'bcv.retry'; predicate = 'retry-create-kanban-terminal' },
    @{ method = 'input.click'; target = 'bcv.retry'; predicate = 'empty-table-terminal' },
    @{ method = 'input.click'; target = 'bcv.kind.kanban'; predicate = 'empty-kanban-terminal' },
    @{ method = 'input.click'; target = 'bcv.kind.calendar'; predicate = 'empty-calendar-terminal' }
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
if ($managedReceipt.schema_id -ne 'hsk.mt027_managed_pg_proof@1' -or
    -not [bool]$managedReceipt.backend_binding.owned -or
    [int]$managedReceipt.backend_binding.backend_pid -ne [int]$backendIdentity.pid -or
    -not ([IO.Path]::GetFullPath([string]$backendIdentity.executable)).Equals(
        $backendBinary, [StringComparison]::OrdinalIgnoreCase) -or
    -not ([IO.Path]::GetFullPath([string]$managedReceipt.backend_binding.backend_binary)).Equals(
        $backendBinary, [StringComparison]::OrdinalIgnoreCase) -or
    $managedReceipt.backend_binding.database_host -ne '127.0.0.1' -or
    [int]$managedReceipt.backend_binding.database_port -ne 5544 -or
    $managedReceipt.backend_binding.database_name -ne 'handshake') {
    throw 'Managed proof receipt is not bound to the exact owned backend and PostgreSQL endpoint'
}
$backendSha256 = (Get-FileHash -LiteralPath $backendBinary -Algorithm SHA256).Hash.ToLowerInvariant()
if ($managedReceipt.backend_binding.backend_binary_sha256 -ne $backendSha256) {
    throw 'Managed proof backend binary hash does not match the supervised current-source build'
}
$managedRuntimeData = [IO.Path]::GetFullPath([string]$managedReceipt.backend_binding.runtime_data_dir)
if (-not $managedRuntimeData.StartsWith(
        [IO.Path]::GetFullPath($runtimeArtifactsRoot) + [IO.Path]::DirectorySeparatorChar,
        [StringComparison]::OrdinalIgnoreCase) -or
    (Test-Path -LiteralPath $managedRuntimeData)) {
    throw 'Fixture-owned backend runtime was not contained and cleaned before receipt publication'
}

$postSourceSha = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $postSourceSha -ne $sourceSha) {
    throw "Repository HEAD changed during MT-027 proof: before='$sourceSha', after='$postSourceSha'"
}
$postRelevantStatus = @(& git -C $repoRoot status --porcelain --untracked-files=all -- `
        '.' `
        ':(exclude)AGENTS.md' `
        ':(exclude)CLAUDE.md')
if ($LASTEXITCODE -ne 0 -or
    @($postRelevantStatus | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }).Count -ne 0) {
    throw "MT-027 source inputs changed during proof: $($postRelevantStatus -join '; ')"
}

$receiptPgPort = [int]$managedReceipt.backend_binding.database_port
$currentPostgresListener = Get-NetTCPConnection -LocalAddress '127.0.0.1' `
    -LocalPort $receiptPgPort -State Listen -ErrorAction Stop | Select-Object -First 1
$currentPostgresIdentity = Get-Identity -TargetPid ([int]$currentPostgresListener.OwningProcess)
$initialPostgresStartUtc = Format-CanonicalUtc ([DateTimeOffset]$postgresProcess.StartTime)
if ($null -eq $currentPostgresIdentity -or
    [int]$currentPostgresIdentity.pid -ne [int]$postgresProcess.Id -or
    $currentPostgresIdentity.start_time_utc -ne $initialPostgresStartUtc -or
    -not ([IO.Path]::GetFileNameWithoutExtension(
            [string]$currentPostgresIdentity.executable)).Equals(
        'postgres', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'The exact PostgreSQL listener PID/start identity changed before receipt acceptance'
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
    if (-not $resolvedTransient.StartsWith(
            [IO.Path]::GetFullPath($runDir) + [IO.Path]::DirectorySeparatorChar,
            [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to clean transient proof path outside the exact run directory: '$resolvedTransient'"
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
    postgres_database = 'handshake'
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
    transient_roots_cleaned = @($stageBindingRoot, $runtimeArtifactsRoot, $argusBindingRoot)
    status = 'COMPLETED'
    completed_at_utc = Format-CanonicalUtc ([DateTimeOffset]::UtcNow)
}
$receipt | ConvertTo-Json -Depth 6 |
    Set-Content -LiteralPath $receiptPath -Encoding utf8NoBOM
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
            $requiredPrefix = [IO.Path]::GetFullPath($runDir) +
                [IO.Path]::DirectorySeparatorChar
            if (-not $resolvedTransient.StartsWith(
                    $requiredPrefix, [StringComparison]::OrdinalIgnoreCase)) {
                throw "path escaped exact run directory: '$resolvedTransient'"
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
