#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import {
  REPO_ROOT,
  currentBranchInRepo,
  dirtyInRepo,
  gitCheckoutExists,
  headShaInRepo,
  runGitInRepo,
} from "./git-topology-lib.mjs";
import {
  defaultCoderBranch,
  defaultCoderWorktreeDir,
  defaultWpValidatorBranch,
  defaultWpValidatorWorktreeDir,
} from "../session/session-policy.mjs";
import { loadJson, loadPacket, packetExists, parseClaimField } from "../lib/role-resume-utils.mjs";

const WORKTREE_CLEANUP_TOKEN_SCHEMA = "hsk.worktree_cleanup_token@1";
const WORKTREE_CLEANUP_TOKEN_VERSION = "worktree_cleanup_token_v1";
const CLEANUP_MANIFEST_SCHEMA = "hsk.worktree_cleanup_script_manifest@1";
const CLEANUP_MANIFEST_VERSION = "worktree_cleanup_script_manifest_v1";
const GENERATED_SCRIPT_ROOT = path.join(".GOV", "generated_cleanup_scripts");
const SESSION_REGISTRY_PATH = path.join(".GOV", "roles_shared", "runtime", "ROLE_SESSION_REGISTRY.json");
const TOKEN_TTL_DAYS = 7;

function fail(message, details = []) {
  console.error(`[GENERATE_WORKTREE_CLEANUP_SCRIPT] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function usage() {
  fail("Usage: node .GOV/roles_shared/scripts/topology/generate-worktree-cleanup-script.mjs <WP_ID> <CODER|WP_VALIDATOR>", [
    "Example: node .GOV/roles_shared/scripts/topology/generate-worktree-cleanup-script.mjs WP-1-Example CODER",
  ]);
}

function parseArgs() {
  const wpId = String(process.argv[2] || "").trim();
  const role = String(process.argv[3] || "").trim().toUpperCase();
  if (!wpId || !wpId.startsWith("WP-")) usage();
  if (!["CODER", "WP_VALIDATOR"].includes(role)) {
    fail("Cleanup script generation currently supports CODER or WP_VALIDATOR targets only", [`role=${role || "<missing>"}`]);
  }
  return { wpId, role };
}

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

function comparablePath(value) {
  return path.resolve(String(value || "")).replace(/\\/g, "/").toLowerCase();
}

function ensureDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function sanitizeFileSegment(value) {
  return String(value || "")
    .trim()
    .replace(/[^A-Za-z0-9._-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "")
    || "cleanup";
}

function sha256(value) {
  return crypto.createHash("sha256").update(String(value || "")).digest("hex");
}

function quotePsLiteral(value) {
  return `'${String(value ?? "").replace(/'/g, "''")}'`;
}

function resolveGitDir(absDir) {
  const gitDir = runGitInRepo(absDir, ["rev-parse", "--git-dir"]);
  return path.resolve(absDir, gitDir);
}

function loadSessionRegistry() {
  return loadJson(SESSION_REGISTRY_PATH, { sessions: [] });
}

function resolveRoleTarget(role, wpId, packetContent, registry) {
  const sessions = Array.isArray(registry?.sessions) ? registry.sessions : [];
  const session = sessions.find((row) => row?.wp_id === wpId && String(row?.role || "").toUpperCase() === role) || null;
  if (session?.local_branch && session?.local_worktree_dir) {
    return {
      branch: String(session.local_branch).trim(),
      worktreeDir: String(session.local_worktree_dir).trim(),
      source: "ROLE_SESSION_REGISTRY",
    };
  }

  if (role === "CODER") {
    return {
      branch: parseClaimField(packetContent, "LOCAL_BRANCH") || defaultCoderBranch(wpId),
      worktreeDir: parseClaimField(packetContent, "LOCAL_WORKTREE_DIR") || defaultCoderWorktreeDir(wpId),
      source: parseClaimField(packetContent, "LOCAL_BRANCH") || parseClaimField(packetContent, "LOCAL_WORKTREE_DIR")
        ? "TASK_PACKET"
        : "SESSION_POLICY_DEFAULT",
    };
  }

  return {
    branch: parseClaimField(packetContent, "WP_VALIDATOR_LOCAL_BRANCH") || defaultWpValidatorBranch(wpId),
    worktreeDir: parseClaimField(packetContent, "WP_VALIDATOR_LOCAL_WORKTREE_DIR") || defaultWpValidatorWorktreeDir(wpId),
    source: parseClaimField(packetContent, "WP_VALIDATOR_LOCAL_BRANCH") || parseClaimField(packetContent, "WP_VALIDATOR_LOCAL_WORKTREE_DIR")
      ? "TASK_PACKET"
      : "SESSION_POLICY_DEFAULT",
  };
}

