[CmdletBinding()]
param(
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]] $Args
)

$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = (Resolve-Path (Join-Path $scriptRoot "..\..\..")).Path
$promptDoc = Join-Path $scriptRoot "..\docs_local\Handshake_Role_Startup_Prompts.md"
$role = if ($env:ORCSTART_ROLE) { $env:ORCSTART_ROLE } else { "ORCHESTRATOR" }
$repoDisplay = "../" + (Split-Path -Leaf $repoRoot)
$promptDocDisplay = ".GOV/operator/docs_local/Handshake_Role_Startup_Prompts.md"
$minimumStartupTimeoutMs = 600000
$recommendedStartupTimeoutMs = 1200000
$startupTimeoutGuidance = "Startup can take several minutes. Use shell timeout >= $minimumStartupTimeoutMs ms / 10 minutes; $recommendedStartupTimeoutMs ms / 20 minutes is recommended on this host under load."
$authorityFiles = @(
  [pscustomobject]@{
    Key = "AGENTS"
    DisplayPath = "../handshake_main/AGENTS.md"
    Path = Join-Path $repoRoot "..\handshake_main\AGENTS.md"
  },
  [pscustomobject]@{
    Key = "CODEX"
    DisplayPath = ".GOV/codex/Handshake_Codex_v1.4.md"
    Path = Join-Path $repoRoot ".GOV\codex\Handshake_Codex_v1.4.md"
  },
  [pscustomobject]@{
    Key = "ORCHESTRATOR_PROTOCOL"
    DisplayPath = ".GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md"
    Path = Join-Path $repoRoot ".GOV\roles\orchestrator\ORCHESTRATOR_PROTOCOL.md"
  }
)

$brief = $false
$injectAuthorityFiles = $true
$noStartup = $false
$printOnly = $false
$script:startupExitCode = 0
$script:authorityExitCode = 0
$script:startupOutput = New-Object 'System.Collections.Generic.List[string]'

function Write-Section {
  param([string] $Title)

  Write-Output ""
  Write-Output ("=" * 88)
  Write-Output $Title
  Write-Output ("=" * 88)
}

function Add-StartupOutputLine {
  param([object] $Line)

  $text = [string] $Line
  [void] $script:startupOutput.Add($text)
  Write-Output $text
}

function Get-StartupWarningCauses {
  $causes = @()

  foreach ($line in $script:startupOutput) {
    $trimmed = $line.Trim()
    if (-not $trimmed) {
      continue
    }

    $cause = $null
    if ($trimmed -match '^FAIL\s+\|\s+(.+)$') {
      $cause = $Matches[1].Trim()
    } elseif ($trimmed -match '^error:\s+Recipe\s+`([^`]+)`\s+failed.*exit code\s+([0-9]+)') {
      $cause = ('recipe `{0}` failed with exit code {1}' -f $Matches[1], $Matches[2])
    } elseif ($trimmed -match '^\[orcstart\]\s+Startup command failed before completion:\s+(.+)$') {
      $cause = ('startup command threw before completion: {0}' -f $Matches[1])
    }

    if ($cause -and -not ($causes -contains $cause)) {
      $causes += $cause
    }
  }

  if ($causes.Count -eq 0) {
    $causes += "no detailed failing check was parsed; inspect FIRST COMMAND OUTPUT above"
  }

  return $causes
}

function Write-StartupWarning {
  if ($script:startupExitCode -eq 0) {
    return
  }

  Write-Section "STARTUP WARNING: FIRST COMMAND NONZERO"
  Write-Output ("WARNING: just orchestrator-startup exited with code {0}." -f $script:startupExitCode)
  Write-Output "This is not an authority-injection failure. The role prompt, repo governing rule set, and required authority-file injection continue."
  Write-Output ""
  Write-Output "LIKELY_CAUSES:"
  foreach ($cause in (Get-StartupWarningCauses)) {
    Write-Output ("- {0}" -f $cause)
  }
  Write-Output ""
  Write-Output "ROLE_STARTUP_CONTINUES: yes"
  Write-Output "ASSISTANT_ACTION: read and obey ROLE STARTUP PROMPT plus REQUIRED AUTHORITY FILES; treat this warning as startup state context."
}

