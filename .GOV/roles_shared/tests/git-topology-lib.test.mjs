import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  formatProtectedWorktreeResolutionDiagnostics,
  parseGitWorktreeListPorcelain,
  resolveProtectedWorktree,
} from "../scripts/topology/git-topology-lib.mjs";

function runGit(repoDir, args) {
  return execFileSync("git", args, {
    cwd: repoDir,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function runGitQuiet(repoDir, args) {
  execFileSync("git", args, {
    cwd: repoDir,
    stdio: "ignore",
  });
}

function writeFile(filePath, contents) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, contents, "utf8");
}

test("parseGitWorktreeListPorcelain extracts paths, heads, and normalized branches", () => {
  const entries = parseGitWorktreeListPorcelain([
    "worktree C:/repo/wt-gov-kernel",
    "HEAD 1111111111111111111111111111111111111111",
    "branch refs/heads/gov_kernel",
    "",
    "worktree C:/repo/handshake_main",
    "HEAD 2222222222222222222222222222222222222222",
    "branch refs/heads/main",
    "",
  ].join("\n"));

  assert.deepEqual(entries, [
    {
      path: "C:/repo/wt-gov-kernel",
      head: "1111111111111111111111111111111111111111",
      branch: "gov_kernel",
      bare: false,
      detached: false,
    },
    {
      path: "C:/repo/handshake_main",
      head: "2222222222222222222222222222222222222222",
      branch: "main",
      bare: false,
      detached: false,
    },
  ]);
});

test("resolveProtectedWorktree prefers git worktree branch truth over configured sibling guesses", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-topology-resolve-"));
  const repoDir = path.join(root, "wt-gov-kernel");
  const mainDir = path.join(root, "custom-main-location");
  try {
    fs.mkdirSync(repoDir, { recursive: true });
    runGitQuiet(repoDir, ["init", "--initial-branch=gov_kernel"]);
    runGitQuiet(repoDir, ["config", "user.name", "Handshake Test"]);
    runGitQuiet(repoDir, ["config", "user.email", "handshake-test@example.com"]);
    writeFile(path.join(repoDir, "README.md"), "base\n");
    runGitQuiet(repoDir, ["add", "README.md"]);
    runGitQuiet(repoDir, ["commit", "-m", "seed"]);
    runGitQuiet(repoDir, ["branch", "main"]);
    runGitQuiet(repoDir, ["worktree", "add", mainDir, "main"]);

    const resolution = resolveProtectedWorktree({
      id: "handshake_main",
      rel_path: "../handshake_main",
      role: "CANONICAL",
      local_branch: "main",
      remote_branch: "origin/main",
    }, { repoRoot: repoDir });

    assert.equal(fs.realpathSync.native(resolution.absDir), fs.realpathSync.native(mainDir));
    assert.equal(resolution.source, "git-worktree-branch");
    assert.equal(resolution.ok, true);
    assert.equal(resolution.currentBranch, "main");

    const diagnostics = formatProtectedWorktreeResolutionDiagnostics(resolution).join("\n");
    assert.match(diagnostics, /configured_path=.*handshake_main/);
    assert.match(diagnostics, /resolved_path=.*custom-main-location/);
    assert.match(diagnostics, /discovered_worktrees:/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
