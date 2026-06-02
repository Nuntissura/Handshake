import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..", "..", "..");
const scriptPath = path.join(repoRoot, ".GOV", "roles_shared", "scripts", "automation-first-audit.mjs");

function makeMinimalTauriRepo() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-automation-audit-"));
  const src = path.join(root, "app", "src-tauri", "src");
  fs.mkdirSync(src, { recursive: true });
  fs.writeFileSync(
    path.join(src, "lib.rs"),
    `
#[tauri::command]
pub fn quiet_command() {}

pub fn invoke_handlers() {
    tauri::generate_handler![quiet_command];
}
`,
    "utf8",
  );
  return root;
}

function runAudit(args, cwd = repoRoot) {
  return spawnSync(process.execPath, [scriptPath, ...args], {
    cwd,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
}

test("automation-first-audit requires runtime evidence by default", () => {
  const root = makeMinimalTauriRepo();
  try {
    const result = runAudit(["--repo-root", root, "--json"]);
    assert.notEqual(result.status, 0, result.stdout + result.stderr);

    const report = JSON.parse(result.stdout);
    assert.equal(report.runtime_probe.required, true);
    assert.equal(report.runtime_probe.evidence_present, false);
    assert.equal(report.status, "FAIL");
    assert.equal(
      report.violations.some((violation) =>
        violation.code === "RUNTIME_PROBE_EVIDENCE_REQUIRED"
        && violation.command === "quiet_command",
      ),
      true,
    );
    assert.equal(report.commands[0].evidence_source, "static_source_scan");
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("automation-first-audit static scan requires explicit non-certifying flag", () => {
  const root = makeMinimalTauriRepo();
  try {
    const result = runAudit(["--repo-root", root, "--json", "--static-source-scan-ok"]);
    assert.equal(result.status, 0, result.stdout + result.stderr);

    const report = JSON.parse(result.stdout);
    assert.equal(report.certification_mode, "static_source_scan_explicit");
    assert.equal(report.runtime_probe.required, false);
    assert.equal(report.runtime_probe.static_source_scan_allowed, true);
    assert.equal(report.runtime_probe.evidence_present, false);
    assert.equal(report.status, "PASS");
    assert.equal(report.commands[0].evidence_source, "static_source_scan");
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("automation-first-audit default certifying mode passes with measured command evidence", () => {
  const root = makeMinimalTauriRepo();
  try {
    const evidencePath = path.join(root, "runtime-probe-evidence.json");
    fs.writeFileSync(
      evidencePath,
      JSON.stringify({
        schema_id: "hsk.automation_first_runtime_probe_evidence@1",
        platform: "non-windows",
        focus_audit_measured: false,
        keyboard_injection_measured: false,
        commands: {
          quiet_command: {
            ipc_mock_call_ok: true,
            focus_steal_event_count: 0,
            keyboard_injection_invocation_count: 0,
          },
        },
      }),
      "utf8",
    );

    const result = runAudit([
      "--repo-root", root,
      "--json",
      "--runtime-probe-evidence", evidencePath,
    ]);
    assert.equal(result.status, 0, result.stdout + result.stderr);

    const report = JSON.parse(result.stdout);
    assert.equal(report.certification_mode, "runtime_probe_required");
    assert.equal(report.runtime_probe.required, true);
    assert.equal(report.runtime_probe.static_source_scan_allowed, false);
    assert.equal(report.runtime_probe.evidence_present, true);
    assert.equal(report.runtime_probe.measured_command_count, 1);
    assert.equal(report.status, "PASS");
    assert.equal(report.commands[0].evidence_source, "runtime_probe_measured");
    assert.deepEqual(report.violations, []);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("automation-first-audit rejects same-schema evidence missing measured counters", () => {
  const root = makeMinimalTauriRepo();
  try {
    const evidencePath = path.join(root, "runtime-probe-evidence-malformed.json");
    fs.writeFileSync(
      evidencePath,
      JSON.stringify({
        schema_id: "hsk.automation_first_runtime_probe_evidence@1",
        platform: "non-windows",
        focus_audit_measured: false,
        keyboard_injection_measured: false,
        commands: {
          quiet_command: {
            ipc_mock_call_ok: true,
          },
        },
      }),
      "utf8",
    );

    const result = runAudit([
      "--repo-root", root,
      "--json",
      "--runtime-probe-evidence", evidencePath,
    ]);
    assert.notEqual(result.status, 0, result.stdout + result.stderr);
    assert.match(result.stderr, /missing non-negative integer focus_steal_event_count/);
    assert.equal(result.stdout.trim(), "");
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("automation-first-audit rejects same-schema evidence with failed ipc mock", () => {
  const root = makeMinimalTauriRepo();
  try {
    const evidencePath = path.join(root, "runtime-probe-evidence-ipc-failed.json");
    fs.writeFileSync(
      evidencePath,
      JSON.stringify({
        schema_id: "hsk.automation_first_runtime_probe_evidence@1",
        platform: "non-windows",
        focus_audit_measured: false,
        keyboard_injection_measured: false,
        commands: {
          quiet_command: {
            ipc_mock_call_ok: false,
            focus_steal_event_count: 0,
            keyboard_injection_invocation_count: 0,
          },
        },
      }),
      "utf8",
    );

    const result = runAudit([
      "--repo-root", root,
      "--json",
      "--runtime-probe-evidence", evidencePath,
    ]);
    assert.notEqual(result.status, 0, result.stdout + result.stderr);
    assert.match(result.stderr, /ipc_mock_call_ok must be true/);
    assert.equal(result.stdout.trim(), "");
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