function buildPowerShellScript({
  repoRoot,
  wpId,
  role,
  worktreeId,
  absWorktreeDir,
  branch,
  headSha,
  tokenFile,
  approvalExact,
}) {
  return [
    "param(",
    "  [Parameter(Mandatory=$true)][string]$Approval,",
    "  [Parameter(Mandatory=$true)][string]$Token",
    ")",
    "",
    "$ErrorActionPreference = 'Stop'",
    `\$RepoRoot = ${quotePsLiteral(normalizePath(repoRoot))}`,
    `\$WorktreeId = ${quotePsLiteral(worktreeId)}`,
    `\$WpId = ${quotePsLiteral(wpId)}`,
    `\$Role = ${quotePsLiteral(role)}`,
    `\$WorktreePath = ${quotePsLiteral(normalizePath(absWorktreeDir))}`,
    `\$ExpectedBranch = ${quotePsLiteral(branch)}`,
    `\$ExpectedHead = ${quotePsLiteral(headSha)}`,
    `\$TokenFile = ${quotePsLiteral(normalizePath(tokenFile))}`,
    `\$ApprovalExact = ${quotePsLiteral(approvalExact)}`,
    "",
    "function Fail([string]$Message, [string[]]$Details = @()) {",
    "  Write-Error \"[GENERATED_WORKTREE_CLEANUP] $Message\"",
    "  foreach ($Line in $Details) { Write-Error \"  - $Line\" }",
    "  exit 1",
    "}",
    "",
    "if ($Approval -ne $ApprovalExact) {",
    "  Fail 'Approval text mismatch.' @(\"required=$ApprovalExact\")",
    "}",
    "",
    "if (-not (Test-Path -LiteralPath $RepoRoot)) {",
    "  Fail 'Repo root not found.' @(\"repo_root=$RepoRoot\")",
    "}",
    "",
    "$ResolvedRepoRoot = (& git -C $RepoRoot rev-parse --show-toplevel).Trim()",
    "if (-not $ResolvedRepoRoot) {",
    "  Fail 'Unable to resolve git repo root from generated cleanup script.'",
    "}",
    "",
    "if ([IO.Path]::GetFullPath($ResolvedRepoRoot) -ne [IO.Path]::GetFullPath($RepoRoot)) {",
    "  Fail 'Repo root mismatch for generated cleanup script.' @(\"expected=$RepoRoot\", \"actual=$ResolvedRepoRoot\")",
    "}",
    "",
    "if (-not (Test-Path -LiteralPath $WorktreePath)) {",
    "  Fail 'Target worktree path not found.' @(\"worktree_path=$WorktreePath\")",
    "}",
    "",
    "if (-not (Test-Path -LiteralPath $TokenFile)) {",
    "  Fail 'Cleanup token file not found in the target worktree git admin dir.' @(\"token_file=$TokenFile\")",
    "}",
    "",
    "Push-Location $RepoRoot",
    "try {",
    "  & node '.GOV/roles_shared/scripts/topology/delete-local-worktree.mjs' $WorktreeId `",
    "    --approve $Approval `",
    "    --approval-exact $ApprovalExact `",
    "    --expect-abs-path $WorktreePath `",
    "    --expect-branch $ExpectedBranch `",
    "    --expect-head $ExpectedHead `",
    "    --require-main-contains $ExpectedHead `",
    "    --token-file $TokenFile `",
    "    --token $Token `",
    "    --wp-id $WpId `",
    "    --role $Role",
    "} finally {",
    "  Pop-Location",
    "}",
  ].join("\n");
}

