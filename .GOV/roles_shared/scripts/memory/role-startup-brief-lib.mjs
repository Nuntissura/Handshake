import fs from "node:fs";
import path from "node:path";
import { GOV_ROOT_ABS } from "../lib/runtime-paths.mjs";

export const ROLE_STARTUP_BRIEF_PATHS = Object.freeze({
  ORCHESTRATOR: ".GOV/roles/orchestrator/docs/ORCHESTRATOR_STARTUP_BRIEF.md",
  CLASSIC_ORCHESTRATOR: ".GOV/roles/classic_orchestrator/docs/CLASSIC_ORCHESTRATOR_STARTUP_BRIEF.md",
  ACTIVATION_MANAGER: ".GOV/roles/activation_manager/docs/ACTIVATION_MANAGER_STARTUP_BRIEF.md",
  CODER: ".GOV/roles/coder/docs/CODER_STARTUP_BRIEF.md",
  WP_VALIDATOR: ".GOV/roles/wp_validator/docs/WP_VALIDATOR_STARTUP_BRIEF.md",
  INTEGRATION_VALIDATOR: ".GOV/roles/integration_validator/docs/INTEGRATION_VALIDATOR_STARTUP_BRIEF.md",
  VALIDATOR: ".GOV/roles/validator/docs/VALIDATOR_STARTUP_BRIEF.md",
  MEMORY_MANAGER: ".GOV/roles/memory_manager/docs/MEMORY_MANAGER_STARTUP_BRIEF.md",
});

export const SHARED_STARTUP_BRIEF_PATH = ".GOV/roles_shared/docs/SHARED_STARTUP_BRIEF.md";
export const STARTUP_BRIEF_SCHEMA_PATH = ".GOV/roles_shared/docs/STARTUP_BRIEF_SCHEMA.md";

export function normalizeStartupBriefRole(role = "") {
  return String(role || "").trim().toUpperCase();
}

export function startupBriefPathForRole(role = "") {
  const normalizedRole = normalizeStartupBriefRole(role);
  return ROLE_STARTUP_BRIEF_PATHS[normalizedRole] || "";
}

export function repoRelativeToAbs(repoRelativePath = "") {
  const normalized = String(repoRelativePath || "").replace(/\\/g, "/");
  const withoutGov = normalized.replace(/^\.GOV\//, "");
  return path.join(GOV_ROOT_ABS, withoutGov);
}

export function loadStartupBriefFile(repoRelativePath = "") {
  const absPath = repoRelativeToAbs(repoRelativePath);
  if (!fs.existsSync(absPath)) {
    throw new Error(`Missing startup brief file: ${repoRelativePath}`);
  }
  return fs.readFileSync(absPath, "utf8");
}

export function validateStartupBriefShape({ role = "", content = "", shared = false } = {}) {
  const errors = [];
  const normalizedRole = normalizeStartupBriefRole(role);
  const expectedTitle = shared ? "# Shared Startup Brief" : "Startup Brief";

  if (!String(content || "").includes(expectedTitle)) {
    errors.push(`missing title ${expectedTitle}`);
  }
  if (!shared && !new RegExp(`^- ROLE:\\s*${normalizedRole}$`, "m").test(content)) {
    errors.push(`missing role marker ${normalizedRole}`);
  }
  for (const required of ["## Status", "## Use", "## Action Cards"]) {
    if (!String(content || "").includes(required)) errors.push(`missing section ${required}`);
  }
  if (!/SCHEMA_VERSION:\s*`hsk\.startup_brief@1`/.test(content)) {
    errors.push("missing startup brief schema version");
  }
  if (!/### RAM-[A-Z_]+-[A-Z0-9_]+-\d{3}/.test(content)) {
    errors.push("missing RAM action card");
  }
  for (const field of ["ACTION", "TRIGGER", "FAILURE_PATTERN", "DO", "DO_NOT", "VERIFY", "SOURCE"]) {
    if (!new RegExp(`^- ${field}:`, "m").test(content)) errors.push(`missing action-card field ${field}`);
  }
  return errors;
}

export function loadStartupBriefsForRole(role = "") {
  const normalizedRole = normalizeStartupBriefRole(role);
  const rolePath = startupBriefPathForRole(normalizedRole);
  if (!rolePath) {
    throw new Error(`Unknown startup brief role: ${role}`);
  }
  const sharedContent = loadStartupBriefFile(SHARED_STARTUP_BRIEF_PATH);
  const roleContent = loadStartupBriefFile(rolePath);
  const errors = [
    ...validateStartupBriefShape({ role: "SHARED", content: sharedContent, shared: true }).map((error) => `${SHARED_STARTUP_BRIEF_PATH}: ${error}`),
    ...validateStartupBriefShape({ role: normalizedRole, content: roleContent }).map((error) => `${rolePath}: ${error}`),
  ];
  return {
    role: normalizedRole,
    sharedPath: SHARED_STARTUP_BRIEF_PATH,
    rolePath,
    sharedContent,
    roleContent,
    errors,
  };
}

export function formatStartupBriefForRole(role = "") {
  const brief = loadStartupBriefsForRole(role);
  if (brief.errors.length > 0) {
    throw new Error(`Startup brief validation failed:\n${brief.errors.join("\n")}`);
  }
  return [
    `STARTUP_BRIEF_BEGIN role=${brief.role}`,
    `SOURCE: ${STARTUP_BRIEF_SCHEMA_PATH}`,
    `SOURCE: ${brief.sharedPath}`,
    `SOURCE: ${brief.rolePath}`,
    "",
    brief.sharedContent.trimEnd(),
    "",
    brief.roleContent.trimEnd(),
    "",
    `STARTUP_BRIEF_END role=${brief.role}`,
  ].join("\n");
}
