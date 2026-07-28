[CmdletBinding()]
param(
    [string]$RunId = ("MT045-RUN-" + [guid]::NewGuid().ToString("N")),
    [ValidateRange(60, 1800)]
    [int]$CommandTimeoutSeconds = 1800,
    [string]$PostgresDsn = "postgresql://postgres@127.0.0.1:5544/handshake"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$mt045JobRunnerExpectedSourceId = "mt045-job-runner-20260729-v4"
$mt045JobRunnerSource = @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

public sealed class Mt045JobRunResult
{
    public int RootProcessId { get; set; }
    public int ExitCode { get; set; }
    public bool TimedOut { get; set; }
    public uint LeakedProcessCount { get; set; }
}

public static class Mt045JobRunner
{
    public const string SourceId = "mt045-job-runner-20260729-v4";
    private const uint CREATE_SUSPENDED = 0x00000004;
    private const uint CREATE_NO_WINDOW = 0x08000000;
    private const uint STARTF_USESTDHANDLES = 0x00000100;
    private const uint GENERIC_WRITE = 0x40000000;
    private const uint FILE_SHARE_READ = 0x00000001;
    private const uint FILE_SHARE_WRITE = 0x00000002;
    private const uint CREATE_ALWAYS = 2;
    private const uint FILE_ATTRIBUTE_NORMAL = 0x00000080;
    private const uint JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000;
    private const uint WAIT_OBJECT_0 = 0x00000000;
    private const uint WAIT_TIMEOUT = 0x00000102;
    private const uint INFINITE = 0xffffffff;
    private const int STD_INPUT_HANDLE = -10;
    private const int JobObjectBasicAccountingInformation = 1;
    private const int JobObjectExtendedLimitInformation = 9;

    [StructLayout(LayoutKind.Sequential)]
    private struct SECURITY_ATTRIBUTES
    {
        public int nLength;
        public IntPtr lpSecurityDescriptor;
        [MarshalAs(UnmanagedType.Bool)]
        public bool bInheritHandle;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct STARTUPINFO
    {
        public int cb;
        public string lpReserved;
        public string lpDesktop;
        public string lpTitle;
        public int dwX;
        public int dwY;
        public int dwXSize;
        public int dwYSize;
        public int dwXCountChars;
        public int dwYCountChars;
        public int dwFillAttribute;
        public uint dwFlags;
        public short wShowWindow;
        public short cbReserved2;
        public IntPtr lpReserved2;
        public IntPtr hStdInput;
        public IntPtr hStdOutput;
        public IntPtr hStdError;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct PROCESS_INFORMATION
    {
        public IntPtr hProcess;
        public IntPtr hThread;
        public int dwProcessId;
        public int dwThreadId;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct IO_COUNTERS
    {
        public ulong ReadOperationCount;
        public ulong WriteOperationCount;
        public ulong OtherOperationCount;
        public ulong ReadTransferCount;
        public ulong WriteTransferCount;
        public ulong OtherTransferCount;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct JOBOBJECT_BASIC_LIMIT_INFORMATION
    {
        public long PerProcessUserTimeLimit;
        public long PerJobUserTimeLimit;
        public uint LimitFlags;
        public UIntPtr MinimumWorkingSetSize;
        public UIntPtr MaximumWorkingSetSize;
        public uint ActiveProcessLimit;
        public UIntPtr Affinity;
        public uint PriorityClass;
        public uint SchedulingClass;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION
    {
        public JOBOBJECT_BASIC_LIMIT_INFORMATION BasicLimitInformation;
        public IO_COUNTERS IoInfo;
        public UIntPtr ProcessMemoryLimit;
        public UIntPtr JobMemoryLimit;
        public UIntPtr PeakProcessMemoryUsed;
        public UIntPtr PeakJobMemoryUsed;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct JOBOBJECT_BASIC_ACCOUNTING_INFORMATION
    {
        public long TotalUserTime;
        public long TotalKernelTime;
        public long ThisPeriodTotalUserTime;
        public long ThisPeriodTotalKernelTime;
        public uint TotalPageFaultCount;
        public uint TotalProcesses;
        public uint ActiveProcesses;
        public uint TotalTerminatedProcesses;
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateJobObject(IntPtr jobAttributes, string name);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool SetInformationJobObject(
        IntPtr job,
        int informationClass,
        IntPtr information,
        uint informationLength);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool QueryInformationJobObject(
        IntPtr job,
        int informationClass,
        out JOBOBJECT_BASIC_ACCOUNTING_INFORMATION information,
        uint informationLength,
        IntPtr returnLength);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool TerminateJobObject(IntPtr job, uint exitCode);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool TerminateProcess(IntPtr process, uint exitCode);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool CreateProcess(
        string applicationName,
        StringBuilder commandLine,
        IntPtr processAttributes,
        IntPtr threadAttributes,
        bool inheritHandles,
        uint creationFlags,
        IntPtr environment,
        string currentDirectory,
        ref STARTUPINFO startupInfo,
        out PROCESS_INFORMATION processInformation);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateFile(
        string fileName,
        uint desiredAccess,
        uint shareMode,
        ref SECURITY_ATTRIBUTES securityAttributes,
        uint creationDisposition,
        uint flagsAndAttributes,
        IntPtr templateFile);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern uint ResumeThread(IntPtr thread);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetExitCodeProcess(IntPtr process, out uint exitCode);

    [DllImport("kernel32.dll")]
    private static extern IntPtr GetStdHandle(int standardHandle);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool CloseHandle(IntPtr handle);

    private static string QuoteArgument(string value)
    {
        if (value.Length == 0)
        {
            return "\"\"";
        }
        if (value.IndexOfAny(new[] { ' ', '\t', '\n', '\v', '"' }) < 0)
        {
            return value;
        }
        var output = new StringBuilder();
        output.Append('"');
        var backslashes = 0;
        foreach (var character in value)
        {
            if (character == '\\')
            {
                backslashes++;
                continue;
            }
            if (character == '"')
            {
                output.Append('\\', backslashes * 2 + 1);
                output.Append('"');
                backslashes = 0;
                continue;
            }
            output.Append('\\', backslashes);
            backslashes = 0;
            output.Append(character);
        }
        output.Append('\\', backslashes * 2);
        output.Append('"');
        return output.ToString();
    }

    private static StringBuilder BuildCommandLine(string executable, string[] arguments)
    {
        var commandLine = new StringBuilder(QuoteArgument(executable));
        foreach (var argument in arguments)
        {
            commandLine.Append(' ');
            commandLine.Append(QuoteArgument(argument));
        }
        return commandLine;
    }

    private static void ThrowLastWin32(string operation)
    {
        throw new Win32Exception(Marshal.GetLastWin32Error(), operation);
    }

    private static uint QueryActiveProcessCount(IntPtr job)
    {
        JOBOBJECT_BASIC_ACCOUNTING_INFORMATION accounting;
        if (!QueryInformationJobObject(
            job,
            JobObjectBasicAccountingInformation,
            out accounting,
            (uint)Marshal.SizeOf(typeof(JOBOBJECT_BASIC_ACCOUNTING_INFORMATION)),
            IntPtr.Zero))
        {
            ThrowLastWin32("QueryInformationJobObject failed");
        }
        return accounting.ActiveProcesses;
    }

    private static void WaitForJobDrain(IntPtr job, int timeoutMilliseconds, string operation)
    {
        var deadline = Environment.TickCount64 + timeoutMilliseconds;
        while (QueryActiveProcessCount(job) != 0)
        {
            if (Environment.TickCount64 >= deadline)
            {
                throw new TimeoutException(operation + " did not drain the Windows Job Object");
            }
            Thread.Sleep(100);
        }
    }

    private static void TerminateJobAndDrain(IntPtr job, string operation)
    {
        if (!TerminateJobObject(job, 0x4d543435))
        {
            ThrowLastWin32(operation + " could not terminate the Windows Job Object");
        }
        WaitForJobDrain(job, 15000, operation);
    }

    public static Mt045JobRunResult Run(
        string executable,
        string[] arguments,
        string workingDirectory,
        string stdoutPath,
        string stderrPath,
        int timeoutMilliseconds,
        int descendantExitGraceMilliseconds)
    {
        IntPtr job = IntPtr.Zero;
        IntPtr stdoutHandle = IntPtr.Zero;
        IntPtr stderrHandle = IntPtr.Zero;
        var processInformation = new PROCESS_INFORMATION();
        var processCreated = false;
        var processAssignedToJob = false;
        var runCompleted = false;
        Exception primaryFailure = null;
        try
        {
            job = CreateJobObject(IntPtr.Zero, null);
            if (job == IntPtr.Zero)
            {
                ThrowLastWin32("CreateJobObject failed");
            }

            var limits = new JOBOBJECT_EXTENDED_LIMIT_INFORMATION();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            var limitsSize = Marshal.SizeOf(typeof(JOBOBJECT_EXTENDED_LIMIT_INFORMATION));
            var limitsPointer = Marshal.AllocHGlobal(limitsSize);
            try
            {
                Marshal.StructureToPtr(limits, limitsPointer, false);
                if (!SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    limitsPointer,
                    (uint)limitsSize))
                {
                    ThrowLastWin32("SetInformationJobObject failed");
                }
            }
            finally
            {
                Marshal.FreeHGlobal(limitsPointer);
            }

            var security = new SECURITY_ATTRIBUTES
            {
                nLength = Marshal.SizeOf(typeof(SECURITY_ATTRIBUTES)),
                bInheritHandle = true
            };
            stdoutHandle = CreateFile(
                stdoutPath,
                GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ref security,
                CREATE_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                IntPtr.Zero);
            if (stdoutHandle == new IntPtr(-1))
            {
                stdoutHandle = IntPtr.Zero;
                ThrowLastWin32("Opening stdout log failed");
            }
            stderrHandle = CreateFile(
                stderrPath,
                GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ref security,
                CREATE_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                IntPtr.Zero);
            if (stderrHandle == new IntPtr(-1))
            {
                stderrHandle = IntPtr.Zero;
                ThrowLastWin32("Opening stderr log failed");
            }

            var startupInfo = new STARTUPINFO
            {
                cb = Marshal.SizeOf(typeof(STARTUPINFO)),
                dwFlags = STARTF_USESTDHANDLES,
                hStdInput = GetStdHandle(STD_INPUT_HANDLE),
                hStdOutput = stdoutHandle,
                hStdError = stderrHandle
            };
            var commandLine = BuildCommandLine(executable, arguments);
            if (!CreateProcess(
                executable,
                commandLine,
                IntPtr.Zero,
                IntPtr.Zero,
                true,
                CREATE_SUSPENDED | CREATE_NO_WINDOW,
                IntPtr.Zero,
                workingDirectory,
                ref startupInfo,
                out processInformation))
            {
                ThrowLastWin32("CreateProcess failed");
            }
            processCreated = true;

            if (!AssignProcessToJobObject(job, processInformation.hProcess))
            {
                var assignmentError = new Win32Exception(
                    Marshal.GetLastWin32Error(),
                    "AssignProcessToJobObject failed");
                if (!TerminateProcess(processInformation.hProcess, 0x4d543435))
                {
                    var cleanupError = new Win32Exception(
                        Marshal.GetLastWin32Error(),
                        "TerminateProcess after Job Object assignment failure failed");
                    throw new AggregateException(assignmentError, cleanupError);
                }
                if (WaitForSingleObject(processInformation.hProcess, 15000) != WAIT_OBJECT_0)
                {
                    throw new AggregateException(
                        assignmentError,
                        new TimeoutException("Suspended root did not exit after Job Object assignment failure"));
                }
                runCompleted = true;
                throw assignmentError;
            }
            processAssignedToJob = true;
            if (ResumeThread(processInformation.hThread) == uint.MaxValue)
            {
                var resumeError = new Win32Exception(Marshal.GetLastWin32Error(), "ResumeThread failed");
                TerminateJobAndDrain(job, "ResumeThread failure cleanup");
                runCompleted = true;
                throw resumeError;
            }

            var wait = WaitForSingleObject(processInformation.hProcess, (uint)timeoutMilliseconds);
            var timedOut = wait == WAIT_TIMEOUT;
            if (timedOut)
            {
                TerminateJobAndDrain(job, "Timeout cleanup");
                WaitForSingleObject(processInformation.hProcess, INFINITE);
            }
            else if (wait != WAIT_OBJECT_0)
            {
                TerminateJobAndDrain(job, "Wait failure cleanup");
                ThrowLastWin32("WaitForSingleObject failed");
            }

            uint exitCode;
            if (!GetExitCodeProcess(processInformation.hProcess, out exitCode))
            {
                TerminateJobAndDrain(job, "Exit-code failure cleanup");
                ThrowLastWin32("GetExitCodeProcess failed");
            }

            uint activeProcesses = 0;
            if (!timedOut)
            {
                var descendantDeadline = Environment.TickCount64 + descendantExitGraceMilliseconds;
                while (true)
                {
                    activeProcesses = QueryActiveProcessCount(job);
                    if (activeProcesses == 0 || Environment.TickCount64 >= descendantDeadline)
                    {
                        break;
                    }
                    Thread.Sleep(100);
                }
                if (activeProcesses != 0)
                {
                    TerminateJobAndDrain(job, "Descendant-leak cleanup");
                }
            }

            var result = new Mt045JobRunResult
            {
                RootProcessId = processInformation.dwProcessId,
                ExitCode = unchecked((int)exitCode),
                TimedOut = timedOut,
                LeakedProcessCount = activeProcesses
            };
            runCompleted = true;
            return result;
        }
        catch (Exception failure)
        {
            primaryFailure = failure;
            throw;
        }
        finally
        {
            Exception cleanupFailure = null;
            try
            {
                if (processCreated && !runCompleted)
                {
                    if (processAssignedToJob)
                    {
                        TerminateJobAndDrain(job, "Exceptional cleanup");
                    }
                    else
                    {
                        if (!TerminateProcess(processInformation.hProcess, 0x4d543435))
                        {
                            ThrowLastWin32("Exceptional suspended-root cleanup failed");
                        }
                        if (WaitForSingleObject(processInformation.hProcess, 15000) != WAIT_OBJECT_0)
                        {
                            throw new TimeoutException(
                                "Exceptional suspended-root cleanup did not confirm exit");
                        }
                    }
                }
            }
            catch (Exception failure)
            {
                cleanupFailure = failure;
            }
            finally
            {
                if (processCreated)
                {
                    if (processInformation.hThread != IntPtr.Zero)
                    {
                        CloseHandle(processInformation.hThread);
                    }
                    if (processInformation.hProcess != IntPtr.Zero)
                    {
                        CloseHandle(processInformation.hProcess);
                    }
                }
                if (stderrHandle != IntPtr.Zero)
                {
                    CloseHandle(stderrHandle);
                }
                if (stdoutHandle != IntPtr.Zero)
                {
                    CloseHandle(stdoutHandle);
                }
                if (job != IntPtr.Zero)
                {
                    CloseHandle(job);
                }
            }
            if (cleanupFailure != null)
            {
                if (primaryFailure != null)
                {
                    throw new AggregateException(primaryFailure, cleanupFailure);
                }
                throw cleanupFailure;
            }
        }
    }
}
'@

function Invoke-GitText {
    param(
        [Parameter(Mandatory)][string]$Repository,
        [Parameter(Mandatory)][string[]]$Arguments
    )
    $text = & git -C $Repository @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed: $text"
    }
    return (($text | Out-String).Trim())
}

function Assert-SourceBindingClean {
    param(
        [Parameter(Mandatory)][string]$Repository,
        [Parameter(Mandatory)][string[]]$Paths
    )
    $status = Invoke-GitText -Repository $Repository -Arguments (@("status", "--porcelain=v1", "--") + $Paths)
    if (-not [string]::IsNullOrWhiteSpace($status)) {
        throw "Canonical MT-045 source-binding paths must match committed HEAD:`n$status"
    }
}

function Write-JsonAtomic {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)]$Value
    )
    $parent = Split-Path $Path -Parent
    [void][IO.Directory]::CreateDirectory($parent)
    $temporary = Join-Path $parent (".$(Split-Path $Path -Leaf).$([guid]::NewGuid().ToString('N')).tmp")
    $json = ($Value | ConvertTo-Json -Depth 30) + [Environment]::NewLine
    [IO.File]::WriteAllText($temporary, $json, [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $Path -Force
}

function Write-ImmutableJson {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)]$Value
    )
    $json = ($Value | ConvertTo-Json -Depth 30) + [Environment]::NewLine
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
    if (Test-Path -LiteralPath $digestPath) {
        throw "Immutable digest path already exists: $digestPath"
    }
    [IO.File]::WriteAllText(
        $digestPath,
        "$digest  $(Split-Path $Path -Leaf)$([Environment]::NewLine)",
        $encoding
    )
    (Get-Item -LiteralPath $Path).IsReadOnly = $true
    (Get-Item -LiteralPath $digestPath).IsReadOnly = $true
    return $digest
}

function Invoke-BoundedCargo {
    param(
        [Parameter(Mandatory)][string]$Label,
        [Parameter(Mandatory)][string[]]$Arguments,
        [Parameter(Mandatory)][string]$WorkingDirectory,
        [Parameter(Mandatory)][string]$LogRoot,
        [int]$TimeoutSeconds = $CommandTimeoutSeconds
    )
    $safeLabel = $Label -replace "[^A-Za-z0-9_.-]", "-"
    $stdoutPath = Join-Path $LogRoot "$safeLabel.stdout.log"
    $stderrPath = Join-Path $LogRoot "$safeLabel.stderr.log"
    $startedAt = [DateTimeOffset]::UtcNow
    $cargoPath = (Get-Command cargo -CommandType Application -ErrorAction Stop).Source
    $nativeResult = [Mt045JobRunner]::Run(
        $cargoPath,
        [string[]]$Arguments,
        $WorkingDirectory,
        $stdoutPath,
        $stderrPath,
        ($TimeoutSeconds * 1000),
        15000
    )
    if ($nativeResult.TimedOut) {
        throw "$Label exceeded its hard ${TimeoutSeconds}s deadline; its Windows Job Object was terminated"
    }
    if ($nativeResult.LeakedProcessCount -ne 0) {
        throw "$Label leaked $($nativeResult.LeakedProcessCount) owned descendant process(es); its Windows Job Object was terminated"
    }
    if ($nativeResult.ExitCode -ne 0) {
        throw "$Label failed with exit code $($nativeResult.ExitCode)"
    }
    return [ordered]@{
        label = $Label
        command = "cargo " + ($Arguments -join " ")
        started_at = $startedAt.ToString("O")
        completed_at = [DateTimeOffset]::UtcNow.ToString("O")
        timeout_seconds = $TimeoutSeconds
        exit_code = $nativeResult.ExitCode
        root_process_id = $nativeResult.RootProcessId
        process_containment = "windows_job_object_kill_on_close"
        stdout = $stdoutPath
        stdout_sha256 = (Get-FileHash -LiteralPath $stdoutPath -Algorithm SHA256).Hash.ToLowerInvariant()
        stderr = $stderrPath
        stderr_sha256 = (Get-FileHash -LiteralPath $stderrPath -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

function Assert-ExactDiagnosticResult {
    param(
        [Parameter(Mandatory)]$CommandResult,
        [Parameter(Mandatory)][string]$ExpectedTest
    )
    $stdout = [IO.File]::ReadAllText($CommandResult.stdout)
    $runningMatches = [regex]::Matches($stdout, "(?m)^running 1 test\r?$")
    $summaryMatches = [regex]::Matches(
        $stdout,
        "(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; [0-9]+ filtered out; finished in [^\r\n]+\r?$"
    )
    if ($runningMatches.Count -ne 1 -or $summaryMatches.Count -ne 1) {
        throw "Diagnostic $ExpectedTest did not execute exactly one passing test; inspect $($CommandResult.stdout)"
    }
    $CommandResult["expected_test"] = $ExpectedTest
    $CommandResult["verified_executed_test_count"] = 1
}

if ($RunId -cnotmatch "^MT045-RUN-[A-Za-z0-9_-]{8,96}$") {
    throw "RunId must be a safe leaf matching ^MT045-RUN-[A-Za-z0-9_-]{8,96}$"
}

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
$supervisorRoot = [IO.Path]::GetFullPath((Join-Path $artifactRoot "wp-kernel-012\mt-045\supervisor"))
$runRoot = [IO.Path]::GetFullPath((Join-Path $supervisorRoot $RunId))
$artifactPrefix = $artifactRoot.TrimEnd("\") + "\"
if (-not $runRoot.StartsWith($artifactPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Resolved run path escaped the existing Handshake_Artifacts root: $runRoot"
}
if (Test-Path -LiteralPath $runRoot) {
    throw "Supervisor run id already exists: $RunId"
}

$sourcePaths = @(
    ".cargo/config.toml",
    "src/backend/handshake_core/build.rs",
    "src/backend/handshake_core/Cargo.toml",
    "src/backend/handshake_core/Cargo.lock",
    "src/backend/handshake_core/mechanical_engines.json",
    "src/backend/handshake_core/src",
    "src/backend/handshake_core/migrations",
    "src/backend/handshake_core/schemas",
    "src/frontend/palmistry",
    "src/frontend/handshake_native/build.rs",
    "src/frontend/handshake_native/Cargo.toml",
    "src/frontend/handshake_native/Cargo.lock",
    "src/frontend/handshake_native/diag_ring",
    "src/frontend/handshake_native/src",
    "src/frontend/handshake_native/tests/perf_proof_support/mod.rs",
    "src/frontend/handshake_native/tests/pg_proof_support/mod.rs",
    "src/frontend/handshake_native/tests/test_heartbeat.rs",
    "src/frontend/handshake_native/tests/test_diagnostics_panel.rs",
    "src/frontend/handshake_native/tests/test_perf_large_code.rs",
    "src/frontend/handshake_native/tests/test_perf_large_rich.rs",
    "src/frontend/handshake_native/tests/test_perf_large_knowledge.rs",
    "src/frontend/handshake_native/tests/run_mt045_perf_proof.ps1"
)
$manifestRepoPath = "src/frontend/handshake_native/tests/perf_proof/perf_manifest.json"
$manifestPath = Join-Path $repoRoot $manifestRepoPath
$sourceSha = Invoke-GitText -Repository $repoRoot -Arguments @("rev-parse", "HEAD")
Assert-SourceBindingClean -Repository $repoRoot -Paths $sourcePaths
Assert-SourceBindingClean -Repository $repoRoot -Paths @($manifestRepoPath)
$initialManifestGitObject = Invoke-GitText -Repository $repoRoot -Arguments @(
    "rev-parse", "${sourceSha}:$manifestRepoPath"
)
$initialManifestSha256 = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant()

$forbiddenBudgetOverrides = Get-ChildItem Env: |
    Where-Object { $_.Name -like "PERF_BUDGET_*" -and -not [string]::IsNullOrWhiteSpace($_.Value) }
if ($forbiddenBudgetOverrides) {
    throw "Canonical MT-045 proof forbids PERF_BUDGET_* overrides: $($forbiddenBudgetOverrides.Name -join ', ')"
}
if (-not [string]::IsNullOrWhiteSpace($env:SKIP_PERF_TESTS)) {
    throw "Canonical MT-045 proof forbids SKIP_PERF_TESTS"
}
if (-not [string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    throw "Canonical MT-045 proof forbids inherited CARGO_TARGET_DIR; the supervisor owns --target-dir"
}

$postgresUri = [Uri]$PostgresDsn
if (
    $postgresUri.Scheme -cne "postgresql" -or
    $postgresUri.Host -cne "127.0.0.1" -or
    $postgresUri.Port -ne 5544 -or
    $postgresUri.AbsolutePath.Trim("/") -cne "handshake"
) {
    throw "Canonical MT-045 PostgreSQL DSN must target postgresql://...@127.0.0.1:5544/handshake"
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
$pgPid = $pgProcess.Id
$pgStartTimeUtc = $pgProcess.StartTime.ToUniversalTime()

function Assert-PostgresPreserved {
    $listener = Get-NetTCPConnection -LocalAddress "127.0.0.1" -LocalPort 5544 -State Listen -ErrorAction Stop |
        Select-Object -First 1
    $process = Get-Process -Id $pgPid -ErrorAction Stop
    if (
        $listener.OwningProcess -ne $pgPid -or
        $process.ProcessName -cne "postgres" -or
        $process.StartTime.ToUniversalTime() -ne $pgStartTimeUtc
    ) {
        throw "Handshake internal PostgreSQL identity changed during MT-045 proof"
    }
}

[void][IO.Directory]::CreateDirectory($runRoot)
$measurementRoot = Join-Path $artifactRoot "wp-kernel-012\mt-045\measurements"
[void][IO.Directory]::CreateDirectory($measurementRoot)
$supervisorCurrentPath = Join-Path $measurementRoot "supervisor-current.json"
$currentRunPath = Join-Path $measurementRoot "current-run.json"
$latestRunPath = Join-Path $measurementRoot "latest-run-summary.json"
$expectedScenarioIds = @(
    "LC-01", "LC-02", "LC-03", "LC-04", "LC-05", "LC-06", "LC-07", "LC-08",
    "LR-01", "LR-02", "LR-03", "LR-04", "LR-05", "LR-06", "LR-07",
    "LK-01", "LK-02", "LK-03", "LK-04", "LK-05"
)

function New-SupervisorProjection {
    param(
        [Parameter(Mandatory)][ValidateSet("RUNNING", "FAIL")][string]$Status,
        [string]$Reason
    )
    return [ordered]@{
        schema_id = "hsk.wp_kernel_012.mt045_supervisor_projection@1"
        work_packet_id = "WP-KERNEL-012"
        micro_task_id = "MT-045"
        run_id = $RunId
        source_sha = $sourceSha
        status = $Status
        supervisor_preflight = $true
        terminal_reason = $Reason
        scenarios = [ordered]@{}
        started_at = $supervisorStartedAt.ToString("O")
        updated_at = [DateTimeOffset]::UtcNow.ToString("O")
    }
}

function Set-ManifestTerminalState {
    param(
        [Parameter(Mandatory)][ValidateSet("RUNNING", "FAIL")][string]$Status
    )
    $parsedRows = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    $rows = @()
    foreach ($entry in $parsedRows) {
        $rows += $entry
    }
    $actualIds = @($rows | ForEach-Object { $_.scenario_id } | Sort-Object)
    $expectedIds = @($expectedScenarioIds | Sort-Object)
    if (($actualIds -join "`n") -cne ($expectedIds -join "`n")) {
        throw "MT-045 manifest scenario set drifted"
    }
    foreach ($row in $rows) {
        $row.status = $Status
        $row.measured_value = $null
        $row.measured_profile = "release"
        $row.gated = $false
        $row.suite_run_id = $RunId
        $row.override_applied = $false
        $row.effective_budget = if ($null -ne $row.budget_ms) { $row.budget_ms } else { $row.budget_mb }
    }
    Write-JsonAtomic -Path $manifestPath -Value $rows
}

$commands = [Collections.Generic.List[object]]::new()
$runSucceeded = $false
$supervisorStartedAt = [DateTimeOffset]::UtcNow
try {
    $runningProjection = New-SupervisorProjection -Status "RUNNING"
    # supervisor-current is the top-level attempt gate. Publish it first, then invalidate both
    # historical run projections and finally the manifest; any later failure is terminally republished.
    Write-JsonAtomic -Path $supervisorCurrentPath -Value $runningProjection
    Write-JsonAtomic -Path $currentRunPath -Value $runningProjection
    Write-JsonAtomic -Path $latestRunPath -Value $runningProjection
    Set-ManifestTerminalState -Status "RUNNING"

    if ("Mt045JobRunner" -as [type]) {
        if ([Mt045JobRunner]::SourceId -cne $mt045JobRunnerExpectedSourceId) {
            throw "A stale Mt045JobRunner type is already loaded in this PowerShell host"
        }
    }
    else {
        Add-Type -Language CSharp -TypeDefinition $mt045JobRunnerSource
        if ([Mt045JobRunner]::SourceId -cne $mt045JobRunnerExpectedSourceId) {
            throw "The compiled Mt045JobRunner source id does not match the supervisor"
        }
    }

    $env:HSK_MT045_CANONICAL_RUN = "1"
    $env:HSK_MT045_RUN_ID = $RunId
    $env:HSK_MT045_SOURCE_SHA = $sourceSha
    $env:HANDSHAKE_ARTIFACTS_ROOT = $artifactRoot
    $env:HANDSHAKE_TEST_PG_DSN = $PostgresDsn
    $env:HANDSHAKE_TEST_STAGE_BINDING_ROOT = (Join-Path $runRoot "binding")
    Remove-Item Env:HSK_TEST_BASE -ErrorAction SilentlyContinue

    $commands.Add((Invoke-BoundedCargo -Label "build-handshake-core-release" -Arguments @(
        "build", "--release", "--locked", "--target-dir", $targetRoot,
        "--manifest-path", "..\..\backend\handshake_core\Cargo.toml",
        "--bin", "handshake_core", "--features", "app-runtime"
    ) -WorkingDirectory $crateRoot -LogRoot $runRoot))
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
            "test", "--release", "--locked", "--target-dir", $targetRoot,
            "--test", $bin, $test, "--", "--exact", "--nocapture", "--test-threads=1"
        ) -WorkingDirectory $crateRoot -LogRoot $runRoot
        Assert-ExactDiagnosticResult -CommandResult $result -ExpectedTest $test
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
            "test", "--release", "--locked", "--target-dir", $targetRoot,
            "--test", $bin, $test, "--", "--exact", "--nocapture", "--test-threads=1"
        ) -WorkingDirectory $crateRoot -LogRoot $runRoot))
    }

    $immutableRunPath = Join-Path $measurementRoot "runs\$RunId.json"
    $immutableRunDigestPath = "$immutableRunPath.sha256"
    foreach ($required in @($currentRunPath, $latestRunPath, $manifestPath, $immutableRunPath, $immutableRunDigestPath)) {
        if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
            throw "Canonical MT-045 completion artifact is missing: $required"
        }
    }
    $currentRun = Get-Content -LiteralPath $currentRunPath -Raw | ConvertFrom-Json
    $immutableRun = Get-Content -LiteralPath $immutableRunPath -Raw | ConvertFrom-Json
    foreach ($projection in @($currentRun, $immutableRun)) {
        if (
            $projection.run_id -cne $RunId -or
            $projection.status -cne "PASS" -or
            $projection.provenance.source_sha -cne $sourceSha -or
            @($projection.scenarios.psobject.Properties).Count -ne 20 -or
            @($projection.test_binaries.psobject.Properties).Count -ne 3
        ) {
            throw "MT-045 current/immutable run projection is not an exact canonical PASS for $RunId"
        }
    }
    $immutableDigest = (Get-FileHash -LiteralPath $immutableRunPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $recordedImmutableDigest = ((Get-Content -LiteralPath $immutableRunDigestPath -Raw).Trim() -split "\s+")[0]
    if ($recordedImmutableDigest -cne $immutableDigest) {
        throw "MT-045 immutable run digest sidecar does not match its JSON"
    }

    $parsedManifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    $manifest = @()
    foreach ($entry in $parsedManifest) {
        $manifest += $entry
    }
    if ($manifest.Count -ne 20) {
        throw "MT-045 manifest must contain exactly 20 rows, found $($manifest.Count)"
    }
    $manifestIds = @($manifest | ForEach-Object { $_.scenario_id } | Sort-Object)
    if (($manifestIds -join "`n") -cne (@($expectedScenarioIds | Sort-Object) -join "`n")) {
        throw "MT-045 manifest does not contain the exact 20 scenario ids"
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
            [double]$row.measured_value -lt 0 -or
            [double]$row.measured_value -gt [double]$contractBudget -or
            $row.effective_budget -ne $contractBudget
        ) {
            throw "MT-045 manifest row $($row.scenario_id) is not an exact canonical PASS"
        }
    }

    Assert-SourceBindingClean -Repository $repoRoot -Paths $sourcePaths
    $backendFinalSha256 = (Get-FileHash -LiteralPath $env:HSK_TEST_BACKEND_BIN -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($backendFinalSha256 -cne $backendSha256) {
        throw "MT-045 backend binary changed during the canonical run"
    }
    Assert-PostgresPreserved

    $manifestSnapshotPath = Join-Path $runRoot "perf-manifest-final.json"
    $manifestSnapshotSha256 = Write-ImmutableJson -Path $manifestSnapshotPath -Value $manifest
    $supervisorSummaryPath = Join-Path $runRoot "supervisor-summary.json"
    $supervisorSummary = [ordered]@{
        schema_id = "hsk.wp_kernel_012.mt045_supervisor_summary@1"
        work_packet_id = "WP-KERNEL-012"
        micro_task_id = "MT-045"
        run_id = $RunId
        status = "PASS"
        source_sha = $sourceSha
        cargo_profile = "release"
        cargo_locked = $true
        canonical_target_root = $targetRoot
        budget_overrides = @()
        postgres = [ordered]@{
            endpoint = "127.0.0.1:5544"
            database = "handshake"
            owning_pid = $pgPid
            process_name = $pgProcess.ProcessName
            process_start_time_utc = $pgStartTimeUtc.ToString("O")
            lifecycle = "existing_internal_postgresql_never_stopped"
            database_health_proven_by = "scenario-owned handshake_core /health status=ok and db_status=ok"
        }
        backend = [ordered]@{
            path = $env:HSK_TEST_BACKEND_BIN
            sha256 = $backendSha256
            managed_postgres = $false
        }
        diagnostics_receipt = $diagnosticReceiptPath
        diagnostics_receipt_sha256 = $diagnosticSha256
        immutable_run_summary = $immutableRunPath
        immutable_run_summary_sha256 = $immutableDigest
        immutable_manifest_snapshot = $manifestSnapshotPath
        immutable_manifest_snapshot_sha256 = $manifestSnapshotSha256
        initial_manifest_source = [ordered]@{
            repo_path = $manifestRepoPath
            git_object = $initialManifestGitObject
            sha256 = $initialManifestSha256
        }
        commands = $commands
        completed_at = [DateTimeOffset]::UtcNow.ToString("O")
    }
    Assert-PostgresPreserved
    $supervisorSha256 = Write-ImmutableJson -Path $supervisorSummaryPath -Value $supervisorSummary
    Write-JsonAtomic -Path $supervisorCurrentPath -Value ([ordered]@{
        schema_id = "hsk.wp_kernel_012.mt045_supervisor_projection@1"
        work_packet_id = "WP-KERNEL-012"
        micro_task_id = "MT-045"
        run_id = $RunId
        source_sha = $sourceSha
        status = "PASS"
        supervisor_summary = $supervisorSummaryPath
        supervisor_summary_sha256 = $supervisorSha256
        updated_at = [DateTimeOffset]::UtcNow.ToString("O")
    })
    $runSucceeded = $true
    Write-Output ([ordered]@{
        run_id = $RunId
        status = "PASS"
        source_sha = $sourceSha
        supervisor_summary = $supervisorSummaryPath
        supervisor_summary_sha256 = $supervisorSha256
    } | ConvertTo-Json -Depth 5)
}
catch {
    $failure = $_.Exception.Message
    $failedProjection = New-SupervisorProjection -Status "FAIL" -Reason $failure
    Write-JsonAtomic -Path $supervisorCurrentPath -Value $failedProjection
    Write-JsonAtomic -Path $currentRunPath -Value $failedProjection
    Write-JsonAtomic -Path $latestRunPath -Value $failedProjection
    Set-ManifestTerminalState -Status "FAIL"
    throw
}
finally {
    if (-not $runSucceeded) {
        try {
            Assert-PostgresPreserved
        }
        catch {
            Write-Error "PostgreSQL preservation check also failed: $($_.Exception.Message)"
        }
    }
}
