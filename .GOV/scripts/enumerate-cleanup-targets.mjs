#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import {
  PROTECTED_BRANCHES,
  WORKSPACE_ROOT,
  absFromRepo,
  compareStrings,
  currentBranchInRepo,
  discoverGitCheckouts,
  gitCheckoutExists,
  listLocalBranches,
  listRemoteHeads,
} from "./git-topology-lib.mjs";

const protectedWorktreeIds = new Set(["handshake_main", "wt-ilja", "wt-orchestrator", "wt-validator"]);
const checkoutDirs = discoverGitCheckouts();

const localBranchCandidates = [];
for (const checkout of checkoutDirs) {
  const current = currentBranchInRepo(checkout.abs_dir);
  for (const branch of listLocalBranches(checkout.abs_dir)) {
    if (PROTECTED_BRANCHES.includes(branch)) continue;
    localBranchCandidates.push({
      checkout: checkout.id,
      branch,
      current: branch === current,
    });
  }
}
localBranchCandidates.sort((a, b) => {
  const byCheckout = compareStrings(a.checkout, b.checkout);
  if (byCheckout !== 0) return byCheckout;
  return compareStrings(a.branch, b.branch);
});

const worktreeCandidates = fs.readdirSync(WORKSPACE_ROOT, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .filter((entry) => !protectedWorktreeIds.has(entry.name))
  .filter((entry) => entry.name.startsWith("wt-"))
  .map((entry) => ({
    id: entry.name,
    rel_path: path.relative(absFromRepo("."), path.join(WORKSPACE_ROOT, entry.name)).replace(/\\/g, "/"),
    is_git_checkout: gitCheckoutExists(path.join(WORKSPACE_ROOT, entry.name)),
  }))
  .sort((a, b) => compareStrings(a.id, b.id));

const remoteBranchCandidates = listRemoteHeads()
  .filter((row) => !PROTECTED_BRANCHES.includes(row.branch))
  .sort((a, b) => compareStrings(a.branch, b.branch));

console.log("CLEANUP_TARGETS");
console.log("- Local worktree candidates:");
if (worktreeCandidates.length === 0) {
  console.log("  - NONE");
} else {
  for (const row of worktreeCandidates) {
    console.log(`  - ${row.id} | rel_path=${row.rel_path} | git_checkout=${row.is_git_checkout ? "YES" : "NO"} | approval_example=APPROVE DELETE LOCAL WORKTREE ${row.id}`);
  }
}

console.log("- Local branch candidates:");
if (localBranchCandidates.length === 0) {
  console.log("  - NONE");
} else {
  for (const row of localBranchCandidates) {
    console.log(`  - checkout=${row.checkout} | branch=${row.branch} | current=${row.current ? "YES" : "NO"} | approval_example=APPROVE DELETE LOCAL BRANCH ${row.branch}`);
  }
}

console.log("- Remote branch candidates:");
if (remoteBranchCandidates.length === 0) {
  console.log("  - NONE");
} else {
  for (const row of remoteBranchCandidates) {
    console.log(`  - branch=${row.branch} | sha=${row.sha} | approval_example=APPROVE DELETE REMOTE BRANCH ${row.branch}`);
  }
}

console.log("- Fast-forward examples:");
for (const branch of PROTECTED_BRANCHES.filter((name) => name !== "main")) {
  console.log(`  - APPROVE FAST_FORWARD REMOTE BRANCH ${branch} TO main`);
}
