@echo off
setlocal EnableExtensions DisableDelayedExpansion

set "SCRIPT_DIR=%~dp0"
call "%SCRIPT_DIR%.GOV\operator\scripts\orcstart.cmd" %*
exit /b %ERRORLEVEL%
