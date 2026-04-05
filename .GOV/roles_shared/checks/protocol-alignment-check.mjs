import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  ROLE_MODEL_PROFILE_POLICY,
  ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_6_THINKING_MAX,
  ROLE_MODEL_PROFILE_OPENAI_GPT_5_4_XHIGH,
  CLI_ESCALATION_HOST_DEFAULT,
  CLI_ESCALATION_HOST_LEGACY_ALIAS,
  EXECUTION_OWNER_RANGE_HELP,
  ROLE_SESSION_FALLBACK_MODEL,
  ROLE_SESSION_PRIMARY_MODEL,
  ROLE_SESSION_REASONING_CONFIG_KEY,
  ROLE_SESSION_REASONING_CONFIG_VALUE,
  SESSION_COMMAND_KINDS,
  SESSION_ROLES,
  SESSION_START_AUTHORITY,
  roleNextCommand,
  roleStartupCommand,
} from "../scripts/session/session-policy.mjs";
import { GOV_ROOT_REPO_REL } from "../scripts/lib/runtime-paths.mjs";

const JUSTFILE_PATH = "justfile";
// The justfile uses {{GOV_ROOT}} (a just variable) — when matching raw justfile text,
// we must use the literal justfile variable syntax, not the resolved GOV_ROOT_REPO_REL.
const JUSTFILE_GOV_PREFIX = "{{GOV_ROOT}}";
const ORCHESTRATOR_PROTOCOL_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "ORCHESTRATOR_PROTOCOL.md");
const CODER_PROTOCOL_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "coder", "CODER_PROTOCOL.md");
const VALIDATOR_PROTOCOL_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "validator", "VALIDATOR_PROTOCOL.md");
const ORCHESTRATOR_GATES_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "checks", "orchestrator_gates.mjs");
const ORCHESTRATOR_NEXT_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "orchestrator-next.mjs");
const CREATE_TASK_PACKET_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "create-task-packet.mjs");
const LAUNCH_CLI_SESSION_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "launch-cli-session.mjs");
const SESSION_CONTROL_COMMAND_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "session-control-command.mjs");
const SESSION_CONTROL_CANCEL_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "session-control-cancel.mjs");
const SESSION_CONTROL_LIB_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "scripts", "session", "session-control-lib.mjs");
const ROLE_SESSION_WORKTREE_ADD_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "role-session-worktree-add.mjs");
const PRE_WORK_CHECK_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "coder", "checks", "pre-work-check.mjs");
const SESSION_POLICY_CHECK_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "checks", "session-policy-check.mjs");
const REFINEMENT_TEMPLATE_PATH = path.join(GOV_ROOT_REPO_REL, "templates", "REFINEMENT_TEMPLATE.md");
const TASK_PACKET_TEMPLATE_PATH = path.join(GOV_ROOT_REPO_REL, "templates", "TASK_PACKET_TEMPLATE.md");
const ROLE_WORKTREES_DOC_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "docs", "ROLE_WORKTREES.md");
const ROLE_SESSION_REGISTRY_SCHEMA_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "schemas", "ROLE_SESSION_REGISTRY.schema.json");

const ACTIVE_SURFACE_PATHS = [
  JUSTFILE_PATH,
  ORCHESTRATOR_PROTOCOL_PATH,
  CODER_PROTOCOL_PATH,
  VALIDATOR_PROTOCOL_PATH,
  ORCHESTRATOR_GATES_PATH,
  ORCHESTRATOR_NEXT_PATH,
  CREATE_TASK_PACKET_PATH,
  LAUNCH_CLI_SESSION_PATH,
  SESSION_CONTROL_COMMAND_PATH,
  SESSION_CONTROL_CANCEL_PATH,
  SESSION_CONTROL_LIB_PATH,
  ROLE_SESSION_WORKTREE_ADD_PATH,
  PRE_WORK_CHECK_PATH,
  SESSION_POLICY_CHECK_PATH,
  REFINEMENT_TEMPLATE_PATH,
  TASK_PACKET_TEMPLATE_PATH,
  ROLE_WORKTREES_DOC_PATH,
  ROLE_SESSION_REGISTRY_SCHEMA_PATH,
];

