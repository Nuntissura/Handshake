import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  ROLE_MODEL_PROFILE_POLICY,
  ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH,
  ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_6_THINKING_MAX,
  ROLE_MODEL_PROFILE_OPENAI_GPT_5_5_XHIGH,
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
import { GOV_ROOT_ABS, GOV_ROOT_REPO_REL } from "../scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("protocol-alignment-check.mjs", { role: "SHARED" });

const JUSTFILE_PATH = "justfile";
// The justfile uses {{GOV_ROOT}} (a just variable) — when matching raw justfile text,
// we must use the literal justfile variable syntax, not the resolved GOV_ROOT_REPO_REL.
const JUSTFILE_GOV_PREFIX = "{{GOV_ROOT}}";
const CODEX_PATH = path.join(GOV_ROOT_REPO_REL, "codex", "Handshake_Codex_v1.4.md");
const ORCHESTRATOR_PROTOCOL_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "ORCHESTRATOR_PROTOCOL.md");
const CLASSIC_ORCHESTRATOR_PROTOCOL_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "classic_orchestrator", "CLASSIC_ORCHESTRATOR_PROTOCOL.md");
const CODER_PROTOCOL_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "coder", "CODER_PROTOCOL.md");
const WP_VALIDATOR_PROTOCOL_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "wp_validator", "WP_VALIDATOR_PROTOCOL.md");
const INTEGRATION_VALIDATOR_PROTOCOL_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "integration_validator", "INTEGRATION_VALIDATOR_PROTOCOL.md");
const VALIDATOR_PROTOCOL_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "validator", "VALIDATOR_PROTOCOL.md");
const MEMORY_MANAGER_PROTOCOL_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "memory_manager", "MEMORY_MANAGER_PROTOCOL.md");
const ACTIVATION_MANAGER_PROTOCOL_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "activation_manager", "ACTIVATION_MANAGER_PROTOCOL.md");
const ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "docs", "ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md");
const SHARED_STARTUP_BRIEF_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "docs", "SHARED_STARTUP_BRIEF.md");
const COMMAND_SURFACE_REFERENCE_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "docs", "COMMAND_SURFACE_REFERENCE.md");
const ROLE_WORKFLOW_QUICKREF_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "docs", "ROLE_WORKFLOW_QUICKREF.md");
const ROLE_SESSION_ORCHESTRATION_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "docs", "ROLE_SESSION_ORCHESTRATION.md");
const GOVERNED_WORKFLOW_EXAMPLES_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "docs", "GOVERNED_WORKFLOW_EXAMPLES.md");
const STARTUP_BRIEF_SCHEMA_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "docs", "STARTUP_BRIEF_SCHEMA.md");
const OPERATOR_STARTUP_PROMPTS_PATH = path.join(GOV_ROOT_REPO_REL, "operator", "docs_local", "Handshake_Role_Startup_Prompts.md");
const ORCSTART_PROMPT_PATH = path.join(GOV_ROOT_REPO_REL, "operator", "scripts", "orcstart.prompt.txt");
const ORCHESTRATOR_STARTUP_BRIEF_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "docs", "ORCHESTRATOR_STARTUP_BRIEF.md");
const CLASSIC_ORCHESTRATOR_STARTUP_BRIEF_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "classic_orchestrator", "docs", "CLASSIC_ORCHESTRATOR_STARTUP_BRIEF.md");
const ACTIVATION_MANAGER_STARTUP_BRIEF_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "activation_manager", "docs", "ACTIVATION_MANAGER_STARTUP_BRIEF.md");
const WP_VALIDATOR_STARTUP_BRIEF_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "wp_validator", "docs", "WP_VALIDATOR_STARTUP_BRIEF.md");
const INTEGRATION_VALIDATOR_STARTUP_BRIEF_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "integration_validator", "docs", "INTEGRATION_VALIDATOR_STARTUP_BRIEF.md");
const VALIDATOR_STARTUP_BRIEF_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "validator", "docs", "VALIDATOR_STARTUP_BRIEF.md");
const MEMORY_MANAGER_STARTUP_BRIEF_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "memory_manager", "docs", "MEMORY_MANAGER_STARTUP_BRIEF.md");
const ORCHESTRATOR_GATES_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "checks", "orchestrator_gates.mjs");
const ORCHESTRATOR_NEXT_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "orchestrator-next.mjs");
const ORCHESTRATOR_RESCUE_LIB_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "orchestrator-rescue-lib.mjs");
const CREATE_TASK_PACKET_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "create-task-packet.mjs");
const LAUNCH_CLI_SESSION_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "launch-cli-session.mjs");
const SESSION_CONTROL_COMMAND_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "session-control-command.mjs");
const SESSION_CONTROL_CANCEL_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "session-control-cancel.mjs");
const SESSION_CONTROL_LIB_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "scripts", "session", "session-control-lib.mjs");
const WP_LANE_HEALTH_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "scripts", "session", "wp-lane-health.mjs");
const WP_RELAY_WATCHDOG_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "wp-relay-watchdog.mjs");
const WP_AUTONOMOUS_MONITOR_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "wp-autonomous-monitor.mjs");
const ROLE_SESSION_WORKTREE_ADD_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "role-session-worktree-add.mjs");
const PRE_WORK_CHECK_PATH = path.join(GOV_ROOT_REPO_REL, "roles", "coder", "checks", "pre-work-check.mjs");
const SESSION_POLICY_CHECK_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "checks", "session-policy-check.mjs");
const REFINEMENT_TEMPLATE_PATH = path.join(GOV_ROOT_REPO_REL, "templates", "REFINEMENT_TEMPLATE.md");
const TASK_PACKET_TEMPLATE_PATH = path.join(GOV_ROOT_REPO_REL, "templates", "TASK_PACKET_TEMPLATE.md");
const ROLE_WORKTREES_DOC_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "docs", "ROLE_WORKTREES.md");
const ROLE_SESSION_REGISTRY_SCHEMA_PATH = path.join(GOV_ROOT_REPO_REL, "roles_shared", "schemas", "ROLE_SESSION_REGISTRY.schema.json");

