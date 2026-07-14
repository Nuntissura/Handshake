---
file_id: AUDIT-20260714-KBSTART-AUTHORITY-READ-TRUNCATION-GATE
file_kind: governance_audit
updated_at: "2026-07-14"
---

<topic id="driver" status="active" version="1" summary="Observed authority-output truncation allowed a false startup-completion handoff." updated_at="2026-07-14">

# Driver

The Operator requested a deterministic repair after `kbstart.cmd` emitted 5,560 lines, the shell caller exposed only a truncated stream while retaining the final `KBSTART COMPLETE` section, and the assistant stopped after exit code `0` without reading every required binding file.

The command exit was correctly reported, but Kernel Builder startup was not authority-read complete. The missing guard was a final wrapper-owned state that remained visible after any inner launcher output and explicitly separated command/injection completion from assistant reading.

</topic>

<topic id="research-basis" status="complete" version="1" summary="Windows exit codes and PowerShell streams support a truthful tail marker without converting startup warnings into failures." updated_at="2026-07-14">

# Research Basis

- Microsoft documents that `exit /b <exitcode>` sets the batch script's `ERRORLEVEL`, so the wrapper can print a final state and still return the delegated launcher code unchanged: <https://learn.microsoft.com/windows-server/administration/windows-commands/exit>.
- Microsoft documents that PowerShell success output maps to the native stdout stream and can be redirected or truncated by the caller independently of process exit status: <https://learn.microsoft.com/powershell/module/microsoft.powershell.core/about/about_output_streams> and <https://learn.microsoft.com/powershell/module/microsoft.powershell.core/about/about_redirection>.
- Local prior authority in `AUDIT-20260506-ORCSTART-NONZERO-STARTUP-WARNING` rejects converting deterministic startup warnings into launcher failure when authority injection succeeds.

Selected approach: append a root-wrapper-owned machine-readable final state after the delegated launcher returns, preserve its exact exit code, and make the canonical protocol-alignment check require the marker.

Rejected approach: force the first launcher phase to exit nonzero until a later acknowledgment. That would conflate successful injection with failure and regress the established nonzero-warning continuation contract.

</topic>

<topic id="red-team" status="complete" version="1" summary="The repair addresses accidental truncation confusion but does not claim to prove human or model comprehension." updated_at="2026-07-14">

# Red Team

- Risk: the inner launcher still prints `KBSTART COMPLETE`. Control: the root wrapper prints the last output and explicitly prohibits treating that inner marker as authority-read completion.
- Risk: a wrapper change could mask a real launcher error. Control: capture `%ERRORLEVEL%` immediately after `call` and return it unchanged after printing the guard.
- Risk: help, print, no-startup, or no-authority invocations could be mistaken for role startup. Control: the final state is unconditional and always says `authority_read=UNVERIFIED role_startup_complete=NO`.
- Risk: the marker is later removed. Control: `protocol-alignment-check.mjs`, already part of `gov-check`, requires both the machine state and the explicit prohibited-claim marker.
- Limit: no command can mechanically prove semantic comprehension by a model. This control prevents the observed accidental success path; deliberate false acknowledgment remains a truthfulness violation governed by higher authority.

</topic>

<topic id="verification" status="complete" version="1" summary="Wrapper success, failure, alternate-cwd, full-startup, and generated-projection checks passed; broader gov-check debt remains separated." updated_at="2026-07-14">

# Verification

Proof results:

```text
PASS success path: exit=0; final line=KBSTART_FINAL_STATE command_exit=0 authority_read=UNVERIFIED role_startup_complete=NO required_action=READ_ALL_BINDING_FILES_THEN_ACK
PASS failure path: invalid argument preserved exit=1; final line carried command_exit=1 and the same authority gate
PASS alternate cwd: absolute kbstart path invoked from the system temp directory and ended with the required marker
PASS structural boundary: call index < ERRORLEVEL capture index < final gate index < exit /b index
PASS full startup: exit=0; runtime=55.7s; captured lines=5,587; inner KBSTART COMPLETE markers=1; required final wrapper state present
PASS node --check .GOV/roles_shared/checks/protocol-alignment-check.mjs
PASS repo-governance-board-check
PASS governance-topology-check
PASS public-surface-consolidation-check
PASS docs-check
PASS git diff --check
PARTIAL protocol-alignment-check: zero kbstart.cmd violations; unrelated existing Orchestrator-protocol string drift remains
PARTIAL just gov-check --sync-topology: projections synced; aggregate remained nonzero on 15 broader existing checks recorded in the external failure dossier
```

The scoped acceptance surface passed: the last launcher line contains `KBSTART_FINAL_STATE`, preserves the delegated exit code, declares authority reading unverified, and denies role-startup completion until the assistant reads every binding file and issues the exact acknowledgment.

</topic>

<topic id="adversarial-review" status="complete" version="1" summary="Medium-risk independent review found no scoped defect and records the remaining comprehension limit." updated_at="2026-07-14">

# Adversarial Review

## DIFF_ATTACK_SURFACES

- delegated `ERRORLEVEL` could be overwritten before capture;
- a different working directory could break `%~dp0` path resolution;
- invalid arguments or missing launcher failures could be masked by the wrapper;
- the regression checker could pass only because its broader run failed before examining `kbstart.cmd`;
- generated topology could remain stale after the check and wrapper changed.

## INDEPENDENT_CHECKS_RUN

- invoked the absolute launcher path from the system temp directory;
- inspected statement ordering by byte index instead of relying on runtime output alone;
- ran the canonical protocol-alignment check and isolated all emitted `kbstart.cmd` violations, finding zero;
- ran syntax, governance-board, topology, and public-surface checks independently.

## COUNTERFACTUAL_CHECKS

- If `set "KBSTART_LAUNCHER_EXIT_CODE=%ERRORLEVEL%"` moved below any intervening command, the wrapper could return a status unrelated to the delegated launcher.
- If the root `KBSTART_FINAL_STATE` line were removed, a head/tail truncating caller could again preserve the inner `KBSTART COMPLETE` marker without any visible authority-read denial; `protocol-alignment-check` now rejects that removal.

## BOUNDARY_PROBES

- caller/callee success boundary preserved `0`;
- caller/callee invalid-input boundary preserved `1`;
- path boundary from a different current directory still resolved through `%~dp0`.

## NEGATIVE_PATH_CHECKS

- `--definitely-invalid` reached the inner unknown-argument failure and the wrapper printed the gate without converting the failure to success.

## INDEPENDENT_FINDINGS

No scoped defect was found. Review recommendation: accept the governance-only launcher change while keeping the broader `gov-check` failures classified as separate existing debt.

## RESIDUAL_UNCERTAINTY

The command cannot prove that a human or model semantically comprehended the files. It can make the accidental false-completion path explicit and mechanically guarded, which is the observed defect. A deliberate false acknowledgment remains a truthfulness violation rather than a property a batch wrapper can detect.

</topic>
