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
  const fileRelativeRepoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../..");
  try {
    const out = execFileSync("git", ["-C", fileRelativeRepoRoot, "rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return out;
  } catch {
    // Fall back to file-relative resolution below.
  }

  return fileRelativeRepoRoot;
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

export function repoPathAbs(value) {
  const raw = String(value || "").trim();
  if (!raw) return "";
  return path.isAbsolute(raw) ? path.resolve(raw) : path.resolve(REPO_ROOT, raw);
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
  if (directValue) {
    const candidate = path.resolve(directValue);
    if (fs.existsSync(candidate)) return candidate;
  }
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

export function listWorkPacketEntriesAt(taskPacketsRootAbs, taskPacketsRootRel, options = {}) {
  const rootAbs = path.resolve(String(taskPacketsRootAbs || ""));
  const rootRel = normalizePath(taskPacketsRootRel);
  const skipDirNames = new Set(options.skipDirNames || []);
  if (!rootAbs || !fs.existsSync(rootAbs)) return [];

  const entries = [];
  for (const dirent of fs.readdirSync(rootAbs, { withFileTypes: true })) {
    if (dirent.isDirectory()) {
      if (skipDirNames.has(dirent.name)) continue;
      const packetPathAbs = path.join(rootAbs, dirent.name, "packet.md");
      if (!fs.existsSync(packetPathAbs)) continue;
      entries.push({
        wpId: dirent.name,
        packetPath: normalizePath(path.join(rootRel, dirent.name, "packet.md")),
        packetDir: normalizePath(path.join(rootRel, dirent.name)),
        isFolder: true,
      });
      continue;
    }

    if (!dirent.isFile()) continue;
    if (!dirent.name.endsWith(".md") || dirent.name === "README.md") continue;
    const wpId = dirent.name.replace(/\.md$/i, "");
    if (!/^WP-/.test(wpId)) continue;
    entries.push({
      wpId,
      packetPath: normalizePath(path.join(rootRel, dirent.name)),
      packetDir: rootRel,
      isFolder: false,
    });
  }

  return entries.sort((left, right) =>
    left.wpId.localeCompare(right.wpId)
    || left.packetPath.localeCompare(right.packetPath)
  );
}

export function listOfficialWorkPacketEntries() {
  return listWorkPacketEntriesAt(
    govRootAbsPath("task_packets"),
    govRootRelPath("task_packets"),
    { skipDirNames: ["stubs"] },
  );
}

export function listOfficialWorkPacketPaths() {
  return listOfficialWorkPacketEntries().map((entry) => entry.packetPath);
}

export function listStubWorkPacketEntries() {
  return listWorkPacketEntriesAt(
    govRootAbsPath("task_packets", "stubs"),
    govRootRelPath("task_packets", "stubs"),
  );
}

export function listStubWorkPacketPaths() {
  return listStubWorkPacketEntries().map((entry) => entry.packetPath);
}

/**
 * Resolve work packet path — supports both folder structure and flat file.
 * Folder: .GOV/task_packets/WP-{ID}/packet.md (new)
 * Flat:   .GOV/task_packets/WP-{ID}.md (legacy)
 * Returns { packetPath, packetDir, isFolder } or null if not found.
 */
export function resolveWorkPacketPath(wpId) {
  const folderPath = govRootRelPath("task_packets", wpId, "packet.md");
  const folderAbsPath = govRootAbsPath("task_packets", wpId, "packet.md");
  const flatPath = govRootRelPath("task_packets", `${wpId}.md`);
  const flatAbsPath = govRootAbsPath("task_packets", `${wpId}.md`);
  if (fs.existsSync(folderAbsPath)) {
    return {
      packetPath: folderPath,
      packetAbsPath: folderAbsPath,
      packetDir: govRootRelPath("task_packets", wpId),
      packetDirAbs: govRootAbsPath("task_packets", wpId),
      isFolder: true,
    };
  }
  if (fs.existsSync(flatAbsPath)) {
    return {
      packetPath: flatPath,
      packetAbsPath: flatAbsPath,
      packetDir: govRootRelPath("task_packets"),
      packetDirAbs: govRootAbsPath("task_packets"),
      isFolder: false,
    };
  }
  return null;
}

export function workPacketPath(wpId) {
  return resolveWorkPacketPath(wpId)?.packetPath || govRootRelPath("task_packets", `${wpId}.md`);
}

export function workPacketAbsPath(wpId) {
  return resolveWorkPacketPath(wpId)?.packetAbsPath || govRootAbsPath("task_packets", `${wpId}.md`);
}

export function inferWpIdFromPacketPath(packetPath) {
  const normalized = normalizePath(packetPath);
  if (!normalized) return "";
  const baseName = path.posix.basename(normalized);
  if (/^packet\.md$/i.test(baseName)) {
    const parentName = path.posix.basename(path.posix.dirname(normalized));
    return /^WP-/.test(parentName) ? parentName : "";
  }
  const wpId = baseName.replace(/\.md$/i, "");
  return /^WP-/.test(wpId) ? wpId : "";
}

/**
 * Resolve refinement path — supports both folder structure and flat file.
 * Folder: .GOV/task_packets/WP-{ID}/refinement.md (new, co-located)
 * Flat:   .GOV/refinements/WP-{ID}.md (legacy)
 */
export function resolveRefinementPath(wpId) {
  const folderPath = govRootRelPath("task_packets", wpId, "refinement.md");
  const folderAbsPath = govRootAbsPath("task_packets", wpId, "refinement.md");
  const flatPath = govRootRelPath("refinements", `${wpId}.md`);
  const flatAbsPath = govRootAbsPath("refinements", `${wpId}.md`);
  if (fs.existsSync(folderAbsPath)) return folderPath;
  if (fs.existsSync(flatAbsPath)) return flatPath;
  return null;
}

export function refinementAbsPath(wpId) {
  const refinementPath = resolveRefinementPath(wpId);
  return refinementPath ? repoPathAbs(refinementPath) : govRootAbsPath("refinements", `${wpId}.md`);
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

/**
 * Live ORCHESTRATOR_GATES.json authority lives in the external governance runtime root.
 * The repo-local .GOV path is legacy residue only and must not receive live writes.
 */
export const SHARED_GOV_ORCHESTRATOR_GATES_FILE = relWithinGovernanceRuntime("roles_shared", "ORCHESTRATOR_GATES.json");
export const LEGACY_ORCHESTRATOR_GATES_FILE = govRootRelPath("roles", "orchestrator", "runtime", "ORCHESTRATOR_GATES.json");

export function resolveOrchestratorGatesPath() {
  return SHARED_GOV_ORCHESTRATOR_GATES_FILE;
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
export const SHARED_GOV_WP_TOKEN_USAGE_ROOT = relWithinGovernanceRuntime("roles_shared", "WP_TOKEN_USAGE");
export const SHARED_GOV_VALIDATOR_GATES_ROOT = relWithinGovernanceRuntime("roles_shared", "validator_gates");
export const SHARED_GOV_GIT_TOPOLOGY_FILE = relWithinGovernanceRuntime("roles_shared", "GIT_TOPOLOGY_REGISTRY.json");

export function ensureGovernanceRuntimeDir(...segments) {
  const targetDir = governanceRuntimeAbsPath(...segments);
  fs.mkdirSync(targetDir, { recursive: true });
  return targetDir;
}
