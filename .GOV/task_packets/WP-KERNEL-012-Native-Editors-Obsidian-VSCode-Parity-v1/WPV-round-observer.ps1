# WP-KERNEL-012 validator round observer (CX-VAL-007 round tooling; committed; launched detached by the validator).
#
#   Live:    pwsh -NoProfile -File WPV-round-observer.ps1 -Sha <40-hex> [-Kind union|pin-measure|backend-opt-diag|targeted]
#   Replay:  pwsh -NoProfile -File WPV-round-observer.ps1 -ReplayFile <observations.jsonl>   (dry run of the spin trigger; no dump)
#
# Every ~30 s it records: C: target bytes (protective stop of the owned wrapper tree above 147 GB, GP-102), the owned process
# tree below the round wrapper (Get-Process .Parent walk; WMI fails inside Win32_Process.Create children, GP-157) with CPU,
# working set and I/O counters, and log growth. Each poll runs in try/catch and logs its own error (GP-157).
#
# BACKEND_CPU_SPIN_STALLS_MOUNTED_VIEW_LOAD capture (orchestrator 2026-10-01; research cpu_spin_research.md Part B):
# for each owned backend (ProcessName 'handshake_core' whose parent is an owned 'test_*' process) the trigger condition is
#     backend dCPU >= 0.8 x dt   AND   test-process dCPU < 1.0 s   AND   backend dWriteTransferCount < 20 KB
# on 2 consecutive polls (about 1 core is normal for a busy backend, so backend CPU alone is not sufficient). On the first
# trigger of the round (once-marker <prefix>.spin-captured) it records per-thread CPU twice 5 s apart (no suspension) into
# <prefix>.spin-threads.json and starts procdump64 (TOOLS.json, adopted) with a PSS clone:
#     procdump64.exe -accepteula -nobanner -r -ma -n 2 -s 10 <backend-pid> <prefix>.spin-dumps\
# It samples only: it never stops the backend or the test. Dumps stay under the artifact root; the validator deletes them once
# the stack analysis is recorded (Codex :199, they may contain session tokens).
param(
    [string]$Sha = '',
    [ValidateSet('union', 'pin-measure', 'backend-opt-diag', 'targeted')][string]$Kind = 'union',
    [string]$ReplayFile = ''
)
$ErrorActionPreference = 'Stop'

$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../../..'))
$hostProfile = Get-Content -LiteralPath (Join-Path $projectRoot 'gov-runtime/host-profile-wp012.json') -Raw | ConvertFrom-Json
$Lane = $hostProfile.LANE
$Target = Join-Path $hostProfile.ARTIFACT_ROOT 'WP-KERNEL-012'
$ProcDump = Join-Path $projectRoot 'gov-runtime/tools/sysinternals/procdump/procdump64.exe'
$StopBytes = [int64]$hostProfile.WP_CAP_BYTES - 2000000000
$SpinCpuRatio = 0.8
$SpinTestCpuMaxS = 1.0
$SpinWriteMaxBytes = 20KB
$SpinConsecutive = 2
$DumpMinFreeBytes = 5GB

function New-SpinState { @{ prev = @{}; streak = @{}; fired = $false } }

