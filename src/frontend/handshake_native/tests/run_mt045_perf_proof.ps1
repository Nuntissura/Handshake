[CmdletBinding()]
param(
    [string]$RunId = ("MT045-RUN-" + [guid]::NewGuid().ToString("N")),
    [ValidateRange(300, 1800)]
    [int]$CommandTimeoutSeconds = 1800,
    [switch]$DiagnosticsSelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$script:mt045DescendantExitGraceMilliseconds = 15000
$script:mt045SelfTestFixtureTimeoutMilliseconds = 15000
$mt045JobRunnerExpectedSourceId = "mt045-job-runner-20260802-v7"
$mt045JobRunnerSource = @'
using System;
using System.Collections.Generic;
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
    public ulong[] LeakedProcessIds { get; set; }
    public ulong[] PreCleanupDescendantProcessIds { get; set; }
    public ulong[] PostDrainDescendantProcessIds { get; set; }
}

public static class Mt045JobRunner
{
    public const string SourceId = "mt045-job-runner-20260802-v7";
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
    private const int JobObjectBasicProcessIdList = 3;
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

    [DllImport("kernel32.dll", EntryPoint = "QueryInformationJobObject", SetLastError = true)]
    private static extern bool QueryInformationJobObjectRaw(
        IntPtr job,
        int informationClass,
        IntPtr information,
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

    private static ulong[] QueryProcessIds(IntPtr job)
    {
        const int capacity = 4096;
        var headerSize = sizeof(uint) * 2;
        var bufferSize = headerSize + (IntPtr.Size * capacity);
        var buffer = Marshal.AllocHGlobal(bufferSize);
        try
        {
            if (!QueryInformationJobObjectRaw(
                job,
                JobObjectBasicProcessIdList,
                buffer,
                (uint)bufferSize,
                IntPtr.Zero))
            {
                ThrowLastWin32("QueryInformationJobObject process-id list failed");
            }
            var assigned = unchecked((uint)Marshal.ReadInt32(buffer, 0));
            var returned = unchecked((uint)Marshal.ReadInt32(buffer, sizeof(uint)));
            if (returned < assigned)
            {
                throw new InvalidOperationException(
                    "Job Object process-id list exceeded the fixed diagnostic capacity");
            }
            var processIds = new ulong[returned];
            for (var index = 0; index < returned; index++)
            {
                var offset = headerSize + (index * IntPtr.Size);
                processIds[index] = IntPtr.Size == sizeof(uint)
                    ? unchecked((uint)Marshal.ReadInt32(buffer, offset))
                    : unchecked((ulong)Marshal.ReadInt64(buffer, offset));
            }
            return processIds;
        }
        finally
        {
            Marshal.FreeHGlobal(buffer);
        }
    }

    private static ulong[] QueryDescendantProcessIds(IntPtr job, uint rootProcessId)
    {
        var descendants = new List<ulong>();
        foreach (var processId in QueryProcessIds(job))
        {
            if (processId != rootProcessId)
            {
                descendants.Add(processId);
            }
        }
        return descendants.ToArray();
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
            var preCleanupDescendantProcessIds = new ulong[0];
            var postDrainDescendantProcessIds = new ulong[0];
            if (timedOut)
            {
                preCleanupDescendantProcessIds = QueryDescendantProcessIds(
                    job,
                    unchecked((uint)processInformation.dwProcessId));
                TerminateJobAndDrain(job, "Timeout cleanup");
                WaitForSingleObject(processInformation.hProcess, INFINITE);
                postDrainDescendantProcessIds = QueryDescendantProcessIds(
                    job,
                    unchecked((uint)processInformation.dwProcessId));
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

            ulong[] leakedProcessIds = new ulong[0];
            if (!timedOut)
            {
                var descendantTimer = Stopwatch.StartNew();
                while (true)
                {
                    leakedProcessIds = QueryDescendantProcessIds(
                        job,
                        unchecked((uint)processInformation.dwProcessId));
                    if (
                        leakedProcessIds.Length == 0 ||
                        descendantTimer.ElapsedMilliseconds >= descendantExitGraceMilliseconds)
                    {
                        break;
                    }
                    Thread.Sleep(100);
                }
                if (leakedProcessIds.Length != 0)
                {
                    preCleanupDescendantProcessIds = leakedProcessIds;
                    TerminateJobAndDrain(job, "Descendant-leak cleanup");
                    postDrainDescendantProcessIds = QueryDescendantProcessIds(
                        job,
                        unchecked((uint)processInformation.dwProcessId));
                }
            }

            var result = new Mt045JobRunResult
            {
                RootProcessId = processInformation.dwProcessId,
                ExitCode = unchecked((int)exitCode),
                TimedOut = timedOut,
                LeakedProcessCount = unchecked((uint)leakedProcessIds.Length),
                LeakedProcessIds = leakedProcessIds,
                PreCleanupDescendantProcessIds = preCleanupDescendantProcessIds,
                PostDrainDescendantProcessIds = postDrainDescendantProcessIds
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
                $script:mt045DescendantExitGraceMilliseconds
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
            leaked_process_ids = $null
            pre_cleanup_descendant_process_ids = $null
            post_drain_descendant_process_ids = $null
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
        leaked_process_ids = @($nativeResult.LeakedProcessIds)
        pre_cleanup_descendant_process_ids = @($nativeResult.PreCleanupDescendantProcessIds)
        post_drain_descendant_process_ids = @($nativeResult.PostDrainDescendantProcessIds)
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
$requiredArtifactRootPath = [IO.Path]::GetFullPath((Join-Path (Split-Path $repoRoot -Parent) "Handshake_Artifacts"))
$configuredArtifactRoot = [Environment]::GetEnvironmentVariable("HANDSHAKE_ARTIFACTS_ROOT")
$artifactRootPath = if ([string]::IsNullOrWhiteSpace($configuredArtifactRoot)) {
    $requiredArtifactRootPath
}
else {
    [IO.Path]::GetFullPath($configuredArtifactRoot)
}
if (-not (Test-Path -LiteralPath $artifactRootPath -PathType Container)) {
    throw "The existing sibling Handshake_Artifacts root is required; this supervisor will not create it: $artifactRootPath"
}
$artifactRoot = (Resolve-Path -LiteralPath $artifactRootPath).Path
$requiredArtifactRoot = (Resolve-Path -LiteralPath $requiredArtifactRootPath).Path
if (-not $artifactRoot.Equals($requiredArtifactRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw "HANDSHAKE_ARTIFACTS_ROOT must resolve to the worktree-level canonical root $requiredArtifactRoot, got $artifactRoot"
}
if ((Split-Path $artifactRoot -Leaf) -cne "Handshake_Artifacts") {
    throw "Resolved artifact root is not the canonical Handshake_Artifacts directory: $artifactRoot"
}
function Get-Mt045CompactRuntimeComponent {
    param(
        [Parameter(Mandatory)][ValidateSet("cargo", "r", "s")][string]$Prefix,
        [Parameter(Mandatory)][string]$Value
    )
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
        $hashBytes = $hasher.ComputeHash([Text.Encoding]::UTF8.GetBytes($Value))
    }
    finally {
        $hasher.Dispose()
    }
    $hash = -join ($hashBytes | ForEach-Object { $_.ToString("x2") })
    return "$Prefix-$($hash.Substring(0, 16))"
}

$targetOwnerKey = Get-Mt045CompactRuntimeComponent -Prefix "cargo" -Value $RunId
$targetParentRoot = $artifactRoot
$targetRoot = [IO.Path]::GetFullPath((Join-Path $targetParentRoot $targetOwnerKey))
$targetPrefix = $targetParentRoot.TrimEnd("\") + "\"
if (-not $targetRoot.StartsWith($targetPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Resolved owner-scoped Cargo target escaped the canonical target parent: $targetRoot"
}
# Pinned libduckdb-sys 1.4.3 adds up to 141 characters below the target root;
# this cap keeps its longest bundled header path at 251 characters or fewer.
if ($targetRoot.Length -gt 110) {
    throw "Owner-scoped Cargo target is too long for bundled native dependencies on Windows ($($targetRoot.Length) characters; maximum 110): $targetRoot"
}
$supervisorRoot = [IO.Path]::GetFullPath((Join-Path $artifactRoot "wp-kernel-012\mt-045\supervisor"))
$runRoot = [IO.Path]::GetFullPath((Join-Path $supervisorRoot $RunId))
$artifactPrefix = $artifactRoot.TrimEnd("\") + "\"
if (-not $runRoot.StartsWith($artifactPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Resolved run path escaped the existing Handshake_Artifacts root: $runRoot"
}
if (Test-Path -LiteralPath $runRoot) {
    throw "Supervisor run id already exists: $RunId"
}
if (Test-Path -LiteralPath $targetRoot) {
    throw "Owner-scoped Cargo target already exists for run id ${RunId}: $targetRoot"
}

$sourcePaths = @(
    ".cargo/config.toml",
    "rust-toolchain.toml",
    "src/backend/handshake_core/build.rs",
    "src/backend/handshake_core/Cargo.toml",
    "src/backend/handshake_core/Cargo.lock",
    "src/backend/handshake_core/mechanical_engines.json",
    "src/backend/handshake_core/src",
    "src/backend/handshake_core/schemas",
    "src/frontend/palmistry",
    "src/frontend/handshake_native/build.rs",
    "src/frontend/handshake_native/Cargo.toml",
    "src/frontend/handshake_native/Cargo.lock",
    "src/frontend/handshake_native/.cargo/config.toml",
    "src/frontend/handshake_native/diag_ring",
    "src/frontend/handshake_native/src",
    "src/frontend/handshake_native/tests/perf_proof_support/mod.rs",
    "src/frontend/handshake_native/tests/backend_proof_support/mod.rs",
    "src/frontend/handshake_native/tests/test_heartbeat.rs",
    "src/frontend/handshake_native/tests/test_diagnostics_panel.rs",
    "src/frontend/handshake_native/tests/test_perf_large_code.rs",
    "src/frontend/handshake_native/tests/test_perf_large_rich.rs",
    "src/frontend/handshake_native/tests/test_perf_large_knowledge.rs",
    "src/frontend/handshake_native/tests/run_mt045_perf_proof.ps1"
)
$manifestRepoPath = "src/frontend/handshake_native/tests/perf_proof/perf_manifest.json"
$manifestPath = Join-Path $repoRoot $manifestRepoPath
$measurementRoot = Join-Path $artifactRoot "wp-kernel-012\mt-045\measurements"
$supervisorCurrentPath = Join-Path $measurementRoot "supervisor-current.json"
$currentRunPath = Join-Path $measurementRoot "current-run.json"
$latestRunPath = Join-Path $measurementRoot "latest-run-summary.json"
$failureDiagnosticsRoot = Join-Path $artifactRoot "wp-kernel-012\mt-045\failure-diagnostics\$RunId"
$backendRuntimeRunKey = Get-Mt045CompactRuntimeComponent -Prefix "r" -Value $RunId
$backendRuntimeRunRoot = Join-Path $artifactRoot "wp-kernel-012\backend-runtime\$backendRuntimeRunKey"

function Get-Mt045RuntimeScenarioRoot {
    param([Parameter(Mandatory)][string]$ScenarioName)
    return Join-Path $backendRuntimeRunRoot (Get-Mt045CompactRuntimeComponent -Prefix "s" -Value $ScenarioName)
}
$expectedScenarioIds = @(
    "LC-01", "LC-02", "LC-03", "LC-04", "LC-05", "LC-06", "LC-07", "LC-08",
    "LR-01", "LR-02", "LR-03", "LR-04", "LR-05", "LR-06", "LR-07",
    "LK-01", "LK-02", "LK-03", "LK-04", "LK-05"
)
[void][IO.Directory]::CreateDirectory($runRoot)
[void][IO.Directory]::CreateDirectory($measurementRoot)
$sourceSha = $null
$headManifestJson = $null
$preflightStartedAt = [DateTimeOffset]::UtcNow
try {
    $sourceSha = Invoke-GitText -Repository $repoRoot -Arguments @("rev-parse", "HEAD")
    $initialManifestGitObject = Invoke-GitText -Repository $repoRoot -Arguments @(
        "rev-parse", "${sourceSha}:$manifestRepoPath"
    )
    $headManifestJson = Invoke-GitText -Repository $repoRoot -Arguments @(
        "show", "${sourceSha}:$manifestRepoPath"
    )
    $initialManifestSha256 = Get-StringSha256 -Value $headManifestJson
    Assert-SourceBindingClean -Repository $repoRoot -Paths $sourcePaths

    if (-not $DiagnosticsSelfTest) {
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
    }
}
catch {
    $preflightFailure = $_
    $preflightProjection = [ordered]@{
        schema_id = "hsk.wp_kernel_012.mt045_supervisor_projection@1"
        work_packet_id = "WP-KERNEL-012"
        micro_task_id = "MT-045"
        run_id = $RunId
        source_sha = $sourceSha
        status = "FAIL"
        supervisor_preflight = $false
        terminal_reason = $preflightFailure.Exception.Message
        scenarios = [ordered]@{}
        completed_commands = @()
        failed_command = $null
        failure_diagnostics = @()
        cargo_target_owner_key = $targetOwnerKey
        canonical_target_root = $targetRoot
        target_cleanup = [ordered]@{ status = "not_created"; path = $targetRoot }
        started_at = $preflightStartedAt.ToString("O")
        updated_at = [DateTimeOffset]::UtcNow.ToString("O")
    }
    Write-JsonAtomic -Path $supervisorCurrentPath -Value $preflightProjection
    Write-JsonAtomic -Path $currentRunPath -Value $preflightProjection
    Write-JsonAtomic -Path $latestRunPath -Value $preflightProjection
    if (-not [string]::IsNullOrWhiteSpace([string]$headManifestJson)) {
        $preflightManifest = @($headManifestJson | ConvertFrom-Json)
        foreach ($row in $preflightManifest) {
            $row.status = "FAIL"
            $row.measured_value = $null
            $row.measured_profile = "release"
            $row.gated = $false
            $row.suite_run_id = $RunId
            $row.override_applied = $false
            $row.effective_budget = if ($null -ne $row.budget_ms) { $row.budget_ms } else { $row.budget_mb }
        }
        Write-JsonAtomic -Path $manifestPath -Value $preflightManifest
    }
    throw $preflightFailure
}

function Get-Mt045ComparablePath {
    param([Parameter(Mandatory)][string]$Path)
    if ($Path.StartsWith('\\?\UNC\', [StringComparison]::OrdinalIgnoreCase)) {
        throw "extended UNC paths are forbidden in MT-045 evidence: $Path"
    }
    if ($Path.StartsWith('\\?\', [StringComparison]::Ordinal)) {
        $Path = $Path.Substring(4)
    }
    return [IO.Path]::GetFullPath($Path)
}

function Assert-NoReparsePath {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Boundary
    )
    $fullPath = Get-Mt045ComparablePath -Path $Path
    $fullBoundary = (Get-Mt045ComparablePath -Path $Boundary).TrimEnd("\")
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

function Remove-Mt045OwnerTarget {
    if (-not (Test-Path -LiteralPath $targetRoot)) {
        return [ordered]@{
            status = "not_present"
            path = $targetRoot
            removed_at = [DateTimeOffset]::UtcNow.ToString("O")
        }
    }
    $resolvedTarget = Get-Mt045ComparablePath -Path (Resolve-Path -LiteralPath $targetRoot).Path
    $resolvedParent = (Get-Mt045ComparablePath -Path $targetParentRoot).TrimEnd("\")
    $resolvedPrefix = $resolvedParent + "\"
    if (
        -not $resolvedTarget.StartsWith($resolvedPrefix, [StringComparison]::OrdinalIgnoreCase) -or
        (Split-Path $resolvedTarget -Leaf) -cne $targetOwnerKey
    ) {
        throw "refusing to clean owner target outside its exact run-scoped boundary: $resolvedTarget"
    }
    Assert-NoReparsePath -Path $resolvedTarget -Boundary $resolvedParent
    [IO.Directory]::Delete($resolvedTarget, $true)
    if (Test-Path -LiteralPath $resolvedTarget) {
        throw "owner-scoped Cargo target survived cleanup: $resolvedTarget"
    }
    return [ordered]@{
        status = "deleted_and_verified_absent"
        path = $resolvedTarget
        removed_at = [DateTimeOffset]::UtcNow.ToString("O")
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
    # A reaped embedded backend cannot be queried through a second process. Its equivalent residue
    # proof is the same one emitted by backend_proof_support: bind the exact HANDSHAKE_DATA_DIR and
    # handshake-surreal store to this fixture-owned runtime root. The retained failure directory is
    # diagnostic evidence only and is unreachable by every later run because each run/scenario gets a
    # fresh UUID root.
    $dataDirectory = [IO.Path]::GetFullPath((Join-Path $RuntimeDirectory "data"))
    $storePath = [IO.Path]::GetFullPath((Join-Path $dataDirectory "handshake-surreal"))
    $runtimePrefix = [IO.Path]::GetFullPath($RuntimeDirectory).TrimEnd("\") + "\"
    if (-not $dataDirectory.StartsWith($runtimePrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "embedded store data directory escaped the fixture-owned runtime root"
    }
    if (-not (Test-Path -LiteralPath $dataDirectory -PathType Container)) {
        throw "embedded store data directory is absent after owned backend reap: $dataDirectory"
    }
    Assert-NoReparsePath -Path $dataDirectory -Boundary $RuntimeDirectory
    if (-not (Test-Path -LiteralPath $storePath -PathType Container)) {
        throw "embedded SurrealDB store is absent after workspace creation: $storePath"
    }
    Assert-NoReparsePath -Path $storePath -Boundary $RuntimeDirectory
    $proofPath = Join-Path $RuntimeDirectory "workspace-cleanup.json"
    $proof = [ordered]@{
        schema_id = "hsk.wp_kernel_012.mt045_workspace_cleanup@1"
        run_id = $RunId
        scenario_identity = $ExpectedCommand.test_name
        workspace_id = $workspaceId
        status = "contained_in_owned_embedded_store"
        containment_verified = $true
        residue_scope = "fixture_owned_data_directory_only"
        runtime_root = [IO.Path]::GetFullPath($RuntimeDirectory)
        owned_runtime_roots = @([IO.Path]::GetFullPath($RuntimeDirectory))
        data_dir = $dataDirectory
        data_dir_inside_runtime_root = $true
        store_path = $storePath
        store_path_bound_to_owned_runtime_root = $storePath
        store_path_present = $true
        marker_path = $markerItem.FullName
        marker_bytes = $markerItem.Length
        marker_sha256 = Get-FileSha256 -Path $markerItem.FullName
        owned_backend_pid = $OwnedBackendPid
    }
    Write-JsonAtomic -Path $proofPath -Value $proof
    return [ordered]@{
        status = "contained_in_owned_embedded_store"
        workspace_id = $workspaceId
        containment_verified = $true
        residue_scope = $proof.residue_scope
        runtime_root = $proof.runtime_root
        owned_runtime_roots = $proof.owned_runtime_roots
        data_dir = $dataDirectory
        data_dir_inside_runtime_root = $true
        store_path = $storePath
        store_path_bound_to_owned_runtime_root = $storePath
        store_path_present = $true
        proof_path = $proofPath
        proof_bytes = (Get-Item -LiteralPath $proofPath -ErrorAction Stop).Length
        proof_sha256 = Get-FileSha256 -Path $proofPath
        marker_path = $markerItem.FullName
        marker_bytes = $markerItem.Length
        marker_sha256 = $proof.marker_sha256
    }
}

function Get-ValidatedProcessIdArray {
    param(
        [Parameter(Mandatory)]$Owner,
        [Parameter(Mandatory)][string]$PropertyName
    )
    $propertyExists = $false
    $propertyValue = $null
    if ($Owner -is [Collections.IDictionary]) {
        $propertyExists = $Owner.Contains($PropertyName)
        if ($propertyExists) {
            $propertyValue = $Owner[$PropertyName]
        }
    }
    else {
        $property = $Owner.PSObject.Properties[$PropertyName]
        $propertyExists = $null -ne $property
        if ($propertyExists) {
            $propertyValue = $property.Value
        }
    }
    if (-not $propertyExists -or $null -eq $propertyValue) {
        throw "process-containment receipt lacks $PropertyName"
    }
    if ($propertyValue -is [string]) {
        throw "process-containment receipt $PropertyName must be an array"
    }
    $validated = @()
    foreach ($value in @($propertyValue)) {
        if (-not (Test-IsJsonInteger $value) -or [uint64]$value -eq 0) {
            throw "process-containment receipt $PropertyName contains an invalid PID"
        }
        $processId = [uint64]$value
        if ($validated -contains $processId) {
            throw "process-containment receipt $PropertyName contains duplicate PID $processId"
        }
        $validated += $processId
    }
    return @($validated)
}

function Get-PostReapRuntimeBinding {
    param([AllowNull()]$ExpectedCommand)
    try {
        if ($null -eq $ExpectedCommand -or [string]::IsNullOrWhiteSpace([string]$ExpectedCommand.test_name)) {
            throw "failed command does not identify an exact test scenario"
        }
        $leakedProcessIds = @(Get-ValidatedProcessIdArray -Owner $ExpectedCommand -PropertyName "leaked_process_ids")
        $preCleanupDescendantProcessIds = @(
            Get-ValidatedProcessIdArray -Owner $ExpectedCommand -PropertyName "pre_cleanup_descendant_process_ids"
        )
        $postDrainDescendantProcessIds = @(
            Get-ValidatedProcessIdArray -Owner $ExpectedCommand -PropertyName "post_drain_descendant_process_ids"
        )
        if (
            $null -ne $ExpectedCommand.runner_error -or
            $ExpectedCommand.process_containment -cne "windows_job_object_kill_on_close" -or
            $ExpectedCommand.job_drain_confirmed -ne $true -or
            -not (Test-IsJsonInteger $ExpectedCommand.leaked_process_count) -or
            $ExpectedCommand.leaked_process_count -ne 0 -or
            [uint64]$ExpectedCommand.leaked_process_count -ne [uint64]$leakedProcessIds.Count -or
            $postDrainDescendantProcessIds.Count -ne 0 -or
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
        $scenarioRoot = Get-Mt045RuntimeScenarioRoot -ScenarioName ([string]$ExpectedCommand.test_name)
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
                leaked_process_ids = $leakedProcessIds
                pre_cleanup_descendant_process_ids = $preCleanupDescendantProcessIds
                post_drain_descendant_process_ids = $postDrainDescendantProcessIds
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
                    if ($workspaceCleanup.status -notin @("no_workspace", "contained_in_owned_embedded_store")) {
                        throw "failure diagnostic lacks embedded-store containment proof: $receiptPath"
                    }
                    if (
                        $workspaceCleanup.containment_verified -ne $true -or
                        $workspaceCleanup.data_dir_inside_runtime_root -ne $true -or
                        [string]::IsNullOrWhiteSpace([string]$workspaceCleanup.runtime_root) -or
                        [string]::IsNullOrWhiteSpace([string]$workspaceCleanup.data_dir) -or
                        [string]::IsNullOrWhiteSpace([string]$workspaceCleanup.store_path_bound_to_owned_runtime_root) -or
                        $workspaceCleanup.store_path_present -ne $true
                    ) {
                        throw "failure diagnostic embedded-store containment is incomplete: $receiptPath"
                    }
                    $claimedRuntimeRoot = Get-Mt045ComparablePath -Path ([string]$workspaceCleanup.runtime_root)
                    $claimedDataDirectory = Get-Mt045ComparablePath -Path ([string]$workspaceCleanup.data_dir)
                    $claimedStorePath = Get-Mt045ComparablePath -Path ([string]$workspaceCleanup.store_path_bound_to_owned_runtime_root)
                    $claimedRuntimePrefix = $claimedRuntimeRoot.TrimEnd("\") + "\"
                    $expectedStorePath = Get-Mt045ComparablePath -Path (Join-Path $claimedDataDirectory "handshake-surreal")
                    if (
                        -not $claimedDataDirectory.StartsWith($claimedRuntimePrefix, [StringComparison]::OrdinalIgnoreCase) -or
                        -not $claimedStorePath.Equals($expectedStorePath, [StringComparison]::OrdinalIgnoreCase)
                    ) {
                        throw "failure diagnostic embedded-store paths escaped their owned runtime root: $receiptPath"
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
                    $listenBinding = @($retainedFiles | Where-Object { $_.name -ceq "listen-report.json" })
                    if ($listenBinding.Count -ne 1) {
                        throw "failure diagnostic lacks one retained listen report: $receiptPath"
                    }
                    $listenReport = Get-Content -LiteralPath $listenBinding[0].path -Raw | ConvertFrom-Json
                    if (
                        $listenReport.schema_id -cne "handshake.backend-listen-report.v1" -or
                        -not (Test-IsJsonInteger $listenReport.pid) -or
                        [uint64]$listenReport.pid -ne [uint64]$receipt.process.pid -or
                        [string]::IsNullOrWhiteSpace([string]$listenReport.listen_addr)
                    ) {
                        throw "failure diagnostic listen report does not match its owned process: $receiptPath"
                    }
                    $expectedBaseUrl = "http://$($listenReport.listen_addr)"
                    if ($receipt.trigger -ceq "request_failure") {
                        $requestUri = $null
                        $healthUri = $null
                        $baseUri = [Uri]$expectedBaseUrl
                        $labelParts = @(([string]$receipt.label) -split " ", 2)
                        if (
                            $labelParts.Count -ne 2 -or
                            [string]::IsNullOrWhiteSpace([string]$labelParts[0]) -or
                            -not ([string]$labelParts[1]).StartsWith("/", [StringComparison]::Ordinal) -or
                            -not [Uri]::TryCreate([string]$receipt.reqwest_error.url, [UriKind]::Absolute, [ref]$requestUri) -or
                            -not [Uri]::TryCreate([string]$receipt.immediate_health.url, [UriKind]::Absolute, [ref]$healthUri) -or
                            $requestUri.Scheme -cne $baseUri.Scheme -or
                            $requestUri.Authority -cne $baseUri.Authority -or
                            $requestUri.PathAndQuery -cne [string]$labelParts[1] -or
                            $healthUri.AbsoluteUri -cne "$expectedBaseUrl/health"
                        ) {
                            throw "failure diagnostic request/health URLs do not match the retained listener and labeled request: $receiptPath"
                        }
                    }
                    $processExecutable = Get-Mt045ComparablePath -Path ([string]$receipt.process.executable_path)
                    $expectedExecutable = [IO.Path]::GetFullPath((Join-Path $targetRoot "release\handshake_core.exe"))
                    if (
                        -not $processExecutable.Equals($expectedExecutable, [StringComparison]::OrdinalIgnoreCase) -or
                        -not (Test-Path -LiteralPath $processExecutable -PathType Leaf) -or
                        [string]::IsNullOrWhiteSpace([string]$receipt.process.executable_sha256) -or
                        (Get-FileSha256 -Path $processExecutable) -cne [string]$receipt.process.executable_sha256
                    ) {
                        throw "failure diagnostic executable identity/hash does not match the rebuilt backend: $receiptPath"
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
                        workspace_cleanup = $workspaceCleanup
                        backend_identity = [ordered]@{
                            pid = [uint64]$listenReport.pid
                            listen_addr = [string]$listenReport.listen_addr
                            executable = $processExecutable
                            executable_sha256 = [string]$receipt.process.executable_sha256
                        }
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
    [void][IO.Directory]::CreateDirectory($targetRoot)
    $selfTestRoot = [IO.Path]::GetFullPath((Join-Path $artifactRoot "wp-kernel-012\mt-045\diagnostics-self-test\$RunId"))
    $selfTestWorkspaceId = $null
    $retainedFailureProofComplete = $false
    $realFailureProbeStarted = $false
    $canonicalFailureDiagnosticsRoot = $failureDiagnosticsRoot
    $failureDiagnosticsRoot = Join-Path $selfTestRoot "counterfactual-failure-diagnostics"
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
        $malformedRuntime = Join-Path (Get-Mt045RuntimeScenarioRoot -ScenarioName "malformed-test") "runtime-001"
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
        $powerShellExe = (Get-Command powershell.exe -CommandType Application -ErrorAction Stop).Source
        $normalJobResult = [Mt045JobRunner]::Run(
            $powerShellExe,
            [string[]]@("-NoLogo", "-NoProfile", "-NonInteractive", "-Command", "exit 0"),
            $selfTestRoot,
            (Join-Path $selfTestRoot "normal-job.stdout.log"),
            (Join-Path $selfTestRoot "normal-job.stderr.log"),
            10000,
            3000
        )
        if (
            $normalJobResult.TimedOut -or
            $normalJobResult.ExitCode -ne 0 -or
            $normalJobResult.LeakedProcessCount -ne 0 -or
            @($normalJobResult.LeakedProcessIds).Count -ne 0 -or
            @($normalJobResult.PreCleanupDescendantProcessIds).Count -ne 0 -or
            @($normalJobResult.PostDrainDescendantProcessIds).Count -ne 0
        ) {
            throw "diagnostics self-test rejected a normal root-only Job completion"
        }

        $newEncodedParent = {
            param(
                [Parameter(Mandatory)][int]$ChildSleepMilliseconds,
                [Parameter(Mandatory)][int]$ParentSleepMilliseconds
            )
            $childScript = "Start-Sleep -Milliseconds $ChildSleepMilliseconds"
            $encodedChildScript = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($childScript))
            $quotedPowerShell = $powerShellExe.Replace("'", "''")
            $parentScript = @"
Start-Process -FilePath '$quotedPowerShell' -ArgumentList @('-NoLogo','-NoProfile','-NonInteractive','-EncodedCommand','$encodedChildScript') -WindowStyle Hidden
Start-Sleep -Milliseconds $ParentSleepMilliseconds
"@
            return [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($parentScript))
        }

        $graceEncodedParent = & $newEncodedParent -ChildSleepMilliseconds 500 -ParentSleepMilliseconds 0
        $graceJobResult = [Mt045JobRunner]::Run(
            $powerShellExe,
            [string[]]@("-NoLogo", "-NoProfile", "-NonInteractive", "-EncodedCommand", $graceEncodedParent),
            $selfTestRoot,
            (Join-Path $selfTestRoot "grace-job.stdout.log"),
            (Join-Path $selfTestRoot "grace-job.stderr.log"),
            10000,
            $script:mt045DescendantExitGraceMilliseconds
        )
        if (
            $graceJobResult.TimedOut -or
            $graceJobResult.ExitCode -ne 0 -or
            $graceJobResult.LeakedProcessCount -ne 0 -or
            @($graceJobResult.LeakedProcessIds).Count -ne 0 -or
            @($graceJobResult.PostDrainDescendantProcessIds).Count -ne 0
        ) {
            $graceFailure = [ordered]@{
                grace_ms = $script:mt045DescendantExitGraceMilliseconds
                timed_out = $graceJobResult.TimedOut
                exit_code = $graceJobResult.ExitCode
                leaked_process_count = $graceJobResult.LeakedProcessCount
                leaked_process_ids = @($graceJobResult.LeakedProcessIds)
                pre_cleanup_descendant_process_ids = @($graceJobResult.PreCleanupDescendantProcessIds)
                post_drain_descendant_process_ids = @($graceJobResult.PostDrainDescendantProcessIds)
            }
            throw "diagnostics self-test did not allow an owned child to exit inside production grace: $($graceFailure | ConvertTo-Json -Compress)"
        }

        $leakEncodedParent = & $newEncodedParent -ChildSleepMilliseconds 30000 -ParentSleepMilliseconds 0
        $leakJobResult = [Mt045JobRunner]::Run(
            $powerShellExe,
            [string[]]@("-NoLogo", "-NoProfile", "-NonInteractive", "-EncodedCommand", $leakEncodedParent),
            $selfTestRoot,
            (Join-Path $selfTestRoot "leak-job.stdout.log"),
            (Join-Path $selfTestRoot "leak-job.stderr.log"),
            10000,
            500
        )
        if (
            $leakJobResult.TimedOut -or
            $leakJobResult.ExitCode -ne 0 -or
            $leakJobResult.LeakedProcessCount -lt 1 -or
            [uint64]$leakJobResult.LeakedProcessCount -ne [uint64]@($leakJobResult.LeakedProcessIds).Count -or
            @($leakJobResult.PreCleanupDescendantProcessIds).Count -ne @($leakJobResult.LeakedProcessIds).Count -or
            @($leakJobResult.PostDrainDescendantProcessIds).Count -ne 0
        ) {
            throw "diagnostics self-test did not capture and drain an owned descendant leak"
        }

        $timeoutEncodedParent = & $newEncodedParent -ChildSleepMilliseconds 30000 -ParentSleepMilliseconds 30000
        $timeoutChildJobResult = [Mt045JobRunner]::Run(
            $powerShellExe,
            [string[]]@("-NoLogo", "-NoProfile", "-NonInteractive", "-EncodedCommand", $timeoutEncodedParent),
            $selfTestRoot,
            (Join-Path $selfTestRoot "timeout-child-job.stdout.log"),
            (Join-Path $selfTestRoot "timeout-child-job.stderr.log"),
            1500,
            1000
        )
        if (
            -not $timeoutChildJobResult.TimedOut -or
            $timeoutChildJobResult.LeakedProcessCount -ne 0 -or
            @($timeoutChildJobResult.LeakedProcessIds).Count -ne 0 -or
            @($timeoutChildJobResult.PreCleanupDescendantProcessIds).Count -lt 1 -or
            @($timeoutChildJobResult.PostDrainDescendantProcessIds).Count -ne 0
        ) {
            throw "diagnostics self-test did not retain pre-cleanup timeout descendants and prove post-drain zero"
        }

        $forcedTestName = "forced-termination-self-test"
        $selfTestWorkspaceId = "wp012-job-cleanup-$([guid]::NewGuid().ToString('N'))"
        # The legacy server-backed self-test inserted a workspace row for a direct-store DELETE. The
        # embedded replacement plants evidence inside the exact per-backend handshake-surreal path so
        # recovery must prove that path is contained by the fixture-owned UUID runtime root.
        $runtimeLeaf = Join-Path (Get-Mt045RuntimeScenarioRoot -ScenarioName $forcedTestName) "runtime-001"
        $selfTestDataDirectory = Join-Path $runtimeLeaf "data"
        $selfTestStorePath = Join-Path $selfTestDataDirectory "handshake-surreal"
        [void][IO.Directory]::CreateDirectory($selfTestStorePath)
        $selfTestMarker = Join-Path $selfTestStorePath "$selfTestWorkspaceId.marker"
        Set-Content -LiteralPath $selfTestMarker -Value $selfTestWorkspaceId -Encoding ascii
        if (-not (Test-Path -LiteralPath $selfTestMarker -PathType Leaf)) {
            throw "diagnostics self-test could not plant its store-scoped cleanup marker"
        }
        $childScript = @"
`$leaf = '$($runtimeLeaf.Replace("'", "''"))'
`$utf8 = [Text.UTF8Encoding]::new(`$false)
`$listenReport = '{"schema_id":"handshake.backend-listen-report.v1","pid":' + `$PID + ',"listen_addr":"127.0.0.1:1"}'
`$workspaceIdentity = '{"schema_id":"hsk.wp_kernel_012.mt045_workspace_identity@1","run_id":"$RunId","scenario_identity":"$forcedTestName","workspace_id":"$selfTestWorkspaceId","owned_backend_pid":' + `$PID + '}'
[IO.File]::WriteAllText([IO.Path]::Combine(`$leaf, 'listen-report.json'), `$listenReport, `$utf8)
[IO.File]::WriteAllText([IO.Path]::Combine(`$leaf, 'workspace-identity.json'), `$workspaceIdentity, `$utf8)
[IO.File]::WriteAllText([IO.Path]::Combine(`$leaf, 'backend.stdout.log'), "before-reap``nlast-line-before-reap``n", `$utf8)
[IO.File]::WriteAllText([IO.Path]::Combine(`$leaf, 'backend.stderr.log'), "stderr-last-line-before-reap``n", `$utf8)
[Threading.Thread]::Sleep(60000)
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
            $script:mt045SelfTestFixtureTimeoutMilliseconds,
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
            leaked_process_ids = @($jobResult.LeakedProcessIds)
            pre_cleanup_descendant_process_ids = @($jobResult.PreCleanupDescendantProcessIds)
            post_drain_descendant_process_ids = @($jobResult.PostDrainDescendantProcessIds)
            root_process_id = $jobResult.RootProcessId
            exit_code = $jobResult.ExitCode
        }
        $countMismatchExpected = [ordered]@{}
        foreach ($entry in $expectedForced.GetEnumerator()) {
            $countMismatchExpected[$entry.Key] = $entry.Value
        }
        $countMismatchExpected.leaked_process_ids = @([uint64]1234)
        if ((Get-PostReapRuntimeBinding -ExpectedCommand $countMismatchExpected).binding_status -cne "INVALID") {
            throw "process-containment receipt accepted a leaked-process count/array mismatch"
        }
        $recovered = Get-PostReapRuntimeBinding -ExpectedCommand $expectedForced
        if ($recovered.binding_status -cne "BOUND") {
            throw "forced-termination recovery was not BOUND: $($recovered.binding_error)"
        }
        if (
            $recovered.workspace_cleanup.status -cne "contained_in_owned_embedded_store" -or
            $recovered.workspace_cleanup.workspace_id -cne $selfTestWorkspaceId -or
            $recovered.workspace_cleanup.containment_verified -ne $true -or
            $recovered.workspace_cleanup.data_dir_inside_runtime_root -ne $true -or
            $recovered.workspace_cleanup.store_path_present -ne $true -or
            [IO.Path]::GetFullPath([string]$recovered.workspace_cleanup.store_path_bound_to_owned_runtime_root) -cne [IO.Path]::GetFullPath($selfTestStorePath) -or
            (Get-FileSha256 -Path $recovered.workspace_cleanup.proof_path) -cne $recovered.workspace_cleanup.proof_sha256
        ) {
            throw "forced-termination recovery did not prove exact post-Job embedded-store containment"
        }
        $dispatcherFailureRoot = $failureDiagnosticsRoot
        $failureDiagnosticsRoot = Join-Path $selfTestRoot "absent-receipt-root-positive-fallback"
        try {
            $routedFallback = @(Get-FailureDiagnosticBindings -ExpectedCommand $expectedForced)
        }
        finally {
            $failureDiagnosticsRoot = $dispatcherFailureRoot
        }
        if (
            $routedFallback.Count -ne 1 -or
            $routedFallback[0].binding_status -cne "BOUND" -or
            $routedFallback[0].binding_source -cne "post_reap_backend_runtime"
        ) {
            throw "failure-diagnostic dispatcher did not route an absent receipt root to the exact post-reap fallback"
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
        $ambiguousRuntime = Join-Path (Get-Mt045RuntimeScenarioRoot -ScenarioName $forcedTestName) "runtime-002"
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

        # Run one real current-source request failure through the fixture-owned backend. The Rust
        # fixture must publish a complete typed receipt before the expected Cargo failure returns.
        $failureDiagnosticsRoot = $canonicalFailureDiagnosticsRoot
        if (Test-Path -LiteralPath $backendRuntimeRunRoot) {
            [IO.Directory]::Delete($backendRuntimeRunRoot, $true)
        }
        if (Test-Path -LiteralPath $failureDiagnosticsRoot) {
            throw "real retained-failure proof root already exists: $failureDiagnosticsRoot"
        }
        [void][IO.Directory]::CreateDirectory($failureDiagnosticsRoot)
        $backendBuildReceipt = Invoke-BoundedCargo -Label "build-current-source-backend-release" -Arguments @(
            "build", "--release", "--locked", "--target-dir", $targetRoot,
            "--manifest-path", "..\..\backend\handshake_core\Cargo.toml",
            "--bin", "handshake_core", "--features", "app-runtime"
        ) -WorkingDirectory $crateRoot -LogRoot $runRoot
        $backendBinary = Join-Path $targetRoot "release\handshake_core.exe"
        if (-not (Test-Path -LiteralPath $backendBinary -PathType Leaf)) {
            throw "diagnostics self-test requires the current-source release backend at $backendBinary"
        }
        $backendBinary = (Resolve-Path -LiteralPath $backendBinary).Path
        $backendSha256 = Get-FileSha256 -Path $backendBinary
        $env:HSK_MT045_CANONICAL_RUN = "1"
        $env:HSK_MT045_RUN_ID = $RunId
        $env:HSK_MT045_SOURCE_SHA = $sourceSha
        $env:HSK_MT045_CARGO_TARGET_OWNER_KEY = $targetOwnerKey
        $env:HANDSHAKE_ARTIFACTS_ROOT = $artifactRoot
        $env:HANDSHAKE_TEST_STAGE_BINDING_ROOT = (Join-Path $runRoot "binding")
        $env:HSK_TEST_BACKEND_BIN = $backendBinary
        $probeDiagnosticResults = [Collections.Generic.List[object]]::new()
        foreach ($entry in @(
            @("test_heartbeat", "heartbeat_advances_by_n_over_n_frames"),
            @("test_heartbeat", "idle_repaint_cadence_is_bounded"),
            @("test_diagnostics_panel", "panel_projects_live_heartbeat_frame_and_events")
        )) {
            $bin = $entry[0]
            $test = $entry[1]
            $result = Invoke-BoundedCargo -Label "probe-diagnostic-$test" -Arguments @(
                "test", "--release", "--locked", "--target-dir", $targetRoot,
                "--test", $bin, $test, "--", "--exact", "--nocapture", "--test-threads=1"
            ) -WorkingDirectory $crateRoot -LogRoot $runRoot
            Assert-ExactTestResult -CommandResult $result -ExpectedTest $test
            $probeDiagnosticResults.Add($result)
        }
        $probeDiagnosticReceiptPath = Join-Path $runRoot "diagnostics-preflight.json"
        $probeDiagnosticReceiptSha256 = Write-ImmutableJson -Path $probeDiagnosticReceiptPath -Value ([ordered]@{
            schema_id = "hsk.wp_kernel_012.mt045_diagnostics_preflight@1"
            work_packet_id = "WP-KERNEL-012"
            micro_task_id = "MT-045"
            run_id = $RunId
            source_sha = $sourceSha
            status = "PASS"
            tests = $probeDiagnosticResults
            completed_at = [DateTimeOffset]::UtcNow.ToString("O")
        })
        $env:HSK_MT045_DIAGNOSTIC_RECEIPT = $probeDiagnosticReceiptPath
        $env:HSK_MT045_RETAINED_FAILURE_PROBE = "1"
        Remove-Item Env:HSK_TEST_BASE -ErrorAction SilentlyContinue
        $expectedFailureObserved = $false
        $realFailureProbeStarted = $true
        try {
            $null = Invoke-BoundedCargo -Label "retained-request-failure-probe" -Arguments @(
                "test", "--release", "--locked", "--target-dir", $targetRoot,
                "--test", "test_perf_large_knowledge", "perf_proof_perf_lk03_tag_hub", "--",
                "--exact", "--nocapture", "--test-threads=1"
            ) -WorkingDirectory $crateRoot -LogRoot $runRoot -TimeoutSeconds 600
        }
        catch {
            if (
                $null -eq $script:lastFailedCommandReceipt -or
                $script:lastFailedCommandReceipt.timed_out -ne $false -or
                $script:lastFailedCommandReceipt.exit_code -eq 0 -or
                $script:lastFailedCommandReceipt.leaked_process_count -ne 0
            ) {
                throw "retained request-failure probe did not fail cleanly under the bounded Job: $($_.Exception.Message)"
            }
            $expectedFailureObserved = $true
        }
        finally {
            Remove-Item Env:HSK_MT045_RETAINED_FAILURE_PROBE -ErrorAction SilentlyContinue
        }
        if (-not $expectedFailureObserved) {
            throw "retained request-failure probe unexpectedly passed"
        }
        $retainedBindings = @(Get-FailureDiagnosticBindings -ExpectedCommand $script:lastFailedCommandReceipt)
        $boundRetained = @($retainedBindings | Where-Object { $_.binding_status -ceq "BOUND" })
        if (
            $retainedBindings.Count -ne 1 -or
            $boundRetained.Count -ne 1 -or
            $boundRetained[0].binding_source -cne "rust_failure_receipt" -or
            $boundRetained[0].trigger -cne "request_failure" -or
            $boundRetained[0].stage -cne "request_send" -or
            $null -eq $boundRetained[0].reqwest_error -or
            -not (
                $boundRetained[0].reqwest_error.is_request -eq $true -or
                $boundRetained[0].reqwest_error.is_connect -eq $true -or
                $boundRetained[0].reqwest_error.is_timeout -eq $true -or
                $boundRetained[0].reqwest_error.is_body -eq $true -or
                $boundRetained[0].reqwest_error.is_decode -eq $true -or
                $boundRetained[0].reqwest_error.is_builder -eq $true
            ) -or
            @($boundRetained[0].reqwest_error.source_chain).Count -lt 1 -or
            $null -eq $boundRetained[0].immediate_health -or
            $boundRetained[0].immediate_health.reachable -ne $false -or
            $boundRetained[0].process.owned -ne $true -or
            $boundRetained[0].process.termination -notin @("already_exited_and_reaped", "terminated_and_reaped") -or
            -not ([string]$boundRetained[0].backend_identity.executable).Equals($backendBinary, [StringComparison]::OrdinalIgnoreCase) -or
            [string]$boundRetained[0].backend_identity.executable_sha256 -cne $backendSha256 -or
            @($boundRetained[0].retained_files).Count -ne 3
        ) {
            throw "retained request-failure probe did not publish one complete typed Rust receipt"
        }
        # Counterfactual 6: a hash-valid receipt whose reqwest URL uses the retained listener but
        # names a route different from its request label must fail closed. This guards the distinction
        # between the failed request URL and the separate immediate /health probe URL.
        $realReceiptPath = [string]$boundRetained[0].receipt
        $realReceiptSidecarPath = "$realReceiptPath.sha256"
        $originalReceiptBytes = [IO.File]::ReadAllBytes($realReceiptPath)
        $originalSidecarBytes = [IO.File]::ReadAllBytes($realReceiptSidecarPath)
        try {
            $wrongRouteReceipt = Get-Content -LiteralPath $realReceiptPath -Raw | ConvertFrom-Json
            $wrongRouteUri = [Uri][string]$wrongRouteReceipt.reqwest_error.url
            $wrongRouteReceipt.reqwest_error.url = "$($wrongRouteUri.Scheme)://$($wrongRouteUri.Authority)/mt045-wrong-route"
            Write-JsonAtomic -Path $realReceiptPath -Value $wrongRouteReceipt
            [IO.File]::WriteAllText(
                $realReceiptSidecarPath,
                "$(Get-FileSha256 -Path $realReceiptPath)  failure-diagnostics.json`n",
                [Text.UTF8Encoding]::new($false)
            )
            $wrongRouteBindings = @(Get-FailureDiagnosticBindings -ExpectedCommand $script:lastFailedCommandReceipt)
            if (@($wrongRouteBindings | Where-Object { $_.binding_status -ceq "BOUND" }).Count -ne 0) {
                throw "diagnostics self-test accepted a request URL whose route contradicted the receipt label"
            }
        }
        finally {
            [IO.File]::WriteAllBytes($realReceiptPath, $originalReceiptBytes)
            [IO.File]::WriteAllBytes($realReceiptSidecarPath, $originalSidecarBytes)
        }
        Assert-SourceBindingClean -Repository $repoRoot -Paths $sourcePaths
        $retainedProofPath = Join-Path $runRoot "retained-failure-proof.json"
        $retainedProofSha256 = Write-ImmutableJson -Path $retainedProofPath -Value ([ordered]@{
            schema_id = "hsk.wp_kernel_012.mt045_retained_failure_proof@1"
            work_packet_id = "WP-KERNEL-012"
            micro_task_id = "MT-045"
            run_id = $RunId
            source_sha = $sourceSha
            store = [ordered]@{
                kind = "embedded_surrealdb"
                identity = "rust_failure_receipt.workspace_cleanup"
                identity_source = "rust_failure_receipt.workspace_cleanup"
                observed_runtime_root = $boundRetained[0].workspace_cleanup.runtime_root
                observed_data_dir = $boundRetained[0].workspace_cleanup.data_dir
                observed_store_path = $boundRetained[0].workspace_cleanup.store_path_bound_to_owned_runtime_root
                containment_verified = $boundRetained[0].workspace_cleanup.containment_verified
                source_runtime_lifecycle = "removed_after_complete_receipt_publication"
            }
            backend = [ordered]@{
                build_receipt = $backendBuildReceipt
                executable = $backendBinary
                sha256 = $backendSha256
            }
            diagnostics = [ordered]@{
                receipt = $probeDiagnosticReceiptPath
                sha256 = $probeDiagnosticReceiptSha256
            }
            expected_failure_command = $script:lastFailedCommandReceipt
            failure_binding = $boundRetained[0]
            completed_at = [DateTimeOffset]::UtcNow.ToString("O")
        })
        $retainedFailureProofComplete = $true

        Write-Output ([ordered]@{
            status = "PASS"
            source_sha = $sourceSha
            retained_failure_proof = $retainedProofPath
            retained_failure_proof_sha256 = $retainedProofSha256
            retained_failure_receipt = $boundRetained[0].receipt
            retained_failure_receipt_sha256 = $boundRetained[0].receipt_sha256
            malformed_all_missing = "INVALID"
            null_process_exit = "INVALID"
            failed_workspace_cleanup = "INVALID"
            string_process_exit = "INVALID"
            string_workspace_count = "INVALID"
            missing_workspace_count = "INVALID"
            mixed_cleanup_stdout = "INVALID"
            forced_job_timed_out = $jobResult.TimedOut
            forced_job_leaked_process_count = $jobResult.LeakedProcessCount
            normal_root_only_descendants = @($normalJobResult.LeakedProcessIds).Count
            grace_exit_descendants = @($graceJobResult.LeakedProcessIds).Count
            captured_leak_descendants = @($leakJobResult.LeakedProcessIds).Count
            timeout_pre_cleanup_descendants = @($timeoutChildJobResult.PreCleanupDescendantProcessIds).Count
            timeout_post_drain_descendants = @($timeoutChildJobResult.PostDrainDescendantProcessIds).Count
            process_id_count_mismatch = "INVALID"
            post_reap_binding = $recovered.binding_status
            dispatcher_positive_fallback = $routedFallback[0].binding_status
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
        $manifestRestoreFailure = $null
        try {
            [IO.File]::WriteAllText(
                $manifestPath,
                $headManifestJson + [Environment]::NewLine,
                [Text.UTF8Encoding]::new($false)
            )
            $restoredManifestGitObject = Invoke-GitText -Repository $repoRoot -Arguments @(
                "hash-object", "--path=$manifestRepoPath", "--", $manifestRepoPath
            )
            if ($restoredManifestGitObject -cne $initialManifestGitObject) {
                throw "diagnostics self-test did not restore the committed manifest object: expected=$initialManifestGitObject actual=$restoredManifestGitObject"
            }
        }
        catch {
            $manifestRestoreFailure = $_
        }
        if (-not [string]::IsNullOrWhiteSpace($selfTestWorkspaceId) -and ("Mt045JobRunner" -as [type])) {
            Remove-Item -LiteralPath $selfTestMarker -Force -ErrorAction SilentlyContinue
            if (Test-Path -LiteralPath $selfTestMarker -PathType Leaf) {
                throw "diagnostics self-test store-scoped cleanup marker survived teardown"
            }
        }
        $cleanupPaths = @($selfTestRoot)
        if (-not $realFailureProbeStarted -or $retainedFailureProofComplete) {
            $cleanupPaths += $backendRuntimeRunRoot
        }
        foreach ($path in $cleanupPaths) {
            $fullPath = [IO.Path]::GetFullPath($path)
            if (
                $fullPath.StartsWith($artifactPrefix, [StringComparison]::OrdinalIgnoreCase) -and
                (Test-Path -LiteralPath $fullPath)
            ) {
                [IO.Directory]::Delete($fullPath, $true)
            }
        }
        $targetCleanup = Remove-Mt045OwnerTarget
        if ($targetCleanup.status -notin @("deleted_and_verified_absent", "not_present")) {
            throw "diagnostics self-test owner target cleanup did not reach terminal absence"
        }
        if ($null -ne $manifestRestoreFailure) {
            throw $manifestRestoreFailure
        }
    }
    exit 0
}

function New-SupervisorProjection {
    param(
        [Parameter(Mandatory)][ValidateSet("RUNNING", "FAIL")][string]$Status,
        [string]$Reason,
        [object[]]$FailureDiagnostics = @(),
        [AllowNull()]$TargetCleanup = $null
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
        cargo_target_owner_key = $targetOwnerKey
        canonical_target_root = $targetRoot
        target_cleanup = $TargetCleanup
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
$targetCleanup = $null
$supervisorStartedAt = [DateTimeOffset]::UtcNow
try {
    $runningProjection = New-SupervisorProjection -Status "RUNNING"
    # supervisor-current is the top-level attempt gate. Publish it first, then invalidate both
    # historical run projections and finally the manifest; any later failure is terminally republished.
    Write-JsonAtomic -Path $supervisorCurrentPath -Value $runningProjection
    Write-JsonAtomic -Path $currentRunPath -Value $runningProjection
    Write-JsonAtomic -Path $latestRunPath -Value $runningProjection
    Set-ManifestTerminalState -Status "RUNNING"
    [void][IO.Directory]::CreateDirectory($targetRoot)

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
    $env:HSK_MT045_CARGO_TARGET_OWNER_KEY = $targetOwnerKey
    $env:HANDSHAKE_ARTIFACTS_ROOT = $artifactRoot
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
    $targetCleanup = Remove-Mt045OwnerTarget
    if ($targetCleanup.status -cne "deleted_and_verified_absent") {
        throw "successful MT-045 run did not delete its owner-scoped Cargo target"
    }

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
        cargo_target_owner_key = $targetOwnerKey
        canonical_target_root = $targetRoot
        target_cleanup = $targetCleanup
        budget_overrides = @()
        store = [ordered]@{
            kind = "embedded_surrealdb"
            identity = "receipt_bound_per_scenario_embedded_store"
            identity_source = "per_scenario_runtime_diagnostics"
            store_directory_name = "handshake-surreal"
            namespace = "handshake"
            database = "primary"
            external_server = $false
            lifecycle = "opened_and_closed_by_the_backend_process"
            health_proven_by = "scenario-owned handshake_core /health status=ok and db_status=ok"
        }
        backend = [ordered]@{
            path = $env:HSK_TEST_BACKEND_BIN
            sha256 = $backendSha256
            external_database = $false
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
        target_cleanup = $targetCleanup
        updated_at = [DateTimeOffset]::UtcNow.ToString("O")
    })
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
    $targetCleanup = try {
        Remove-Mt045OwnerTarget
    }
    catch {
        [ordered]@{
            status = "cleanup_failed"
            path = $targetRoot
            error = $_.Exception.Message
            observed_at = [DateTimeOffset]::UtcNow.ToString("O")
        }
    }
    $enrichedProjection = New-SupervisorProjection -Status "FAIL" -Reason $failure -FailureDiagnostics $diagnosticBindings -TargetCleanup $targetCleanup
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