function resolveRepoRoot() {
  const injectedRepoRoot = String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim();
  if (injectedRepoRoot) {
    return injectedRepoRoot;
  }

  const fileRelativeRepoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
  try {
    const out = execFileSync("git", ["-C", fileRelativeRepoRoot, "rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return out;
  } catch {
    // Fall back to relative resolution below.
  }

  return fileRelativeRepoRoot;
}

function fail(message, details = []) {
  console.error(`[PROTOCOL_ALIGNMENT_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function escapeRegex(value) {
  return String(value || "").replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function readUtf8(filePath) {
  return fs.readFileSync(filePath, "utf8");
}

function requireFileExists(filePath) {
  if (!fs.existsSync(filePath)) {
    fail("Missing required active governance surface", [filePath]);
  }
}

function requireSubstring(errors, filePath, content, needle, label = needle) {
  if (!content.includes(needle)) {
    errors.push(`${filePath}: missing ${label}`);
  }
}

function requireRegex(errors, filePath, content, regex, label) {
  if (!regex.test(content)) {
    errors.push(`${filePath}: missing ${label}`);
  }
}

function forbidRegex(errors, filePath, content, regex, label) {
  if (regex.test(content)) {
    errors.push(`${filePath}: contains retired/stale reference ${label}`);
  }
}

function findRecipeBody(justfileContent, recipeName) {
  const match = justfileContent.match(
    new RegExp(`^${escapeRegex(recipeName)}(?:\\s[^\\n]*)?:\\s*\\n([\\s\\S]*?)(?=^\\S[^\\n]*?:\\s*$|\\Z)`, "m"),
  );
  return match ? (match[1] || "") : "";
}

function requireRecipe(errors, justfileContent, recipeName, fragments = []) {
  const body = findRecipeBody(justfileContent, recipeName);
  if (!body) {
    errors.push(`${JUSTFILE_PATH}: missing recipe ${recipeName}`);
    return;
  }
  for (const fragment of fragments) {
    if (!body.includes(fragment)) {
      errors.push(`${JUSTFILE_PATH}: recipe ${recipeName} missing ${fragment}`);
    }
  }
}

function justRecipeName(command) {
  const match = String(command || "").trim().match(/^just\s+([^\s]+)/);
  return match ? match[1] : "";
}

const repoRoot = path.resolve(resolveRepoRoot());
process.chdir(repoRoot);

for (const filePath of ACTIVE_SURFACE_PATHS) requireFileExists(filePath);

const contents = new Map(ACTIVE_SURFACE_PATHS.map((filePath) => [filePath, readUtf8(filePath)]));
const errors = [];

const justfileContent = contents.get(JUSTFILE_PATH);
const orchestratorProtocol = contents.get(ORCHESTRATOR_PROTOCOL_PATH);
const coderProtocol = contents.get(CODER_PROTOCOL_PATH);
const validatorProtocol = contents.get(VALIDATOR_PROTOCOL_PATH);
const orchestratorGates = contents.get(ORCHESTRATOR_GATES_PATH);
const orchestratorNext = contents.get(ORCHESTRATOR_NEXT_PATH);
const createTaskPacket = contents.get(CREATE_TASK_PACKET_PATH);
const launchCliSession = contents.get(LAUNCH_CLI_SESSION_PATH);
const sessionControlCommand = contents.get(SESSION_CONTROL_COMMAND_PATH);
const sessionControlCancel = contents.get(SESSION_CONTROL_CANCEL_PATH);
const sessionControlLib = contents.get(SESSION_CONTROL_LIB_PATH);
const roleSessionWorktreeAdd = contents.get(ROLE_SESSION_WORKTREE_ADD_PATH);
const preWorkCheck = contents.get(PRE_WORK_CHECK_PATH);
const sessionPolicyCheck = contents.get(SESSION_POLICY_CHECK_PATH);
const refinementTemplate = contents.get(REFINEMENT_TEMPLATE_PATH);
const taskPacketTemplate = contents.get(TASK_PACKET_TEMPLATE_PATH);
const roleWorktreesDoc = contents.get(ROLE_WORKTREES_DOC_PATH);
const roleSessionRegistrySchema = contents.get(ROLE_SESSION_REGISTRY_SCHEMA_PATH);

const roleAlternation = SESSION_ROLES.join("|");
const commandKindAlternation = SESSION_COMMAND_KINDS.join("|");
const reasoningConfigPair = `${ROLE_SESSION_REASONING_CONFIG_KEY}=${ROLE_SESSION_REASONING_CONFIG_VALUE}`;
const retiredRootScriptsDir = [GOV_ROOT_REPO_REL, "scripts"].join("/");

function quotedJustGovScript(scriptPath, trailingArgs = "") {
  return `"${JUSTFILE_GOV_PREFIX}/${scriptPath}"${trailingArgs ? ` ${trailingArgs}` : ""}`;
}

// justfile recipes must stay aligned with the active orchestrator/session tooling.
// Note: justfile uses {{GOV_ROOT}} variable syntax — match against JUSTFILE_GOV_PREFIX, not resolved path.
requireRecipe(errors, justfileContent, "worktree-add", [
  `${JUSTFILE_GOV_PREFIX}/roles_shared/scripts/topology/worktree-add.mjs`,
]);
requireRecipe(errors, justfileContent, "ensure-wp-communications", [
  `${JUSTFILE_GOV_PREFIX}/roles_shared/scripts/wp/ensure-wp-communications.mjs`,
]);
requireRecipe(errors, justfileContent, "record-role-model-profiles", [
  `${JUSTFILE_GOV_PREFIX}/roles/orchestrator/checks/orchestrator_gates.mjs" profiles`,
]);
requireRecipe(errors, justfileContent, "coder-worktree-add", [
  quotedJustGovScript("roles/orchestrator/scripts/role-session-worktree-add.mjs", "CODER"),
]);
requireRecipe(errors, justfileContent, "wp-validator-worktree-add", [
  quotedJustGovScript("roles/orchestrator/scripts/role-session-worktree-add.mjs", "WP_VALIDATOR"),
]);
requireRecipe(errors, justfileContent, "integration-validator-worktree-add", [
  quotedJustGovScript("roles/orchestrator/scripts/role-session-worktree-add.mjs", "INTEGRATION_VALIDATOR"),
]);
requireRecipe(errors, justfileContent, "launch-coder-session", [
  quotedJustGovScript("roles/orchestrator/scripts/launch-cli-session.mjs", "CODER"),
]);
requireRecipe(errors, justfileContent, "launch-wp-validator-session", [
  quotedJustGovScript("roles/orchestrator/scripts/launch-cli-session.mjs", "WP_VALIDATOR"),
]);
requireRecipe(errors, justfileContent, "launch-integration-validator-session", [
  quotedJustGovScript("roles/orchestrator/scripts/launch-cli-session.mjs", "INTEGRATION_VALIDATOR"),
]);
requireRecipe(errors, justfileContent, "session-start", [
  quotedJustGovScript("roles/orchestrator/scripts/session-control-command.mjs", "START_SESSION"),
]);
requireRecipe(errors, justfileContent, "session-send", [
  quotedJustGovScript("roles/orchestrator/scripts/session-control-command.mjs", "SEND_PROMPT"),
]);
requireRecipe(errors, justfileContent, "session-cancel", [
  `${JUSTFILE_GOV_PREFIX}/roles/orchestrator/scripts/session-control-cancel.mjs`,
]);
requireRecipe(errors, justfileContent, "session-close", [
  quotedJustGovScript("roles/orchestrator/scripts/session-control-command.mjs", "CLOSE_SESSION"),
]);
requireRecipe(errors, justfileContent, "integration-validator-closeout-check", [
  `${JUSTFILE_GOV_PREFIX}/roles/validator/checks/integration-validator-closeout-check.mjs`,
]);

for (const command of [
  roleStartupCommand("ORCHESTRATOR"),
  roleStartupCommand("CODER"),
  roleStartupCommand("WP_VALIDATOR"),
  roleNextCommand("ORCHESTRATOR"),
  roleNextCommand("CODER", "WP-ALIGNMENT-CHECK"),
  roleNextCommand("WP_VALIDATOR", "WP-ALIGNMENT-CHECK"),
]) {
  const recipeName = justRecipeName(command);
  if (recipeName) requireRecipe(errors, justfileContent, recipeName);
}

requireRecipe(errors, justfileContent, "orchestrator-startup", [
  `${JUSTFILE_GOV_PREFIX}/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`,
  "just orchestrator-preflight",
]);
requireRecipe(errors, justfileContent, "validator-startup", [
  `${JUSTFILE_GOV_PREFIX}/roles/validator/VALIDATOR_PROTOCOL.md`,
  "just validator-preflight",
]);
requireRecipe(errors, justfileContent, "coder-startup", [
  `${JUSTFILE_GOV_PREFIX}/roles/coder/CODER_PROTOCOL.md`,
  "just coder-preflight",
]);

// Protocols must expose the active orchestrator-managed session contract.
for (const protocolPath of [ORCHESTRATOR_PROTOCOL_PATH, CODER_PROTOCOL_PATH, VALIDATOR_PROTOCOL_PATH]) {
  const content = contents.get(protocolPath);
  requireSubstring(errors, protocolPath, content, SESSION_START_AUTHORITY);
  requireSubstring(errors, protocolPath, content, ROLE_SESSION_PRIMARY_MODEL);
  requireSubstring(errors, protocolPath, content, ROLE_SESSION_FALLBACK_MODEL);
  requireSubstring(errors, protocolPath, content, reasoningConfigPair, reasoningConfigPair);
  requireSubstring(errors, protocolPath, content, ROLE_MODEL_PROFILE_POLICY);
}

requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, "MANUAL_RELAY");
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, "ORCHESTRATOR_MANAGED");
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, EXECUTION_OWNER_RANGE_HELP, EXECUTION_OWNER_RANGE_HELP);
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, "just launch-coder-session");
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, "just launch-wp-validator-session");
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, "just launch-integration-validator-session");
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, "just integration-validator-closeout-check");
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, "just record-role-model-profiles");
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, ROLE_MODEL_PROFILE_OPENAI_GPT_5_4_XHIGH);
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_6_THINKING_MAX);

