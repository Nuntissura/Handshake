import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

export const GOV_ROOT_ENV_VAR = "HANDSHAKE_GOV_ROOT";
export const GOVERNANCE_RUNTIME_ROOT_ENV_VAR = "HANDSHAKE_GOV_RUNTIME_ROOT";
export const PRODUCT_RUNTIME_ROOT_ENV_VAR = "HANDSHAKE_RUNTIME_ROOT";
export const LEGACY_SHARED_GOV_RUNTIME_ROOT = ".GOV/roles_shared/runtime";
export const LEGACY_SHARED_GOV_SESSION_LAUNCH_REQUESTS_FILE = `${LEGACY_SHARED_GOV_RUNTIME_ROOT}/SESSION_LAUNCH_REQUESTS.jsonl`;
export const LEGACY_SHARED_GOV_SESSION_REGISTRY_FILE = `${LEGACY_SHARED_GOV_RUNTIME_ROOT}/ROLE_SESSION_REGISTRY.json`;
export const LEGACY_SHARED_GOV_SESSION_CONTROL_REQUESTS_FILE = `${LEGACY_SHARED_GOV_RUNTIME_ROOT}/SESSION_CONTROL_REQUESTS.jsonl`;
export const LEGACY_SHARED_GOV_SESSION_CONTROL_RESULTS_FILE = `${LEGACY_SHARED_GOV_RUNTIME_ROOT}/SESSION_CONTROL_RESULTS.jsonl`;
export const LEGACY_SHARED_GOV_SESSION_CONTROL_OUTPUT_DIR = `${LEGACY_SHARED_GOV_RUNTIME_ROOT}/SESSION_CONTROL_OUTPUTS`;
export const LEGACY_SHARED_GOV_SESSION_CONTROL_BROKER_STATE_FILE = `${LEGACY_SHARED_GOV_RUNTIME_ROOT}/SESSION_CONTROL_BROKER_STATE.json`;
export const LEGACY_SHARED_GOV_WP_COMMUNICATIONS_ROOT = `${LEGACY_SHARED_GOV_RUNTIME_ROOT}/WP_COMMUNICATIONS`;
export const LEGACY_SHARED_GOV_VALIDATOR_GATES_ROOT = `${LEGACY_SHARED_GOV_RUNTIME_ROOT}/validator_gates`;
export const LEGACY_SHARED_GOV_GIT_TOPOLOGY_FILE = `${LEGACY_SHARED_GOV_RUNTIME_ROOT}/GIT_TOPOLOGY_REGISTRY.json`;

function resolveRepoRoot() {
  try {
    const out = execFileSync("git", ["rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return out;
  } catch {
    // Fall back to file-relative resolution below.
  }

  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

function readPersistedUserEnv(name) {
  if (process.platform !== "win32") return "";
  try {
    return execFileSync(
      "powershell.exe",
      ["-NoLogo", "-NonInteractive", "-Command", `[Environment]::GetEnvironmentVariable('${name}','User')`],
      { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
    ).trim();
  } catch {
    return "";
  }
}

export function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

export const REPO_ROOT = path.resolve(resolveRepoRoot());
export const WORKSPACE_ROOT = path.resolve(REPO_ROOT, "..");

// --- Governance root (kernel worktree) resolution ---

function resolveGovRoot() {
  const directValue = String(
    process.env[GOV_ROOT_ENV_VAR]
      || readPersistedUserEnv(GOV_ROOT_ENV_VAR)
      || "",
  ).trim();
  if (directValue) return path.resolve(directValue);
  return path.resolve(REPO_ROOT, ".GOV");
}

export const GOV_ROOT_ABS = resolveGovRoot();
export const GOV_ROOT_REPO_REL = normalizePath(path.relative(REPO_ROOT, GOV_ROOT_ABS)) || ".GOV";

export function govRootAbsPath(...segments) {
  return path.resolve(GOV_ROOT_ABS, ...segments);
}

export function govRootRelPath(...segments) {
  return normalizePath(path.join(GOV_ROOT_REPO_REL, ...segments));
}

export function resolveGovernanceRuntimeRoot(overrideValue = "") {
  const directValue = String(
    overrideValue
      || process.env[GOVERNANCE_RUNTIME_ROOT_ENV_VAR]
      || readPersistedUserEnv(GOVERNANCE_RUNTIME_ROOT_ENV_VAR)
      || "",
  ).trim();
  if (directValue) return path.resolve(directValue);

  const productRuntimeRoot = String(
    process.env[PRODUCT_RUNTIME_ROOT_ENV_VAR]
      || readPersistedUserEnv(PRODUCT_RUNTIME_ROOT_ENV_VAR)
      || "",
  ).trim();
  if (productRuntimeRoot) {
    return path.resolve(productRuntimeRoot, "repo-governance");
  }

  return path.resolve(WORKSPACE_ROOT, "gov_runtime");
}

export const GOVERNANCE_RUNTIME_ROOT_ABS = resolveGovernanceRuntimeRoot();
export const GOVERNANCE_RUNTIME_ROOT_REPO_REL = normalizePath(path.relative(REPO_ROOT, GOVERNANCE_RUNTIME_ROOT_ABS)) || ".";

function relWithinGovernanceRuntime(...segments) {
  return normalizePath(path.join(GOVERNANCE_RUNTIME_ROOT_REPO_REL, ...segments));
}

export function repoRelativeGovernanceRuntimePath(...segments) {
  return relWithinGovernanceRuntime(...segments);
}

export function governanceRuntimeAbsPath(...segments) {
  return path.resolve(GOVERNANCE_RUNTIME_ROOT_ABS, ...segments);
}

export const SHARED_GOV_RUNTIME_ROOT = relWithinGovernanceRuntime("roles_shared");
export const SHARED_GOV_SESSION_LAUNCH_REQUESTS_FILE = relWithinGovernanceRuntime("roles_shared", "SESSION_LAUNCH_REQUESTS.jsonl");
export const SHARED_GOV_SESSION_REGISTRY_FILE = relWithinGovernanceRuntime("roles_shared", "ROLE_SESSION_REGISTRY.json");
export const SHARED_GOV_SESSION_CONTROL_REQUESTS_FILE = relWithinGovernanceRuntime("roles_shared", "SESSION_CONTROL_REQUESTS.jsonl");
export const SHARED_GOV_SESSION_CONTROL_RESULTS_FILE = relWithinGovernanceRuntime("roles_shared", "SESSION_CONTROL_RESULTS.jsonl");
export const SHARED_GOV_SESSION_CONTROL_OUTPUT_DIR = relWithinGovernanceRuntime("roles_shared", "SESSION_CONTROL_OUTPUTS");
export const SHARED_GOV_SESSION_CONTROL_BROKER_STATE_FILE = relWithinGovernanceRuntime("roles_shared", "SESSION_CONTROL_BROKER_STATE.json");
export const SHARED_GOV_WP_COMMUNICATIONS_ROOT = relWithinGovernanceRuntime("roles_shared", "WP_COMMUNICATIONS");
export const SHARED_GOV_VALIDATOR_GATES_ROOT = relWithinGovernanceRuntime("roles_shared", "validator_gates");
export const SHARED_GOV_GIT_TOPOLOGY_FILE = relWithinGovernanceRuntime("roles_shared", "GIT_TOPOLOGY_REGISTRY.json");

export function ensureGovernanceRuntimeDir(...segments) {
  const targetDir = governanceRuntimeAbsPath(...segments);
  fs.mkdirSync(targetDir, { recursive: true });
  return targetDir;
}
