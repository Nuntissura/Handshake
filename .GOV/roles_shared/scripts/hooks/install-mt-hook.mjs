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
import { defaultCoderWorktreeDir } from "../../scripts/session/session-policy.mjs";
import { REPO_ROOT, repoPathAbs } from "../../scripts/lib/runtime-paths.mjs";

const wpId = String(process.argv[2] || "").trim();
if (!wpId || !wpId.startsWith("WP-")) {
  console.error("Usage: node install-mt-hook.mjs <WP_ID>");
  process.exit(1);
}

const worktreeDir = repoPathAbs(defaultCoderWorktreeDir(wpId));
if (!fs.existsSync(worktreeDir)) {
  console.error(`[INSTALL_MT_HOOK] Worktree not found: ${worktreeDir}`);
  process.exit(1);
}

// Resolve the actual git dir (worktrees use a .git file pointing to the real git dir)
const dotGitPath = path.join(worktreeDir, ".git");
let hooksDir;

if (fs.statSync(dotGitPath).isFile()) {
  // Worktree: .git is a file containing "gitdir: /path/to/real/git/dir"
  const gitdirContent = fs.readFileSync(dotGitPath, "utf8").trim();
  const match = gitdirContent.match(/^gitdir:\s*(.+)$/);
  if (!match) {
    console.error(`[INSTALL_MT_HOOK] Cannot parse .git file: ${dotGitPath}`);
    process.exit(1);
  }
  const realGitDir = path.isAbsolute(match[1]) ? match[1] : path.resolve(worktreeDir, match[1]);
  hooksDir = path.join(realGitDir, "hooks");
} else {
  hooksDir = path.join(dotGitPath, "hooks");
}

fs.mkdirSync(hooksDir, { recursive: true });

const hookScript = `#!/bin/sh
# Auto-installed by just install-mt-hook. Fires wp-review-request on MT commits.
node .GOV/roles_shared/scripts/hooks/post-commit-mt-review-request.mjs
`;

const hookPath = path.join(hooksDir, "post-commit");
fs.writeFileSync(hookPath, hookScript, { mode: 0o755 });
console.log(`[INSTALL_MT_HOOK] Installed post-commit hook: ${hookPath}`);
console.log(`[INSTALL_MT_HOOK] Worktree: ${worktreeDir}`);
console.log(`[INSTALL_MT_HOOK] WP: ${wpId}`);
console.log(`[INSTALL_MT_HOOK] The hook fires wp-review-request on commits matching "feat: MT-NNN"`);
