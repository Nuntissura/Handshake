import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { execFileSync } from "node:child_process";

import {
  ensureGovKernelTracksGov,
  inspectGovTrackingMode,
  ensureGovWorktreeExclude,
  clearSharedGovJunctionSuppression,
  normalizeGovTrackingMode,
  setGovTrackedPathsSkipWorktree,
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

    const suppressed = normalizeGovTrackingMode(repoDir);
    assert.equal(suppressed.mode, "SUPPRESS_SHARED_GOV");
    assert.equal(suppressed.sharedGovJunction, true);
    assert.equal(suppressed.tracksGov, false);

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

test("kernel-style worktrees with a real .GOV directory clear leaked shared-junction suppression", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-reseed-permanent-kernel-"));
  const repoDir = path.join(root, "wt-gov-kernel");
  const trackedGovPath = path.join(repoDir, ".GOV", "tracked.txt");
  const untrackedGovPath = path.join(repoDir, ".GOV", "untracked.txt");

  try {
    fs.mkdirSync(repoDir, { recursive: true });
    runGitInherit(repoDir, ["init", "--initial-branch=gov_kernel"]);
    runGitInherit(repoDir, ["config", "user.name", "Handshake Test"]);
    runGitInherit(repoDir, ["config", "user.email", "handshake-test@example.com"]);

    writeFile(path.join(repoDir, "README.md"), "base\n");
    writeFile(trackedGovPath, "tracked from kernel\n");
    runGitInherit(repoDir, ["add", "README.md", ".GOV/tracked.txt"]);
    runGitInherit(repoDir, ["commit", "-m", "seed"]);

    writeFile(trackedGovPath, "tracked mutation\n");
    writeFile(untrackedGovPath, "new governance file\n");

    ensureGovWorktreeExclude(repoDir);
    setGovTrackedPathsSkipWorktree(repoDir, true);
    assert.equal(runGit(repoDir, ["status", "--short"]), "", "expected leaked suppression to hide real .GOV dirt");
    const leaked = inspectGovTrackingMode(repoDir);
    assert.equal(leaked.mode, "TRACK_GOV");
    assert.equal(leaked.sharedGovJunction, false);
    assert.equal(leaked.tracksGov, true);

    const normalized = normalizeGovTrackingMode(repoDir);
    assert.equal(normalized.mode, "TRACK_GOV");
    assert.equal(normalized.sharedGovJunction, false);
    assert.equal(normalized.tracksGov, true);
    assert.deepEqual(ensureGovKernelTracksGov(repoDir), { normalized: true, sharedGovJunction: false });
    assert.notEqual(
      runGit(repoDir, ["status", "--short"]),
      "",
      "expected real .GOV dirt to reappear after kernel normalization",
    );

    const excludePath = runGit(repoDir, ["rev-parse", "--git-path", "info/exclude"]);
    const excludeText = fs.readFileSync(path.resolve(repoDir, excludePath), "utf8");
    assert.doesNotMatch(excludeText, /HANDSHAKE_SHARED_GOV_JUNCTION/);
    assert.doesNotMatch(excludeText, /\.GOV\//);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