requireSubstring(errors, CODER_PROTOCOL_PATH, coderProtocol, "just coder-startup");
requireSubstring(errors, CODER_PROTOCOL_PATH, coderProtocol, "just coder-next");
requireSubstring(errors, CODER_PROTOCOL_PATH, coderProtocol, "just launch-coder-session");
requireSubstring(errors, CODER_PROTOCOL_PATH, coderProtocol, "CODER_MODEL_PROFILE");

requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, "just validator-startup");
requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, "just validator-next");
requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, "just launch-wp-validator-session");
requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, "just launch-integration-validator-session");
requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, "just integration-validator-closeout-check");
requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, "shared remote WP backup branch");
requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_6_THINKING_MAX);
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, "## Read-Amplification and Ambiguity Discipline");
requireSubstring(errors, CODER_PROTOCOL_PATH, coderProtocol, "## Read-Amplification and Ambiguity Discipline");
requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, "## Read-Amplification and Ambiguity Discipline");
requireSubstring(errors, LAUNCH_CLI_SESSION_PATH, launchCliSession, "buildStartupPrompt");
requireSubstring(errors, LAUNCH_CLI_SESSION_PATH, launchCliSession, "session-control-command.mjs");
requireSubstring(errors, LAUNCH_CLI_SESSION_PATH, launchCliSession, '"START_SESSION"');
requireSubstring(errors, SESSION_CONTROL_COMMAND_PATH, sessionControlCommand, "buildStartupPrompt");
requireSubstring(errors, SESSION_CONTROL_LIB_PATH, sessionControlLib, "MINIMAL LIVE READ SET (MANDATORY):");
requireSubstring(errors, SESSION_CONTROL_LIB_PATH, sessionControlLib, "MODEL PROFILE:");
requireSubstring(errors, SESSION_CONTROL_LIB_PATH, sessionControlLib, ".GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md");
requireSubstring(errors, SESSION_CONTROL_LIB_PATH, sessionControlLib, "ANTI-REDISCOVERY RULE:");
requireSubstring(errors, SESSION_CONTROL_LIB_PATH, sessionControlLib, "just --list");
requireSubstring(errors, SESSION_CONTROL_LIB_PATH, sessionControlLib, "POST-SIGNATURE RELAPSE GUARD (MANDATORY):");
requireSubstring(errors, SESSION_CONTROL_LIB_PATH, sessionControlLib, "ORCHESTRATOR_MANAGED_REAL_BLOCKER_CLASSES");
requireRegex(
  errors,
  SESSION_CONTROL_LIB_PATH,
  sessionControlLib,
  /"POLICY_CONFLICT"[\s\S]*"AUTHORITY_OVERRIDE_REQUIRED"[\s\S]*"OPERATOR_ARTIFACT_REQUIRED"[\s\S]*"ENVIRONMENT_FAILURE"/,
  "orchestrator-managed blocker class set",
);
requireSubstring(errors, SESSION_CONTROL_LIB_PATH, sessionControlLib, "skeleton approval when required");
requireSubstring(errors, SESSION_CONTROL_LIB_PATH, sessionControlLib, "no routine Operator approvals after signature");
requireSubstring(errors, SESSION_CONTROL_LIB_PATH, sessionControlLib, "FLOW:");
requireSubstring(errors, ORCHESTRATOR_NEXT_PATH, orchestratorNext, "printBlockerClass");
requireSubstring(errors, ORCHESTRATOR_NEXT_PATH, orchestratorNext, "LEGACY_SIGNATURE_TUPLE_REPAIR");
requireSubstring(errors, ORCHESTRATOR_NEXT_PATH, orchestratorNext, "PRE_SIGNATURE_APPROVAL_REQUIRED");
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, "BLOCKER_CLASS");
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, "routine Operator interruption ends after signature/prepare");
requireSubstring(errors, CODER_PROTOCOL_PATH, coderProtocol, "do not ask the Operator for routine approval, \"proceed\", or checkpoint actions");
requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, "do not ask the Operator for routine approval, \"proceed\", or checkpoint actions");

