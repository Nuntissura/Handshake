#!/usr/bin/env node
/**
 * Work packet stub generator
 *
 * Creates a backlog-only stub under the resolved Work Packet stub root
 * (current physical storage: `.GOV/task_packets/stubs/`) from the canonical template:
 * - `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`
 *
 * Usage:
 *   node .GOV/roles/orchestrator/scripts/create-task-packet-stub.mjs WP-1-My-Stub "§7.6.3 ..." "123,124-126"
 */

import fs from "node:fs";
import path from "node:path";
import {
  ensureWorkPacketLifecycleLayout,
  CLI_ESCALATION_HOST_DEFAULT,
  CLI_SESSION_TOOL,
  CODEX_MODEL_ALIASES_ALLOWED,
  DEFAULT_ROLE_MODEL_PROFILE_IDS,
  EXECUTION_OWNER_RANGE_HELP,
  modelFamilyPolicyForStubVersion,
  ROLE_MODEL_PROFILE_POLICY,
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
  SESSION_PLUGIN_REQUESTS_FILE,
  SESSION_REGISTRY_FILE,
  SESSION_START_AUTHORITY,
  SESSION_WAKE_CHANNEL_FALLBACK,
  SESSION_WAKE_CHANNEL_PRIMARY,
  SESSION_WATCH_POLICY,
  SESSION_HOST_FALLBACK,
  SESSION_HOST_PREFERENCE,
  SESSION_LAUNCH_POLICY,
  STUB_FORMAT_VERSION,
} from "../../../roles_shared/scripts/session/session-policy.mjs";
import { GOV_ROOT_REPO_REL, WORK_PACKET_STUB_STORAGE_ROOT_REPO_REL } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";

const WP_ID = process.argv[2];
const ROADMAP_POINTER = process.argv[3] || "<fill>";
const LINE_NUMBERS = process.argv[4] || "<fill>";

if (!WP_ID || !WP_ID.startsWith("WP-")) {
  console.error(`Usage: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/create-task-packet-stub.mjs WP-{phase}-{name} [roadmap_pointer] [line_numbers]`);
  console.error(`Example: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/create-task-packet-stub.mjs WP-1-Session-Spawn-Contract "§7.6.3" "123,124-126"`);
  process.exit(1);
}

ensureWorkPacketLifecycleLayout();

const stubsDir = WORK_PACKET_STUB_STORAGE_ROOT_REPO_REL;
if (!fs.existsSync(stubsDir)) {
  fs.mkdirSync(stubsDir, { recursive: true });
}

const filePath = path.join(stubsDir, `${WP_ID}.md`);
if (fs.existsSync(filePath)) {
  console.error(`Work packet stub already exists: ${filePath.replace(/\\/g, "/")}`);
  process.exit(1);
}

const templatePath = path.join(GOV_ROOT_REPO_REL, "templates", "TASK_PACKET_STUB_TEMPLATE.md");
if (!fs.existsSync(templatePath)) {
  console.error(`Missing stub template: ${templatePath.replace(/\\/g, "/")}`);
  process.exit(1);
}

const timestamp = new Date().toISOString();
const raw = fs.readFileSync(templatePath, "utf8");
const fill = (text, token, value) => text.split(token).join(value);