function Show-Help {
  Write-Output "Usage: orcstart.cmd [--print] [--no-startup] [--no-authority-files] [--brief] [--help]"
  Write-Output ""
  Write-Output "Prints the live Handshake Orchestrator startup prompt, startup context, and authority-read contract."
  Write-Output "This is model/provider agnostic: it does not launch a model process."
  Write-Output $startupTimeoutGuidance
  Write-Output ""
  Write-Output "Prompt source:"
  Write-Output "  $promptDocDisplay"
  Write-Output ""
  Write-Output "Options:"
  Write-Output "  --print       Print only the extracted role startup prompt."
  Write-Output "  --no-startup  Do not run the prompt's FIRST COMMAND."
  Write-Output "  --no-authority-files"
  Write-Output "                Print the authority-read contract without embedding authority file contents."
  Write-Output "  --brief       Keep only the contract, prompt, and startup command output."
  Write-Output "  --help        Show this help."
  Write-Output ""
  Write-Output "Environment:"
  Write-Output "  ORCSTART_ROLE=ORCHESTRATOR"
}

function Get-StartupPrompt {
  param(
    [string] $Path,
    [string] $RoleName
  )

  if (-not (Test-Path -LiteralPath $Path)) {
    throw ('[orcstart] Missing startup prompt document: "{0}"' -f $Path)
  }

  $doc = Get-Content -Raw -LiteralPath $Path -Encoding UTF8
  $escapedRole = [regex]::Escape($RoleName)
  $pattern = '(?ms)^##\s+' + $escapedRole + '\s+-\s+Startup Prompt\s*\r?\n\s*```text\s*(.*?)\s*```'
  $match = [regex]::Match($doc, $pattern)
  if (-not $match.Success) {
    throw ('[orcstart] Could not find "{0} - Startup Prompt" fenced text block in "{1}"' -f $RoleName, $Path)
  }

  return $match.Groups[1].Value.Trim()
}

function Invoke-StartupCommand {
  Set-Location -LiteralPath $repoRoot

  Write-Section "FIRST COMMAND OUTPUT: just orchestrator-startup"
  Write-Output "Command: just orchestrator-startup"
  Write-Output "Working directory: $repoDisplay"
  Write-Output $startupTimeoutGuidance
  Write-Output ""

  $previousErrorActionPreference = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  try {
    & cmd.exe /d /c "just orchestrator-startup 2>&1" | ForEach-Object { Add-StartupOutputLine $_ }
    $script:startupExitCode = $LASTEXITCODE
  } catch {
    Add-StartupOutputLine "[orcstart] Startup command failed before completion: $($_.Exception.Message)"
    $script:startupExitCode = 1
  } finally {
    $ErrorActionPreference = $previousErrorActionPreference
  }
}

function Write-AuthorityContract {
  Write-Section "REPO GOVERNING RULE SET"
  Write-Output "Assistant contract:"
  Write-Output "1. Treat this orcstart output as repo-governing instructions for this conversation, subject to higher-priority system, developer, and user instructions."
  Write-Output "2. Treat the ROLE STARTUP PROMPT below as binding role law for the selected role."
  Write-Output "3. Run and follow the FIRST COMMAND exactly. Startup output is required context, not a substitute for the authority files."
  Write-Output "4. If the FIRST COMMAND exits nonzero after emitting startup context, treat that as STARTUP WARNING state context; authority-file injection and role startup continue unless a required authority file is missing."
  Write-Output "5. After startup or startup warning, read and follow the required authority files listed below as a contract before claiming startup is complete or acting as the role."
  Write-Output "6. If any required authority file cannot be read, stop and report the missing file."
  Write-Output "7. When ready, acknowledge truthfully with: AUTHORITY_CONTRACT_ACK read_after_startup=yes files=AGENTS,CODEX,ORCHESTRATOR_PROTOCOL"
  Write-Output ""
  Write-Output "Required authority files:"
  foreach ($file in $authorityFiles) {
    Write-Output ("- {0}: {1}" -f $file.Key, $file.DisplayPath)
  }
}