// Scripts must expose the current role/command contract and point at active paths.
requireRegex(
  errors,
  LAUNCH_CLI_SESSION_PATH,
  launchCliSession,
  new RegExp(`<${escapeRegex(roleAlternation)}>`),
  `role set <${roleAlternation}> in usage/help`,
);
requireRegex(
  errors,
  SESSION_CONTROL_COMMAND_PATH,
  sessionControlCommand,
  new RegExp(`<${escapeRegex(commandKindAlternation)}>`),
  `command kind set <${commandKindAlternation}> in usage/help`,
);
requireRegex(
  errors,
  SESSION_CONTROL_COMMAND_PATH,
  sessionControlCommand,
  new RegExp(`<${escapeRegex(roleAlternation)}>`),
  `role set <${roleAlternation}> in usage/help`,
);
requireRegex(
  errors,
  SESSION_CONTROL_CANCEL_PATH,
  sessionControlCancel,
  new RegExp(`<${escapeRegex(roleAlternation)}>`),
  `role set <${roleAlternation}> in usage/help`,
);
requireRegex(
  errors,
  ROLE_SESSION_WORKTREE_ADD_PATH,
  roleSessionWorktreeAdd,
  new RegExp(`<${escapeRegex(roleAlternation)}>`),
  `role set <${roleAlternation}> in usage/help`,
);
requireRegex(
  errors,
  ROLE_SESSION_WORKTREE_ADD_PATH,
  roleSessionWorktreeAdd,
  /path\.join\(\s*GOV_ROOT_REPO_REL\s*,\s*"roles_shared"\s*,\s*"scripts"\s*,\s*"topology"\s*,\s*"worktree-add\.mjs"\s*\)/,
  "current roles_shared topology worktree-add path",
);
requireRegex(
  errors,
  SESSION_CONTROL_COMMAND_PATH,
  sessionControlCommand,
  /path\.join\(\s*GOV_ROOT_REPO_REL\s*,\s*"roles"\s*,\s*"orchestrator"\s*,\s*"scripts"\s*,\s*"role-session-worktree-add\.mjs"\s*\)/,
  "current orchestrator role-session-worktree-add path",
);

