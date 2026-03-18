import fs from "node:fs";
import path from "node:path";
import {
  buildRemoteBackupUrl,
  CLI_ESCALATION_HOST_DEFAULT,
  CLI_ESCALATION_HOST_LEGACY_ALIAS,
  CLI_SESSION_TOOL,
  CODEX_MODEL_ALIASES_ALLOWED,
  defaultIntegrationValidatorBranch,
  defaultIntegrationValidatorWorktreeDir,
  defaultWpValidatorBranch,
  defaultWpValidatorWorktreeDir,
  EXECUTION_OWNER_RANGE_HELP,
  MODEL_FAMILY_POLICY,
  PACKET_FORMAT_VERSION,
  packetUsesSharedRemoteWpBackup,
  packetUsesStructuredValidationReport,
  packetUsesSessionPolicy,
  ROLE_SESSION_FALLBACK_MODEL,
  ROLE_SESSION_PRIMARY_MODEL,
  ROLE_SESSION_REASONING_CONFIG_KEY,
  ROLE_SESSION_REASONING_CONFIG_VALUE,
  ROLE_SESSION_REASONING_REQUIRED,
  ROLE_SESSION_RUNTIME,
  SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS,
  SESSION_PLUGIN_BRIDGE_COMMAND,
  SESSION_PLUGIN_BRIDGE_ID,
  SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION,
  SESSION_START_AUTHORITY,
  SESSION_WAKE_CHANNEL_FALLBACK,
  SESSION_WAKE_CHANNEL_PRIMARY,
  SESSION_WATCH_POLICY,
  SESSION_HOST_FALLBACK,
  SESSION_HOST_PREFERENCE,
  SESSION_LAUNCH_POLICY,
  STUB_FORMAT_VERSION,
  sessionPluginRequestsFileForPacketVersion,
  sessionPluginRequestsFileForStubVersion,
  sessionRegistryFileForPacketVersion,
  sessionRegistryFileForStubVersion,
  stubUsesSessionPolicy,
  SPEC_CLAUSE_MAP_MIN_VERSION,
} from "../scripts/session/session-policy.mjs";

const PACKETS_DIR = path.join(".GOV", "task_packets");
const STUBS_DIR = path.join(".GOV", "task_packets", "stubs");

