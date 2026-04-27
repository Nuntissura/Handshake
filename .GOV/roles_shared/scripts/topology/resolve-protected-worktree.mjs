#!/usr/bin/env node

import {
  formatProtectedWorktreeResolutionDiagnostics,
  resolveProtectedWorktree,
  toPosix,
} from "./git-topology-lib.mjs";

const id = String(process.argv[2] || "").trim();
const pathOnly = process.argv.slice(3).includes("--path-only");

if (!id) {
  console.error("Usage: node .GOV/roles_shared/scripts/topology/resolve-protected-worktree.mjs <handshake_main|wt-gov-kernel|wt-ilja> [--path-only]");
  process.exit(1);
}

const resolution = resolveProtectedWorktree(id);
if (!resolution.ok) {
  if (pathOnly) {
    console.error(`PROTECTED_WORKTREE_RESOLVE_FAIL: ${resolution.reason || id}`);
  } else {
    console.error("PROTECTED_WORKTREE_RESOLVE_FAIL");
    for (const line of formatProtectedWorktreeResolutionDiagnostics(resolution)) {
      console.error(line);
    }
  }
  process.exit(1);
}

if (pathOnly) {
  console.log(resolution.absDir);
} else {
  console.log("PROTECTED_WORKTREE_RESOLUTION");
  console.log(`- id: ${resolution.id}`);
  console.log(`- expected_branch: ${resolution.expectedBranch}`);
  console.log(`- resolved_path: ${toPosix(resolution.absDir)}`);
  console.log(`- source: ${resolution.source}`);
}