let content = raw;
content = fill(content, "{{WP_ID}}", WP_ID);
content = fill(content, "{{DATE_ISO}}", timestamp);
content = fill(content, "{{STUB_FORMAT_VERSION}}", STUB_FORMAT_VERSION);
content = fill(content, "{{ROADMAP_POINTER}}", ROADMAP_POINTER);
content = fill(content, "{{LINE_NUMBERS_COMMA_SEPARATED}}", LINE_NUMBERS);
content = fill(content, "{{SPEC_ANCHOR_1}}", "<fill>");
content = fill(content, "{{SPEC_ANCHOR_2}}", "<fill>");
content = fill(content, "{{SESSION_START_AUTHORITY}}", SESSION_START_AUTHORITY);
content = fill(content, "{{SESSION_HOST_PREFERENCE}}", SESSION_HOST_PREFERENCE);
content = fill(content, "{{SESSION_HOST_FALLBACK}}", SESSION_HOST_FALLBACK);
content = fill(content, "{{SESSION_LAUNCH_POLICY}}", SESSION_LAUNCH_POLICY);
content = fill(content, "{{ROLE_SESSION_RUNTIME}}", ROLE_SESSION_RUNTIME);
content = fill(content, "{{CLI_SESSION_TOOL}}", CLI_SESSION_TOOL);
content = fill(content, "{{SESSION_PLUGIN_BRIDGE_ID}}", SESSION_PLUGIN_BRIDGE_ID);
content = fill(content, "{{SESSION_PLUGIN_BRIDGE_COMMAND}}", SESSION_PLUGIN_BRIDGE_COMMAND);
content = fill(content, "{{SESSION_PLUGIN_REQUESTS_FILE}}", SESSION_PLUGIN_REQUESTS_FILE);
content = fill(content, "{{SESSION_REGISTRY_FILE}}", SESSION_REGISTRY_FILE);
content = fill(content, "{{SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION}}", String(SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION));
content = fill(content, "{{SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS}}", String(SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS));
content = fill(content, "{{SESSION_WATCH_POLICY}}", SESSION_WATCH_POLICY);
content = fill(content, "{{SESSION_WAKE_CHANNEL_PRIMARY}}", SESSION_WAKE_CHANNEL_PRIMARY);
content = fill(content, "{{SESSION_WAKE_CHANNEL_FALLBACK}}", SESSION_WAKE_CHANNEL_FALLBACK);
content = fill(content, "{{CLI_ESCALATION_HOST_DEFAULT}}", CLI_ESCALATION_HOST_DEFAULT);
content = fill(content, "{{MODEL_FAMILY_POLICY}}", modelFamilyPolicyForStubVersion(STUB_FORMAT_VERSION));
content = fill(content, "{{ROLE_MODEL_PROFILE_POLICY}}", ROLE_MODEL_PROFILE_POLICY);
content = fill(content, "{{CODEX_MODEL_ALIASES_ALLOWED}}", CODEX_MODEL_ALIASES_ALLOWED);
content = fill(content, "{{ROLE_SESSION_PRIMARY_MODEL}}", ROLE_SESSION_PRIMARY_MODEL);
content = fill(content, "{{ROLE_SESSION_FALLBACK_MODEL}}", ROLE_SESSION_FALLBACK_MODEL);
content = fill(content, "{{ROLE_SESSION_REASONING_REQUIRED}}", ROLE_SESSION_REASONING_REQUIRED);
content = fill(content, "{{ROLE_SESSION_REASONING_CONFIG_KEY}}", ROLE_SESSION_REASONING_CONFIG_KEY);
content = fill(content, "{{ROLE_SESSION_REASONING_CONFIG_VALUE}}", ROLE_SESSION_REASONING_CONFIG_VALUE);
content = fill(content, "{{ORCHESTRATOR_MODEL_PROFILE}}", DEFAULT_ROLE_MODEL_PROFILE_IDS.ORCHESTRATOR);
content = fill(content, "{{CODER_MODEL_PROFILE}}", DEFAULT_ROLE_MODEL_PROFILE_IDS.CODER);
content = fill(content, "{{WP_VALIDATOR_MODEL_PROFILE}}", DEFAULT_ROLE_MODEL_PROFILE_IDS.WP_VALIDATOR);
content = fill(content, "{{INTEGRATION_VALIDATOR_MODEL_PROFILE}}", DEFAULT_ROLE_MODEL_PROFILE_IDS.INTEGRATION_VALIDATOR);
content = fill(content, "{{EXECUTION_OWNER_RANGE_HELP}}", EXECUTION_OWNER_RANGE_HELP);

fs.writeFileSync(filePath, content, "utf8");

console.log(`task-packet-stub created: ${filePath.replace(/\\/g, "/")}`);
console.log("");
console.log("Next steps:");
console.log(`1) Fill BUILD_ORDER_* + SPEC_ANCHOR_CANDIDATES + DEPENDENCIES/BLOCKERS in ${filePath.replace(/\\/g, "/")}`);
console.log("2) Fill UI_UX_SKETCH + PRIMITIVES_AND_MATRIX_NOTES (draft; prefer too many UI controls early)");
console.log("3) Ensure TASK_BOARD lists the stub under 'Stub Backlog (Not Activated)'");
console.log("4) Run: just build-order-sync");
