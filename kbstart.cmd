@echo off
setlocal EnableExtensions DisableDelayedExpansion
call "%~dp0.GOV\operator\scripts\kbstart.cmd" %*
exit /b %ERRORLEVEL%
