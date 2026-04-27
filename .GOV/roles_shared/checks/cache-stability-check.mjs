import fs from "node:fs";
import { GOV_ROOT_REPO_REL, repoPathAbs } from "../scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import {
  appendCheckDetails,
  createCheckResult,
  formatCheckResultSummary,
} from "../scripts/lib/check-result-lib.mjs";

registerFailCaptureHook("cache-stability-check.mjs", { role: "SHARED" });
const verboseMode = process.argv.includes("--verbose");

const EPHEMERAL_HELPER_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/scripts/session/ephemeral-injection-lib.mjs`;

const ACTIVE_SESSION_MUTATION_SURFACES = [
  `${GOV_ROOT_REPO_REL}/roles_shared/scripts/wp/wp-receipt-append.mjs`,
  `${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/task-board-set.mjs`,
  `${GOV_ROOT_REPO_REL}/roles_shared/scripts/audit/workflow-dossier.mjs`,
  `${GOV_ROOT_REPO_REL}/roles_shared/scripts/audit/workflow-dossier-lib.mjs`,
  `${GOV_ROOT_REPO_REL}/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`,
  `${GOV_ROOT_REPO_REL}/roles_shared/scripts/lib/wp-review-projection-lib.mjs`,
  `${GOV_ROOT_REPO_REL}/roles_shared/scripts/session/send-mt-prompt.mjs`,
  `${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/orchestrator-steer-next.mjs`,
  `${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/lib/manual-relay-envelope-lib.mjs`,
];

const REQUIRED_EPHEMERAL_PROMPT_BUILDERS = [
  `${GOV_ROOT_REPO_REL}/roles_shared/scripts/session/send-mt-prompt.mjs`,
  `${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/orchestrator-steer-next.mjs`,
  `${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/lib/manual-relay-envelope-lib.mjs`,
];

const FORBIDDEN_ACTIVE_SESSION_SYSTEM_MUTATION_PATTERNS = [
  {
    label: "systemPrompt assignment",
    regex: /\bsystemPrompt\s*[=:]/,
  },
  {
    label: "system_prompt assignment",
    regex: /\bsystem_prompt\s*[=:]/i,
  },
  {
    label: "SYSTEM_PROMPT assignment",
    regex: /\bSYSTEM_PROMPT\s*[=:]/,
  },
  {
    label: "system role message construction",
    regex: /\brole\s*:\s*["']system["']/i,
  },
  {
    label: "startup prompt rebuild from active mutation surface",
    regex: /\bbuildStartupPrompt\s*\(/,
  },
  {
    label: "active mutation surface dispatching session/new",
    regex: /["']session\/new["']/,
  },
];

function fail(message, details = []) {
  const result = createCheckResult({
    verdict: "FAIL",
    summary: "cache-stability violations found",
    details: { errors: details },
  });
  appendCheckDetails({ check: "cache-stability-check", result });
  failWithMemory("cache-stability-check.mjs", message, { role: "SHARED", details });
}

function readRequired(filePath) {
  const absPath = repoPathAbs(filePath);
  if (!fs.existsSync(absPath)) {
    fail("Missing cache-stability surface", [filePath]);
  }
  return fs.readFileSync(absPath, "utf8");
}

function lineNumberFor(content, index) {
  return content.slice(0, index).split(/\r?\n/).length;
}

const errors = [];
const helper = readRequired(EPHEMERAL_HELPER_PATH);
if (!helper.includes("export function buildEphemeralContextBlock")) {
  errors.push(`${EPHEMERAL_HELPER_PATH}: missing buildEphemeralContextBlock export`);
}
if (!helper.includes("<governance-context")) {
  errors.push(`${EPHEMERAL_HELPER_PATH}: helper must emit governance-context fence`);
}
if (!helper.includes("not user input")) {
  errors.push(`${EPHEMERAL_HELPER_PATH}: helper must mark injected context as not user input`);
}

for (const filePath of ACTIVE_SESSION_MUTATION_SURFACES) {
  const content = readRequired(filePath);
  for (const pattern of FORBIDDEN_ACTIVE_SESSION_SYSTEM_MUTATION_PATTERNS) {
    const match = pattern.regex.exec(content);
    if (match) {
      errors.push(`${filePath}:${lineNumberFor(content, match.index)}: forbidden ${pattern.label}; active-session governance updates must use durable storage plus fenced user-message context`);
    }
  }
}

for (const filePath of REQUIRED_EPHEMERAL_PROMPT_BUILDERS) {
  const content = readRequired(filePath);
  if (!content.includes("buildEphemeralContextBlock")) {
    errors.push(`${filePath}: SEND_PROMPT builder must wrap injected governance context with buildEphemeralContextBlock`);
  }
}

if (errors.length > 0) {
  fail("Cache-stability violations found", errors);
}

const result = createCheckResult({
  verdict: "OK",
  summary: "cache-stability-check ok",
  details: {
    helper: EPHEMERAL_HELPER_PATH,
    scanned_surfaces: ACTIVE_SESSION_MUTATION_SURFACES,
    required_ephemeral_prompt_builders: REQUIRED_EPHEMERAL_PROMPT_BUILDERS,
  },
});
const writeResult = appendCheckDetails({ check: "cache-stability-check", result });

if (verboseMode) {
  console.log(JSON.stringify(writeResult.entry, null, 2));
} else {
  console.log(formatCheckResultSummary(result));
}
