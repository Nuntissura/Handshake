@echo off
setlocal EnableExtensions DisableDelayedExpansion

rem Force kernel-builder startup to acknowledge all exposed binding files.
set "ORCSTART_EXPOSE_LAUNCHER_BINDINGS=1"
set "SCRIPT_DIR=%~dp0"
call "%SCRIPT_DIR%.GOV\operator\scripts\kbstart.cmd" %*
exit /b %ERRORLEVEL%
