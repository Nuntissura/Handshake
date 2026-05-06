#!/usr/bin/env node
/**
 * Installs the post-commit MT auto-relay hook in a coder worktree.
 *
 * Usage: node install-mt-hook.mjs <WP_ID>
 *
 * The hook detects commits matching "feat: MT-NNN" and fires wp-review-request
 * to trigger the auto-relay loop.
 */

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { defaultCoderWorktreeDir } from "../../scripts/session/session-policy.mjs";
import { repoPathAbs } from "../../scripts/lib/runtime-paths.mjs";

const wpId = String(process.argv[2] || "").trim();

export function buildMtPostCommitHookScript() {
  return `#!/bin/sh
# Auto-installed by just install-mt-hook. Fires wp-review-request on MT commits.
node .GOV/roles_shared/scripts/hooks/post-commit-mt-review-request.mjs
`;
}

export function normalizeGitHookPath(gitPathOutput, worktreeDir) {
  const raw = String(gitPathOutput || "").trim();
  if (!raw) {
    throw new Error("git rev-parse --git-path hooks/post-commit returned an empty path");
  }
  return path.normalize(path.isAbsolute(raw) ? raw : path.resolve(worktreeDir, raw));
}

export function resolveEffectivePostCommitHookPath(worktreeDir, {
  execFileSyncImpl = execFileSync,
} = {}) {
  const output = execFileSyncImpl("git", [
    "-C",
    worktreeDir,
    "rev-parse",
    "--git-path",
    "hooks/post-commit",
  ], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });
  return normalizeGitHookPath(output, worktreeDir);
}

export function installMtHook({
  wpId,
  worktreeDir = repoPathAbs(defaultCoderWorktreeDir(wpId)),
  fsImpl = fs,
  execFileSyncImpl = execFileSync,
} = {}) {
  if (!wpId || !String(wpId).startsWith("WP-")) {
    throw new Error("Usage: node install-mt-hook.mjs <WP_ID>");
  }
  if (!fsImpl.existsSync(worktreeDir)) {
    throw new Error(`[INSTALL_MT_HOOK] Worktree not found: ${worktreeDir}`);
  }

  const hookPath = resolveEffectivePostCommitHookPath(worktreeDir, { execFileSyncImpl });
  fsImpl.mkdirSync(path.dirname(hookPath), { recursive: true });
  fsImpl.writeFileSync(hookPath, buildMtPostCommitHookScript(), { mode: 0o755 });
  return { hookPath, worktreeDir };
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  try {
    const result = installMtHook({ wpId });
    console.log(`[INSTALL_MT_HOOK] Installed post-commit hook: ${result.hookPath}`);
    console.log(`[INSTALL_MT_HOOK] Worktree: ${result.worktreeDir}`);
    console.log(`[INSTALL_MT_HOOK] WP: ${wpId}`);
    console.log(`[INSTALL_MT_HOOK] The hook fires wp-review-request on commits matching "feat: MT-NNN"`);
  } catch (error) {
    console.error(error?.message || String(error || ""));
    process.exit(1);
  }
}
