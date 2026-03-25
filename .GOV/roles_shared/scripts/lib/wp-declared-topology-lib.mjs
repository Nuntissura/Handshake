import { execFileSync } from "node:child_process";
import path from "node:path";
import { parseClaimField } from "./role-resume-utils.mjs";
import {
  defaultCoderBranch,
  defaultCoderWorktreeDir,
  defaultIntegrationValidatorBranch,
  defaultIntegrationValidatorWorktreeDir,
  defaultWpValidatorBranch,
  defaultWpValidatorWorktreeDir,
} from "../session/session-policy.mjs";

const WP_PATH_TOKEN_STOPWORDS = new Set([
  "wp",
  "structured",
  "collaboration",
  "workflow",
  "governed",
  "contract",
  "alignment",
  "packet",
  "validator",
  "review",
]);

function normalizeBranch(branch) {
  return String(branch || "").replace(/^refs\/heads\//, "").trim();
}

function comparablePath(value) {
  return path.resolve(String(value || "")).replace(/\\/g, "/").toLowerCase();
}

function basenameHint(absPath) {
  return path.basename(String(absPath || "")).replace(/\\/g, "/").toLowerCase();
}

function runGit(repoRoot, args) {
  return execFileSync("git", args, {
    cwd: repoRoot,
    stdio: ["ignore", "pipe", "ignore"],
    encoding: "utf8",
  }).trim();
}

export function parseGitWorktreeList(repoRoot) {
  const out = runGit(repoRoot, ["worktree", "list", "--porcelain"]);
  const entries = [];
  let current = null;

  for (const line of out.split(/\r?\n/)) {
    if (!line.trim()) {
      if (current) entries.push(current);
      current = null;
      continue;
    }
    if (line.startsWith("worktree ")) {
      if (current) entries.push(current);
      current = { path: line.slice("worktree ".length).trim(), branch: "", head: "" };
      continue;
    }
    if (!current) continue;
    if (line.startsWith("branch ")) current.branch = line.slice("branch ".length).trim();
    if (line.startsWith("HEAD ")) current.head = line.slice("HEAD ".length).trim();
  }

  if (current) entries.push(current);
  return entries;
}

function branchHeadSha(repoRoot, branch, branchHeads) {
  const normalized = normalizeBranch(branch);
  if (!normalized) return null;
  if (branchHeads && Object.prototype.hasOwnProperty.call(branchHeads, normalized)) {
    return String(branchHeads[normalized] || "").trim() || null;
  }
  try {
    return runGit(repoRoot, ["rev-parse", `refs/heads/${normalized}`]) || null;
  } catch {
    return null;
  }
}

function wpPathTokens(wpId) {
  return String(wpId || "")
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .map((part) => part.trim())
    .filter((part) =>
      part
      && !/^v\d+$/.test(part)
      && !/^\d+$/.test(part)
      && part.length >= 5
      && !WP_PATH_TOKEN_STOPWORDS.has(part)
    );
}

function declaredTopology(repoRoot, wpId, packetContent) {
  const coderBranch = parseClaimField(packetContent, "LOCAL_BRANCH") || defaultCoderBranch(wpId);
  const coderWorktreeDir = parseClaimField(packetContent, "LOCAL_WORKTREE_DIR") || defaultCoderWorktreeDir(wpId);
  const wpValidatorBranch = parseClaimField(packetContent, "WP_VALIDATOR_LOCAL_BRANCH") || defaultWpValidatorBranch(wpId);
  const wpValidatorWorktreeDir = parseClaimField(packetContent, "WP_VALIDATOR_LOCAL_WORKTREE_DIR") || defaultWpValidatorWorktreeDir(wpId);
  const integrationBranch =
    parseClaimField(packetContent, "INTEGRATION_VALIDATOR_LOCAL_BRANCH") || defaultIntegrationValidatorBranch(wpId);
  const integrationWorktreeDir =
    parseClaimField(packetContent, "INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR") || defaultIntegrationValidatorWorktreeDir(wpId);

  const coderWorktreeAbs = path.resolve(repoRoot, coderWorktreeDir);
  const wpValidatorWorktreeAbs = path.resolve(repoRoot, wpValidatorWorktreeDir);
  const integrationWorktreeAbs = path.resolve(repoRoot, integrationWorktreeDir);
  const allowedSpecificPaths = Array.from(new Set([coderWorktreeAbs, wpValidatorWorktreeAbs].map(comparablePath)));
  const allowedBasenames = Array.from(new Set([coderWorktreeAbs, wpValidatorWorktreeAbs].map(basenameHint)));

  return {
    coderBranch: normalizeBranch(coderBranch),
    coderWorktreeDir,
    coderWorktreeAbs,
    wpValidatorBranch: normalizeBranch(wpValidatorBranch),
    wpValidatorWorktreeDir,
    wpValidatorWorktreeAbs,
    integrationBranch: normalizeBranch(integrationBranch),
    integrationWorktreeDir,
    integrationWorktreeAbs,
    allowedSpecificPaths,
    allowedBasenames,
  };
}

function isRelatedWorktree(entry, topology, wpId, expectedBranchHead) {
  const branch = normalizeBranch(entry?.branch);
  const worktreePath = String(entry?.path || "");
  const basename = basenameHint(worktreePath);
  const branchMatchesWp = branch.includes(wpId);
  const pathMatchesDeclaredBase = topology.allowedBasenames.some((base) => basename === base || basename.startsWith(`${base}-`));
  const tokenMatch = wpPathTokens(wpId).some((token) => basename.includes(token));
  const detachedHeadMatch = !branch && expectedBranchHead && String(entry?.head || "").trim() === expectedBranchHead && tokenMatch;
  return branchMatchesWp || pathMatchesDeclaredBase || detachedHeadMatch;
}

function describeWorktree(entry) {
  const branch = normalizeBranch(entry?.branch);
  if (branch) return `${entry.path} [branch=${branch}]`;
  return `${entry.path} [detached:${String(entry?.head || "").trim() || "<unknown>"}]`;
}

export function evaluateWpDeclaredTopology({
  repoRoot,
  wpId,
  packetContent,
  worktrees = null,
  branchHeads = null,
} = {}) {
  const topology = declaredTopology(repoRoot, wpId, packetContent);
  const entries = Array.isArray(worktrees) ? worktrees : parseGitWorktreeList(repoRoot);
  const expectedBranchHead = branchHeadSha(repoRoot, topology.coderBranch, branchHeads);
  const issues = [];

  const coderBranchEntry = entries.find((entry) => normalizeBranch(entry.branch) === topology.coderBranch) || null;
  if (!coderBranchEntry) {
    issues.push(`no linked worktree found for expected coder branch ${topology.coderBranch}`);
  } else if (comparablePath(coderBranchEntry.path) !== comparablePath(topology.coderWorktreeAbs)) {
    issues.push(
      `coder worktree mismatch (expected ${topology.coderWorktreeDir} -> ${topology.coderWorktreeAbs}, git has ${coderBranchEntry.path})`,
    );
  }

  const relatedWorktrees = entries.filter((entry) => isRelatedWorktree(entry, topology, wpId, expectedBranchHead));
  const undeclaredWorktrees = relatedWorktrees.filter((entry) => !topology.allowedSpecificPaths.includes(comparablePath(entry.path)));

  for (const entry of undeclaredWorktrees) {
    issues.push(`undeclared WP-adjacent worktree detected: ${describeWorktree(entry)}`);
  }

  return {
    ok: issues.length === 0,
    topology,
    relatedWorktrees,
    undeclaredWorktrees,
    expectedBranchHead,
    issues,
  };
}
