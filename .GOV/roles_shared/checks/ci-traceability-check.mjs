#!/usr/bin/env node
/**
 * CI traceability check [CX-903]
 * Validates commit messages reference WP_IDs and that task packets exist.
 * Task Board + task packets are the primary micro-log; logger is optional.
 */

import { execFileSync } from "node:child_process";
import fs from "node:fs";

import { resolveGovernanceReference } from "./governance-reference.mjs";
import { GOV_ROOT_REPO_REL, REPO_ROOT, repoPathAbs } from "../scripts/lib/runtime-paths.mjs";

let governanceRef = null;
try {
  governanceRef = resolveGovernanceReference();
} catch {
  governanceRef = null;
}

const bannerRef = governanceRef
  ? `Governance Reference: ${governanceRef.codexFilename}`
  : `Governance Reference: UNRESOLVED (see ${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md)`;

console.log(`\nCI Traceability Check (${bannerRef})...\n`);

const errors = [];
const warnings = [];

function git(args) {
  return execFileSync("git", ["-C", REPO_ROOT, ...args], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function readRecentCommits() {
  const commitOutput = git(["log", "-10", "--pretty=format:%H|%s|%an|%ae"]);
  return commitOutput
    .split("\n")
    .filter(Boolean)
    .map((line) => {
      const [hash, subject, author, email] = line.split("|");
      return { hash, subject, author, email };
    });
}

const wpIdPattern = /WP-[\w-]+/;
const governanceOnlyPathAllowlist = [
  (p) => p.startsWith(`${GOV_ROOT_REPO_REL}/`),
  (p) => p.startsWith(".github/"),
  (p) => p.startsWith(".claude/"),
  (p) => p === "justfile",
  (p) => p === "AGENTS.md",
  (p) => p === ".GOV/codex/Handshake_Codex_v1.4.md",
  (p) => /^\.GOV\/spec\/Handshake_Master_Spec_.*\.md$/.test(p),
  (p) => /^Handshake_logger_.*\.md$/.test(p),
];

function isGovernanceOnlyPath(filePath) {
  return governanceOnlyPathAllowlist.some((fn) => fn(filePath));
}

function getCommitTouchedPaths(commitHash) {
  try {
    const output = git(["show", "--name-only", "--no-patch", "--pretty=format:", commitHash]);
    return output
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
  } catch {
    return null;
  }
}

const commitIsGovernanceOnlyCache = new Map();
function isGovernanceOnlyCommit(commitHash) {
  if (commitIsGovernanceOnlyCache.has(commitHash)) {
    return commitIsGovernanceOnlyCache.get(commitHash);
  }
  const touchedPaths = getCommitTouchedPaths(commitHash);
  const isGovOnly = Array.isArray(touchedPaths) && touchedPaths.every(isGovernanceOnlyPath);
  commitIsGovernanceOnlyCache.set(commitHash, isGovOnly);
  return isGovOnly;
}

let commits = [];
try {
  commits = readRecentCommits();
} catch (error) {
  console.error("Could not retrieve git commits");
  console.error(error.message);
  process.exit(1);
}

console.log(`Found ${commits.length} recent commits to check\n`);

console.log("Check 1: WP_ID references in commits");
const commitsWithWpId = commits.filter((commit) => wpIdPattern.test(commit.subject));
const commitsWithoutWpId = commits.filter(
  (commit) => !wpIdPattern.test(commit.subject) && !isGovernanceOnlyCommit(commit.hash),
);
console.log(`  OK ${commitsWithWpId.length} commits reference WP_ID`);
if (commitsWithoutWpId.length > 0) {
  console.log(`  WARN ${commitsWithoutWpId.length} commits without WP_ID:`);
  commitsWithoutWpId.slice(0, 3).forEach((commit) => {
    console.log(`    - ${commit.hash.slice(0, 7)}: ${commit.subject}`);
  });
  warnings.push(`${commitsWithoutWpId.length} commits without WP_ID reference (non-governance)`);
}

console.log("\nCheck 2: Task packets exist for referenced WP_IDs");
const canonicalTaskPacketDir = `${GOV_ROOT_REPO_REL}/task_packets`;
const canonicalTaskPacketDirAbs = repoPathAbs(canonicalTaskPacketDir);
if (!fs.existsSync(canonicalTaskPacketDirAbs)) {
  errors.push(`${GOV_ROOT_REPO_REL}/task_packets/ directory does not exist [CX-213]`);
  console.log("  FAIL: No task_packets directory");
  console.log(`  Run: mkdir -p ${GOV_ROOT_REPO_REL}/task_packets`);
} else {
  const canonicalPackets = fs.readdirSync(canonicalTaskPacketDirAbs).filter((file) => file.endsWith(".md"));
  console.log(`  OK ${GOV_ROOT_REPO_REL}/task_packets/ exists (${canonicalPackets.length} packets)`);

  const missingPackets = [];
  commitsWithWpId.forEach((commit) => {
    const wpId = commit.subject.match(wpIdPattern)?.[0];
    if (!wpId) return;
    const hasCanonical = canonicalPackets.some((packet) => packet.includes(wpId));
    if (!hasCanonical) missingPackets.push({ commit, wpId });
  });

  if (missingPackets.length > 0) {
    console.log(`  FAIL ${missingPackets.length} WP_IDs in commits without task packet files:`);
    missingPackets.slice(0, 3).forEach(({ commit, wpId }) => {
      console.log(`    - ${commit.hash.slice(0, 7)}: ${wpId}`);
    });
    errors.push(`${missingPackets.length} commits reference WP_ID without matching task packet`);
  } else {
    console.log("  OK All WP_IDs in commits have task packets");
  }
}

console.log("\nCheck 3: Logger (optional, milestones/hard bugs)");
const loggerFiles = fs
  .readdirSync(REPO_ROOT)
  .filter((file) => file.startsWith("Handshake_logger_") && file.endsWith(".md"))
  .sort()
  .reverse();
if (loggerFiles.length === 0) {
  console.log("  INFO Logger not present (optional)");
} else {
  console.log(`  INFO Logger present: ${loggerFiles[0]} (milestones/hard bugs only)`);
}

console.log(`\nCheck 4: Governance Reference exists (from ${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md)`);
try {
  const ref = governanceRef || resolveGovernanceReference();
  if (!fs.existsSync(ref.codexPathAbs)) {
    errors.push(
      `Governance Reference file not found: ${ref.codexFilename} (resolved from ${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md)`,
    );
    console.log(`  FAIL Governance Reference missing: ${ref.codexFilename}`);
  } else {
    console.log(`  OK ${ref.codexFilename} exists`);
  }
} catch (error) {
  errors.push(`Could not resolve Governance Reference from ${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md: ${error.message}`);
  console.log(`  FAIL Could not resolve Governance Reference from ${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md`);
}

console.log("\nCheck 5: Protocol files exist");
const protocolFiles = [
  `${GOV_ROOT_REPO_REL}/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`,
  `${GOV_ROOT_REPO_REL}/roles/coder/CODER_PROTOCOL.md`,
  `${GOV_ROOT_REPO_REL}/roles/validator/VALIDATOR_PROTOCOL.md`,
  `${GOV_ROOT_REPO_REL}/roles/orchestrator/agentic/AGENTIC_PROTOCOL.md`,
  `${GOV_ROOT_REPO_REL}/roles/coder/agentic/AGENTIC_PROTOCOL.md`,
  `${GOV_ROOT_REPO_REL}/roles/validator/agentic/AGENTIC_PROTOCOL.md`,
  `${GOV_ROOT_REPO_REL}/roles_shared/docs/BOUNDARY_RULES.md`,
  `${GOV_ROOT_REPO_REL}/roles_shared/docs/EVIDENCE_LEDGER.md`,
];
protocolFiles.forEach((filePath) => {
  if (!fs.existsSync(repoPathAbs(filePath))) {
    errors.push(`${filePath} not found [CX-900]`);
    console.log(`  FAIL ${filePath} missing`);
  } else {
    console.log(`  OK ${filePath} exists`);
  }
});

console.log(`\n${"=".repeat(50)}`);
if (errors.length === 0 && warnings.length === 0) {
  console.log("CI traceability check PASSED\n");
  process.exit(0);
}

if (errors.length === 0 && warnings.length > 0) {
  console.log("CI traceability check PASSED with warnings\n");
  console.log("Warnings:");
  warnings.forEach((warning, index) => console.log(`  ${index + 1}. ${warning}`));
  console.log("\nWarnings do not block CI, but should be addressed.");
  process.exit(0);
}

console.log("CI traceability check FAILED\n");
console.log("Errors:");
errors.forEach((error, index) => console.log(`  ${index + 1}. ${error}`));
if (warnings.length > 0) {
  console.log("\nWarnings:");
  warnings.forEach((warning, index) => console.log(`  ${index + 1}. ${warning}`));
}
console.log("\nFix these issues to pass CI traceability check.");
console.log(`See: ${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md`);
process.exit(1);
