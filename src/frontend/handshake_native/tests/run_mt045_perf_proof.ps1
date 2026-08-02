[CmdletBinding()]
param(
    [string]$RunId = ("MT045-RUN-" + [guid]::NewGuid().ToString("N")),
    [ValidateRange(300, 1800)]
    [int]$CommandTimeoutSeconds = 1800,
    [string]$PostgresDsn = "postgresql://postgres@127.0.0.1:5544/handshake",
    [switch]$DiagnosticsSelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$mt045JobRunnerExpectedSourceId = "mt045-job-runner-20260729-v5"
$mt045JobRunnerSource = @'
using System;
using System.ComponentModel;
using System.Diagnostics;
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
    public const string SourceId = "mt045-job-runner-20260729-v5";
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
        var timer = Stopwatch.StartNew();
        while (QueryActiveProcessCount(job) != 0)
        {
            if (timer.ElapsedMilliseconds >= timeoutMilliseconds)
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
                var descendantTimer = Stopwatch.StartNew();
                while (true)
                {
                    activeProcesses = QueryActiveProcessCount(job);
                    if (
                        activeProcesses == 0 ||
                        descendantTimer.ElapsedMilliseconds >= descendantExitGraceMilliseconds)
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

function Get-FileSha256 {
    param(
        [Parameter(Mandatory)][string]$Path
    )
    $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($hasher.ComputeHash($stream))).Replace("-", "").ToLowerInvariant()
    }
    finally {
        $hasher.Dispose()
        $stream.Dispose()
    }
}

function Get-StringSha256 {
    param(
        [Parameter(Mandatory)][string]$Value
    )
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.UTF8Encoding]::new($false).GetBytes($Value)
        return ([BitConverter]::ToString($hasher.ComputeHash($bytes))).Replace("-", "").ToLowerInvariant()
    }
    finally {
        $hasher.Dispose()
    }
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
    $digest = Get-FileSha256 -Path $Path
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
    $testSwitchIndex = [Array]::IndexOf($Arguments, "--test")
    $testBinary = if ($testSwitchIndex -ge 0 -and $testSwitchIndex + 1 -lt $Arguments.Count) {
        [string]$Arguments[$testSwitchIndex + 1]
    } else { $null }
    $testName = if (
        $testSwitchIndex -ge 0 -and
        $testSwitchIndex + 2 -lt $Arguments.Count -and
        $Arguments[$testSwitchIndex + 2] -cne "--"
    ) { [string]$Arguments[$testSwitchIndex + 2] } else { $null }
    $stdoutPath = Join-Path $LogRoot "$safeLabel.stdout.log"
    $stderrPath = Join-Path $LogRoot "$safeLabel.stderr.log"
    $startedAt = [DateTimeOffset]::UtcNow
    $cargoPath = (Get-Command cargo -CommandType Application -ErrorAction Stop).Source
    $cleanupMarginSeconds = 180
    if ($TimeoutSeconds -le $cleanupMarginSeconds) {
        throw "$Label timeout must reserve more than ${cleanupMarginSeconds}s for contained cleanup"
    }
    $deadlineVariable = "HSK_MT045_COMMAND_DEADLINE_UNIX_MS"
    $qpcDeadlineVariable = "HSK_MT045_COMMAND_DEADLINE_QPC_TICKS"
    $budgetVariable = "HSK_MT045_COMMAND_BUDGET_MS"
    $previousDeadline = [Environment]::GetEnvironmentVariable($deadlineVariable, "Process")
    $previousQpcDeadline = [Environment]::GetEnvironmentVariable($qpcDeadlineVariable, "Process")
    $previousBudget = [Environment]::GetEnvironmentVariable($budgetVariable, "Process")
    $previousCommandLabel = [Environment]::GetEnvironmentVariable("HSK_MT045_COMMAND_LABEL", "Process")
    $previousTestBinary = [Environment]::GetEnvironmentVariable("HSK_MT045_TEST_BINARY", "Process")
    $previousTestName = [Environment]::GetEnvironmentVariable("HSK_MT045_TEST_NAME", "Process")
    $proofBudgetMs = ($TimeoutSeconds - $cleanupMarginSeconds) * 1000
    $proofDeadlineUnixMs = $startedAt.AddSeconds(
        $TimeoutSeconds - $cleanupMarginSeconds
    ).ToUnixTimeMilliseconds()
    [long]$proofDeadlineQpcTicks = [Diagnostics.Stopwatch]::GetTimestamp() + [long][Math]::Floor(
        (($TimeoutSeconds - $cleanupMarginSeconds) * [double][Diagnostics.Stopwatch]::Frequency)
    )
    $nativeResult = $null
    $runnerFailure = $null
    try {
        try {
            [Environment]::SetEnvironmentVariable(
                $deadlineVariable,
                $proofDeadlineUnixMs.ToString([Globalization.CultureInfo]::InvariantCulture),
                "Process"
            )
            [Environment]::SetEnvironmentVariable(
                $qpcDeadlineVariable,
                $proofDeadlineQpcTicks.ToString([Globalization.CultureInfo]::InvariantCulture),
                "Process"
            )
            [Environment]::SetEnvironmentVariable(
                $budgetVariable,
                $proofBudgetMs.ToString([Globalization.CultureInfo]::InvariantCulture),
                "Process"
            )
            [Environment]::SetEnvironmentVariable("HSK_MT045_COMMAND_LABEL", $Label, "Process")
            [Environment]::SetEnvironmentVariable("HSK_MT045_TEST_BINARY", $testBinary, "Process")
            [Environment]::SetEnvironmentVariable("HSK_MT045_TEST_NAME", $testName, "Process")
            $nativeResult = [Mt045JobRunner]::Run(
                $cargoPath,
                [string[]]$Arguments,
                $WorkingDirectory,
                $stdoutPath,
                $stderrPath,
                ($TimeoutSeconds * 1000),
                15000
            )
        }
        catch {
            $runnerFailure = $_.Exception.Message
        }
    }
    finally {
        [Environment]::SetEnvironmentVariable($deadlineVariable, $previousDeadline, "Process")
        [Environment]::SetEnvironmentVariable(
            $qpcDeadlineVariable,
            $previousQpcDeadline,
            "Process"
        )
        [Environment]::SetEnvironmentVariable($budgetVariable, $previousBudget, "Process")
        [Environment]::SetEnvironmentVariable("HSK_MT045_COMMAND_LABEL", $previousCommandLabel, "Process")
        [Environment]::SetEnvironmentVariable("HSK_MT045_TEST_BINARY", $previousTestBinary, "Process")
        [Environment]::SetEnvironmentVariable("HSK_MT045_TEST_NAME", $previousTestName, "Process")
    }
    if ($null -ne $runnerFailure) {
        $script:lastFailedCommandReceipt = [ordered]@{
            label = $Label
            test_binary = $testBinary
            test_name = $testName
            command = "cargo " + ($Arguments -join " ")
            started_at = $startedAt.ToString("O")
            completed_at = [DateTimeOffset]::UtcNow.ToString("O")
            timeout_seconds = $TimeoutSeconds
            proof_deadline_unix_ms = $proofDeadlineUnixMs
            proof_deadline_qpc_ticks = $proofDeadlineQpcTicks
            proof_budget_ms = $proofBudgetMs
            cleanup_margin_seconds = $cleanupMarginSeconds
            runner_error = $runnerFailure
            timed_out = $null
            leaked_process_count = $null
            exit_code = $null
            root_process_id = $null
            process_containment = "unknown_runner_failed_before_confirmation"
            process_containment_attempted = "windows_job_object_kill_on_close"
            job_drain_confirmed = $false
            stdout = $stdoutPath
            stdout_sha256 = if (Test-Path -LiteralPath $stdoutPath -PathType Leaf) { Get-FileSha256 -Path $stdoutPath } else { $null }
            stderr = $stderrPath
            stderr_sha256 = if (Test-Path -LiteralPath $stderrPath -PathType Leaf) { Get-FileSha256 -Path $stderrPath } else { $null }
        }
        throw "$Label Job runner failed: $runnerFailure"
    }
    $commandReceipt = [ordered]@{
        label = $Label
        test_binary = $testBinary
        test_name = $testName
        command = "cargo " + ($Arguments -join " ")
        started_at = $startedAt.ToString("O")
        completed_at = [DateTimeOffset]::UtcNow.ToString("O")
        timeout_seconds = $TimeoutSeconds
        proof_deadline_unix_ms = $proofDeadlineUnixMs
        proof_deadline_qpc_ticks = $proofDeadlineQpcTicks
        proof_budget_ms = $proofBudgetMs
        cleanup_margin_seconds = $cleanupMarginSeconds
        runner_error = $null
        timed_out = $nativeResult.TimedOut
        leaked_process_count = $nativeResult.LeakedProcessCount
        exit_code = $nativeResult.ExitCode
        root_process_id = $nativeResult.RootProcessId
        process_containment = "windows_job_object_kill_on_close"
        job_drain_confirmed = $true
        stdout = $stdoutPath
        stdout_sha256 = Get-FileSha256 -Path $stdoutPath
        stderr = $stderrPath
        stderr_sha256 = Get-FileSha256 -Path $stderrPath
    }
    if ($nativeResult.TimedOut) {
        $script:lastFailedCommandReceipt = $commandReceipt
        throw "$Label exceeded its hard ${TimeoutSeconds}s deadline; its Windows Job Object was terminated"
    }
    if ($nativeResult.LeakedProcessCount -ne 0) {
        $script:lastFailedCommandReceipt = $commandReceipt
        throw "$Label leaked $($nativeResult.LeakedProcessCount) owned descendant process(es); its Windows Job Object was terminated"
    }
    if ($nativeResult.ExitCode -ne 0) {
        $script:lastFailedCommandReceipt = $commandReceipt
        throw "$Label failed with exit code $($nativeResult.ExitCode)"
    }
    return $commandReceipt
}

