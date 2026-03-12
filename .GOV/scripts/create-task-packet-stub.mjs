#!/usr/bin/env node
/**
 * Task packet stub generator
 *
 * Creates a backlog-only stub under `.GOV/task_packets/stubs/` from the canonical template:
 * - `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`
 *
 * Usage:
 *   node .GOV/scripts/create-task-packet-stub.mjs WP-1-My-Stub "§7.6.3 ..." "123,124-126"
 */

import fs from "node:fs";
import path from "node:path";
import {
  CLI_SESSION_TOOL,
  CODEX_MODEL_ALIASES_ALLOWED,
  EXECUTION_OWNER_RANGE_HELP,
  MODEL_FAMILY_POLICY,
  ROLE_SESSION_FALLBACK_MODEL,
  ROLE_SESSION_PRIMARY_MODEL,
  ROLE_SESSION_REASONING_CONFIG_KEY,
  ROLE_SESSION_REASONING_CONFIG_VALUE,
  ROLE_SESSION_REASONING_REQUIRED,
  ROLE_SESSION_RUNTIME,
  SESSION_HOST_FALLBACK,
  SESSION_HOST_PREFERENCE,
  SESSION_LAUNCH_POLICY,
  STUB_FORMAT_VERSION,
} from "./session-policy.mjs";

const WP_ID = process.argv[2];
const ROADMAP_POINTER = process.argv[3] || "<fill>";
const LINE_NUMBERS = process.argv[4] || "<fill>";

if (!WP_ID || !WP_ID.startsWith("WP-")) {
  console.error("Usage: node .GOV/scripts/create-task-packet-stub.mjs WP-{phase}-{name} [roadmap_pointer] [line_numbers]");
  console.error("Example: node .GOV/scripts/create-task-packet-stub.mjs WP-1-Session-Spawn-Contract \"§7.6.3\" \"123,124-126\"");
  process.exit(1);
}

const stubsDir = path.join(".GOV", "task_packets", "stubs");
if (!fs.existsSync(stubsDir)) {
  fs.mkdirSync(stubsDir, { recursive: true });
}

const filePath = path.join(stubsDir, `${WP_ID}.md`);
if (fs.existsSync(filePath)) {
  console.error(`Task packet stub already exists: ${filePath.replace(/\\/g, "/")}`);
  process.exit(1);
}

const templatePath = path.join(".GOV", "templates", "TASK_PACKET_STUB_TEMPLATE.md");
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
content = fill(content, "{{SESSION_HOST_PREFERENCE}}", SESSION_HOST_PREFERENCE);
content = fill(content, "{{SESSION_HOST_FALLBACK}}", SESSION_HOST_FALLBACK);
content = fill(content, "{{SESSION_LAUNCH_POLICY}}", SESSION_LAUNCH_POLICY);
content = fill(content, "{{ROLE_SESSION_RUNTIME}}", ROLE_SESSION_RUNTIME);
content = fill(content, "{{CLI_SESSION_TOOL}}", CLI_SESSION_TOOL);
content = fill(content, "{{MODEL_FAMILY_POLICY}}", MODEL_FAMILY_POLICY);
content = fill(content, "{{CODEX_MODEL_ALIASES_ALLOWED}}", CODEX_MODEL_ALIASES_ALLOWED);
content = fill(content, "{{ROLE_SESSION_PRIMARY_MODEL}}", ROLE_SESSION_PRIMARY_MODEL);
content = fill(content, "{{ROLE_SESSION_FALLBACK_MODEL}}", ROLE_SESSION_FALLBACK_MODEL);
content = fill(content, "{{ROLE_SESSION_REASONING_REQUIRED}}", ROLE_SESSION_REASONING_REQUIRED);
content = fill(content, "{{ROLE_SESSION_REASONING_CONFIG_KEY}}", ROLE_SESSION_REASONING_CONFIG_KEY);
content = fill(content, "{{ROLE_SESSION_REASONING_CONFIG_VALUE}}", ROLE_SESSION_REASONING_CONFIG_VALUE);
content = fill(content, "{{EXECUTION_OWNER_RANGE_HELP}}", EXECUTION_OWNER_RANGE_HELP);

fs.writeFileSync(filePath, content, "utf8");

console.log(`task-packet-stub created: ${filePath.replace(/\\/g, "/")}`);
console.log("");
console.log("Next steps:");
console.log(`1) Fill BUILD_ORDER_* + SPEC_ANCHOR_CANDIDATES + DEPENDENCIES/BLOCKERS in ${filePath.replace(/\\/g, "/")}`);
console.log("2) Fill UI_UX_SKETCH + PRIMITIVES_AND_MATRIX_NOTES (draft; prefer too many UI controls early)");
console.log("3) Ensure TASK_BOARD lists the stub under 'Stub Backlog (Not Activated)'");
console.log("4) Run: just build-order-sync");
