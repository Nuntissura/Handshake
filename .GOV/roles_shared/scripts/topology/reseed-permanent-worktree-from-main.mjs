#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  REPO_ROOT,
  WORKTREE_SPECS,
  absFromRepo,
  currentBranchInRepo,
  dirtyInRepo,
  gitCheckoutExists,
  localBranchExists,
  refExists,
  runGitInherit,
} from "./git-topology-lib.mjs";

const RESEEDABLE_WORKTREE_ROLES = new Set(["OPERATOR"]);
const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));

function fail(message, details = []) {
  console.error(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function usage() {
  fail(
    "Usage: node .GOV/roles_shared/scripts/topology/reseed-permanent-worktree-from-main.mjs <WORKTREE_ID> --approve \"approved|proceed\" [--label <snapshot-label>]",
    [
      "This helper safety-pushes the matching backup branch, creates an immutable snapshot, resets the local role/user branch to local main, and repairs the .GOV junction.",
      "Supported permanent worktrees: wt-ilja",
    ],
  );
}

function normalizeApproval(value) {
  return String(value || "").trim().toLowerCase();
}

function parseArgs() {
  const worktreeId = String(process.argv[2] || "").trim();
  if (!worktreeId) usage();

  let approval = "";
  let label = "";
  const args = process.argv.slice(3);
  for (let index = 0; index < args.length; index += 1) {
    const token = String(args[index] || "").trim();
    if (!token) continue;
    const next = () => {
      const value = String(args[index + 1] || "").trim();
      if (!value) usage();
      index += 1;
      return value;
    };

    if (token === "--approve") {
      approval = next();
      continue;
    }
    if (token === "--label") {
      label = next();
      continue;
    }
    fail("Unknown argument", [`arg=${token}`]);
  }

  return { worktreeId, approval, label };
}

function requireApproval(worktreeId, approval) {
  const normalized = normalizeApproval(approval);
  if (normalized === "approved" || normalized === "proceed") return;
  fail("Missing valid approval acknowledgement", [
    "accepted approvals: approved | proceed",
    `worktree_id=${worktreeId}`,
  ]);
}

function createSafetySnapshot(worktreeId, label = "") {
  const snapshotLabel = String(label || "").trim() || `pre-reseed-${worktreeId}-from-main`;
  execFileSync(
    process.execPath,
    [path.join(SCRIPT_DIR, "backup-snapshot.mjs"), "--label", snapshotLabel],
    { cwd: REPO_ROOT, stdio: "inherit" },
  );
}

function ensureGovJunction(absDir) {
  const govDir = path.join(absDir, ".GOV");
  const govKernelAbs = path.resolve(absDir, "..", "wt-gov-kernel", ".GOV");

  if (!fs.existsSync(govKernelAbs)) {
    fail("Governance kernel .GOV path not found", [`expected=${govKernelAbs}`]);
  }

  if (fs.existsSync(govDir)) {
    const stat = fs.lstatSync(govDir);
    if (stat.isSymbolicLink()) {
      const actualTarget = path.resolve(fs.realpathSync(govDir));
      const expectedTarget = path.resolve(fs.realpathSync(govKernelAbs));
      if (actualTarget === expectedTarget) {
        console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] .GOV already linked in ${absDir}`);
        return;
      }
      console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] replacing incorrect .GOV junction in ${absDir}`);
      fs.rmSync(govDir, { recursive: true, force: true });
    } else {
      console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] replacing inherited .GOV with junction in ${absDir}`);
      fs.rmSync(govDir, { recursive: true, force: true });
    }
  }

  if (process.platform === "win32") {
    execFileSync("cmd", ["/c", "mklink", "/J", govDir, govKernelAbs], { stdio: "inherit" });
  } else {
    fs.symlinkSync(govKernelAbs, govDir, "junction");
  }
  console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] .GOV junction created -> ${govKernelAbs}`);
}

function main() {
  const { worktreeId, approval, label } = parseArgs();
  requireApproval(worktreeId, approval);

  const spec = WORKTREE_SPECS.find((entry) => entry.id === worktreeId);
  if (!spec || !RESEEDABLE_WORKTREE_ROLES.has(spec.role)) {
    fail("Unsupported worktree_id for reseed helper", [
      `worktree_id=${worktreeId}`,
      "supported=wt-ilja",
    ]);
  }

  const absDir = absFromRepo(spec.rel_path);
  if (!fs.existsSync(absDir) || !gitCheckoutExists(absDir)) {
    fail("Target worktree is missing or is not a git checkout", [`path=${absDir}`]);
  }

  if (dirtyInRepo(absDir)) {
    fail("Refusing to reseed a dirty worktree", [
      `path=${absDir}`,
      "Commit, stash, or recover the changes first.",
    ]);
  }

  const originalBranch = currentBranchInRepo(absDir);
  if (originalBranch && ![spec.local_branch, "main"].includes(originalBranch)) {
    fail("Refusing to reseed while the worktree is checked out to an unexpected branch", [
      `path=${absDir}`,
      `current_branch=${originalBranch}`,
      `expected=${spec.local_branch} or main`,
    ]);
  }

  runGitInherit(absDir, ["fetch", "origin"]);

  if (!localBranchExists(absDir, spec.local_branch)) {
    fail("Permanent role/user branch not found locally", [
      `path=${absDir}`,
      `branch=${spec.local_branch}`,
    ]);
  }

  console.log(
    `[RESEED_PERMANENT_WORKTREE_FROM_MAIN] safety push origin/${spec.local_branch} <= ${spec.local_branch}`,
  );
  runGitInherit(absDir, ["push", "-u", "origin", `${spec.local_branch}:${spec.local_branch}`]);

  createSafetySnapshot(worktreeId, label);

  if (originalBranch === "main") {
    runGitInherit(absDir, ["checkout", spec.local_branch]);
  }

  if (!refExists(absDir, "refs/remotes/origin/main")) {
    fail("Remote canonical branch not found", [
      `path=${absDir}`,
      "expected=refs/remotes/origin/main",
    ]);
  }

  console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] resetting local main to origin/main in ${absDir}`);
  runGitInherit(absDir, ["branch", "-f", "main", "origin/main"]);

  console.log(
    `[RESEED_PERMANENT_WORKTREE_FROM_MAIN] reseeding ${spec.local_branch} from local main in ${absDir}`,
  );
  runGitInherit(absDir, ["branch", "-f", spec.local_branch, "main"]);
  runGitInherit(absDir, ["checkout", spec.local_branch]);
  runGitInherit(absDir, ["branch", "--set-upstream-to", spec.remote_branch, spec.local_branch]);

  ensureGovJunction(absDir);

  if (dirtyInRepo(absDir)) {
    fail("Worktree is dirty after reseed", [
      `path=${absDir}`,
      "Expected a clean worktree after branch reset and .GOV junction repair.",
    ]);
  }

  console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] reseed complete: ${worktreeId}`);
  console.log(`[RESEED_PERMANENT_WORKTREE_FROM_MAIN] local main base: origin/main`);
  console.log(
    `[RESEED_PERMANENT_WORKTREE_FROM_MAIN] backup branch preserved remotely at origin/${spec.local_branch} before local reseed`,
  );
}

main();