const ACTIVE_SURFACE_PATHS = [
  JUSTFILE_PATH,
  CODEX_PATH,
  ORCHESTRATOR_PROTOCOL_PATH,
  CLASSIC_ORCHESTRATOR_PROTOCOL_PATH,
  CODER_PROTOCOL_PATH,
  WP_VALIDATOR_PROTOCOL_PATH,
  INTEGRATION_VALIDATOR_PROTOCOL_PATH,
  VALIDATOR_PROTOCOL_PATH,
  MEMORY_MANAGER_PROTOCOL_PATH,
  ACTIVATION_MANAGER_PROTOCOL_PATH,
  ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK_PATH,
  SHARED_STARTUP_BRIEF_PATH,
  COMMAND_SURFACE_REFERENCE_PATH,
  ROLE_WORKFLOW_QUICKREF_PATH,
  ROLE_SESSION_ORCHESTRATION_PATH,
  GOVERNED_WORKFLOW_EXAMPLES_PATH,
  STARTUP_BRIEF_SCHEMA_PATH,
  OPERATOR_STARTUP_PROMPTS_PATH,
  ORCSTART_PROMPT_PATH,
  ORCHESTRATOR_STARTUP_BRIEF_PATH,
  CLASSIC_ORCHESTRATOR_STARTUP_BRIEF_PATH,
  ACTIVATION_MANAGER_STARTUP_BRIEF_PATH,
  WP_VALIDATOR_STARTUP_BRIEF_PATH,
  INTEGRATION_VALIDATOR_STARTUP_BRIEF_PATH,
  VALIDATOR_STARTUP_BRIEF_PATH,
  MEMORY_MANAGER_STARTUP_BRIEF_PATH,
  ORCHESTRATOR_GATES_PATH,
  ORCHESTRATOR_NEXT_PATH,
  ORCHESTRATOR_RESCUE_LIB_PATH,
  CREATE_TASK_PACKET_PATH,
  LAUNCH_CLI_SESSION_PATH,
  SESSION_CONTROL_COMMAND_PATH,
  SESSION_CONTROL_CANCEL_PATH,
  SESSION_CONTROL_LIB_PATH,
  WP_LANE_HEALTH_PATH,
  WP_RELAY_WATCHDOG_PATH,
  WP_AUTONOMOUS_MONITOR_PATH,
  ROLE_SESSION_WORKTREE_ADD_PATH,
  PRE_WORK_CHECK_PATH,
  SESSION_POLICY_CHECK_PATH,
  REFINEMENT_TEMPLATE_PATH,
  TASK_PACKET_TEMPLATE_PATH,
  ROLE_WORKTREES_DOC_PATH,
  ROLE_SESSION_REGISTRY_SCHEMA_PATH,
];

