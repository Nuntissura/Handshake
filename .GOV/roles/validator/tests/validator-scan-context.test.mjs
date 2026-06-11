import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = path.resolve(".");
const VALIDATOR_SCAN = path.join(REPO_ROOT, ".GOV", "roles", "validator", "checks", "validator-scan.mjs");

function withTempProductCheckout(fn) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "validator-scan-hbr-"));
  try {
    const gitInit = spawnSync("git", ["init", "--quiet"], {
      cwd: root,
      encoding: "utf8",
    });
    assert.equal(gitInit.status, 0, gitInit.stderr);
    fs.mkdirSync(path.join(root, "src", "backend", "handshake_core", "src"), { recursive: true });
    fs.writeFileSync(path.join(root, "src", "backend", "handshake_core", "src", "lib.rs"), "pub fn validator_scan_fixture() {}\n", "utf8");
    fs.mkdirSync(path.join(root, "app", "src"), { recursive: true });
    fs.writeFileSync(path.join(root, "app", "src", "main.ts"), "export const validatorScanFixture = true;\n", "utf8");
    return fn(root);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

test("validator-scan reports context mismatch instead of crashing in governance kernel checkout", () => {
  const result = spawnSync(process.execPath, [path.join(".GOV", "roles", "validator", "checks", "validator-scan.mjs")], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  });

  assert.equal(result.status, 2, result.stderr);
  assert.match(result.stderr, /validator-scan: CONTEXT_MISMATCH/i);
  assert.match(result.stderr, /product target paths are unavailable/i);
  assert.doesNotMatch(result.stderr, /IO error for operation on/i);
  assert.doesNotMatch(result.stderr, /SQLite|node:sqlite/i);
});

test("validator-scan --packet delegates to HBR validator evidence scan", () => withTempProductCheckout((root) => {
  const packetPath = path.join(root, ".GOV", "task_packets", "WP-HBR-SCAN-HOOK-v1", "packet.json");
  writeJson(packetPath, {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: "WP-HBR-SCAN-HOOK-v1",
    acceptance_matrix: {
      schema_version: 1,
      hbr: [
        {
          hbr_id: "HBR-MAN-001",
          status: "PROVED",
          evidence_pointer: "artifact://missing-proof.json",
          validator_verdict: "PROVED",
        },
      ],
      hbr_not_applicable: [],
      hbr_evidence_results: [],
    },
  });

  const result = spawnSync(process.execPath, [
    VALIDATOR_SCAN,
    "--packet",
    packetPath,
    "--repo-root",
    root,
  ], {
    cwd: root,
    encoding: "utf8",
  });

  const packet = JSON.parse(fs.readFileSync(packetPath, "utf8"));

  assert.equal(result.status, 2, result.stderr);
  assert.match(result.stderr, /artifact evidence not found/);
  assert.doesNotMatch(result.stderr, /SQLite|node:sqlite/i);
  assert.equal(packet.acceptance_matrix.hbr[0].status, "STEER");
}));