requireSubstring(errors, ORCHESTRATOR_GATES_PATH, orchestratorGates, "MANUAL_RELAY|ORCHESTRATOR_MANAGED");
requireSubstring(errors, ORCHESTRATOR_GATES_PATH, orchestratorGates, "ROLE_MODEL_PROFILES");
requireSubstring(
  errors,
  ORCHESTRATOR_GATES_PATH,
  orchestratorGates,
  "EXECUTION_OWNER_RANGE_HELP",
  "shared execution-owner range constant",
);
requireRegex(
  errors,
  CREATE_TASK_PACKET_PATH,
  createTaskPacket,
  /const wpValidatorRemoteBackupBranch = remoteBackupBranch;/,
  "shared WP backup branch assignment for WP Validator packet fields",
);
requireRegex(
  errors,
  CREATE_TASK_PACKET_PATH,
  createTaskPacket,
  /const integrationValidatorRemoteBackupBranch = remoteBackupBranch;/,
  "shared WP backup branch assignment for Integration Validator packet fields",
);
requireSubstring(
  errors,
  TASK_PACKET_TEMPLATE_PATH,
  taskPacketTemplate,
  "Do not create separate validator-only remote WP backup branches.",
);
requireSubstring(
  errors,
  ROLE_WORKTREES_DOC_PATH,
  roleWorktreesDoc,
  "single packet-declared WP backup branch on GitHub",
);
requireSubstring(
  errors,
  PRE_WORK_CHECK_PATH,
  preWorkCheck,
  "WP_VALIDATOR_REMOTE_BACKUP_URL must mirror REMOTE_BACKUP_URL",
);
requireSubstring(
  errors,
  PRE_WORK_CHECK_PATH,
  preWorkCheck,
  "INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL must mirror REMOTE_BACKUP_URL",
);
requireSubstring(
  errors,
  SESSION_POLICY_CHECK_PATH,
  sessionPolicyCheck,
  'checkMirrorField(errors, rel, text, "WP_VALIDATOR_REMOTE_BACKUP_BRANCH", "REMOTE_BACKUP_BRANCH");',
);
requireSubstring(
  errors,
  SESSION_POLICY_CHECK_PATH,
  sessionPolicyCheck,
  'checkMirrorField(errors, rel, text, "INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH", "REMOTE_BACKUP_BRANCH");',
);
requireSubstring(
  errors,
  SESSION_POLICY_CHECK_PATH,
  sessionPolicyCheck,
  'checkMirrorField(errors, rel, text, "WP_VALIDATOR_REMOTE_BACKUP_URL", "REMOTE_BACKUP_URL");',
);
requireSubstring(
  errors,
  SESSION_POLICY_CHECK_PATH,
  sessionPolicyCheck,
  'checkMirrorField(errors, rel, text, "INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL", "REMOTE_BACKUP_URL");',
);
requireSubstring(
  errors,
  ROLE_SESSION_REGISTRY_SCHEMA_PATH,
  roleSessionRegistrySchema,
  'roles_shared/SESSION_LAUNCH_REQUESTS\\\\.jsonl',
  "runtime SESSION_LAUNCH_REQUESTS schema filename",
);
requireSubstring(
  errors,
  ROLE_SESSION_REGISTRY_SCHEMA_PATH,
  roleSessionRegistrySchema,
  'roles_shared/SESSION_CONTROL_REQUESTS\\\\.jsonl',
  "runtime SESSION_CONTROL_REQUESTS schema filename",
);
requireSubstring(
  errors,
  ROLE_SESSION_REGISTRY_SCHEMA_PATH,
  roleSessionRegistrySchema,
  'roles_shared/SESSION_CONTROL_RESULTS\\\\.jsonl',
  "runtime SESSION_CONTROL_RESULTS schema filename",
);
requireSubstring(
  errors,
  ROLE_SESSION_REGISTRY_SCHEMA_PATH,
  roleSessionRegistrySchema,
  'roles_shared/SESSION_CONTROL_OUTPUTS',
  "runtime SESSION_CONTROL_OUTPUTS schema directory",
);

