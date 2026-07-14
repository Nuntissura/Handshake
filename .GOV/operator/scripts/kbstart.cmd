@echo off
setlocal EnableExtensions DisableDelayedExpansion

set "ORCSTART_SCRIPT_DIR=%~dp0"
if not exist "%ORCSTART_SCRIPT_DIR%orcstart.ps1" (
  set "ORCSTART_SCRIPT_DIR=%~dp0.GOV\operator\scripts\"
)

set "ORCSTART_ROLE=KERNEL_BUILDER"
rem Never inline large authority files into one caller-truncatable stdout stream.
rem The launcher still prints the complete required-file manifest; the assistant
rem must read each listed file directly from disk before acknowledging startup.
powershell -NoProfile -ExecutionPolicy Bypass -File "%ORCSTART_SCRIPT_DIR%orcstart.ps1" %* --no-authority-files
exit /b %ERRORLEVEL%
