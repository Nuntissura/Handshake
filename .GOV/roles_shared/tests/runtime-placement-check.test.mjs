import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const RUNTIME_ROOT = path.resolve(".GOV", "roles_shared", "runtime");
const CHECK_SCRIPT = path.resolve(".GOV", "roles_shared", "checks", "runtime-placement-check.mjs");

function writeFile(filePath, content = "") {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content, "utf8");
}

test("repo-local roles_shared runtime bucket contains only the sanctioned local exceptions", () => {
  const entries = fs.readdirSync(RUNTIME_ROOT).sort();
  assert.deepEqual(entries, [
    "PRODUCT_GOVERNANCE_SNAPSHOT.json",
    "validator_gates",
  ]);
});

test("runtime-placement-check passes when repo-local runtime residue is cleaned", () => {
  const result = spawnSync(
    process.execPath,
    [CHECK_SCRIPT],
    {
      cwd: process.cwd(),
      encoding: "utf8",
    },
  );

  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /runtime-placement-check ok/i);
});

test("runtime-placement-check follows HANDSHAKE_GOV_ROOT instead of the active repo root", () => {
  const activeRepoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "runtime-placement-active-"));
  const govRepoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "runtime-placement-gov-"));

  try {
    const activeGitInit = spawnSync("git", ["init"], {
      cwd: activeRepoRoot,
      encoding: "utf8",
    });
    assert.equal(activeGitInit.status, 0, activeGitInit.stderr || activeGitInit.stdout);

    writeFile(path.join(activeRepoRoot, ".GOV", "roles_shared", "runtime", "GIT_TOPOLOGY_REGISTRY.json"), "{}\n");
    writeFile(path.join(activeRepoRoot, ".GOV", "roles_shared", "runtime", "SESSION_LAUNCH_REQUESTS.jsonl"), "{}\n");
    writeFile(path.join(activeRepoRoot, ".GOV", "roles_shared", "runtime", "SESSION_CONTROL_OUTPUTS", "stale.jsonl"), "{}\n");

    writeFile(path.join(govRepoRoot, ".GOV", "roles_shared", "runtime", "PRODUCT_GOVERNANCE_SNAPSHOT.json"), "{}\n");
    fs.mkdirSync(path.join(govRepoRoot, ".GOV", "roles_shared", "runtime", "validator_gates"), { recursive: true });

    const result = spawnSync(
      process.execPath,
      [CHECK_SCRIPT],
      {
        cwd: activeRepoRoot,
        encoding: "utf8",
        env: {
          ...process.env,
          HANDSHAKE_GOV_ROOT: path.join(govRepoRoot, ".GOV"),
        },
      },
    );

    assert.equal(result.status, 0, result.stderr || result.stdout);
    assert.match(result.stdout, /runtime-placement-check ok/i);
  } finally {
    fs.rmSync(activeRepoRoot, { recursive: true, force: true });
    fs.rmSync(govRepoRoot, { recursive: true, force: true });
  }
});
