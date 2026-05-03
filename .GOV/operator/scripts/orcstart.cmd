@echo off
setlocal EnableExtensions DisableDelayedExpansion

set "ORCSTART_SCRIPT_DIR=%~dp0"
if not exist "%ORCSTART_SCRIPT_DIR%orcstart.ps1" (
  set "ORCSTART_SCRIPT_DIR=%~dp0.GOV\operator\scripts\"
)

powershell -NoProfile -ExecutionPolicy Bypass -File "%ORCSTART_SCRIPT_DIR%orcstart.ps1" %*
exit /b %ERRORLEVEL%
