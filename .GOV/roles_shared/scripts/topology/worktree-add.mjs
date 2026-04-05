import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { defaultCoderWorktreeDir } from "../session/session-policy.mjs";
import { suppressSharedGovJunctionDirt } from "./reseed-permanent-worktree-from-main.mjs";

function runGit(args) {
  return execFileSync("git", args, { stdio: "pipe" }).toString().trim();
}

function runGitInherit(args) {
  execFileSync("git", args, { stdio: "inherit" });
}

function normalizeWorktreePathForCompare(targetPath) {
  const normalized = path.resolve(targetPath).replace(/\\/g, "/").replace(/\/+$/, "");
  return process.platform === "win32" ? normalized.toLowerCase() : normalized;
}

function removeDirectoryLinkOnly(linkPath) {
  if (process.platform === "win32") {
    execFileSync("cmd", ["/c", "rmdir", linkPath], { stdio: "ignore" });
    return;
  }
  fs.unlinkSync(linkPath);
}

function parseWorktreeListPorcelain(raw) {
  const entries = [];
  const lines = String(raw || "").split(/\r?\n/);
  let current = null;

  function flush() {
    if (!current?.worktree) return;
    entries.push(current);
    current = null;
  }

  for (const line of lines) {
    if (!line.trim()) {
      flush();
      continue;
    }
    if (line.startsWith("worktree ")) {
      flush();
      current = {
        worktree: line.slice("worktree ".length).trim(),
        branch: "",
        prunable: false,
      };
      continue;
    }
    if (!current) continue;
    if (line.startsWith("branch ")) {
      current.branch = line.slice("branch ".length).trim();
      continue;
    }
    if (line.startsWith("prunable")) {
      current.prunable = true;
    }
  }

  flush();
  return entries;
}

function findRegisteredWorktree(targetPath) {
  const target = normalizeWorktreePathForCompare(targetPath);
  const entries = parseWorktreeListPorcelain(runGit(["worktree", "list", "--porcelain"]));
  return entries.find((entry) => normalizeWorktreePathForCompare(entry.worktree) === target) || null;
}

function fail(message) {
  console.error(`[WORKTREE_ADD] ${message}`);
  process.exit(1);
}

function isBranchPresent(branch) {
  try {
    execFileSync("git", ["show-ref", "--verify", "--quiet", `refs/heads/${branch}`]);
    return true;
  } catch {
    return false;
  }
}

function isForbiddenWorktreeDir(dir) {
  const input = dir.trim();
  // The governance contract is drive-agnostic: worktree dirs must be repo-relative placeholders.
  // Reject all absolute paths (including UNC) and drive-designator paths like "C:foo".
  if (path.isAbsolute(input)) return true;
  if (/^[A-Za-z]:/.test(input)) return true;
  // Avoid a RegExp literal here: Node v24.11.1 fails to parse `/^(\\\\|\\/\\/)/` (Invalid flags).
  if (input.startsWith('\\\\') || input.startsWith('//')) return true;
  return false;
}

