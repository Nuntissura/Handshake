# WP-KERNEL-012 WPV entrypoint helper: move ONE owned path to the Windows Recycle Bin (restorable).
# Shell API SHFileOperation, silent flags (no dialogs, no focus). Refuses paths outside -AllowedRoot,
# missing paths and reparse points. Used by WPV-union-round.sh for the MT-165 disposable git repo
# (Operator decision WP012-MT165-DISPOSABLE-GIT-REPO-20260930). Prints RECYCLED <path> on success.
param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$AllowedRoot
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class WpvRecycleNative {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct SHFILEOPSTRUCT {
        public IntPtr hwnd;
        public UInt32 wFunc;
        public string pFrom;
        public string pTo;
        public UInt16 fFlags;
        [MarshalAs(UnmanagedType.Bool)] public bool fAnyOperationsAborted;
        public IntPtr hNameMappings;
        public string lpszProgressTitle;
    }
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    public static extern int SHFileOperation(ref SHFILEOPSTRUCT op);
}
'@

$full = [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
$root = [System.IO.Path]::GetFullPath($AllowedRoot).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
if (-not $full.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)) { throw "outside_allowed_root: $full not under $root" }
if (-not (Test-Path -LiteralPath $full)) { throw "missing: $full" }
$item = Get-Item -LiteralPath $full -Force
if ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) { throw "reparse_point_refused: $full" }

$op = New-Object WpvRecycleNative+SHFILEOPSTRUCT
$op.hwnd = [IntPtr]::Zero
$op.wFunc = 3                                   # FO_DELETE
$op.pFrom = $full + "`0`0"
$op.pTo = $null
$op.fFlags = [UInt16](0x0040 -bor 0x0010 -bor 0x0004 -bor 0x0400 -bor 0x0200)  # ALLOWUNDO|NOCONFIRMATION|SILENT|NOERRORUI|NOCONFIRMMKDIR
$rc = [WpvRecycleNative]::SHFileOperation([ref]$op)
if ($rc -ne 0 -or $op.fAnyOperationsAborted) { throw "shfileoperation_failed rc=$rc aborted=$($op.fAnyOperationsAborted): $full" }
if (Test-Path -LiteralPath $full) { throw "still_present: $full" }
Write-Output "RECYCLED $full"
