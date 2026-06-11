import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const DOCS = {
  startHere: ".GOV/roles_shared/docs/START_HERE.md",
  roleSession: ".GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md",
  playbook: ".GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md",
  justfile: "justfile",
};

function defaultRepoRoot() {
  const injectedGovRoot = String(process.env.HANDSHAKE_GOV_ROOT || "").trim();
  if (injectedGovRoot) return path.resolve(injectedGovRoot, "..");
  const injected = String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim();
  if (injected) return path.resolve(injected);
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

function parseArgs(argv) {
  const args = { repoRoot: defaultRepoRoot() };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--repo-root") {
      const value = argv[index + 1];
      if (!value) throw new Error("--repo-root requires a path");
      args.repoRoot = path.resolve(value);
      index += 1;
      continue;
    }
    throw new Error(`unknown argument: ${arg}`);
  }
  return args;
}

function readRequired(root, relativePath, failures) {
  const absPath = path.join(root, relativePath);
  try {
    return fs.readFileSync(absPath, "utf8");
  } catch (error) {
    failures.push(`${relativePath} unreadable: ${error.message}`);
    return null;
  }
}

function requireIncludes(failures, relativePath, text, needle) {
  if (text === null) return;
  if (!text.includes(needle)) {
    failures.push(`${relativePath} missing ${needle}`);
  }
}

function requireRecipe(failures, text, recipeName) {
  if (text === null) return;
  const pattern = new RegExp(`^${recipeName}(?:\\s|:)`, "m");
  if (!pattern.test(text)) {
    failures.push(`justfile missing recipe ${recipeName}`);
  }
}

export function validateDiscoveryHbrPointers(repoRoot = defaultRepoRoot()) {
  const failures = [];
  const startHere = readRequired(repoRoot, DOCS.startHere, failures);
  const roleSession = readRequired(repoRoot, DOCS.roleSession, failures);
  const playbook = readRequired(repoRoot, DOCS.playbook, failures);
  const justfile = readRequired(repoRoot, DOCS.justfile, failures);

  for (const needle of [
    "## Build Rules (HBR)",
    ".GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json",
    ".GOV/spec/master-spec-v02.186/spec-modules/05-security-and-observability.md#5.6",
    "CX-131",
    "CX-503B1",
    "packet.acceptance_matrix.hbr",
    "just hbr-matrix-check",
  ]) {
    requireIncludes(failures, DOCS.startHere, startHere, needle);
  }

  for (const needle of [
    "## HBR Handoff Gate",
    "HandoffGate (MT-004) MUST PASS",
    "refinement->coder",
    "coder->WP_VALIDATOR",
    "WP_VALIDATOR->INTEGRATION_VALIDATOR",
    "INTEGRATION_VALIDATOR->ORCHESTRATOR",
    ".GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json",
    "packet.acceptance_matrix.hbr",
    ".GOV/roles/coder/CODER_PROTOCOL.md",
    ".GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md",
    ".GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md",
    ".GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md",
    ".GOV/roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md",
    ".GOV/roles/validator/VALIDATOR_PROTOCOL.md",
    ".GOV/roles/kernel_builder/KERNEL_BUILDER_PROTOCOL.md",
  ]) {
    requireIncludes(failures, DOCS.roleSession, roleSession, needle);
  }

  for (const needle of [
    "just hbr-matrix-check",
    "before any Coder handoff",
    "just hbr-visual-smoke",
    "just hbr-swarm-n8",
    "just hbr-inspector-smoke",
    "before Integration Validator closeout",
  ]) {
    requireIncludes(failures, DOCS.playbook, playbook, needle);
  }

  for (const recipe of [
    "hbr-matrix-check",
    "hbr-visual-smoke",
    "hbr-swarm-n8",
    "hbr-inspector-smoke",
  ]) {
    requireRecipe(failures, justfile, recipe);
  }

  return failures;
}

export function runCli(argv = process.argv.slice(2)) {
  let args;
  try {
    args = parseArgs(argv);
  } catch (error) {
    console.error(JSON.stringify({
      check: "discovery-hbr-pointers",
      verdict: "ERROR",
      failures: [error instanceof Error ? error.message : String(error)],
    }, null, 2));
    return 3;
  }

  const failures = validateDiscoveryHbrPointers(args.repoRoot);
  if (failures.length > 0) {
    console.error(JSON.stringify({
      check: "discovery-hbr-pointers",
      verdict: "FAIL",
      failures,
    }, null, 2));
    return 1;
  }

  console.log("discovery-hbr-pointers ok (START_HERE, ROLE_SESSION_ORCHESTRATION, ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK)");
  return 0;
}

function isInvokedAsMain() {
  if (!process.argv[1]) return false;
  const invoked = fs.realpathSync.native(path.resolve(process.argv[1]));
  const current = fs.realpathSync.native(fileURLToPath(import.meta.url));
  return invoked === current;
}

if (isInvokedAsMain()) {
  process.exitCode = runCli();
}