// Refinement scaffolding must stay on the current refinement workflow contract.
requireSubstring(errors, REFINEMENT_TEMPLATE_PATH, refinementTemplate, "REFINEMENT_FORMAT_VERSION: 2026-03-16");
requireSubstring(errors, REFINEMENT_TEMPLATE_PATH, refinementTemplate, "USER_APPROVAL_EVIDENCE");
forbidRegex(
  errors,
  REFINEMENT_TEMPLATE_PATH,
  refinementTemplate,
  /ORCHESTRATOR_PROTOCOL Part/i,
  "stale ORCHESTRATOR_PROTOCOL part-number reference",
);

// Active surfaces must not point at retired orchestrator/session paths.
const retiredPatterns = [
  {
    regex: new RegExp(`${escapeRegex(retiredRootScriptsDir)}/session/role-session-worktree-add\\.mjs`),
    label: `retired ${retiredRootScriptsDir}/session/role-session-worktree-add.mjs`,
  },
  {
    regex: new RegExp(`${escapeRegex(retiredRootScriptsDir)}/topology/worktree-add\\.mjs`),
    label: `retired ${retiredRootScriptsDir}/topology/worktree-add.mjs`,
  },
  { regex: /path\.join\(\s*["']\.GOV["']\s*,\s*["']scripts["']\s*,\s*["']session["']/, label: 'path.join(".GOV", "scripts", "session", ...)' },
  { regex: /path\.join\(\s*["']\.GOV["']\s*,\s*["']scripts["']\s*,\s*["']topology["']/, label: 'path.join(".GOV", "scripts", "topology", ...)' },
];

for (const filePath of ACTIVE_SURFACE_PATHS) {
  const content = contents.get(filePath);
  for (const { regex, label } of retiredPatterns) {
    forbidRegex(errors, filePath, content, regex, label);
  }
}

if (errors.length > 0) {
  fail("Protocol alignment violations found", errors);
}

console.log("protocol-alignment-check ok");