# Pure decision step shared by live and replay. $procs: objects with ProcessId, ParentProcessId, Name, cpu_s, WriteTransferCount.
# Returns $null or the trigger record for the first backend that meets the condition on $SpinConsecutive consecutive polls.
function Step-SpinState($state, [datetime]$utc, $procs) {
    $byId = @{}
    foreach ($p in $procs) { $byId[[int]$p.ProcessId] = $p }
    $trigger = $null
    $seen = @{}
    foreach ($p in $procs) {
        if ($p.Name -ne 'handshake_core') { continue }
        $parent = $byId[[int]$p.ParentProcessId]
        if ($null -eq $parent -or -not ([string]$parent.Name).StartsWith('test_')) { continue }
        if ($null -eq $p.cpu_s -or $null -eq $p.WriteTransferCount -or $null -eq $parent.cpu_s) { continue }
        $id = [int]$p.ProcessId
        $seen[$id] = $true
        $cur = @{ utc = $utc; cpu = [double]$p.cpu_s; w = [double]$p.WriteTransferCount; tcpu = [double]$parent.cpu_s; tpid = [int]$parent.ProcessId }
        $prev = $state.prev[$id]
        $state.prev[$id] = $cur
        if ($null -eq $prev -or $prev.tpid -ne $cur.tpid) { $state.streak[$id] = 0; continue }
        $dt = ($cur.utc - $prev.utc).TotalSeconds
        if ($dt -le 0) { continue }
        $dcpu = $cur.cpu - $prev.cpu; $dtest = $cur.tcpu - $prev.tcpu; $dw = $cur.w - $prev.w
        $hit = ($dcpu -ge $SpinCpuRatio * $dt) -and ($dtest -lt $SpinTestCpuMaxS) -and ($dw -lt $SpinWriteMaxBytes)
        $state.streak[$id] = if ($hit) { [int]$state.streak[$id] + 1 } else { 0 }
        if ($hit -and $state.streak[$id] -ge $SpinConsecutive -and -not $state.fired -and $null -eq $trigger) {
            $trigger = [ordered]@{ backend_pid = $id; test_pid = $cur.tpid; test_name = [string]$parent.Name; utc = $utc.ToString('o');
                dt_s = [math]::Round($dt, 1); backend_dcpu_s = [math]::Round($dcpu, 1); test_dcpu_s = [math]::Round($dtest, 2); backend_dwrite_bytes = $dw;
                consecutive = $state.streak[$id] }
        }
    }
    foreach ($k in @($state.prev.Keys)) { if (-not $seen[$k]) { $state.prev.Remove($k); $state.streak.Remove($k) } }
    if ($trigger) { $state.fired = $true }
    return $trigger
}

if ($ReplayFile) {
    # Dry run: feed a recorded observations.jsonl through the same decision step; no process is touched, no dump is taken.
    $state = New-SpinState
    $polls = 0; $fires = @()
    foreach ($line in [System.IO.File]::ReadLines($ReplayFile)) {
        if (-not $line.Trim()) { continue }
        $o = $line | ConvertFrom-Json
        if (-not $o.processes) { continue }
        $polls++
        # ConvertFrom-Json (PS7) already yields a DateTime for the ISO 'utc' field; re-parsing it would shift it by the local offset.
        $ts = if ($o.utc -is [datetime]) { $o.utc.ToUniversalTime() } else { [datetime]::Parse([string]$o.utc, $null, [System.Globalization.DateTimeStyles]::RoundtripKind).ToUniversalTime() }
        $t = Step-SpinState $state $ts @($o.processes)
        if ($t) { $fires += $t }
    }
    [ordered]@{ replay = $ReplayFile; polls_with_processes = $polls; would_fire = ($fires.Count -gt 0); triggers = $fires } | ConvertTo-Json -Depth 6
    exit 0
}

if ($Sha -notmatch '^[0-9a-f]{40}$') { throw 'full 40-hex -Sha required' }
Add-Type -TypeDefinition @"
using System; using System.Runtime.InteropServices;
public static class WpvIoC {
  [StructLayout(LayoutKind.Sequential)] public struct IO_COUNTERS { public ulong ReadOperationCount, WriteOperationCount, OtherOperationCount, ReadTransferCount, WriteTransferCount, OtherTransferCount; }
  [DllImport("kernel32.dll", SetLastError=true)] public static extern bool GetProcessIoCounters(IntPtr h, out IO_COUNTERS c);
}
"@
$prefix = Join-Path $Lane "logs/$Kind-$Sha"
$obsFile = "$prefix.observations.jsonl"
$self = Get-Process -Id $PID
[ordered]@{ pid = $PID; start_utc = $self.StartTime.ToUniversalTime().ToString('o'); candidate = $Sha; kind = $Kind; owner = 'wp_validator'
    purpose = 'capacity, owned process observations, BACKEND_CPU_SPIN capture'; script = $PSCommandPath; stop_bytes = $StopBytes; ready = $true
    spin_trigger = "backend dCPU >= $SpinCpuRatio*dt AND test dCPU < $SpinTestCpuMaxS s AND backend dWrite < $SpinWriteMaxBytes B on $SpinConsecutive consecutive polls; once" } |
    ConvertTo-Json | Set-Content -LiteralPath "$prefix.observer.identity.json" -Encoding utf8NoBOM

