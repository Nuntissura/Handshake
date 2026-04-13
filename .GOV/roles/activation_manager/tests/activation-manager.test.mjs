import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const TEST_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(TEST_DIR, "../../../..");
const SCRIPT_PATH = path.resolve(TEST_DIR, "../scripts/activation-manager.mjs");
const TASK_PACKETS_ROOT = path.resolve(REPO_ROOT, ".GOV/task_packets");

function findSampleWp() {
  if (!fs.existsSync(TASK_PACKETS_ROOT)) return "";
  const entries = fs.readdirSync(TASK_PACKETS_ROOT, { withFileTypes: true });
  for (const entry of entries) {
    if (!entry.name.startsWith("WP-")) continue;
    if (entry.isDirectory()) {
      const packetPath = path.join(TASK_PACKETS_ROOT, entry.name, "packet.md");
      const refinementPath = path.join(TASK_PACKETS_ROOT, entry.name, "refinement.md");
      if (fs.existsSync(packetPath) && fs.existsSync(refinementPath)) {
        return entry.name;
      }
      continue;
    }
    if (!entry.isFile() || !entry.name.endsWith(".md")) continue;
    const wpId = entry.name.replace(/\.md$/i, "");
    const refinementPath = path.resolve(REPO_ROOT, `.GOV/refinements/${wpId}.md`);
    if (fs.existsSync(refinementPath)) {
      return wpId;
    }
  }
  return "";
}

function runCli(args) {
  return spawnSync(process.execPath, [SCRIPT_PATH, ...args], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  });
}

test("activation-manager startup prints the role startup brief", () => {
  const result = runCli(["startup"]);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /ACTIVATION_MANAGER_STARTUP/);
  assert.match(result.stdout, /just activation-manager prompt WP-\{ID\}/);
  assert.match(result.stdout, /just activation-manager record-refinement WP-\{ID\}/);
  assert.match(result.stdout, /just activation-manager prepare-and-packet WP-\{ID\}/);
  assert.match(result.stdout, /ACTIVATION_MANAGER as the mandatory temporary pre-launch worker/i);
  assert.match(result.stdout, /REFINEMENT_STANDARD:/);
  assert.match(result.stdout, /HANDOFF_MODE:/);
  assert.match(result.stdout, /EXCERPT_FALLBACK_RULE:/);
  assert.match(result.stdout, /HANDOFF_SUMMARY_REQUIRED:/);
  assert.match(result.stdout, /REFINEMENT_CHECK_RULE:/);
  assert.match(result.stdout, /UPGRADE_DISCIPLINE:/);
});

test("activation-manager prompt and next produce WP-scoped guidance", () => {
  const wpId = findSampleWp();
  assert.ok(wpId, "expected at least one packet/refinement pair in .GOV/task_packets");

  const promptResult = runCli(["prompt", wpId]);
  assert.equal(promptResult.status, 0, promptResult.stderr);
  assert.match(promptResult.stdout, new RegExp(`WP_ID: ${wpId}`));
  assert.match(promptResult.stdout, /ROLE LOCK: You are the ACTIVATION_MANAGER\./);
  assert.match(promptResult.stdout, /REFINEMENT STANDARD:/);
  assert.match(promptResult.stdout, /FILE-FIRST HANDOFF RULE:/);
  assert.match(promptResult.stdout, /REFINEMENT_HANDOFF_SUMMARY REQUIRED FIELDS:/);
  assert.match(promptResult.stdout, /REFINEMENT_CHECK RULE:/);
  assert.match(promptResult.stdout, /UPGRADE DISCIPLINE:/);
  assert.match(promptResult.stdout, /EXCERPT FALLBACK RULE:/);
  assert.match(promptResult.stdout, /SIGNATURE ROUND-TRIP:/);
  assert.match(promptResult.stdout, new RegExp(`just activation-manager record-refinement ${wpId}`));

  const nextResult = runCli(["next", wpId, "--json"]);
  assert.equal(nextResult.status, 0, nextResult.stderr);
  const payload = JSON.parse(nextResult.stdout);
  assert.equal(payload.wpId, wpId);
  assert.ok(typeof payload.verdict === "string" && payload.verdict.length > 0);
  assert.ok(Array.isArray(payload.findings));
});

test("activation-manager readiness renders the structured readiness contract", () => {
  const wpId = findSampleWp();
  assert.ok(wpId, "expected at least one packet/refinement pair in .GOV/task_packets");

  const result = runCli(["readiness", wpId]);
  assert.ok([0, 2].includes(result.status ?? 1), result.stderr);
  assert.match(result.stdout, /ACTIVATION_READINESS/);
  assert.match(result.stdout, new RegExp(`- WP_ID: ${wpId}`));
  assert.match(result.stdout, /- VERDICT: /);
  assert.match(result.stdout, /- LOCAL_WORKTREE_DIR: /);
  assert.match(result.stdout, /- GOV_KERNEL_LINK: /);
  assert.match(result.stdout, /- MICROTASK_STATUS: /);
  assert.match(result.stdout, /- wp-declared-topology-check: /);
  assert.match(result.stdout, /- NEXT_ORCHESTRATOR_ACTION: /);
});

test("activation-manager fail capture stays attributed to ACTIVATION_MANAGER", () => {
  const source = fs.readFileSync(SCRIPT_PATH, "utf8");
  assert.match(source, /registerFailCaptureHook\("activation-manager\.mjs", \{ role: "ACTIVATION_MANAGER" \}\);/);
  assert.match(source, /failWithMemory\("activation-manager\.mjs", message, \{ role: "ACTIVATION_MANAGER", details \}\);/);
});