function fail(message, details = []) {
  console.error(`[SESSION_POLICY_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function isPlaceholder(value) {
  const token = String(value || "").trim();
  if (!token) return true;
  if (/^\{.+\}$/.test(token)) return true;
  if (/^<fill/i.test(token)) return true;
  if (/^<pending>/i.test(token)) return true;
  if (/^<unclaimed>/i.test(token)) return true;
  return false;
}

function checkExpected(errors, rel, text, label, expected) {
  const actual = parseSingleField(text, label);
  if (actual !== expected) {
    errors.push(`${rel}: ${label} must be ${expected} (got: ${actual || "<missing>"})`);
  }
}

function checkAllowed(errors, rel, text, label, allowed) {
  const actual = parseSingleField(text, label);
  if (!allowed.includes(actual)) {
    errors.push(`${rel}: ${label} must be one of ${allowed.join(" | ")} (got: ${actual || "<missing>"})`);
  }
}

function checkExpectedWithLegacyAlias(errors, rel, text, label, expected, legacyAlias) {
  const actual = parseSingleField(text, label);
  if (actual !== expected && actual !== legacyAlias) {
    errors.push(`${rel}: ${label} must be ${expected} or legacy ${legacyAlias} (got: ${actual || "<missing>"})`);
  }
}

function checkMirrorField(errors, rel, text, label, sourceLabel) {
  const expected = parseSingleField(text, sourceLabel);
  if (!expected) {
    errors.push(`${rel}: ${sourceLabel} missing/invalid; cannot validate ${label}`);
    return;
  }

  const actual = parseSingleField(text, label);
  if (actual !== expected) {
    errors.push(`${rel}: ${label} must mirror ${sourceLabel} (${expected}; got: ${actual || "<missing>"})`);
  }
}

function listMarkdownFiles(dirPath) {
  if (!fs.existsSync(dirPath)) return [];
  return fs
    .readdirSync(dirPath)
    .filter((name) => name.endsWith(".md"))
    .map((name) => path.join(dirPath, name));
}

function checkPacket(filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  const rel = filePath.split(path.sep).join("/");
  const version = parseSingleField(text, "PACKET_FORMAT_VERSION");
  if (!packetUsesSessionPolicy(version)) return;

  const wpId = parseSingleField(text, "WP_ID") || path.basename(filePath, ".md");
  const errors = [];
  const allowedPacketVersions = new Set(["2026-03-12", "2026-03-15", "2026-03-16", PACKET_FORMAT_VERSION]);

  if (!allowedPacketVersions.has(version)) {
    errors.push(`${rel}: PACKET_FORMAT_VERSION must be one of ${Array.from(allowedPacketVersions).join(" | ")} (got: ${version || "<missing>"})`);
  }
  checkExpected(errors, rel, text, "SESSION_START_AUTHORITY", SESSION_START_AUTHORITY);
  checkExpected(errors, rel, text, "SESSION_HOST_PREFERENCE", SESSION_HOST_PREFERENCE);
  checkExpected(errors, rel, text, "SESSION_HOST_FALLBACK", SESSION_HOST_FALLBACK);
  checkExpected(errors, rel, text, "SESSION_LAUNCH_POLICY", SESSION_LAUNCH_POLICY);
  checkExpected(errors, rel, text, "ROLE_SESSION_RUNTIME", ROLE_SESSION_RUNTIME);
  checkExpected(errors, rel, text, "CLI_SESSION_TOOL", CLI_SESSION_TOOL);
  checkExpected(errors, rel, text, "SESSION_PLUGIN_BRIDGE_ID", SESSION_PLUGIN_BRIDGE_ID);
  checkExpected(errors, rel, text, "SESSION_PLUGIN_BRIDGE_COMMAND", SESSION_PLUGIN_BRIDGE_COMMAND);
  checkExpected(errors, rel, text, "SESSION_PLUGIN_REQUESTS_FILE", sessionPluginRequestsFileForPacketVersion(version));
  checkExpected(errors, rel, text, "SESSION_REGISTRY_FILE", sessionRegistryFileForPacketVersion(version));
  checkExpected(errors, rel, text, "SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION", String(SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION));
  checkExpected(errors, rel, text, "SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS", String(SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS));
  checkExpected(errors, rel, text, "SESSION_WATCH_POLICY", SESSION_WATCH_POLICY);
  checkExpected(errors, rel, text, "SESSION_WAKE_CHANNEL_PRIMARY", SESSION_WAKE_CHANNEL_PRIMARY);
  checkExpected(errors, rel, text, "SESSION_WAKE_CHANNEL_FALLBACK", SESSION_WAKE_CHANNEL_FALLBACK);
  checkExpectedWithLegacyAlias(errors, rel, text, "CLI_ESCALATION_HOST_DEFAULT", CLI_ESCALATION_HOST_DEFAULT, CLI_ESCALATION_HOST_LEGACY_ALIAS);
  checkExpected(errors, rel, text, "MODEL_FAMILY_POLICY", MODEL_FAMILY_POLICY);
  checkExpected(errors, rel, text, "CODEX_MODEL_ALIASES_ALLOWED", CODEX_MODEL_ALIASES_ALLOWED);
  checkExpected(errors, rel, text, "ROLE_SESSION_PRIMARY_MODEL", ROLE_SESSION_PRIMARY_MODEL);
  checkExpected(errors, rel, text, "ROLE_SESSION_FALLBACK_MODEL", ROLE_SESSION_FALLBACK_MODEL);
  checkExpected(errors, rel, text, "ROLE_SESSION_REASONING_REQUIRED", ROLE_SESSION_REASONING_REQUIRED);
  checkExpected(errors, rel, text, "ROLE_SESSION_REASONING_CONFIG_KEY", ROLE_SESSION_REASONING_CONFIG_KEY);
  checkExpected(errors, rel, text, "ROLE_SESSION_REASONING_CONFIG_VALUE", ROLE_SESSION_REASONING_CONFIG_VALUE);
  checkExpected(errors, rel, text, "CODER_STARTUP_COMMAND", "just coder-startup");
  checkExpected(errors, rel, text, "CODER_RESUME_COMMAND", `just coder-next ${wpId}`);
  checkExpected(errors, rel, text, "WP_VALIDATOR_LOCAL_BRANCH", defaultWpValidatorBranch(wpId));
  checkExpected(errors, rel, text, "WP_VALIDATOR_LOCAL_WORKTREE_DIR", defaultWpValidatorWorktreeDir(wpId));
  checkExpected(errors, rel, text, "WP_VALIDATOR_STARTUP_COMMAND", "just validator-startup");
  checkExpected(errors, rel, text, "WP_VALIDATOR_RESUME_COMMAND", `just validator-next ${wpId}`);
  if (version >= SPEC_CLAUSE_MAP_MIN_VERSION) {
    checkExpected(errors, rel, text, "INTEGRATION_VALIDATOR_LOCAL_BRANCH", defaultIntegrationValidatorBranch(wpId));
    checkExpected(errors, rel, text, "INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR", defaultIntegrationValidatorWorktreeDir(wpId));
  } else {
    // Legacy packets used per-WP integration branches and worktrees; skip enforcement for those.
  }
  checkExpected(errors, rel, text, "INTEGRATION_VALIDATOR_STARTUP_COMMAND", "just validator-startup");
  checkExpected(errors, rel, text, "INTEGRATION_VALIDATOR_RESUME_COMMAND", `just validator-next ${wpId}`);
  if (packetUsesSharedRemoteWpBackup(version)) {
    checkMirrorField(errors, rel, text, "WP_VALIDATOR_REMOTE_BACKUP_BRANCH", "REMOTE_BACKUP_BRANCH");
    if (version >= SPEC_CLAUSE_MAP_MIN_VERSION) {
      checkMirrorField(errors, rel, text, "INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH", "REMOTE_BACKUP_BRANCH");
    }
    checkMirrorField(errors, rel, text, "WP_VALIDATOR_REMOTE_BACKUP_URL", "REMOTE_BACKUP_URL");
    if (version >= SPEC_CLAUSE_MAP_MIN_VERSION) {
      checkMirrorField(errors, rel, text, "INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL", "REMOTE_BACKUP_URL");
    }
  } else {
    const legacyOriginTreeBase = parseSingleField(text, "REMOTE_BACKUP_URL").replace(/\/tree\/.*$/, "");
    checkExpected(errors, rel, text, "WP_VALIDATOR_REMOTE_BACKUP_BRANCH", defaultWpValidatorBranch(wpId));
    // Legacy integration validator branch/url: skip enforcement for pre-2026-03-18 packets.
    checkExpected(errors, rel, text, "WP_VALIDATOR_REMOTE_BACKUP_URL", buildRemoteBackupUrl(legacyOriginTreeBase, defaultWpValidatorBranch(wpId)));
  }
  if (packetUsesStructuredValidationReport(version)) {
    const reportProfile = parseSingleField(text, "GOVERNED_VALIDATOR_REPORT_PROFILE");
    checkAllowed(errors, rel, text, "GOVERNED_VALIDATOR_REPORT_PROFILE", [
      "SPLIT_DIFF_SCOPED_V1",
      "SPLIT_DIFF_SCOPED_RIGOR_V3",
    ]);
    if (/^SPLIT_DIFF_SCOPED_RIGOR_V3$/i.test(reportProfile)) {
      checkExpected(
        errors,
        rel,
        text,
        "GOVERNED_VALIDATOR_SPLIT_FIELDS",
        "VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE"
      );
    } else {
      checkExpected(
        errors,
        rel,
        text,
        "GOVERNED_VALIDATOR_SPLIT_FIELDS",
        "VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE"
      );
    }
  }

  const coderModel = parseSingleField(text, "CODER_MODEL");
  const coderStrength = parseSingleField(text, "CODER_REASONING_STRENGTH");
  if (coderModel && !isPlaceholder(coderModel) && /codex/i.test(coderModel)) {
    errors.push(`${rel}: CODER_MODEL must not use Codex model aliases in new-format packets`);
  }
  if (coderStrength && !isPlaceholder(coderStrength) && !/^(LOW|MEDIUM|HIGH|EXTRA_HIGH)$/i.test(coderStrength)) {
    errors.push(`${rel}: CODER_REASONING_STRENGTH must be LOW|MEDIUM|HIGH|EXTRA_HIGH when claimed`);
  }

  if (errors.length > 0) fail("Packet session policy violations found", errors);
}

function checkStub(filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  const rel = filePath.split(path.sep).join("/");
  const version = parseSingleField(text, "STUB_FORMAT_VERSION");
  if (!stubUsesSessionPolicy(version)) return;

  const errors = [];
  const allowedStubVersions = new Set(["2026-03-12", STUB_FORMAT_VERSION]);
  if (!allowedStubVersions.has(version)) {
    errors.push(`${rel}: STUB_FORMAT_VERSION must be one of ${Array.from(allowedStubVersions).join(" | ")} (got: ${version || "<missing>"})`);
  }
  checkExpected(errors, rel, text, "SESSION_START_AUTHORITY", SESSION_START_AUTHORITY);
  checkExpected(errors, rel, text, "SESSION_HOST_PREFERENCE", SESSION_HOST_PREFERENCE);
  checkExpected(errors, rel, text, "SESSION_HOST_FALLBACK", SESSION_HOST_FALLBACK);
  checkExpected(errors, rel, text, "SESSION_LAUNCH_POLICY", SESSION_LAUNCH_POLICY);
  checkExpected(errors, rel, text, "ROLE_SESSION_RUNTIME", ROLE_SESSION_RUNTIME);
  checkExpected(errors, rel, text, "CLI_SESSION_TOOL", CLI_SESSION_TOOL);
  checkExpected(errors, rel, text, "SESSION_PLUGIN_BRIDGE_ID", SESSION_PLUGIN_BRIDGE_ID);
  checkExpected(errors, rel, text, "SESSION_PLUGIN_BRIDGE_COMMAND", SESSION_PLUGIN_BRIDGE_COMMAND);
  checkExpected(errors, rel, text, "SESSION_PLUGIN_REQUESTS_FILE", sessionPluginRequestsFileForStubVersion(version));
  checkExpected(errors, rel, text, "SESSION_REGISTRY_FILE", sessionRegistryFileForStubVersion(version));
  checkExpected(errors, rel, text, "SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION", String(SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION));
  checkExpected(errors, rel, text, "SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS", String(SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS));
  checkExpected(errors, rel, text, "SESSION_WATCH_POLICY", SESSION_WATCH_POLICY);
  checkExpected(errors, rel, text, "SESSION_WAKE_CHANNEL_PRIMARY", SESSION_WAKE_CHANNEL_PRIMARY);
  checkExpected(errors, rel, text, "SESSION_WAKE_CHANNEL_FALLBACK", SESSION_WAKE_CHANNEL_FALLBACK);
  checkExpectedWithLegacyAlias(errors, rel, text, "CLI_ESCALATION_HOST_DEFAULT", CLI_ESCALATION_HOST_DEFAULT, CLI_ESCALATION_HOST_LEGACY_ALIAS);
  checkExpected(errors, rel, text, "MODEL_FAMILY_POLICY", MODEL_FAMILY_POLICY);
  checkExpected(errors, rel, text, "CODEX_MODEL_ALIASES_ALLOWED", CODEX_MODEL_ALIASES_ALLOWED);
  checkExpected(errors, rel, text, "ROLE_SESSION_PRIMARY_MODEL", ROLE_SESSION_PRIMARY_MODEL);
  checkExpected(errors, rel, text, "ROLE_SESSION_FALLBACK_MODEL", ROLE_SESSION_FALLBACK_MODEL);
  checkExpected(errors, rel, text, "ROLE_SESSION_REASONING_REQUIRED", ROLE_SESSION_REASONING_REQUIRED);
  checkExpected(errors, rel, text, "ROLE_SESSION_REASONING_CONFIG_KEY", ROLE_SESSION_REASONING_CONFIG_KEY);
  checkExpected(errors, rel, text, "ROLE_SESSION_REASONING_CONFIG_VALUE", ROLE_SESSION_REASONING_CONFIG_VALUE);
  checkExpected(errors, rel, text, "PLANNED_EXECUTION_OWNER_RANGE", EXECUTION_OWNER_RANGE_HELP);

  if (errors.length > 0) fail("Stub session policy violations found", errors);
}

for (const filePath of listMarkdownFiles(PACKETS_DIR)) checkPacket(filePath);
for (const filePath of listMarkdownFiles(STUBS_DIR)) checkStub(filePath);

console.log("session-policy-check ok");