function Write-Obs($obj) { ($obj | ConvertTo-Json -Depth 8 -Compress) | Add-Content -LiteralPath $obsFile -Encoding utf8NoBOM }

function Get-ThreadCpu([int]$procId) {
    $p = Get-Process -Id $procId -ErrorAction Stop
    $m = @{}
    foreach ($t in $p.Threads) {
        $ms = $null; try { $ms = $t.TotalProcessorTime.TotalMilliseconds } catch {}
        $m[[int]$t.Id] = [ordered]@{ tid = [int]$t.Id; cpu_ms = $ms; state = [string]$t.ThreadState; wait = $(try { [string]$t.WaitReason } catch { '' })
            start_address = $(try { '0x{0:x}' -f [int64]$t.StartAddress } catch { '' }) }
    }
    return $m
}

function Invoke-SpinCapture($trigger) {
    $marker = "$prefix.spin-captured"
    if (Test-Path -LiteralPath $marker) { return }
    [ordered]@{ utc = [DateTime]::UtcNow.ToString('o'); trigger = $trigger } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $marker -Encoding utf8NoBOM
    $bpid = [int]$trigger.backend_pid
    $threads = $null
    try {
        $a = Get-ThreadCpu $bpid; Start-Sleep -Seconds 5; $b = Get-ThreadCpu $bpid
        $rows = foreach ($k in $b.Keys) {
            $r = $b[$k]; $d = if ($a.ContainsKey($k) -and $null -ne $a[$k].cpu_ms -and $null -ne $r.cpu_ms) { $r.cpu_ms - $a[$k].cpu_ms } else { $null }
            [ordered]@{ tid = $r.tid; dcpu_ms_5s = $d; cpu_ms = $r.cpu_ms; state = $r.state; wait = $r.wait; start_address = $r.start_address }
        }
        $threads = @($rows | Sort-Object -Property { if ($null -eq $_.dcpu_ms_5s) { -1 } else { $_.dcpu_ms_5s } } -Descending)
        [ordered]@{ utc = [DateTime]::UtcNow.ToString('o'); backend_pid = $bpid; interval_s = 5; thread_count = $threads.Count
            hot_tid = $(if ($threads.Count) { $threads[0].tid } else { $null }); top = @($threads | Select-Object -First 5); all = $threads } |
            ConvertTo-Json -Depth 6 | Set-Content -LiteralPath "$prefix.spin-threads.json" -Encoding utf8NoBOM
    } catch {
        [ordered]@{ utc = [DateTime]::UtcNow.ToString('o'); backend_pid = $bpid; error = $_.Exception.Message } | ConvertTo-Json | Set-Content -LiteralPath "$prefix.spin-threads.json" -Encoding utf8NoBOM
    }
    $dumpDir = "$prefix.spin-dumps"
    $free = (Get-PSDrive -Name (Split-Path -Qualifier $Lane).TrimEnd(':')).Free
    $dump = [ordered]@{ utc = [DateTime]::UtcNow.ToString('o'); backend_pid = $bpid; dump_dir = $dumpDir; artifact_drive_free_bytes = $free }
    if ($free -lt $DumpMinFreeBytes) {
        $dump.skipped = "artifact drive free $free < $DumpMinFreeBytes"
    } elseif (-not (Test-Path -LiteralPath $ProcDump)) {
        $dump.skipped = "procdump64 missing at $ProcDump"
    } else {
        New-Item -ItemType Directory -Force -Path $dumpDir | Out-Null
        $argLine = "-accepteula -nobanner -r -ma -n 2 -s 10 $bpid `"$dumpDir`""
        $pd = Start-Process -FilePath $ProcDump -ArgumentList $argLine -WindowStyle Hidden -PassThru `
            -RedirectStandardOutput "$prefix.spin-procdump.stdout.log" -RedirectStandardError "$prefix.spin-procdump.stderr.log"
        $dump.procdump_pid = $pd.Id; $dump.command = "procdump64.exe $argLine"
        $script:procdumpProc = $pd
    }
    $dump | ConvertTo-Json | Set-Content -LiteralPath "$prefix.spin-dumps.json" -Encoding utf8NoBOM
    Write-Obs ([ordered]@{ utc = [DateTime]::UtcNow.ToString('o'); candidate = $Sha; spin_signature = $true; trigger = $trigger; threads_file = "$prefix.spin-threads.json"; dumps = $dump })
}