function resolveGovernanceRepoRoot() {
  const governedRepoRoot = path.resolve(GOV_ROOT_ABS, "..");
  if (fs.existsSync(governedRepoRoot)) {
    return governedRepoRoot;
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
  failWithMemory("protocol-alignment-check.mjs", message, { role: "SHARED", details });
}

function escapeRegex(value) {
  return String(value || "").replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function readUtf8(filePath) {
  return fs.readFileSync(path.join(governanceRepoRoot, filePath), "utf8");
}

function requireFileExists(filePath) {
  if (!fs.existsSync(path.join(governanceRepoRoot, filePath))) {
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

const governanceRepoRoot = path.resolve(resolveGovernanceRepoRoot());
process.chdir(governanceRepoRoot);

for (const filePath of ACTIVE_SURFACE_PATHS) requireFileExists(filePath);

const contents = new Map(ACTIVE_SURFACE_PATHS.map((filePath) => [filePath, readUtf8(filePath)]));
const errors = [];

const justfileContent = contents.get(JUSTFILE_PATH);
const codexContent = contents.get(CODEX_PATH);
const orchestratorProtocol = contents.get(ORCHESTRATOR_PROTOCOL_PATH);
const classicOrchestratorProtocol = contents.get(CLASSIC_ORCHESTRATOR_PROTOCOL_PATH);
const coderProtocol = contents.get(CODER_PROTOCOL_PATH);
const wpValidatorProtocol = contents.get(WP_VALIDATOR_PROTOCOL_PATH);
const integrationValidatorProtocol = contents.get(INTEGRATION_VALIDATOR_PROTOCOL_PATH);
const validatorProtocol = contents.get(VALIDATOR_PROTOCOL_PATH);
const memoryManagerProtocol = contents.get(MEMORY_MANAGER_PROTOCOL_PATH);
const activationManagerProtocol = contents.get(ACTIVATION_MANAGER_PROTOCOL_PATH);
const orchestratorManagedWorkflowPlaybook = contents.get(ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK_PATH);
const sharedStartupBrief = contents.get(SHARED_STARTUP_BRIEF_PATH);
const commandSurfaceReference = contents.get(COMMAND_SURFACE_REFERENCE_PATH);
const roleWorkflowQuickref = contents.get(ROLE_WORKFLOW_QUICKREF_PATH);
const roleSessionOrchestration = contents.get(ROLE_SESSION_ORCHESTRATION_PATH);
const governedWorkflowExamples = contents.get(GOVERNED_WORKFLOW_EXAMPLES_PATH);
const startupBriefSchema = contents.get(STARTUP_BRIEF_SCHEMA_PATH);
const operatorStartupPrompts = contents.get(OPERATOR_STARTUP_PROMPTS_PATH);
const orcstartPrompt = contents.get(ORCSTART_PROMPT_PATH);
const orchestratorStartupBrief = contents.get(ORCHESTRATOR_STARTUP_BRIEF_PATH);
const classicOrchestratorStartupBrief = contents.get(CLASSIC_ORCHESTRATOR_STARTUP_BRIEF_PATH);
const activationManagerStartupBrief = contents.get(ACTIVATION_MANAGER_STARTUP_BRIEF_PATH);
const wpValidatorStartupBrief = contents.get(WP_VALIDATOR_STARTUP_BRIEF_PATH);
const integrationValidatorStartupBrief = contents.get(INTEGRATION_VALIDATOR_STARTUP_BRIEF_PATH);
const validatorStartupBrief = contents.get(VALIDATOR_STARTUP_BRIEF_PATH);
const memoryManagerStartupBrief = contents.get(MEMORY_MANAGER_STARTUP_BRIEF_PATH);
const orchestratorGates = contents.get(ORCHESTRATOR_GATES_PATH);
const orchestratorNext = contents.get(ORCHESTRATOR_NEXT_PATH);
const orchestratorRescueLib = contents.get(ORCHESTRATOR_RESCUE_LIB_PATH);
const createTaskPacket = contents.get(CREATE_TASK_PACKET_PATH);
const launchCliSession = contents.get(LAUNCH_CLI_SESSION_PATH);
const sessionControlCommand = contents.get(SESSION_CONTROL_COMMAND_PATH);
const sessionControlCancel = contents.get(SESSION_CONTROL_CANCEL_PATH);
const sessionControlLib = contents.get(SESSION_CONTROL_LIB_PATH);
const wpLaneHealth = contents.get(WP_LANE_HEALTH_PATH);
const wpRelayWatchdog = contents.get(WP_RELAY_WATCHDOG_PATH);
const wpAutonomousMonitor = contents.get(WP_AUTONOMOUS_MONITOR_PATH);
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
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, "just phase-check CLOSEOUT");
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, "just record-role-model-profiles");
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, ROLE_MODEL_PROFILE_OPENAI_GPT_5_5_XHIGH);
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, ROLE_MODEL_PROFILE_OPENAI_GPT_5_4_XHIGH);
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH);
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, ROLE_MODEL_PROFILE_CLAUDE_CODE_OPUS_4_6_THINKING_MAX);

