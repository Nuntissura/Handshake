#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import {
  WORKTREE_SPECS,
  absFromRepo,
  currentBranchInRepo,
  dirtyInRepo,
  gitCheckoutExists,
  localBranchExists,
  runGitInherit,
} from "./git-topology-lib.mjs";

for (const spec of WORKTREE_SPECS) {
  // Skip the governance kernel: it flows to main via sync-gov-to-main, not via FF merge.
  if (spec.role === "GOV_KERNEL") {
    console.log(`[SYNC_ALL_ROLE_WORKTREES] skip governance kernel: ${spec.id}`);
    continue;
  }

  const absDir = absFromRepo(spec.rel_path);
  if (!fs.existsSync(absDir) || !gitCheckoutExists(absDir)) {
    console.log(`[SYNC_ALL_ROLE_WORKTREES] skip missing checkout: ${spec.id} (${spec.rel_path})`);
    continue;
  }

  if (dirtyInRepo(absDir)) {
    throw new Error(`Refusing to sync dirty checkout: ${spec.id} (${spec.rel_path})`);
  }

  const originalBranch = currentBranchInRepo(absDir);
  runGitInherit(absDir, ["fetch", "origin"]);

  if (localBranchExists(absDir, "main")) {
    if (originalBranch !== "main") {
      runGitInherit(absDir, ["checkout", "main"]);
    }
    runGitInherit(absDir, ["merge", "--ff-only", "origin/main"]);
  } else {
    console.log(`[SYNC_ALL_ROLE_WORKTREES] skip local main refresh for ${spec.id}: no local main branch`);
  }

  if (originalBranch && originalBranch !== "main" && localBranchExists(absDir, originalBranch)) {
    runGitInherit(absDir, ["checkout", originalBranch]);
  }

  if (["OPERATOR", "ORCHESTRATOR"].includes(spec.role)) {
    console.log(
      `[SYNC_ALL_ROLE_WORKTREES] note: ${spec.id} role branch ${spec.local_branch} was left unchanged; `
      + `use just reseed-permanent-worktree-from-main ${spec.id} "<approval>" to reseed it from local main`
    );
  }

  console.log(`[SYNC_ALL_ROLE_WORKTREES] ok: ${spec.id}`);
}