function Complete-SpinDumps {
    if (-not $script:procdumpProc -or $script:procdumpDone) { return }
    $script:procdumpProc.Refresh()
    if (-not $script:procdumpProc.HasExited) { return }
    $script:procdumpDone = $true
    $files = @(Get-ChildItem -LiteralPath "$prefix.spin-dumps" -File -Filter '*.dmp' -ErrorAction SilentlyContinue | ForEach-Object {
            [ordered]@{ path = $_.FullName; bytes = $_.Length; sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() } })
    $rec = Get-Content -LiteralPath "$prefix.spin-dumps.json" -Raw | ConvertFrom-Json -AsHashtable
    $rec.procdump_exit_code = $script:procdumpProc.ExitCode; $rec.completed_utc = [DateTime]::UtcNow.ToString('o'); $rec.dumps = $files
    $rec | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath "$prefix.spin-dumps.json" -Encoding utf8NoBOM
    Write-Obs ([ordered]@{ utc = [DateTime]::UtcNow.ToString('o'); candidate = $Sha; spin_dumps_complete = $true; procdump_exit_code = $script:procdumpProc.ExitCode; dumps = $files })
}

$state = New-SpinState
if (Test-Path -LiteralPath "$prefix.spin-captured") { $state.fired = $true }
$script:procdumpProc = $null; $script:procdumpDone = $false
$poll = 0
$commitBaselineBytes = $null
$commitMaxSampledBytes = $null
$freeCommitMinSampledBytes = $null
while ($true) {
    $poll++
    $done = Test-Path -LiteralPath "$prefix.exit.json"
    try {
        $identity = $null
        if (Test-Path -LiteralPath "$prefix.identity.json") {
            try { $identity = Get-Content -LiteralPath "$prefix.identity.json" -Raw | ConvertFrom-Json } catch { $identity = $null }
        }
        $all = @(Get-Process | ForEach-Object { [pscustomobject]@{ ProcessId = $_.Id; ParentProcessId = $(try { $_.Parent.Id } catch { $null }); Name = $_.ProcessName; P = $_ } })
        $owned = @()
        if ($identity -and $identity.wrapper_pid) {
            $owned = @([int]$identity.wrapper_pid)
            do {
                $added = @($all | Where-Object { $_.ParentProcessId -ne $null -and $owned -contains [int]$_.ParentProcessId -and $owned -notcontains [int]$_.ProcessId } | ForEach-Object { [int]$_.ProcessId })
                $owned += $added
            } while ($added.Count -gt 0)
        }
        $now = [DateTime]::UtcNow
        $processes = @($all | Where-Object { $owned -contains [int]$_.ProcessId } | ForEach-Object {
                $io = New-Object WpvIoC+IO_COUNTERS; $ok = $false; try { $ok = [WpvIoC]::GetProcessIoCounters($_.P.Handle, [ref]$io) } catch {}
                [pscustomobject][ordered]@{ ProcessId = $_.ProcessId; ParentProcessId = $_.ParentProcessId; Name = $_.Name
                    cpu_s = $(try { [math]::Round($_.P.TotalProcessorTime.TotalSeconds, 1) } catch { $null }); ws_mb = $(try { [math]::Round($_.P.WorkingSet64 / 1MB) } catch { $null })
                    ReadTransferCount = $(if ($ok) { $io.ReadTransferCount } else { $null }); WriteTransferCount = $(if ($ok) { $io.WriteTransferCount } else { $null }) } })
        $gciErr = $null
        $bytes = [int64](Get-ChildItem -LiteralPath $Target -File -Recurse -Force -ErrorAction SilentlyContinue -ErrorVariable gciErr | Measure-Object -Property Length -Sum).Sum
        $logs = @('stdout', 'stderr', 'tests' | ForEach-Object { $file = "$prefix.$_.log"; if (Test-Path -LiteralPath $file) { $i = Get-Item -LiteralPath $file; [ordered]@{ path = $file; bytes = $i.Length; mtime_utc = $i.LastWriteTimeUtc.ToString('o') } } })
        $exceeded = ($bytes -gt $StopBytes)
        $commitSampleAvailable = $false
        $commitSampleErrorType = $null
        $commitLimitBytes = $null
        $committedBytes = $null
        $freeCommitBytes = $null
        try {
            $memory = Get-CimInstance -ClassName Win32_PerfFormattedData_PerfOS_Memory -ErrorAction Stop
            if ($null -eq $memory) { throw [System.InvalidOperationException]::new('commit telemetry returned no row') }
            if ($null -eq $memory.CommitLimit -or $null -eq $memory.CommittedBytes) {
                throw [System.InvalidOperationException]::new('commit telemetry returned a missing counter')
            }
            $limit = [int64]$memory.CommitLimit
            $committed = [int64]$memory.CommittedBytes
            if ($limit -le 0 -or $committed -lt 0 -or $committed -gt $limit) {
                throw [System.InvalidOperationException]::new('commit telemetry returned invalid counters')
            }
            $commitLimitBytes = $limit
            $committedBytes = $committed
            $freeCommitBytes = $limit - $committed
            $commitSampleAvailable = $true
            if ($null -eq $commitBaselineBytes) { $commitBaselineBytes = $committed }
            if ($null -eq $commitMaxSampledBytes -or $committed -gt $commitMaxSampledBytes) { $commitMaxSampledBytes = $committed }
            if ($null -eq $freeCommitMinSampledBytes -or $freeCommitBytes -lt $freeCommitMinSampledBytes) { $freeCommitMinSampledBytes = $freeCommitBytes }
        } catch {
            $commitSampleErrorType = $_.Exception.GetType().FullName
        }
        $commitSampleUtc = [DateTime]::UtcNow.ToString('o')
        Write-Obs ([ordered]@{ utc = $now.ToString('o'); poll = $poll; candidate = $Sha; C_bytes = $bytes; gci_errors = @($gciErr).Count; stop_bytes = $StopBytes; cap_bytes = [int64]$hostProfile.WP_CAP_BYTES
                headroom_to_stop = $StopBytes - $bytes; stop_exceeded = $exceeded; processes = $processes; logs = $logs; exit_record_present = $done
                commit_sample_utc = $commitSampleUtc; commit_sample_available = $commitSampleAvailable; commit_sample_error_type = $commitSampleErrorType
                commit_limit_bytes = $commitLimitBytes; committed_bytes = $committedBytes; free_commit_bytes = $freeCommitBytes
                baseline_committed_bytes = $commitBaselineBytes; max_sampled_committed_bytes = $commitMaxSampledBytes
                min_sampled_free_commit_bytes = $freeCommitMinSampledBytes })
        if ($exceeded -and $owned.Count -gt 0 -and -not $done) {
            foreach ($p in ($owned | Select-Object -Skip 1 | Sort-Object -Descending)) { try { Stop-Process -Id $p -Force -ErrorAction Stop } catch {} }
            [ordered]@{ utc = [DateTime]::UtcNow.ToString('o'); C_bytes = $bytes; stopped = $owned } | ConvertTo-Json -Compress | Set-Content -LiteralPath "$prefix.protective-stop.json" -Encoding utf8NoBOM
        }
        if (-not $done) {
            $trigger = Step-SpinState $state $now $processes
            if ($trigger) { Invoke-SpinCapture $trigger }
        }
        Complete-SpinDumps
    } catch {
        try { Write-Obs ([ordered]@{ utc = [DateTime]::UtcNow.ToString('o'); poll = $poll; candidate = $Sha; observer_error = $_.Exception.Message; line = $_.InvocationInfo.ScriptLineNumber }) } catch {}
    }
    if ($done -and (-not $script:procdumpProc -or $script:procdumpDone)) { break }
    Start-Sleep -Seconds 30
}
Write-Output 'WPV_ROUND_OBSERVER completed'
