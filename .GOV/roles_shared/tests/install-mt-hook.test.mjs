import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  installMtHook,
  normalizeGitHookPath,
} from "../scripts/hooks/install-mt-hook.mjs";

test("normalizeGitHookPath preserves an absolute effective hook path", () => {
  const hookPath = path.resolve(os.tmpdir(), "handshake-hook-test", ".git", "hooks", "post-commit");
  assert.equal(
    normalizeGitHookPath(`${hookPath}\n`, path.resolve(os.tmpdir(), "handshake-hook-test", "wtc-demo")),
    path.normalize(hookPath),
  );
});

test("installMtHook writes to git rev-parse effective hook path for linked worktrees", (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-install-mt-hook-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const worktreeDir = path.join(root, "wtc-demo");
  const commonHookPath = path.join(root, ".git", "hooks", "post-commit");
  fs.mkdirSync(worktreeDir, { recursive: true });

  const result = installMtHook({
    wpId: "WP-1-Demo-v1",
    worktreeDir,
    execFileSyncImpl(command, args) {
      assert.equal(command, "git");
      assert.deepEqual(args, ["-C", worktreeDir, "rev-parse", "--git-path", "hooks/post-commit"]);
      return `${commonHookPath}\n`;
    },
  });

  assert.equal(result.hookPath, path.normalize(commonHookPath));
  assert.match(fs.readFileSync(commonHookPath, "utf8"), /post-commit-mt-review-request\.mjs/);
});