function Assert-ExactTestResult {
    param(
        [Parameter(Mandatory)]$CommandResult,
        [Parameter(Mandatory)][string]$ExpectedTest
    )
    $stdout = [IO.File]::ReadAllText($CommandResult.stdout)
    $stderr = [IO.File]::ReadAllText($CommandResult.stderr)
    $runningMatches = [regex]::Matches($stdout, "(?m)^running 1 test\r?$")
    $summaryMatches = [regex]::Matches(
        $stdout,
        "(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; [0-9]+ filtered out; finished in [^\r\n]+\r?$"
    )
    if ($runningMatches.Count -ne 1 -or $summaryMatches.Count -ne 1) {
        $script:lastFailedCommandReceipt = $CommandResult
        throw "Test $ExpectedTest did not execute exactly one passing test; inspect $($CommandResult.stdout)"
    }
    if ($stdout.Contains("panicked at") -or $stderr.Contains("panicked at")) {
        $script:lastFailedCommandReceipt = $CommandResult
        throw "Test $ExpectedTest emitted a panic even though Cargo reported PASS; inspect $($CommandResult.stdout) and $($CommandResult.stderr)"
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
if (-not $DiagnosticsSelfTest) {
    $sourceSha = Invoke-GitText -Repository $repoRoot -Arguments @("rev-parse", "HEAD")
    Assert-SourceBindingClean -Repository $repoRoot -Paths $sourcePaths
    $initialManifestGitObject = Invoke-GitText -Repository $repoRoot -Arguments @(
        "rev-parse", "${sourceSha}:$manifestRepoPath"
    )
    $headManifestJson = Invoke-GitText -Repository $repoRoot -Arguments @(
        "show", "${sourceSha}:$manifestRepoPath"
    )
    $initialManifestSha256 = Get-StringSha256 -Value $headManifestJson

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
}
else {
    $sourceSha = "diagnostics-self-test"
    $initialManifestGitObject = $null
    $headManifestJson = "[]"
    $initialManifestSha256 = Get-StringSha256 -Value $headManifestJson
    $pgPid = 0
    $pgStartTimeUtc = [DateTime]::MinValue
}

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
$failureDiagnosticsRoot = Join-Path $artifactRoot "wp-kernel-012\mt-045\failure-diagnostics\$RunId"
$backendRuntimeRunRoot = Join-Path $artifactRoot "wp-kernel-012\backend-runtime\$RunId"
$expectedScenarioIds = @(
    "LC-01", "LC-02", "LC-03", "LC-04", "LC-05", "LC-06", "LC-07", "LC-08",
    "LR-01", "LR-02", "LR-03", "LR-04", "LR-05", "LR-06", "LR-07",
    "LK-01", "LK-02", "LK-03", "LK-04", "LK-05"
)

function Assert-NoReparsePath {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Boundary
    )
    $fullPath = [IO.Path]::GetFullPath($Path)
    $fullBoundary = [IO.Path]::GetFullPath($Boundary).TrimEnd("\")
    $boundaryPrefix = $fullBoundary + "\"
    if (
        $fullPath -cne $fullBoundary -and
        -not $fullPath.StartsWith($boundaryPrefix, [StringComparison]::OrdinalIgnoreCase)
    ) {
        throw "path escaped canonical boundary: $fullPath (boundary $fullBoundary)"
    }
    $current = $fullPath
    while ($true) {
        $item = Get-Item -LiteralPath $current -Force -ErrorAction Stop
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "reparse point is forbidden in diagnostic binding path: $current"
        }
        if ($current -ceq $fullBoundary) {
            break
        }
        $parent = [IO.Path]::GetDirectoryName($current)
        if ([string]::IsNullOrEmpty($parent) -or $parent -ceq $current) {
            throw "diagnostic binding path did not reach boundary: $fullPath"
        }
        $current = $parent
    }
}

function Test-IsJsonInteger {
    param([AllowNull()]$Value)
    return (
        $Value -is [byte] -or $Value -is [sbyte] -or
        $Value -is [int16] -or $Value -is [uint16] -or
        $Value -is [int32] -or $Value -is [uint32] -or
        $Value -is [int64] -or $Value -is [uint64]
    )
}

function Resolve-Mt045Psql {
    $configured = [Environment]::GetEnvironmentVariable("HSK_PSQL_BIN")
    if (-not [string]::IsNullOrWhiteSpace($configured)) {
        $item = Get-Item -LiteralPath $configured -ErrorAction Stop
        if (-not $item.PSIsContainer) { return $item.FullName }
        throw "HSK_PSQL_BIN is not a file: $configured"
    }
    $command = Get-Command psql.exe -CommandType Application -ErrorAction SilentlyContinue
    if ($null -ne $command) { return $command.Source }
    $postgresRoot = Join-Path $env:ProgramFiles "PostgreSQL"
    if (Test-Path -LiteralPath $postgresRoot -PathType Container) {
        $candidate = Get-ChildItem -LiteralPath $postgresRoot -Directory -ErrorAction Stop |
            Where-Object { $_.Name -match "^[0-9]+$" } |
            Sort-Object { [int]$_.Name } -Descending |
            ForEach-Object { Join-Path $_.FullName "bin\psql.exe" } |
            Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
            Select-Object -First 1
        if ($null -ne $candidate) { return [IO.Path]::GetFullPath($candidate) }
    }
    throw "MT-045 requires an explicit psql executable for post-Job workspace cleanup"
}

$script:mt045PsqlPath = Resolve-Mt045Psql

function Invoke-Mt045PostReapWorkspaceCleanup {
    param(
        [Parameter(Mandatory)][string]$RuntimeDirectory,
        [Parameter(Mandatory)]$ExpectedCommand,
        [Parameter(Mandatory)][uint64]$OwnedBackendPid
    )
    $markerPath = Join-Path $RuntimeDirectory "workspace-identity.json"
    try {
        $markerItem = Get-Item -LiteralPath $markerPath -Force -ErrorAction Stop
    }
    catch {
        if ($_.FullyQualifiedErrorId -like "PathNotFound,*" -or $_.Exception -is [IO.FileNotFoundException]) {
            throw "workspace identity marker is absent; pre-creation termination cannot be proven"
        }
        throw
    }
    if ($markerItem.PSIsContainer) {
        throw "workspace identity marker is not a regular file: $markerPath"
    }
    Assert-NoReparsePath -Path $markerItem.FullName -Boundary $RuntimeDirectory
    $marker = Get-Content -LiteralPath $markerItem.FullName -Raw -ErrorAction Stop | ConvertFrom-Json
    if (
        $marker.schema_id -cne "hsk.wp_kernel_012.mt045_workspace_identity@1" -or
        $marker.run_id -cne $RunId -or
        $marker.scenario_identity -cne $ExpectedCommand.test_name -or
        [string]::IsNullOrWhiteSpace([string]$marker.workspace_id) -or
        -not (Test-IsJsonInteger $marker.owned_backend_pid) -or
        [uint64]$marker.owned_backend_pid -ne $OwnedBackendPid
    ) {
        throw "workspace identity marker does not bind the exact owned run/scenario/backend"
    }
    $workspaceId = [string]$marker.workspace_id
    $literal = $workspaceId.Replace("'", "''")
    $stdoutPath = Join-Path $RuntimeDirectory "workspace-cleanup.stdout.log"
    $stderrPath = Join-Path $RuntimeDirectory "workspace-cleanup.stderr.log"
    $cleanup = [Mt045JobRunner]::Run(
        $script:mt045PsqlPath,
        [string[]]@(
            "--no-psqlrc", "--no-password", "--set", "ON_ERROR_STOP=1", "--quiet",
            "--tuples-only", "--no-align", "--dbname", $PostgresDsn,
            "--command", "DELETE FROM workspaces WHERE id = '$literal'; SELECT COUNT(*) FROM workspaces WHERE id = '$literal';"
        ),
        $RuntimeDirectory,
        $stdoutPath,
        $stderrPath,
        30000,
        3000
    )
    if (
        $cleanup.TimedOut -or $cleanup.LeakedProcessCount -ne 0 -or
        $cleanup.ExitCode -ne 0
    ) {
        throw "bounded post-Job workspace cleanup failed (exit=$($cleanup.ExitCode), timeout=$($cleanup.TimedOut), leaked=$($cleanup.LeakedProcessCount))"
    }
    $stdout = Get-Content -LiteralPath $stdoutPath -Raw -ErrorAction Stop
    if ($stdout.Trim() -cne "0") {
        throw "post-Job workspace cleanup absence proof is not the exact scalar zero: $($stdout.Trim())"
    }
    $proofPath = Join-Path $RuntimeDirectory "workspace-cleanup.json"
    $proof = [ordered]@{
        schema_id = "hsk.wp_kernel_012.mt045_workspace_cleanup@1"
        run_id = $RunId
        scenario_identity = $ExpectedCommand.test_name
        workspace_id = $workspaceId
        status = "deleted_and_verified_absent"
        verified_absent = $true
        remaining_workspace_count = [int64]0
        marker_path = $markerItem.FullName
        marker_bytes = $markerItem.Length
        marker_sha256 = Get-FileSha256 -Path $markerItem.FullName
        stdout_path = $stdoutPath
        stdout_bytes = (Get-Item -LiteralPath $stdoutPath -ErrorAction Stop).Length
        stdout_sha256 = Get-FileSha256 -Path $stdoutPath
        stderr_path = $stderrPath
        stderr_bytes = (Get-Item -LiteralPath $stderrPath -ErrorAction Stop).Length
        stderr_sha256 = Get-FileSha256 -Path $stderrPath
        cleanup_process = [ordered]@{
            root_process_id = $cleanup.RootProcessId
            exit_code = $cleanup.ExitCode
            timed_out = $cleanup.TimedOut
            leaked_process_count = $cleanup.LeakedProcessCount
        }
    }
    Write-JsonAtomic -Path $proofPath -Value $proof
    return [ordered]@{
        status = "deleted_and_verified_absent"
        workspace_id = $workspaceId
        verified_absent = $true
        remaining_workspace_count = [int64]0
        proof_path = $proofPath
        proof_bytes = (Get-Item -LiteralPath $proofPath -ErrorAction Stop).Length
        proof_sha256 = Get-FileSha256 -Path $proofPath
        marker_path = $markerItem.FullName
        marker_bytes = $markerItem.Length
        marker_sha256 = $proof.marker_sha256
        stdout_path = $stdoutPath
        stdout_bytes = $proof.stdout_bytes
        stdout_sha256 = $proof.stdout_sha256
        stderr_path = $stderrPath
        stderr_bytes = $proof.stderr_bytes
        stderr_sha256 = $proof.stderr_sha256
    }
}

function Get-PostReapRuntimeBinding {
    param([AllowNull()]$ExpectedCommand)
    try {
        if ($null -eq $ExpectedCommand -or [string]::IsNullOrWhiteSpace([string]$ExpectedCommand.test_name)) {
            throw "failed command does not identify an exact test scenario"
        }
        if (
            $null -ne $ExpectedCommand.runner_error -or
            $ExpectedCommand.process_containment -cne "windows_job_object_kill_on_close" -or
            $ExpectedCommand.job_drain_confirmed -ne $true -or
            $ExpectedCommand.leaked_process_count -ne 0 -or
            $null -eq $ExpectedCommand.root_process_id -or
            $null -eq $ExpectedCommand.exit_code
        ) {
            throw "failed command does not prove Windows Job containment and complete descendant drain"
        }
        $runtimeRunItem = Get-Item -LiteralPath $backendRuntimeRunRoot -Force -ErrorAction Stop
        if (-not $runtimeRunItem.PSIsContainer) {
            throw "post-reap backend runtime run root is not a directory: $backendRuntimeRunRoot"
        }
        Assert-NoReparsePath -Path $backendRuntimeRunRoot -Boundary (Join-Path $artifactRoot "wp-kernel-012")
        $scenarioRoot = Join-Path $backendRuntimeRunRoot ([string]$ExpectedCommand.test_name)
        $scenarioItem = Get-Item -LiteralPath $scenarioRoot -Force -ErrorAction Stop
        if (-not $scenarioItem.PSIsContainer) {
            throw "post-reap backend runtime scenario root is not a directory: $scenarioRoot"
        }
        Assert-NoReparsePath -Path $scenarioRoot -Boundary $backendRuntimeRunRoot
        $candidates = @()
        foreach ($candidate in @(Get-ChildItem -LiteralPath $scenarioRoot -Directory -Force -ErrorAction Stop)) {
            Assert-NoReparsePath -Path $candidate.FullName -Boundary $scenarioRoot
            $paths = [ordered]@{
                "listen-report.json" = Join-Path $candidate.FullName "listen-report.json"
                "backend.stdout.log" = Join-Path $candidate.FullName "backend.stdout.log"
                "backend.stderr.log" = Join-Path $candidate.FullName "backend.stderr.log"
            }
            $complete = $true
            foreach ($path in $paths.Values) {
                try {
                    $pathItem = Get-Item -LiteralPath $path -Force -ErrorAction Stop
                }
                catch {
                    if ($_.FullyQualifiedErrorId -like "PathNotFound,*" -or $_.Exception -is [IO.FileNotFoundException]) {
                        $complete = $false
                        break
                    }
                    throw
                }
                if ($pathItem.PSIsContainer) {
                    $complete = $false
                    break
                }
                Assert-NoReparsePath -Path $path -Boundary $candidate.FullName
            }
            if ($complete) {
                $candidates += [pscustomobject]@{
                    directory = $candidate.FullName
                    modified = $candidate.LastWriteTimeUtc
                    paths = $paths
                }
            }
        }
        if ($candidates.Count -ne 1) {
            throw "post-reap recovery requires exactly one complete runtime candidate for $($ExpectedCommand.test_name), found $($candidates.Count)"
        }
        $selected = $candidates[0]
        $listenReport = Get-Content -LiteralPath $selected.paths["listen-report.json"] -Raw | ConvertFrom-Json
        if (
            $listenReport.schema_id -cne "handshake.backend-listen-report.v1" -or
            $null -eq $listenReport.pid -or
            [uint64]$listenReport.pid -eq 0
        ) {
            throw "post-reap listen report lacks exact owned backend PID identity"
        }
        $workspaceCleanup = Invoke-Mt045PostReapWorkspaceCleanup `
            -RuntimeDirectory $selected.directory `
            -ExpectedCommand $ExpectedCommand `
            -OwnedBackendPid ([uint64]$listenReport.pid)
        $retainedFiles = @()
        foreach ($name in @("listen-report.json", "backend.stdout.log", "backend.stderr.log")) {
            $path = [IO.Path]::GetFullPath([string]$selected.paths[$name])
            $retainedFiles += [ordered]@{
                runtime_root_index = 0
                name = $name
                status = "retained"
                path = $path
                bytes = (Get-Item -LiteralPath $path).Length
                sha256 = Get-FileSha256 -Path $path
            }
        }
        return [ordered]@{
            binding_status = "BOUND"
            binding_source = "post_reap_backend_runtime"
            receipt = $null
            receipt_sha256 = $null
            receipt_sha256_sidecar = $null
            scenario_identity = [string]$ExpectedCommand.test_name
            trigger = "supervisor_job_recovery"
            stage = if ($ExpectedCommand.timed_out -eq $true) { "hard_timeout" } else { "forced_or_abnormal_exit" }
            command_binding = [ordered]@{
                label = $ExpectedCommand.label
                test_binary = $ExpectedCommand.test_binary
                test_name = $ExpectedCommand.test_name
            }
            process = [ordered]@{
                owned = $true
                pid = [uint64]$listenReport.pid
                try_wait = "reaped_by_windows_job_object"
                termination = "job_drained_before_hashing"
                exit_code = $null
                success = $null
            }
            job_containment = [ordered]@{
                root_process_id = $ExpectedCommand.root_process_id
                exit_code = $ExpectedCommand.exit_code
                success = ($null -ne $ExpectedCommand.exit_code -and [int]$ExpectedCommand.exit_code -eq 0)
                process_containment = $ExpectedCommand.process_containment
                job_drain_confirmed = $ExpectedCommand.job_drain_confirmed
                leaked_process_count = $ExpectedCommand.leaked_process_count
                runner_error = $ExpectedCommand.runner_error
            }
            immediate_health = $null
            reqwest_error = $null
            workspace_cleanup = $workspaceCleanup
            runtime_candidate_count = $candidates.Count
            retained_files = $retainedFiles
        }
    }
    catch {
        return [ordered]@{
            binding_status = "INVALID"
            binding_source = "post_reap_backend_runtime"
            receipt = $null
            binding_error = $_.Exception.Message
        }
    }
}

function Get-FailureDiagnosticBindings {
    param([AllowNull()]$ExpectedCommand)
    $bindings = @()
    $rustReceiptCount = 0
    $rustReceiptEnumerationTrusted = $true
    try {
        if (
            $DiagnosticsSelfTest -and
            (Get-Variable -Name injectMt045ReceiptEnumerationFailure -Scope Script -ErrorAction SilentlyContinue) -and
            $script:injectMt045ReceiptEnumerationFailure
        ) {
            throw "injected inaccessible failure diagnostics root"
        }
        $failureRootItem = $null
        try {
            $failureRootItem = Get-Item -LiteralPath $failureDiagnosticsRoot -Force -ErrorAction Stop
        }
        catch {
            if (-not ($_.FullyQualifiedErrorId -like "PathNotFound,*" -or $_.Exception -is [IO.FileNotFoundException])) {
                throw
            }
        }
        if ($null -ne $failureRootItem) {
            if (-not $failureRootItem.PSIsContainer) {
                throw "failure diagnostics root exists but is not a directory: $failureDiagnosticsRoot"
            }
            Assert-NoReparsePath -Path $failureDiagnosticsRoot -Boundary (Join-Path $artifactRoot "wp-kernel-012")
            foreach ($receiptFile in @(Get-ChildItem -LiteralPath $failureDiagnosticsRoot -Recurse -File -Filter "failure-diagnostics.json" -ErrorAction Stop | Sort-Object FullName)) {
                $rustReceiptCount += 1
                try {
                    $receiptPath = [IO.Path]::GetFullPath($receiptFile.FullName)
                    Assert-NoReparsePath -Path $receiptPath -Boundary $failureDiagnosticsRoot
                    $receiptDirectory = [IO.Path]::GetDirectoryName($receiptPath)
                    $sidecarPath = "$receiptPath.sha256"
                    if (-not (Test-Path -LiteralPath $sidecarPath -PathType Leaf)) {
                        throw "failure diagnostic digest sidecar is missing: $sidecarPath"
                    }
                    Assert-NoReparsePath -Path $sidecarPath -Boundary $receiptDirectory
                    $receiptSha256 = Get-FileSha256 -Path $receiptPath
                    $recordedSha256 = ((Get-Content -LiteralPath $sidecarPath -Raw).Trim() -split "\s+")[0]
                    if ($recordedSha256 -cne $receiptSha256) {
                        throw "failure diagnostic receipt digest mismatch: $receiptPath"
                    }
                    $receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
                    if (
                        $receipt.schema_id -cne "hsk.wp_kernel_012.mt045_failure_diagnostics@1" -or
                        $receipt.run_id -cne $RunId -or
                        $receipt.retention_status -cne "complete"
                    ) {
                        throw "failure diagnostic receipt identity/completeness mismatch: $receiptPath"
                    }
                    if (
                        $null -eq $ExpectedCommand -or
                        $receipt.scenario_identity -cne $ExpectedCommand.test_name -or
                        $receipt.command_binding.label -cne $ExpectedCommand.label -or
                        $receipt.command_binding.test_binary -cne $ExpectedCommand.test_binary -or
                        $receipt.command_binding.test_name -cne $ExpectedCommand.test_name
                    ) {
                        throw "failure diagnostic command/test/scenario binding mismatch: $receiptPath"
                    }
                    if (
                        $receipt.process.owned -ne $true -or
                        $null -eq $receipt.process.pid -or
                        [uint64]$receipt.process.pid -eq 0 -or
                        [string]::IsNullOrWhiteSpace([string]$receipt.process.try_wait) -or
                        $receipt.process.termination -notin @("already_exited_and_reaped", "terminated_and_reaped") -or
                        -not (Test-IsJsonInteger $receipt.process.exit_code) -or
                        $receipt.process.success -isnot [bool] -or
                        ($receipt.process.termination -ceq "terminated_and_reaped" -and $receipt.process.success -ne $false)
                    ) {
                        throw "failure diagnostic lacks reaped fixture-owned process identity: $receiptPath"
                    }
                    $workspaceCleanup = $receipt.workspace_cleanup
                    if ($workspaceCleanup.status -ceq "no_workspace") {
                        if (
                            $workspaceCleanup.verified_absent -ne $true -or
                            -not (Test-IsJsonInteger $workspaceCleanup.remaining_workspace_count) -or
                            [int64]$workspaceCleanup.remaining_workspace_count -ne 0
                        ) {
                            throw "no-workspace cleanup receipt lacks verified zero-count proof: $receiptPath"
                        }
                    }
                    elseif ($workspaceCleanup.status -ceq "deleted_and_verified_absent") {
                        if (
                            $workspaceCleanup.verified_absent -ne $true -or
                            -not (Test-IsJsonInteger $workspaceCleanup.remaining_workspace_count) -or
                            [int64]$workspaceCleanup.remaining_workspace_count -ne 0
                        ) {
                            throw "workspace cleanup receipt lacks verified zero-count proof: $receiptPath"
                        }
                        foreach ($cleanupStream in @("stdout", "stderr")) {
                            $pathField = "retained_${cleanupStream}_path"
                            $bytesField = "retained_${cleanupStream}_bytes"
                            $hashField = "retained_${cleanupStream}_sha256"
                            $cleanupPath = [IO.Path]::GetFullPath([string]$workspaceCleanup.$pathField)
                            Assert-NoReparsePath -Path $cleanupPath -Boundary $receiptDirectory
                            $cleanupItem = Get-Item -LiteralPath $cleanupPath -Force -ErrorAction Stop
                            if ($cleanupItem.PSIsContainer) {
                                throw "retained workspace cleanup $cleanupStream proof is missing: $cleanupPath"
                            }
                            if ($cleanupItem.Length -ne [int64]$workspaceCleanup.$bytesField) {
                                throw "retained workspace cleanup $cleanupStream byte count mismatch: $cleanupPath"
                            }
                            if ((Get-FileSha256 -Path $cleanupPath) -cne [string]$workspaceCleanup.$hashField) {
                                throw "retained workspace cleanup $cleanupStream digest mismatch: $cleanupPath"
                            }
                            if (
                                $cleanupStream -ceq "stdout" -and
                                (Get-Content -LiteralPath $cleanupPath -Raw -ErrorAction Stop).Trim() -cne "0"
                            ) {
                                throw "retained workspace cleanup stdout is not the exact scalar zero: $cleanupPath"
                            }
                        }
                    }
                    else {
                        throw "failure diagnostic lacks successful post-reap workspace cleanup proof: $receiptPath"
                    }
                    if ($receipt.trigger -ceq "request_failure") {
                        if (
                            $null -eq $receipt.reqwest_error -or
                            $receipt.stage -notin @("request_send", "response_body") -or
                            -not (
                                $receipt.reqwest_error.is_request -eq $true -or
                                $receipt.reqwest_error.is_connect -eq $true -or
                                $receipt.reqwest_error.is_timeout -eq $true -or
                                $receipt.reqwest_error.is_body -eq $true -or
                                $receipt.reqwest_error.is_decode -eq $true -or
                                $receipt.reqwest_error.is_builder -eq $true
                            )
                        ) {
                            throw "request-failure receipt lacks typed reqwest classification: $receiptPath"
                        }
                    }
                    elseif ($receipt.trigger -ceq "panic_drop") {
                        if ($null -ne $receipt.reqwest_error -or $receipt.stage -cne "unwind") {
                            throw "panic-drop receipt has invalid reqwest/stage classification: $receiptPath"
                        }
                    }
                    else {
                        throw "unknown Rust failure diagnostic trigger: $($receipt.trigger)"
                    }
                    $files = @($receipt.retained_files)
                    $names = @($files | ForEach-Object { [string]$_.name } | Sort-Object)
                    $expectedNames = @("backend.stderr.log", "backend.stdout.log", "listen-report.json")
                    if (
                        $files.Count -ne 3 -or
                        ($names -join "`n") -cne ($expectedNames -join "`n") -or
                        @($files | Where-Object { $_.runtime_root_index -ne 0 -or $_.status -cne "retained" }).Count -ne 0
                    ) {
                        throw "failure diagnostic must bind exactly one complete listen/stdout/stderr set: $receiptPath"
                    }
                    $retainedFiles = @()
                    foreach ($file in $files) {
                        $retainedPath = [IO.Path]::GetFullPath([string]$file.retained_path)
                        Assert-NoReparsePath -Path $retainedPath -Boundary $receiptDirectory
                        if (-not (Test-Path -LiteralPath $retainedPath -PathType Leaf)) {
                            throw "retained backend diagnostic is missing: $retainedPath"
                        }
                        if ((Get-Item -LiteralPath $retainedPath).Length -ne [int64]$file.bytes) {
                            throw "retained backend diagnostic byte count mismatch: $retainedPath"
                        }
                        if ((Get-FileSha256 -Path $retainedPath) -cne [string]$file.sha256) {
                            throw "retained backend diagnostic digest mismatch: $retainedPath"
                        }
                        $retainedFiles += [ordered]@{
                            runtime_root_index = 0
                            name = $file.name
                            status = "retained"
                            path = $retainedPath
                            bytes = $file.bytes
                            sha256 = $file.sha256
                        }
                    }
                    $bindings += [ordered]@{
                        binding_status = "BOUND"
                        binding_source = "rust_failure_receipt"
                        receipt = $receiptPath
                        receipt_sha256 = $receiptSha256
                        receipt_sha256_sidecar = $sidecarPath
                        scenario_identity = $receipt.scenario_identity
                        trigger = $receipt.trigger
                        stage = $receipt.stage
                        command_binding = $receipt.command_binding
                        process = $receipt.process
                        immediate_health = $receipt.immediate_health
                        reqwest_error = $receipt.reqwest_error
                        retained_files = $retainedFiles
                    }
                }
                catch {
                    $bindings += [ordered]@{
                        binding_status = "INVALID"
                        binding_source = "rust_failure_receipt"
                        receipt = $receiptFile.FullName
                        binding_error = $_.Exception.Message
                    }
                }
            }
        }
    }
    catch {
        $rustReceiptEnumerationTrusted = $false
        $bindings += [ordered]@{
            binding_status = "INVALID"
            binding_source = "receipt_enumeration"
            receipt = $failureDiagnosticsRoot
            binding_error = $_.Exception.Message
        }
    }
    if ($rustReceiptEnumerationTrusted -and $rustReceiptCount -eq 0) {
        $bindings += Get-PostReapRuntimeBinding -ExpectedCommand $ExpectedCommand
    }
    return $bindings
}

if ($DiagnosticsSelfTest) {
    $selfTestRoot = [IO.Path]::GetFullPath((Join-Path $artifactRoot "wp-kernel-012\mt-045\diagnostics-self-test\$RunId"))
    $selfTestWorkspaceId = $null
    if (-not $selfTestRoot.StartsWith($artifactPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "diagnostics self-test root escaped Handshake_Artifacts: $selfTestRoot"
    }
    try {
        [void][IO.Directory]::CreateDirectory($selfTestRoot)
        [void][IO.Directory]::CreateDirectory($failureDiagnosticsRoot)

        # Counterfactual 1: a structurally valid envelope with no owned PID and no retained set must
        # never become BOUND, even when its receipt and sidecar hashes are internally consistent.
        $malformedDirectory = Join-Path $failureDiagnosticsRoot "malformed-test\all-missing"
        [void][IO.Directory]::CreateDirectory($malformedDirectory)
        $malformedPath = Join-Path $malformedDirectory "failure-diagnostics.json"
        $expectedMalformed = [ordered]@{
            label = "malformed-test"
            test_binary = "test_perf_large_knowledge"
            test_name = "malformed-test"
            timed_out = $false
        }
        $malformedReceipt = [ordered]@{
            schema_id = "hsk.wp_kernel_012.mt045_failure_diagnostics@1"
            run_id = $RunId
            scenario_identity = "malformed-test"
            retention_status = "partial"
            trigger = "panic_drop"
            stage = "unwind"
            command_binding = $expectedMalformed
            process = [ordered]@{ owned = $false; pid = $null; try_wait = "not_owned" }
            reqwest_error = $null
            retained_files = @()
        }
        Write-JsonAtomic -Path $malformedPath -Value $malformedReceipt
        [IO.File]::WriteAllText(
            "$malformedPath.sha256",
            "$(Get-FileSha256 -Path $malformedPath)  failure-diagnostics.json`n",
            [Text.UTF8Encoding]::new($false)
        )
        $malformedRuntime = Join-Path $backendRuntimeRunRoot "malformed-test\runtime-001"
        [void][IO.Directory]::CreateDirectory($malformedRuntime)
        [IO.File]::WriteAllText((Join-Path $malformedRuntime "listen-report.json"), '{"schema_id":"handshake.backend-listen-report.v1","pid":4242,"listen_addr":"127.0.0.1:1"}')
        [IO.File]::WriteAllText((Join-Path $malformedRuntime "backend.stdout.log"), "complete-but-invalid-receipt`n")
        [IO.File]::WriteAllText((Join-Path $malformedRuntime "backend.stderr.log"), "complete-but-invalid-receipt`n")
        $malformedBindings = @(Get-FailureDiagnosticBindings -ExpectedCommand $expectedMalformed)
        if (@($malformedBindings | Where-Object { $_.binding_status -ceq "BOUND" }).Count -ne 0) {
            throw "diagnostics self-test accepted an all-missing/unowned receipt as BOUND"
        }

        # Counterfactual 1b: a hash-valid, complete receipt is still invalid when its reaped process
        # evidence or post-reap cleanup evidence is incomplete.
        function New-ForgedCompleteReceipt {
            param(
                [Parameter(Mandatory)][string]$Scenario,
                [AllowNull()]$ExitCode,
                [Parameter(Mandatory)][bool]$ProcessSuccess,
                [Parameter(Mandatory)]$WorkspaceCleanup,
                [AllowNull()][string]$CleanupStdout
            )
            $directory = Join-Path $failureDiagnosticsRoot "$Scenario\receipt"
            $retainedDirectory = Join-Path $directory "backend-00"
            [void][IO.Directory]::CreateDirectory($retainedDirectory)
            $retained = @()
            foreach ($name in @("listen-report.json", "backend.stdout.log", "backend.stderr.log")) {
                $path = Join-Path $retainedDirectory $name
                [IO.File]::WriteAllText($path, "forged-$Scenario-$name`n", [Text.UTF8Encoding]::new($false))
                $retained += [ordered]@{
                    runtime_root_index = 0
                    name = $name
                    retained_path = $path
                    status = "retained"
                    bytes = (Get-Item -LiteralPath $path).Length
                    sha256 = Get-FileSha256 -Path $path
                }
            }
            if ($null -ne $CleanupStdout) {
                $cleanupStdoutPath = Join-Path $retainedDirectory "workspace-cleanup.stdout.log"
                $cleanupStderrPath = Join-Path $retainedDirectory "workspace-cleanup.stderr.log"
                [IO.File]::WriteAllText($cleanupStdoutPath, $CleanupStdout, [Text.UTF8Encoding]::new($false))
                [IO.File]::WriteAllText($cleanupStderrPath, "", [Text.UTF8Encoding]::new($false))
                $WorkspaceCleanup = [ordered]@{
                    status = "deleted_and_verified_absent"
                    verified_absent = $true
                    remaining_workspace_count = [int64]0
                    retained_stdout_path = $cleanupStdoutPath
                    retained_stdout_bytes = (Get-Item -LiteralPath $cleanupStdoutPath).Length
                    retained_stdout_sha256 = Get-FileSha256 -Path $cleanupStdoutPath
                    retained_stderr_path = $cleanupStderrPath
                    retained_stderr_bytes = (Get-Item -LiteralPath $cleanupStderrPath).Length
                    retained_stderr_sha256 = Get-FileSha256 -Path $cleanupStderrPath
                }
            }
            $expected = [ordered]@{
                label = $Scenario
                test_binary = "test_perf_large_knowledge"
                test_name = $Scenario
            }
            $receiptPath = Join-Path $directory "failure-diagnostics.json"
            $receipt = [ordered]@{
                schema_id = "hsk.wp_kernel_012.mt045_failure_diagnostics@1"
                run_id = $RunId
                scenario_identity = $Scenario
                retention_status = "complete"
                trigger = "panic_drop"
                stage = "unwind"
                command_binding = $expected
                process = [ordered]@{
                    owned = $true
                    pid = 4242
                    try_wait = "reaped"
                    termination = "terminated_and_reaped"
                    exit_code = $ExitCode
                    success = $ProcessSuccess
                }
                reqwest_error = $null
                workspace_cleanup = $WorkspaceCleanup
                retained_files = $retained
            }
            Write-JsonAtomic -Path $receiptPath -Value $receipt
            [IO.File]::WriteAllText(
                "$receiptPath.sha256",
                "$(Get-FileSha256 -Path $receiptPath)  failure-diagnostics.json`n",
                [Text.UTF8Encoding]::new($false)
            )
            return [ordered]@{ expected = $expected; path = $receiptPath }
        }

        $nullExitReceipt = New-ForgedCompleteReceipt `
            -Scenario "null-exit-self-test" `
            -ExitCode $null `
            -ProcessSuccess $false `
            -WorkspaceCleanup ([ordered]@{ status = "no_workspace"; verified_absent = $true; remaining_workspace_count = 0 })
        $nullExitBindings = @(Get-FailureDiagnosticBindings -ExpectedCommand $nullExitReceipt.expected)
        if (@($nullExitBindings | Where-Object { $_.binding_status -ceq "BOUND" }).Count -ne 0) {
            throw "diagnostics self-test accepted a null process exit code as BOUND"
        }

        $failedCleanupReceipt = New-ForgedCompleteReceipt `
            -Scenario "failed-cleanup-self-test" `
            -ExitCode 1 `
            -ProcessSuccess $false `
            -WorkspaceCleanup ([ordered]@{ status = "failed"; verified_absent = $false; remaining_workspace_count = 1 })
        $failedCleanupBindings = @(Get-FailureDiagnosticBindings -ExpectedCommand $failedCleanupReceipt.expected)
        if (@($failedCleanupBindings | Where-Object { $_.binding_status -ceq "BOUND" }).Count -ne 0) {
            throw "diagnostics self-test accepted failed workspace cleanup as BOUND"
        }

        $stringExitReceipt = New-ForgedCompleteReceipt `
            -Scenario "string-exit-self-test" `
            -ExitCode "not-an-exit-code" `
            -ProcessSuccess $false `
            -WorkspaceCleanup ([ordered]@{ status = "no_workspace"; verified_absent = $true; remaining_workspace_count = [int64]0 })
        if (@(Get-FailureDiagnosticBindings -ExpectedCommand $stringExitReceipt.expected | Where-Object { $_.binding_status -ceq "BOUND" }).Count -ne 0) {
            throw "diagnostics self-test accepted a string process exit code as BOUND"
        }

        $stringCountReceipt = New-ForgedCompleteReceipt `
            -Scenario "string-count-self-test" `
            -ExitCode 1 `
            -ProcessSuccess $false `
            -WorkspaceCleanup ([ordered]@{ status = "no_workspace"; verified_absent = $true; remaining_workspace_count = "0" })
        if (@(Get-FailureDiagnosticBindings -ExpectedCommand $stringCountReceipt.expected | Where-Object { $_.binding_status -ceq "BOUND" }).Count -ne 0) {
            throw "diagnostics self-test accepted a string workspace count as BOUND"
        }

        $missingCountReceipt = New-ForgedCompleteReceipt `
            -Scenario "missing-count-self-test" `
            -ExitCode 1 `
            -ProcessSuccess $false `
            -WorkspaceCleanup ([ordered]@{ status = "no_workspace"; verified_absent = $true })
        if (@(Get-FailureDiagnosticBindings -ExpectedCommand $missingCountReceipt.expected | Where-Object { $_.binding_status -ceq "BOUND" }).Count -ne 0) {
            throw "diagnostics self-test accepted a missing workspace count as BOUND"
        }

        $mixedStdoutReceipt = New-ForgedCompleteReceipt `
            -Scenario "mixed-cleanup-stdout-self-test" `
            -ExitCode 1 `
            -ProcessSuccess $false `
            -WorkspaceCleanup ([ordered]@{}) `
            -CleanupStdout "1`n0`n"
        if (@(Get-FailureDiagnosticBindings -ExpectedCommand $mixedStdoutReceipt.expected | Where-Object { $_.binding_status -ceq "BOUND" }).Count -ne 0) {
            throw "diagnostics self-test accepted mixed cleanup stdout as exact scalar zero"
        }

        # Counterfactual 2: kill the writer through the same Job runner as Cargo. Recovery runs only
        # after Run() confirms timeout termination and Job drain, so the final log line is stable.
        if (-not ("Mt045JobRunner" -as [type])) {
            Add-Type -Language CSharp -TypeDefinition $mt045JobRunnerSource
        }
        $forcedTestName = "forced-termination-self-test"
        $selfTestWorkspaceId = "wp012-job-cleanup-$([guid]::NewGuid().ToString('N'))"
        $insertResult = [Mt045JobRunner]::Run(
            $script:mt045PsqlPath,
            [string[]]@(
                "--no-psqlrc", "--no-password", "--set", "ON_ERROR_STOP=1", "--dbname", $PostgresDsn,
                "--command", "INSERT INTO workspaces (id, name) VALUES ('$selfTestWorkspaceId', 'MT045 Job cleanup self-test');"
            ),
            $selfTestRoot,
            (Join-Path $selfTestRoot "workspace-insert.stdout.log"),
            (Join-Path $selfTestRoot "workspace-insert.stderr.log"),
            30000,
            3000
        )
        if ($insertResult.TimedOut -or $insertResult.LeakedProcessCount -ne 0 -or $insertResult.ExitCode -ne 0) {
            throw "diagnostics self-test could not insert exact post-Job cleanup workspace"
        }
        $runtimeLeaf = Join-Path $backendRuntimeRunRoot "$forcedTestName\runtime-001"
        $childScript = @"
`$leaf = '$($runtimeLeaf.Replace("'", "''"))'
[void][IO.Directory]::CreateDirectory(`$leaf)
@{ schema_id = 'handshake.backend-listen-report.v1'; pid = `$PID; listen_addr = '127.0.0.1:1' } | ConvertTo-Json -Compress | Set-Content -LiteralPath (Join-Path `$leaf 'listen-report.json') -Encoding utf8
@{ schema_id = 'hsk.wp_kernel_012.mt045_workspace_identity@1'; run_id = '$RunId'; scenario_identity = '$forcedTestName'; workspace_id = '$selfTestWorkspaceId'; owned_backend_pid = `$PID } | ConvertTo-Json -Compress | Set-Content -LiteralPath (Join-Path `$leaf 'workspace-identity.json') -Encoding utf8
[IO.File]::WriteAllText((Join-Path `$leaf 'backend.stdout.log'), "before-reap``nlast-line-before-reap``n")
[IO.File]::WriteAllText((Join-Path `$leaf 'backend.stderr.log'), "stderr-last-line-before-reap``n")
Start-Sleep -Seconds 30
"@
        $encodedChild = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($childScript))
        $childStdout = Join-Path $selfTestRoot "job.stdout.log"
        $childStderr = Join-Path $selfTestRoot "job.stderr.log"
        $jobResult = [Mt045JobRunner]::Run(
            (Get-Command powershell.exe -CommandType Application -ErrorAction Stop).Source,
            [string[]]@("-NoLogo", "-NoProfile", "-NonInteractive", "-EncodedCommand", $encodedChild),
            $repoRoot,
            $childStdout,
            $childStderr,
            1000,
            1000
        )
        if (-not $jobResult.TimedOut -or $jobResult.LeakedProcessCount -ne 0) {
            throw "diagnostics self-test Job runner did not prove timeout termination and drain"
        }
        $expectedForced = [ordered]@{
            label = $forcedTestName
            test_binary = "test_perf_large_knowledge"
            test_name = $forcedTestName
            timed_out = $true
            runner_error = $null
            process_containment = "windows_job_object_kill_on_close"
            job_drain_confirmed = $true
            leaked_process_count = 0
            root_process_id = $jobResult.RootProcessId
            exit_code = $jobResult.ExitCode
        }
        $recovered = Get-PostReapRuntimeBinding -ExpectedCommand $expectedForced
        if ($recovered.binding_status -cne "BOUND") {
            throw "forced-termination recovery was not BOUND: $($recovered.binding_error)"
        }
        if (
            $recovered.workspace_cleanup.status -cne "deleted_and_verified_absent" -or
            $recovered.workspace_cleanup.workspace_id -cne $selfTestWorkspaceId -or
            -not (Test-IsJsonInteger $recovered.workspace_cleanup.remaining_workspace_count) -or
            [int64]$recovered.workspace_cleanup.remaining_workspace_count -ne 0 -or
            (Get-Content -LiteralPath $recovered.workspace_cleanup.stdout_path -Raw).Trim() -cne "0" -or
            (Get-FileSha256 -Path $recovered.workspace_cleanup.proof_path) -cne $recovered.workspace_cleanup.proof_sha256
        ) {
            throw "forced-termination recovery did not prove exact post-Job workspace cleanup"
        }
        $workspaceMarkerPath = Join-Path $runtimeLeaf "workspace-identity.json"
        $workspaceMarkerBytes = [IO.File]::ReadAllBytes($workspaceMarkerPath)
        [IO.File]::Delete($workspaceMarkerPath)
        if ((Get-PostReapRuntimeBinding -ExpectedCommand $expectedForced).binding_status -cne "INVALID") {
            throw "missing workspace identity marker was incorrectly accepted as no_workspace"
        }
        [IO.File]::WriteAllBytes($workspaceMarkerPath, $workspaceMarkerBytes)
        $stdoutBinding = @($recovered.retained_files | Where-Object { $_.name -ceq "backend.stdout.log" })
        if (
            $stdoutBinding.Count -ne 1 -or
            -not ([IO.File]::ReadAllText($stdoutBinding[0].path).Contains("last-line-before-reap")) -or
            (Get-FileSha256 -Path $stdoutBinding[0].path) -cne $stdoutBinding[0].sha256
        ) {
            throw "forced-termination recovery did not retain and hash the stable final log line"
        }

        $unknownContainment = [ordered]@{} + $expectedForced
        $unknownContainment.process_containment = "unknown_runner_failed_before_confirmation"
        if ((Get-PostReapRuntimeBinding -ExpectedCommand $unknownContainment).binding_status -cne "INVALID") {
            throw "unknown process containment was incorrectly accepted for post-reap recovery"
        }
        $leakedContainment = [ordered]@{} + $expectedForced
        $leakedContainment.leaked_process_count = 1
        if ((Get-PostReapRuntimeBinding -ExpectedCommand $leakedContainment).binding_status -cne "INVALID") {
            throw "leaked Job descendants were incorrectly accepted for post-reap recovery"
        }
        $ambiguousRuntime = Join-Path $backendRuntimeRunRoot "$forcedTestName\runtime-002"
        [void][IO.Directory]::CreateDirectory($ambiguousRuntime)
        foreach ($name in @("listen-report.json", "backend.stdout.log", "backend.stderr.log")) {
            [IO.File]::Copy((Join-Path $runtimeLeaf $name), (Join-Path $ambiguousRuntime $name))
        }
        if ((Get-PostReapRuntimeBinding -ExpectedCommand $expectedForced).binding_status -cne "INVALID") {
            throw "multiple complete runtime candidates were incorrectly resolved by newest timestamp"
        }
        [IO.Directory]::Delete($ambiguousRuntime, $true)

        # Counterfactual 3: a present non-directory Rust receipt root is an explicit INVALID state;
        # it must not be treated as absent and upgraded by the post-Job runtime fallback.
        $originalFailureRoot = $failureDiagnosticsRoot
        $wrongTypeFailureRoot = Join-Path $selfTestRoot "receipt-root-is-a-file"
        [IO.File]::WriteAllText($wrongTypeFailureRoot, "not-a-directory", [Text.UTF8Encoding]::new($false))
        $failureDiagnosticsRoot = $wrongTypeFailureRoot
        $wrongTypeBindings = @(Get-FailureDiagnosticBindings -ExpectedCommand $expectedForced)
        $failureDiagnosticsRoot = $originalFailureRoot
        if (
            $wrongTypeBindings.Count -eq 0 -or
            @($wrongTypeBindings | Where-Object { $_.binding_status -ceq "BOUND" }).Count -ne 0
        ) {
            throw "present non-directory failure diagnostics root was upgraded through fallback"
        }

        # Counterfactual 4: an inaccessible/enumeration failure is INVALID and cannot enable fallback.
        $script:injectMt045ReceiptEnumerationFailure = $true
        try {
            $inaccessibleBindings = @(Get-FailureDiagnosticBindings -ExpectedCommand $expectedForced)
        }
        finally {
            $script:injectMt045ReceiptEnumerationFailure = $false
        }
        if (
            $inaccessibleBindings.Count -eq 0 -or
            @($inaccessibleBindings | Where-Object { $_.binding_status -ceq "BOUND" }).Count -ne 0 -or
            @($inaccessibleBindings | Where-Object { $_.binding_source -ceq "receipt_enumeration" }).Count -ne 1
        ) {
            throw "injected inaccessible receipt enumeration did not remain INVALID without fallback"
        }

        # Counterfactual 5: absent receipt/runtime roots degrade nonthrowingly to INVALID.
        $originalRuntimeRoot = $backendRuntimeRunRoot
        $failureDiagnosticsRoot = Join-Path $selfTestRoot "absent-receipt-root"
        $backendRuntimeRunRoot = Join-Path $selfTestRoot "absent-runtime-root"
        $enumerationBindings = @(Get-FailureDiagnosticBindings -ExpectedCommand $expectedForced)
        $failureDiagnosticsRoot = $originalFailureRoot
        $backendRuntimeRunRoot = $originalRuntimeRoot
        if (
            $enumerationBindings.Count -eq 0 -or
            @($enumerationBindings | Where-Object { $_.binding_status -cne "INVALID" }).Count -ne 0
        ) {
            throw "enumeration-failure self-test did not degrade nonthrowingly to INVALID"
        }

        Write-Output ([ordered]@{
            status = "PASS"
            malformed_all_missing = "INVALID"
            null_process_exit = "INVALID"
            failed_workspace_cleanup = "INVALID"
            string_process_exit = "INVALID"
            string_workspace_count = "INVALID"
            missing_workspace_count = "INVALID"
            mixed_cleanup_stdout = "INVALID"
            forced_job_timed_out = $jobResult.TimedOut
            forced_job_leaked_process_count = $jobResult.LeakedProcessCount
            post_reap_binding = $recovered.binding_status
            post_reap_workspace_cleanup = $recovered.workspace_cleanup.status
            missing_workspace_marker = "INVALID"
            stable_last_line_hash_bound = $true
            unknown_containment = "INVALID"
            leaked_descendants = "INVALID"
            ambiguous_runtime_candidates = "INVALID"
            invalid_receipt_with_complete_runtime = "INVALID_NO_UPGRADE"
            wrong_type_receipt_root = "INVALID_NO_UPGRADE"
            inaccessible_receipt_enumeration = "INVALID_NO_UPGRADE"
            enumeration_failure = "INVALID_NONTHROWING"
        } | ConvertTo-Json -Depth 8)
    }
    finally {
        if (-not [string]::IsNullOrWhiteSpace($selfTestWorkspaceId) -and ("Mt045JobRunner" -as [type])) {
            $literal = $selfTestWorkspaceId.Replace("'", "''")
            $finalCleanup = [Mt045JobRunner]::Run(
                $script:mt045PsqlPath,
                [string[]]@(
                    "--no-psqlrc", "--no-password", "--set", "ON_ERROR_STOP=1", "--quiet",
                    "--tuples-only", "--no-align", "--dbname", $PostgresDsn,
                    "--command", "DELETE FROM workspaces WHERE id = '$literal'; SELECT COUNT(*) FROM workspaces WHERE id = '$literal';"
                ),
                $selfTestRoot,
                (Join-Path $selfTestRoot "workspace-final-cleanup.stdout.log"),
                (Join-Path $selfTestRoot "workspace-final-cleanup.stderr.log"),
                30000,
                3000
            )
            if (
                $finalCleanup.TimedOut -or $finalCleanup.LeakedProcessCount -ne 0 -or
                $finalCleanup.ExitCode -ne 0 -or
                (Get-Content -LiteralPath (Join-Path $selfTestRoot "workspace-final-cleanup.stdout.log") -Raw).Trim() -cne "0"
            ) {
                throw "diagnostics self-test final exact workspace cleanup failed"
            }
        }
        foreach ($path in @($selfTestRoot, $failureDiagnosticsRoot, $backendRuntimeRunRoot, $runRoot)) {
            $fullPath = [IO.Path]::GetFullPath($path)
            if (
                $fullPath.StartsWith($artifactPrefix, [StringComparison]::OrdinalIgnoreCase) -and
                (Test-Path -LiteralPath $fullPath)
            ) {
                [IO.Directory]::Delete($fullPath, $true)
            }
        }
    }
    exit 0
}

function New-SupervisorProjection {
    param(
        [Parameter(Mandatory)][ValidateSet("RUNNING", "FAIL")][string]$Status,
        [string]$Reason,
        [object[]]$FailureDiagnostics = @()
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
        completed_commands = @($commands)
        failed_command = $script:lastFailedCommandReceipt
        failure_diagnostics = @($FailureDiagnostics)
        started_at = $supervisorStartedAt.ToString("O")
        updated_at = [DateTimeOffset]::UtcNow.ToString("O")
    }
}

function Set-ManifestTerminalState {
    param(
        [Parameter(Mandatory)][ValidateSet("RUNNING", "FAIL")][string]$Status
    )
    $parsedRows = $headManifestJson | ConvertFrom-Json
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
$script:lastFailedCommandReceipt = $null
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
    $env:HSK_PSQL_BIN = $script:mt045PsqlPath
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
    $backendSha256 = Get-FileSha256 -Path $env:HSK_TEST_BACKEND_BIN

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
        Assert-ExactTestResult -CommandResult $result -ExpectedTest $test
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
        $result = Invoke-BoundedCargo -Label $test -Arguments @(
            "test", "--release", "--locked", "--target-dir", $targetRoot,
            "--test", $bin, $test, "--", "--exact", "--nocapture", "--test-threads=1"
        ) -WorkingDirectory $crateRoot -LogRoot $runRoot
        Assert-ExactTestResult -CommandResult $result -ExpectedTest $test
        $commands.Add($result)
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
    $immutableDigest = Get-FileSha256 -Path $immutableRunPath
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
    $backendFinalSha256 = Get-FileSha256 -Path $env:HSK_TEST_BACKEND_BIN
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
    $failureRecord = $_
    $failure = $failureRecord.Exception.Message
    # Publish terminal state before any diagnostic enumeration. A scanner/reparse/hash failure may
    # degrade enrichment, but it can never strand the canonical projections at RUNNING.
    $failedProjection = New-SupervisorProjection -Status "FAIL" -Reason $failure -FailureDiagnostics @()
    Write-JsonAtomic -Path $supervisorCurrentPath -Value $failedProjection
    Write-JsonAtomic -Path $currentRunPath -Value $failedProjection
    Write-JsonAtomic -Path $latestRunPath -Value $failedProjection
    Set-ManifestTerminalState -Status "FAIL"

    $diagnosticBindings = try {
        @(Get-FailureDiagnosticBindings -ExpectedCommand $script:lastFailedCommandReceipt)
    }
    catch {
        @([ordered]@{
            binding_status = "INVALID"
            binding_source = "nonthrowing_failure_enrichment"
            receipt = $failureDiagnosticsRoot
            binding_error = $_.Exception.Message
        })
    }
    $enrichedProjection = New-SupervisorProjection -Status "FAIL" -Reason $failure -FailureDiagnostics $diagnosticBindings
    try {
        Write-JsonAtomic -Path $supervisorCurrentPath -Value $enrichedProjection
        Write-JsonAtomic -Path $currentRunPath -Value $enrichedProjection
        Write-JsonAtomic -Path $latestRunPath -Value $enrichedProjection
    }
    catch {
        Write-Warning "MT-045 terminal FAIL was published, but diagnostic enrichment publication failed: $($_.Exception.Message)"
    }
    throw $failureRecord
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
