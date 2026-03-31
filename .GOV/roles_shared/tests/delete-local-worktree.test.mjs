import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { execFileSync } from "node:child_process";

import { detachExternalGovLink } from "../scripts/topology/delete-local-worktree.mjs";

function makeDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function makeGovLink(linkPath, targetPath) {
  if (process.platform === "win32") {
    execFileSync("cmd", ["/c", "mklink", "/J", linkPath, targetPath], { stdio: "ignore" });
    return;
  }
  fs.symlinkSync(targetPath, linkPath, "junction");
}

test("detachExternalGovLink removes only the external .GOV link entry and keeps the target intact", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-delete-worktree-"));
  const worktreeDir = path.join(root, "wt-example");
  const govKernelDir = path.join(root, "wt-gov-kernel", ".GOV");
  const sentinelPath = path.join(govKernelDir, "sentinel.txt");
  const worktreeGovLink = path.join(worktreeDir, ".GOV");

  try {
    makeDir(worktreeDir);
    makeDir(govKernelDir);
    fs.writeFileSync(sentinelPath, "still here\n", "utf8");
    makeGovLink(worktreeGovLink, govKernelDir);

    const result = detachExternalGovLink(worktreeDir);

    assert.equal(result.detached, true);
    assert.equal(fs.existsSync(worktreeGovLink), false);
    assert.equal(fs.existsSync(govKernelDir), true);
    assert.equal(fs.readFileSync(sentinelPath, "utf8"), "still here\n");
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