requireSubstring(errors, CODER_PROTOCOL_PATH, coderProtocol, "just coder-startup");
requireSubstring(errors, CODER_PROTOCOL_PATH, coderProtocol, "just coder-next");
requireSubstring(errors, CODER_PROTOCOL_PATH, coderProtocol, "just launch-coder-session");
requireSubstring(errors, CODER_PROTOCOL_PATH, coderProtocol, "CODER_MODEL_PROFILE");

requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, "just validator-startup WP_VALIDATOR|INTEGRATION_VALIDATOR|VALIDATOR");
requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, "just validator-next WP_VALIDATOR|INTEGRATION_VALIDATOR|VALIDATOR");
requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, "just launch-wp-validator-session");
requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, "just launch-integration-validator-session");
requireSubstring(errors, ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol, "just phase-check STARTUP");
requireSubstring(errors, CODER_PROTOCOL_PATH, coderProtocol, "just phase-check STARTUP");
requireSubstring(errors, VALIDATOR_PROTOCOL_PATH, validatorProtocol, "just phase-check CLOSEOUT");
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
for (const [filePath, content] of [
  [ORCHESTRATOR_PROTOCOL_PATH, orchestratorProtocol],
  [CODER_PROTOCOL_PATH, coderProtocol],
  [VALIDATOR_PROTOCOL_PATH, validatorProtocol],
  [SESSION_CONTROL_LIB_PATH, sessionControlLib],
]) {
  forbidRegex(errors, filePath, content, /just post-work\b/, "just post-work");
  forbidRegex(errors, filePath, content, /just pre-work\b/, "just pre-work");
  forbidRegex(errors, filePath, content, /just gate-check\b/, "just gate-check");
  forbidRegex(errors, filePath, content, /just validator-packet-complete\b/, "just validator-packet-complete");
  forbidRegex(errors, filePath, content, /just validator-handoff-check\b/, "just validator-handoff-check");
  forbidRegex(errors, filePath, content, /just integration-validator-closeout-check\b/, "just integration-validator-closeout-check");
}
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

