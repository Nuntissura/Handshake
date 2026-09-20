@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "KBSTART_KERNEL_ROOT=%~dp0..\..\.."

rem Kernel Builder has a read-only startup; it does not use Orchestrator ceremony.
set "KBSTART_CONTEXT_ONLY=0"
:parse
if "%~1"=="" goto dispatch
if /i "%~1"=="--help" goto help
if /i "%~1"=="-help" goto help
if /i "%~1"=="/?" goto help
if /i "%~1"=="--print" goto context_flag
if /i "%~1"=="-print" goto context_flag
if /i "%~1"=="--no-startup" goto context_flag
if /i "%~1"=="-nostartup" goto context_flag
if /i "%~1"=="--brief" goto next_arg
if /i "%~1"=="-brief" goto next_arg
if /i "%~1"=="--no-authority-files" goto next_arg
if /i "%~1"=="-noauthorityfiles" goto next_arg
echo Unknown Kernel Builder startup argument.
exit /b 2

:context_flag
set "KBSTART_CONTEXT_ONLY=1"
:next_arg
shift
goto parse

:dispatch
if "%KBSTART_CONTEXT_ONLY%"=="1" goto context
pushd "%KBSTART_KERNEL_ROOT%"
if errorlevel 1 exit /b 1
call just kernel-builder-startup
set "KBSTART_EXIT=%ERRORLEVEL%"
popd
exit /b %KBSTART_EXIT%

:context
echo Read .GOV/codex/Handshake_Codex_v1.4.md
echo Read .GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json
echo Read .GOV/roles/kernel_builder/KERNEL_BUILDER_PROTOCOL.md
echo Resolve product requirements through .GOV/spec/SPEC_CURRENT.md
echo Verify the current assignment, branch and worktree before edits.
exit /b 0

:help
echo kbstart.cmd [--brief] [--no-authority-files] [--print ^| --no-startup]
echo Lists authority paths and checkout state. No memory or acknowledgment writes.
echo --brief and --no-authority-files are accepted compatibility flags; output is always concise.
exit /b 0
