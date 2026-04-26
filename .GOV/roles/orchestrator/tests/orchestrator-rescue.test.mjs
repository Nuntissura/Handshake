import assert from "node:assert/strict";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  buildOrchestratorRescuePrompt,
  buildRescuePowershellScript,
  buildVisibleLaunchPlan,
  rescueScriptPath,
} from "../scripts/orchestrator-rescue-lib.mjs";

test("rescue prompt preserves orchestrator role lock and visible takeover guard", () => {
  const prompt = buildOrchestratorRescuePrompt({ wpId: "WP-TEST-v1" });

  assert.match(prompt, /ROLE LOCK: You are the ORCHESTRATOR/);
  assert.match(prompt, /FIRST COMMAND: just orchestrator-startup/);
  assert.match(prompt, /VISIBLE_RESCUE_EXCEPTION/);
  assert.match(prompt, /just orchestrator-health WP-TEST-v1/);
  assert.match(prompt, /RESCUE_SINGLE_AUTHORITY_GUARD/);
});

test("rescue script runs health before launching codex and leaves manual fallback text", () => {
  const script = buildRescuePowershellScript({
    repoRoot: "D:/repo/wt-gov-kernel",
    wpId: "WP-TEST-v1",
    prompt: "ROLE LOCK: You are the ORCHESTRATOR.",
  });

  assert.match(script, /Set-Location -LiteralPath \$repoRoot/);
  assert.match(script, /just orchestrator-health \$wpId/);
  assert.match(script, /codex @codexArgs/);
  assert.match(script, /manual prompt follows/);
});

test("visible launch plan prefers Windows Terminal then PowerShell then manual script", () => {
  assert.deepEqual(buildVisibleLaunchPlan({
    platform: "win32",
    wtAvailable: true,
    powershellAvailable: true,
  }), ["windows-terminal", "visible-powershell", "manual-script"]);
  assert.deepEqual(buildVisibleLaunchPlan({
    platform: "linux",
    wtAvailable: true,
    powershellAvailable: true,
  }), ["manual-script"]);
});

test("rescueScriptPath is stable and filesystem safe", () => {
  const file = rescueScriptPath(os.tmpdir(), "WP-TEST/v1", new Date("2026-04-26T10:11:12.000Z"));
  assert.equal(path.basename(file), "handshake-orchestrator-rescue-WP-TEST_v1-20260426-101112Z.ps1");
});