// CX-218K: mechanical intervention discipline must stay present on every role
// that can patch, steer, relay, validate, activate, or propose workflow repairs.
const MECHANICAL_INTERVENTION_SURFACES = [
  { filePath: CODEX_PATH, content: codexContent, requirePlaybook: true },
  { filePath: ORCHESTRATOR_PROTOCOL_PATH, content: orchestratorProtocol, requirePlaybook: true },
  { filePath: CLASSIC_ORCHESTRATOR_PROTOCOL_PATH, content: classicOrchestratorProtocol, requirePlaybook: false },
  { filePath: CODER_PROTOCOL_PATH, content: coderProtocol, requirePlaybook: true },
  { filePath: WP_VALIDATOR_PROTOCOL_PATH, content: wpValidatorProtocol, requirePlaybook: true },
  { filePath: INTEGRATION_VALIDATOR_PROTOCOL_PATH, content: integrationValidatorProtocol, requirePlaybook: true },
  { filePath: VALIDATOR_PROTOCOL_PATH, content: validatorProtocol, requirePlaybook: false },
  { filePath: MEMORY_MANAGER_PROTOCOL_PATH, content: memoryManagerProtocol, requirePlaybook: true },
  { filePath: ACTIVATION_MANAGER_PROTOCOL_PATH, content: activationManagerProtocol, requirePlaybook: true },
];

for (const { filePath, content, requirePlaybook } of MECHANICAL_INTERVENTION_SURFACES) {
  requireSubstring(errors, filePath, content, "CX-218K", "CX-218K mechanical intervention authority");
  requireRegex(errors, filePath, content, /3-5\s+plausible\s+causes/i, "3-5 plausible causes triage");
  requireSubstring(errors, filePath, content, "documentation/protocol drift", "documentation/protocol drift cause");
  requireRegex(errors, filePath, content, /session\/ACP drift|ACP\/session ambiguity|session\/ACP/i, "session/ACP drift cause");
  requireRegex(errors, filePath, content, /cheapest\s+deterministic/i, "cheapest deterministic read/repair/helper rule");
  requireRegex(errors, filePath, content, /manually\s+(relay|broker)\s+ordinary/i, "no manual ordinary-content relay when typed helper exists");
  if (requirePlaybook) {
    requireSubstring(
      errors,
      filePath,
      content,
      ".GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md",
      "orchestrator-managed workflow playbook reference",
    );
  }
}

const ROLE_STARTUP_BRIEF_MECHANICAL_SURFACES = [
  { filePath: ORCHESTRATOR_STARTUP_BRIEF_PATH, content: orchestratorStartupBrief },
  { filePath: CLASSIC_ORCHESTRATOR_STARTUP_BRIEF_PATH, content: classicOrchestratorStartupBrief },
  { filePath: ACTIVATION_MANAGER_STARTUP_BRIEF_PATH, content: activationManagerStartupBrief },
  { filePath: WP_VALIDATOR_STARTUP_BRIEF_PATH, content: wpValidatorStartupBrief },
  { filePath: INTEGRATION_VALIDATOR_STARTUP_BRIEF_PATH, content: integrationValidatorStartupBrief },
  { filePath: VALIDATOR_STARTUP_BRIEF_PATH, content: validatorStartupBrief },
  { filePath: MEMORY_MANAGER_STARTUP_BRIEF_PATH, content: memoryManagerStartupBrief },
];

