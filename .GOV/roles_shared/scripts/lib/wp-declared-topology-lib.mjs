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

function uniqueResolvedPaths(values = []) {
  const unique = new Map();
  for (const value of values) {
    const normalized = String(value || "").trim();
    if (!normalized) continue;
    const resolved = path.resolve(normalized);
    const comparable = comparablePath(resolved);
    if (!unique.has(comparable)) {
      unique.set(comparable, resolved);
    }
  }
  return Array.from(unique.values());
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

function probeDeclaredWorktree(worktreeAbs, declaredWorktreeProbe = null) {
  const normalizedWorktreeAbs = path.resolve(String(worktreeAbs || ""));
  if (typeof declaredWorktreeProbe === "function") {
    const probed = declaredWorktreeProbe(normalizedWorktreeAbs);
    return probed ? { ...probed } : null;
  }
  try {
    const branch = runGit(normalizedWorktreeAbs, ["branch", "--show-current"]);
    const topLevel = runGit(normalizedWorktreeAbs, ["rev-parse", "--show-toplevel"]);
    const head = runGit(normalizedWorktreeAbs, ["rev-parse", "HEAD"]);
    return {
      path: topLevel || normalizedWorktreeAbs,
      branch,
      head,
      direct_probe: true,
    };
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
  const allowedSpecificPaths = Array.from(new Set(
    [coderWorktreeAbs, wpValidatorWorktreeAbs, integrationWorktreeAbs].map(comparablePath),
  ));
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

function isRelatedWorktree(entry, topology, wpId, expectedBranchHeads) {
  const branch = normalizeBranch(entry?.branch);
  const worktreePath = String(entry?.path || "");
  const basename = basenameHint(worktreePath);
  const branchMatchesWp = branch.includes(wpId);
  const pathMatchesDeclaredBase = topology.allowedBasenames.some((base) => basename === base || basename.startsWith(`${base}-`));
  const tokenMatch = wpPathTokens(wpId).some((token) => basename.includes(token));
  const heads = Array.isArray(expectedBranchHeads) ? expectedBranchHeads.filter(Boolean) : [];
  const detachedHeadMatch = !branch && heads.includes(String(entry?.head || "").trim()) && tokenMatch;
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
  declaredWorktreeProbe = null,
} = {}) {
  const topology = declaredTopology(repoRoot, wpId, packetContent);
  const entries = Array.isArray(worktrees) ? worktrees : parseGitWorktreeList(repoRoot);
  const expectedCoderBranchHead = branchHeadSha(repoRoot, topology.coderBranch, branchHeads);
  const expectedWpValidatorBranchHead = branchHeadSha(repoRoot, topology.wpValidatorBranch, branchHeads);
  const issues = [];

  const coderBranchEntry = entries.find((entry) => normalizeBranch(entry.branch) === topology.coderBranch) || null;
  const directProbeEntry = coderBranchEntry
    ? null
    : probeDeclaredWorktree(topology.coderWorktreeAbs, declaredWorktreeProbe);
  const effectiveCoderEntry = coderBranchEntry || directProbeEntry;

  if (!effectiveCoderEntry) {
    issues.push(`no linked worktree found for expected coder branch ${topology.coderBranch}`);
  } else if (normalizeBranch(effectiveCoderEntry.branch) !== topology.coderBranch) {
    issues.push(
      `coder worktree branch mismatch (expected ${topology.coderBranch}, git has ${normalizeBranch(effectiveCoderEntry.branch) || "<unknown>"})`,
    );
  } else if (comparablePath(effectiveCoderEntry.path) !== comparablePath(topology.coderWorktreeAbs)) {
    issues.push(
      `coder worktree mismatch (expected ${topology.coderWorktreeDir} -> ${topology.coderWorktreeAbs}, git has ${effectiveCoderEntry.path})`,
    );
  }

  const wpValidatorBranchEntry = entries.find((entry) => normalizeBranch(entry.branch) === topology.wpValidatorBranch) || null;
  const directWpValidatorProbeEntry = wpValidatorBranchEntry
    ? null
    : probeDeclaredWorktree(topology.wpValidatorWorktreeAbs, declaredWorktreeProbe);
  const effectiveWpValidatorEntry = wpValidatorBranchEntry || directWpValidatorProbeEntry;

  // [CX-503G] WP Validator shares coder worktree. No distinct worktree required.
  // The per-MT stop pattern ensures only one role is active at a time.

  if (!effectiveWpValidatorEntry) {
    issues.push(`no linked worktree found for expected WP validator branch ${topology.wpValidatorBranch}`);
  } else if (normalizeBranch(effectiveWpValidatorEntry.branch) !== topology.wpValidatorBranch) {
    issues.push(
      `WP validator worktree branch mismatch (expected ${topology.wpValidatorBranch}, git has ${normalizeBranch(effectiveWpValidatorEntry.branch) || "<unknown>"})`,
    );
  } else if (comparablePath(effectiveWpValidatorEntry.path) !== comparablePath(topology.wpValidatorWorktreeAbs)) {
    issues.push(
      `WP validator worktree mismatch (expected ${topology.wpValidatorWorktreeDir} -> ${topology.wpValidatorWorktreeAbs}, git has ${effectiveWpValidatorEntry.path})`,
    );
  }

  const relatedWorktrees = entries.filter((entry) => isRelatedWorktree(entry, topology, wpId, [
    expectedCoderBranchHead,
    expectedWpValidatorBranchHead,
  ]));
  const undeclaredWorktrees = relatedWorktrees.filter((entry) => !topology.allowedSpecificPaths.includes(comparablePath(entry.path)));

  for (const entry of undeclaredWorktrees) {
    issues.push(`undeclared WP-adjacent worktree detected: ${describeWorktree(entry)}`);
  }

  return {
    ok: issues.length === 0,
    topology,
    coderBranchEntry: effectiveCoderEntry,
    wpValidatorBranchEntry: effectiveWpValidatorEntry,
    directProbeUsed: Boolean(directProbeEntry || directWpValidatorProbeEntry),
    relatedWorktrees,
    undeclaredWorktrees,
    expectedBranchHead: expectedCoderBranchHead,
    expectedCoderBranchHead,
    expectedWpValidatorBranchHead,
    issues,
  };
}

export function activeDeclaredTopologyRepoRoots({
  repoRoot,
  topology = null,
  governanceRootAbs = "",
} = {}) {
  const governanceRepoRootAbs = String(governanceRootAbs || "").trim()
    ? path.resolve(governanceRootAbs, "..")
    : "";
  return uniqueResolvedPaths([
    repoRoot,
    topology?.coderWorktreeAbs,
    topology?.wpValidatorWorktreeAbs,
    topology?.integrationWorktreeAbs,
    governanceRepoRootAbs,
  ]).sort((left, right) => comparablePath(left).localeCompare(comparablePath(right)));
}
