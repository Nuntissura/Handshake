@echo off
setlocal EnableExtensions DisableDelayedExpansion

rem Force kernel-builder startup to acknowledge every required binding file.
set "ORCSTART_EXPOSE_LAUNCHER_BINDINGS=1"
set "SCRIPT_DIR=%~dp0"
call "%SCRIPT_DIR%.GOV\operator\scripts\kbstart.cmd" %*
set "KBSTART_LAUNCHER_EXIT_CODE=%ERRORLEVEL%"

echo(
echo ========================================================================================
echo KBSTART POST-COMMAND AUTHORITY GATE
echo ========================================================================================
echo The launcher exit code proves only that the command and authority injection ran.
echo Authority file contents are intentionally never inlined into the truncatable command stream.
echo It never proves that the assistant read each required binding file directly from disk.
echo ASSISTANT_MUST_NOT_REPORT=KERNEL_BUILDER_STARTUP_COMPLETE_FROM_COMMAND_EXIT_OR_KBSTART_COMPLETE_MARKER
echo KBSTART_FINAL_STATE command_exit=%KBSTART_LAUNCHER_EXIT_CODE% authority_read=UNVERIFIED role_startup_complete=NO required_action=READ_EACH_BINDING_FILE_DIRECTLY_THEN_ACK
exit /b %KBSTART_LAUNCHER_EXIT_CODE%