for (const { filePath, content } of ROLE_STARTUP_BRIEF_MECHANICAL_SURFACES) {
  requireSubstring(errors, filePath, content, "CX-218K", "CX-218K role-startup intervention card");
  requireRegex(errors, filePath, content, /3-5\s+plausible\s+causes/i, "3-5 plausible causes triage");
  requireSubstring(errors, filePath, content, "documentation/protocol drift", "documentation/protocol drift cause");
  requireRegex(errors, filePath, content, /session\/ACP drift|ACP\/session ambiguity|session\/ACP/i, "session/ACP drift cause");
  requireRegex(errors, filePath, content, /cheapest\s+deterministic/i, "cheapest deterministic read/repair/helper rule");
  requireRegex(errors, filePath, content, /manually\s+(relay|broker)\s+ordinary/i, "no manual ordinary-content relay when typed helper exists");
}

requireSubstring(errors, ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK_PATH, orchestratorManagedWorkflowPlaybook, "## Stall Patterns");
requireSubstring(errors, ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK_PATH, orchestratorManagedWorkflowPlaybook, "Post-Commit Auto-Relay Does Not Fire");
requireSubstring(errors, ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK_PATH, orchestratorManagedWorkflowPlaybook, "Final Handoff Closeout Inversion");
requireSubstring(errors, ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK_PATH, orchestratorManagedWorkflowPlaybook, "Closeout Report Materialization Drift");
requireSubstring(errors, ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK_PATH, orchestratorManagedWorkflowPlaybook, "Merge-Pending Terminal Projection");
requireSubstring(errors, ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK_PATH, orchestratorManagedWorkflowPlaybook, "Main Containment Drift");
requireSubstring(errors, ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK_PATH, orchestratorManagedWorkflowPlaybook, "TERMINAL_HISTORY_HIDDEN");
requireSubstring(errors, STARTUP_BRIEF_SCHEMA_PATH, startupBriefSchema, "Every active role startup brief must include one `CX-218K` mechanical intervention card");
requireSubstring(errors, STARTUP_BRIEF_SCHEMA_PATH, startupBriefSchema, "cheapest deterministic read, repair, or typed helper");
requireRegex(errors, STARTUP_BRIEF_SCHEMA_PATH, startupBriefSchema, /manually\s+relaying\s+or\s+brokering\s+ordinary/i, "startup schema no manual ordinary-content relay");

const MECHANICAL_STARTUP_REFERENCE_SURFACES = [
  { filePath: SHARED_STARTUP_BRIEF_PATH, content: sharedStartupBrief, requirePlaybook: true },
  { filePath: COMMAND_SURFACE_REFERENCE_PATH, content: commandSurfaceReference, requirePlaybook: true },
  { filePath: ROLE_WORKFLOW_QUICKREF_PATH, content: roleWorkflowQuickref, requirePlaybook: true },
  { filePath: ROLE_SESSION_ORCHESTRATION_PATH, content: roleSessionOrchestration, requirePlaybook: false },
  { filePath: OPERATOR_STARTUP_PROMPTS_PATH, content: operatorStartupPrompts, requirePlaybook: true },
  { filePath: ORCSTART_PROMPT_PATH, content: orcstartPrompt, requirePlaybook: true },
  { filePath: ORCHESTRATOR_STARTUP_BRIEF_PATH, content: orchestratorStartupBrief, requirePlaybook: false },
  { filePath: SESSION_CONTROL_LIB_PATH, content: sessionControlLib, requirePlaybook: true },
  { filePath: ORCHESTRATOR_RESCUE_LIB_PATH, content: orchestratorRescueLib, requirePlaybook: false },
];

for (const { filePath, content, requirePlaybook } of MECHANICAL_STARTUP_REFERENCE_SURFACES) {
  requireSubstring(errors, filePath, content, "CX-218K", "CX-218K startup/reference intervention discipline");
  requireRegex(errors, filePath, content, /3-5\s+plausible\s+causes/i, "3-5 plausible causes triage");
  requireSubstring(errors, filePath, content, "documentation/protocol drift", "documentation/protocol drift cause");
  requireRegex(errors, filePath, content, /session\/ACP drift|ACP\/session ambiguity|session\/ACP/i, "session/ACP drift cause");
  if (requirePlaybook) {
    requireSubstring(errors, filePath, content, ".GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md", "orchestrator-managed workflow playbook reference");
  }
}