function Write-AuthorityFiles {
  Write-Section "REQUIRED AUTHORITY FILES"

  foreach ($file in $authorityFiles) {
    Write-Output ("AUTHORITY_FILE_BEGIN key={0} path={1}" -f $file.Key, $file.DisplayPath)
    if (-not (Test-Path -LiteralPath $file.Path)) {
      Write-Output ("[orcstart] MISSING required authority file: {0}" -f $file.DisplayPath)
      $script:authorityExitCode = 1
    } else {
      Get-Content -LiteralPath $file.Path -Encoding UTF8 | ForEach-Object { Write-Output $_ }
    }
    Write-Output ("AUTHORITY_FILE_END key={0}" -f $file.Key)
    Write-Output ""
  }
}

foreach ($arg in $Args) {
  switch -Regex ($arg) {
    '^(--help|-help|/\?)$' {
      Show-Help
      exit 0
    }
    '^(--print|-print)$' {
      $printOnly = $true
      continue
    }
    '^(--no-startup|-nostartup|-NoStartup)$' {
      $noStartup = $true
      continue
    }
    '^(--no-authority-files|-noauthorityfiles|-NoAuthorityFiles)$' {
      $injectAuthorityFiles = $false
      continue
    }
    '^(--brief|-brief|-Brief)$' {
      $brief = $true
      $injectAuthorityFiles = $false
      continue
    }
    default {
      throw "[orcstart] Unknown argument: $arg"
    }
  }
}

$prompt = Get-StartupPrompt -Path $promptDoc -RoleName $role

if ($printOnly) {
  Write-Output $prompt
  exit 0
}

Set-Location -LiteralPath $repoRoot

Write-Section "ORCSTART BOOTSTRAP"
Write-Output "Purpose: inject the live Handshake $role startup prompt, startup command output, and authority-read contract into the current assistant conversation."
Write-Output "Repo: $repoDisplay"
Write-Output "Prompt source: $promptDocDisplay"
Write-Output "Role: $role"
Write-Output "Run from wt-gov-kernel with: .\orcstart.cmd"
Write-Output $startupTimeoutGuidance
Write-Output "Assistant instruction: treat the REPO GOVERNING RULE SET and ROLE STARTUP PROMPT below as the active repo-governed startup contract."

if (-not $brief) {
  Write-Output ""
  Write-Output "This command is model/provider agnostic. It does not start Codex, Claude, ChatGPT, or any other model process."
  Write-Output "Changing the fenced '$role - Startup Prompt' block in the prompt source changes this launcher output automatically."
}

Write-AuthorityContract

Write-Section "ROLE STARTUP PROMPT"
Write-Output $prompt

if ($noStartup) {
  Write-Section "FIRST COMMAND NOT RUN"
  Write-Output "Skipped by --no-startup."
  Write-Output "The startup prompt still requires: just orchestrator-startup"
  if ($injectAuthorityFiles) {
    Write-AuthorityFiles
  }
  exit $script:authorityExitCode
}

Invoke-StartupCommand
Write-StartupWarning

if ($injectAuthorityFiles) {
  Write-AuthorityFiles
}

$startupWarning = ($script:startupExitCode -ne 0)
$exitCode = if ($script:authorityExitCode -ne 0) { $script:authorityExitCode } else { 0 }

Write-Section "ORCSTART COMPLETE"
if ($script:authorityExitCode -ne 0) {
  Write-Output "Startup prompt was injected, but authority injection failed because at least one required authority file was missing."
  Write-Output "ROLE_STARTUP_CONTINUES: no"
} elseif (-not $startupWarning) {
  if ($injectAuthorityFiles) {
    Write-Output "Startup prompt, just orchestrator-startup output, authority-read contract, and required authority files were injected successfully."
  } else {
    Write-Output "Startup prompt, just orchestrator-startup output, and authority-read contract were injected successfully."
    Write-Output "AUTHORITY_FILES_EMBEDDED: no (disabled by option)"
  }
} else {
  if ($injectAuthorityFiles) {
    Write-Output "Startup prompt, startup warning, authority-read contract, and required authority files were injected successfully."
  } else {
    Write-Output "Startup prompt, startup warning, and authority-read contract were injected successfully."
    Write-Output "AUTHORITY_FILES_EMBEDDED: no (disabled by option)"
  }
  Write-Output ("FIRST_COMMAND_EXIT_CODE: {0}" -f $script:startupExitCode)
  Write-Output "ROLE_STARTUP_CONTINUES: yes"
}

exit $exitCode