function main() {
  const { wpId, role } = parseArgs();
  if (!packetExists(wpId)) {
    fail("Task packet not found", [`packet=.GOV/task_packets/${wpId}.md`]);
  }

  const packetContent = loadPacket(wpId);
  const registry = loadSessionRegistry();
  const target = resolveRoleTarget(role, wpId, packetContent, registry);
  const absWorktreeDir = path.resolve(REPO_ROOT, target.worktreeDir);
  const worktreeId = path.basename(absWorktreeDir);

  if (!fs.existsSync(absWorktreeDir)) {
    fail("Target worktree path does not exist", [
      `role=${role}`,
      `worktree_dir=${target.worktreeDir}`,
      `resolved_path=${normalizePath(absWorktreeDir)}`,
    ]);
  }

  if (!gitCheckoutExists(absWorktreeDir)) {
    fail("Target path is not a git checkout", [
      `resolved_path=${normalizePath(absWorktreeDir)}`,
    ]);
  }

  const currentBranch = currentBranchInRepo(absWorktreeDir);
  if (currentBranch !== target.branch) {
    fail("Target worktree branch does not match the recorded cleanup target", [
      `role=${role}`,
      `expected_branch=${target.branch}`,
      `actual_branch=${currentBranch || "<detached>"}`,
    ]);
  }

  if (dirtyInRepo(absWorktreeDir)) {
    fail("Refusing to generate a cleanup script for a dirty worktree", [
      `role=${role}`,
      `worktree_path=${normalizePath(absWorktreeDir)}`,
      "Commit, stash, or recover changes before issuing a deletion script.",
    ]);
  }

  const headSha = headShaInRepo(absWorktreeDir);
  const gitDir = resolveGitDir(absWorktreeDir);
  const tokenPlaintext = crypto.randomBytes(24).toString("base64url");
  const tokenHash = sha256(tokenPlaintext);
  const issuedAt = new Date().toISOString();
  const expiresAt = new Date(Date.now() + TOKEN_TTL_DAYS * 24 * 60 * 60 * 1000).toISOString();
  const tokenFile = path.join(gitDir, `handshake-cleanup-token-${sanitizeFileSegment(role.toLowerCase())}-${sanitizeFileSegment(wpId)}.json`);
  const approvalExact = `APPROVE DELETE LOCAL WORKTREE ${worktreeId} FOR ${wpId} ROLE ${role}`;

  const tokenPayload = {
    schema_id: WORKTREE_CLEANUP_TOKEN_SCHEMA,
    schema_version: WORKTREE_CLEANUP_TOKEN_VERSION,
    issued_at: issuedAt,
    expires_at: expiresAt,
    issued_by_role: "ORCHESTRATOR",
    wp_id: wpId,
    role,
    worktree_id: worktreeId,
    expected_abs_path: normalizePath(absWorktreeDir),
    expected_branch: target.branch,
    expected_head_sha: headSha,
    token_sha256: tokenHash,
    repo_root: normalizePath(REPO_ROOT),
    source: target.source,
  };
  fs.writeFileSync(tokenFile, `${JSON.stringify(tokenPayload, null, 2)}\n`);

  const outputDir = path.join(REPO_ROOT, GENERATED_SCRIPT_ROOT, sanitizeFileSegment(wpId));
  ensureDir(outputDir);
  const fileBase = `${sanitizeFileSegment(role.toLowerCase())}-${sanitizeFileSegment(worktreeId)}-cleanup`;
  const scriptPath = path.join(outputDir, `${fileBase}.ps1`);
  const manifestPath = path.join(outputDir, `${fileBase}.json`);
  const scriptContent = buildPowerShellScript({
    repoRoot: REPO_ROOT,
    wpId,
    role,
    worktreeId,
    absWorktreeDir,
    branch: target.branch,
    headSha,
    tokenFile,
    approvalExact,
  });
  fs.writeFileSync(scriptPath, `${scriptContent}\n`);

  const manifest = {
    schema_id: CLEANUP_MANIFEST_SCHEMA,
    schema_version: CLEANUP_MANIFEST_VERSION,
    generated_at: issuedAt,
    generated_by_role: "ORCHESTRATOR",
    wp_id: wpId,
    role,
    worktree_id: worktreeId,
    worktree_dir: target.worktreeDir,
    abs_worktree_dir: normalizePath(absWorktreeDir),
    expected_branch: target.branch,
    expected_head_sha: headSha,
    source: target.source,
    approval_exact: approvalExact,
    token_file: normalizePath(tokenFile),
    token_sha256: tokenHash,
    expires_at: expiresAt,
    script_path: normalizePath(scriptPath),
  };
  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

  console.log("WORKTREE_CLEANUP_SCRIPT_GENERATED");
  console.log(`- WP_ID: ${wpId}`);
  console.log(`- ROLE: ${role}`);
  console.log(`- SOURCE: ${target.source}`);
  console.log(`- TARGET_WORKTREE_ID: ${worktreeId}`);
  console.log(`- TARGET_WORKTREE_DIR: ${target.worktreeDir}`);
  console.log(`- TARGET_BRANCH: ${target.branch}`);
  console.log(`- TARGET_HEAD_SHA: ${headSha}`);
  console.log(`- SCRIPT_PATH: ${normalizePath(scriptPath)}`);
  console.log(`- MANIFEST_PATH: ${normalizePath(manifestPath)}`);
  console.log(`- TOKEN_FILE: ${normalizePath(tokenFile)}`);
  console.log(`- APPROVAL_TEXT: ${approvalExact}`);
  console.log(`- EXPIRES_AT: ${expiresAt}`);
  console.log(`- TOKEN: ${tokenPlaintext}`);
  console.log("- RUN_EXAMPLE:");
  console.log(`  powershell -ExecutionPolicy Bypass -File "${normalizePath(scriptPath)}" -Approval "${approvalExact}" -Token "${tokenPlaintext}"`);
}

main();