requireSubstring(errors, OPERATOR_STARTUP_PROMPTS_PATH, operatorStartupPrompts, "phase-check HANDOFF WP-{ID} WP_VALIDATOR --range <base>..<head>");
requireSubstring(errors, OPERATOR_STARTUP_PROMPTS_PATH, operatorStartupPrompts, "phase-check VERDICT");
requireSubstring(errors, ORCSTART_PROMPT_PATH, orcstartPrompt, "phase-check HANDOFF WP-{ID} WP_VALIDATOR --range <base>..<head>");
requireSubstring(errors, ORCSTART_PROMPT_PATH, orcstartPrompt, "review/verdict response");
requireSubstring(errors, ORCHESTRATOR_STARTUP_BRIEF_PATH, orchestratorStartupBrief, "phase-check VERDICT WP-{ID} INTEGRATION_VALIDATOR <session>");
requireSubstring(errors, ORCHESTRATOR_RESCUE_LIB_PATH, orchestratorRescueLib, "phase-check HANDOFF WP-{ID} WP_VALIDATOR --range <base>..<head>");
requireSubstring(errors, GOVERNED_WORKFLOW_EXAMPLES_PATH, governedWorkflowExamples, "phase-check VERDICT WP-{ID} INTEGRATION_VALIDATOR <intval_session>");
forbidRegex(errors, GOVERNED_WORKFLOW_EXAMPLES_PATH, governedWorkflowExamples, /Final merge-readiness review request/, "stale final Integration Validator review request example");
forbidRegex(errors, GOVERNED_WORKFLOW_EXAMPLES_PATH, governedWorkflowExamples, /REVIEW_REQUEST WP-\{ID\} INTEGRATION_VALIDATOR/, "Integration Validator opening final review request to Coder");
forbidRegex(errors, OPERATOR_STARTUP_PROMPTS_PATH, operatorStartupPrompts, /unresolved overlap queue at 2 or less/i, "stale overlap queue size");

