import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

function writeFile(targetPath, content) {
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.writeFileSync(targetPath, content, "utf8");
}

test("validator-report-structure-check scans folder packets instead of only flat packet files", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "validator-report-structure-"));
  const govRoot = path.join(tempRoot, ".GOV");
  const packetPath = path.join(govRoot, "task_packets", "WP-TEST-FOLDER-v1", "packet.md");

  writeFile(
    packetPath,
    [
      "# WP-TEST-FOLDER-v1",
      "",
      "- **Status:** Done",
      "- PACKET_FORMAT_VERSION: 2026-03-22",
      "- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3",
      "",
      "## VALIDATION_REPORTS",
      "",
    ].join("\n"),
  );

  const result = spawnSync(
    process.execPath,
    [path.join(".GOV", "roles", "validator", "checks", "validator-report-structure-check.mjs")],
    {
      cwd: process.cwd(),
      encoding: "utf8",
      env: {
        ...process.env,
        HANDSHAKE_GOV_ROOT: govRoot,
      },
    },
  );

  assert.equal(result.status, 1);
  assert.match(result.stderr, /WP-TEST-FOLDER-v1\/packet\.md:/i);
  assert.match(result.stderr, /VALIDATION_REPORTS missing\/empty for closed packet/i);
});
