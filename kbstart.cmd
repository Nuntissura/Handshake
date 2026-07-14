@echo off
setlocal EnableExtensions DisableDelayedExpansion

rem Force kernel-builder startup to acknowledge all exposed binding files.
set "ORCSTART_EXPOSE_LAUNCHER_BINDINGS=1"
set "SCRIPT_DIR=%~dp0"
call "%SCRIPT_DIR%.GOV\operator\scripts\kbstart.cmd" %*
set "KBSTART_LAUNCHER_EXIT_CODE=%ERRORLEVEL%"

echo(
echo ========================================================================================
echo KBSTART POST-COMMAND AUTHORITY GATE
echo ========================================================================================
echo The launcher exit code proves only that the command and authority injection ran.
echo It never proves that the assistant read the injected binding files.
echo ASSISTANT_MUST_NOT_REPORT=KERNEL_BUILDER_STARTUP_COMPLETE_FROM_COMMAND_EXIT_OR_KBSTART_COMPLETE_MARKER
echo KBSTART_FINAL_STATE command_exit=%KBSTART_LAUNCHER_EXIT_CODE% authority_read=UNVERIFIED role_startup_complete=NO required_action=READ_ALL_BINDING_FILES_THEN_ACK
exit /b %KBSTART_LAUNCHER_EXIT_CODE%