// Lane diagnostics must not regress to default-path or active-stall assumptions after terminal closeout.
requireSubstring(errors, WP_LANE_HEALTH_PATH, wpLaneHealth, "loadPacketContextForWp", "packet-declared WP communication context");
requireSubstring(errors, WP_LANE_HEALTH_PATH, wpLaneHealth, "readExecutionPublicationView", "packet/task-board publication terminal view");
requireSubstring(errors, WP_LANE_HEALTH_PATH, wpLaneHealth, "runtimeStatus: parseJsonFile(runtimeStatusFile)", "publication view object-call runtime input");
requireSubstring(errors, WP_LANE_HEALTH_PATH, wpLaneHealth, "isTerminalPacketStatus", "packet terminal status fence");
requireSubstring(errors, WP_LANE_HEALTH_PATH, wpLaneHealth, "packetContext.notificationsFile", "packet-declared notifications file");
requireSubstring(errors, WP_LANE_HEALTH_PATH, wpLaneHealth, "packetContext.receiptsFile", "packet-declared receipts file");
requireSubstring(errors, WP_LANE_HEALTH_PATH, wpLaneHealth, '["-C", worktreeDir, "rev-parse", "--git-path", "hooks/post-commit"]', "Git effective hook path check");
requireSubstring(errors, WP_LANE_HEALTH_PATH, wpLaneHealth, "Terminal history suppression", "terminal history suppression");
for (const [filePath, content] of [
  [WP_RELAY_WATCHDOG_PATH, wpRelayWatchdog],
  [WP_AUTONOMOUS_MONITOR_PATH, wpAutonomousMonitor],
]) {
  requireSubstring(errors, filePath, content, "readExecutionPublicationView", "packet/task-board publication terminal view");
  requireSubstring(errors, filePath, content, "parsePacketStatus", "packet artifact status parse");
  requireSubstring(errors, filePath, content, "readTaskBoardStatusForWp", "task-board artifact status parse");
  requireSubstring(errors, filePath, content, "isTerminalTaskBoardStatus", "task-board terminal fence");
  forbidRegex(errors, filePath, content, /materializeRuntimeAuthorityView/, "runtime-only terminal projection");
}
requireSubstring(errors, WP_RELAY_WATCHDOG_PATH, wpRelayWatchdog, "TERMINAL_HISTORY_HIDDEN", "watchdog terminal history skip reason");
requireSubstring(errors, WP_AUTONOMOUS_MONITOR_PATH, wpAutonomousMonitor, "terminal=YES publication=", "monitor terminal publication log");
forbidRegex(
  errors,
  WP_LANE_HEALTH_PATH,
  wpLaneHealth,
  /path\.join\(\s*REPO_ROOT\s*,\s*"\.\."\s*,\s*"gov_runtime"\s*,\s*"roles_shared"\s*,\s*"WP_COMMUNICATIONS"/,
  "default WP_COMMUNICATIONS runtime path in lane-health",
);
forbidRegex(
  errors,
  SESSION_CONTROL_LIB_PATH,
  sessionControlLib,
  /WP_COMMUNICATIONS\/\$\{wpId\}\/RUNTIME_STATUS\.json/,
  "startup prompt hard-coded WP runtime status path",
);

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

// RGF-252: every governed role protocol must declare the seven required repomem clauses
// (SESSION_OPEN, FAIL CAPTURE, DECISION, INSIGHT, CONCERN, ESCALATION, SESSION_CLOSE).
// Legacy validator surface previously only declared SESSION_OPEN/INSIGHT/SESSION_CLOSE,
// which left verdict reasoning, abandoned paths, and tool failures unrecorded.
const REQUIRED_REPOMEM_CLAUSES = [
  { label: "SESSION_OPEN", regex: /\*\*SESSION_OPEN\s*\(MUST\)[^*]*\*\*/ },
  // FAIL CAPTURE uses `just memory-capture procedural`, not repomem; phrasing differs
  // across protocols ("Fail capture", "FAIL CAPTURE", "fail capture") so we match loosely.
  { label: "FAIL CAPTURE", regex: /\*\*\s*Fail capture\s*\(MUST\)[^*]*\*\*|memory-capture\s+procedural/i },
  { label: "DECISION", regex: /\*\*DECISION\b[^*]*\*\*/ },
  { label: "INSIGHT", regex: /\*\*INSIGHT\b[^*]*\*\*/ },
  { label: "CONCERN", regex: /\*\*CONCERN\b[^*]*\*\*/ },
  { label: "ESCALATION", regex: /\*\*ESCALATION\b[^*]*\*\*/ },
  { label: "SESSION_CLOSE", regex: /\*\*SESSION_CLOSE\s*\(MUST\)[^*]*\*\*/ },
];

const REPOMEM_CLAUSE_PROTOCOLS = [
  { label: "ORCHESTRATOR_PROTOCOL.md", filePath: ORCHESTRATOR_PROTOCOL_PATH, content: orchestratorProtocol },
  { label: "CODER_PROTOCOL.md", filePath: CODER_PROTOCOL_PATH, content: coderProtocol },
  { label: "VALIDATOR_PROTOCOL.md", filePath: VALIDATOR_PROTOCOL_PATH, content: validatorProtocol },
];

for (const { filePath, content } of REPOMEM_CLAUSE_PROTOCOLS) {
  for (const { label, regex } of REQUIRED_REPOMEM_CLAUSES) {
    if (!regex.test(content)) {
      errors.push(`${filePath}: missing required repomem clause ${label}`);
    }
  }
}

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