function main() {
  const wpId = process.argv[2]?.trim();
  if (!wpId) {
    fail(
      "Usage: node .GOV/roles_shared/scripts/topology/worktree-add.mjs <WP_ID> [base=main] [branch=feat/WP_ID] [dir=<repo-relative worktree dir>]"
    );
  }

  const base = (process.argv[3] ?? "main").trim() || "main";
  const branch = (process.argv[4] ?? "").trim() || `feat/${wpId}`;
  const dir = (process.argv[5] ?? "").trim() || defaultCoderWorktreeDir(wpId);

  const repoRoot = runGit(["rev-parse", "--show-toplevel"]);

  if (isForbiddenWorktreeDir(dir)) {
    fail(`Forbidden worktree dir (must be repo-relative): ${dir}`);
  }

  const absDir = path.resolve(repoRoot, dir);
  const registeredBeforePrune = findRegisteredWorktree(absDir);
  const expectedBranchRef = `refs/heads/${branch}`;

  if (!fs.existsSync(absDir) && registeredBeforePrune) {
    console.log(`[WORKTREE_ADD] Pruning stale worktree metadata before add: ${absDir}`);
    runGitInherit(["worktree", "prune"]);
  }

  const registeredAfterPrune = findRegisteredWorktree(absDir);

  const reuseExistingWorktree = Boolean(
    fs.existsSync(absDir)
    && registeredAfterPrune
    && registeredAfterPrune.branch === expectedBranchRef
  );

  if (fs.existsSync(absDir) && !reuseExistingWorktree) {
    if (registeredAfterPrune) {
      fail(
        `Target directory already exists with mismatched registered branch: ${absDir} `
        + `(expected ${expectedBranchRef}, got ${registeredAfterPrune.branch || "<detached>"})`
      );
    }
    fail(`Target directory already exists: ${absDir}`);
  }
  if (registeredAfterPrune && !reuseExistingWorktree) {
    fail(
      `Target worktree path is still registered in git metadata after prune: ${absDir}. `
      + "Inspect `git worktree list --porcelain` and clear the stale registration before retrying."
    );
  }

  if (reuseExistingWorktree) {
    console.log(`[WORKTREE_ADD] Reusing existing worktree: ${absDir}`);
    console.log(`[WORKTREE_ADD] Registered branch: ${branch}`);
  } else {
    const alreadyHaveBranch = isBranchPresent(branch);
    if (alreadyHaveBranch) {
      console.log(`[WORKTREE_ADD] Using existing branch: ${branch}`);
      runGitInherit(["worktree", "add", absDir, branch]);
    } else {
      console.log(`[WORKTREE_ADD] Creating branch ${branch} from ${base}`);
      runGitInherit(["worktree", "add", "-b", branch, absDir, base]);
    }
  }

  // --- Replace inherited .GOV/ with junction to governance kernel [CX-212D] ---
  const govDir = path.join(absDir, ".GOV");
  const govKernelAbs = path.resolve(absDir, "..", "wt-gov-kernel", ".GOV");

  if (fs.existsSync(govDir) && fs.existsSync(govKernelAbs)) {
    const stat = fs.lstatSync(govDir);
    let needsCreate = false;
    if (stat.isSymbolicLink()) {
      try {
        const actualTarget = path.resolve(fs.realpathSync(govDir));
        const expectedTarget = path.resolve(fs.realpathSync(govKernelAbs));
        if (normalizeWorktreePathForCompare(actualTarget) === normalizeWorktreePathForCompare(expectedTarget)) {
          console.log(`[WORKTREE_ADD] .GOV already linked -> ${govKernelAbs}`);
        } else {
          console.log(`[WORKTREE_ADD] Replacing incorrect .GOV junction in ${absDir}`);
          removeDirectoryLinkOnly(govDir);
          needsCreate = true;
        }
      } catch {
        console.log(`[WORKTREE_ADD] Replacing unreadable .GOV junction in ${absDir}`);
        removeDirectoryLinkOnly(govDir);
        needsCreate = true;
      }
    } else {
      console.log(`[WORKTREE_ADD] Replacing inherited .GOV/ with junction to governance kernel`);
      fs.rmSync(govDir, { recursive: true, force: true });
      needsCreate = true;
    }
    if (needsCreate) {
      if (process.platform === "win32") {
        execFileSync("cmd", ["/c", "mklink", "/J", govDir, govKernelAbs], { stdio: "inherit" });
      } else {
        fs.symlinkSync(govKernelAbs, govDir, "junction");
      }
      console.log(`[WORKTREE_ADD] .GOV/ junction created -> ${govKernelAbs}`);
    }
    suppressSharedGovJunctionDirt(absDir);
    console.log(`[WORKTREE_ADD] Applied local .GOV suppression for shared kernel junction`);
  }

  console.log("");
  console.log(`[WORKTREE_ADD] Worktree ready: ${absDir}`);
  console.log(`[WORKTREE_ADD] Next: cd "${absDir}"`);
}

main();
