import assert from "node:assert/strict";
import test from "node:test";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const TEST_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(TEST_DIR, "../../..");
const LIB_PATH = path.resolve(REPO_ROOT, ".GOV/roles_shared/scripts/lib/fail-capture-lib.mjs");

test("registerFailCaptureHook installs only one process listener pair", () => {
  const script = `
    const before = {
      uncaughtException: process.listenerCount("uncaughtException"),
      unhandledRejection: process.listenerCount("unhandledRejection"),
    };
    const lib = await import(${JSON.stringify(`file://${LIB_PATH.replace(/\\/g, "/")}`)});
    lib.registerFailCaptureHook("script-a.mjs", { role: "SHARED" });
    lib.registerFailCaptureHook("script-b.mjs", { role: "SHARED" });
    lib.registerFailCaptureHook("script-a.mjs", { role: "SHARED" });
    const after = {
      uncaughtException: process.listenerCount("uncaughtException"),
      unhandledRejection: process.listenerCount("unhandledRejection"),
    };
    console.log(JSON.stringify({ before, after }));
  `;
  const result = spawnSync(process.execPath, ["--input-type=module", "-e", script], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  });

  assert.equal(result.status, 0, result.stderr);
  const payload = JSON.parse(String(result.stdout || "").trim());
  assert.equal(payload.after.uncaughtException - payload.before.uncaughtException, 1);
  assert.equal(payload.after.unhandledRejection - payload.before.unhandledRejection, 1);
});
