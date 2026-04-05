import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { execFileSync } from "node:child_process";

import {
  clearSharedGovJunctionSuppression,
  suppressSharedGovJunctionDirt,
} from "../scripts/topology/reseed-permanent-worktree-from-main.mjs";

function runGit(repoDir, args, options = {}) {
  return execFileSync("git", args, {
    cwd: repoDir,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    ...options,
  }).trim();
}

function runGitInherit(repoDir, args) {
  execFileSync("git", args, {
    cwd: repoDir,
    stdio: "ignore",
  });
}

function writeFile(filePath, content) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content, "utf8");
}

function makeGovLink(linkPath, targetPath) {
  if (process.platform === "win32") {
    execFileSync("cmd", ["/c", "mklink", "/J", linkPath, targetPath], { stdio: "ignore" });
    return;
  }
  fs.symlinkSync(targetPath, linkPath, "junction");
}

test("shared .GOV suppression hides tracked and untracked kernel-junction dirt in non-main worktrees", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-reseed-permanent-"));
  const repoDir = path.join(root, "wt-ilja");
  const kernelGovDir = path.join(root, "wt-gov-kernel", ".GOV");
  const trackedGovPath = path.join(repoDir, ".GOV", "tracked.txt");
  const kernelTrackedPath = path.join(kernelGovDir, "tracked.txt");
  const kernelUntrackedPath = path.join(kernelGovDir, "untracked.txt");

  try {
    fs.mkdirSync(repoDir, { recursive: true });
    runGitInherit(repoDir, ["init", "--initial-branch=main"]);
    runGitInherit(repoDir, ["config", "user.name", "Handshake Test"]);
    runGitInherit(repoDir, ["config", "user.email", "handshake-test@example.com"]);

    writeFile(path.join(repoDir, "README.md"), "base\n");
    writeFile(trackedGovPath, "tracked from main\n");
    runGitInherit(repoDir, ["add", "README.md", ".GOV/tracked.txt"]);
    runGitInherit(repoDir, ["commit", "-m", "seed"]);

    fs.rmSync(path.join(repoDir, ".GOV"), { recursive: true, force: true });
    writeFile(kernelTrackedPath, "tracked from kernel\n");
    writeFile(kernelUntrackedPath, "new kernel file\n");
    makeGovLink(path.join(repoDir, ".GOV"), kernelGovDir);

    assert.notEqual(runGit(repoDir, ["status", "--short"]), "", "expected raw shared .GOV dirt before suppression");

    suppressSharedGovJunctionDirt(repoDir);

    assert.equal(runGit(repoDir, ["status", "--short"]), "");
    const excludePath = runGit(repoDir, ["rev-parse", "--git-path", "info/exclude"]);
    assert.match(fs.readFileSync(path.resolve(repoDir, excludePath), "utf8"), /\.GOV\//);

    clearSharedGovJunctionSuppression(repoDir);

    assert.notEqual(
      runGit(repoDir, ["status", "--short"]),
      "",
      "expected shared .GOV dirt to return once local suppression is cleared",
    );
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
