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
export const WORK_PACKETS_LOGICAL_DIRNAME = "work_packets";
export const LEGACY_TASK_PACKETS_DIRNAME = "task_packets";

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

function workPacketRootCandidatesAt(govRootAbs, govRootRel) {
  const normalizedGovRootAbs = path.resolve(String(govRootAbs || GOV_ROOT_ABS));
  const normalizedGovRootRel = normalizePath(govRootRel || GOV_ROOT_REPO_REL) || ".GOV";
  return [
    {
      dirName: WORK_PACKETS_LOGICAL_DIRNAME,
      rootAbs: path.join(normalizedGovRootAbs, WORK_PACKETS_LOGICAL_DIRNAME),
      rootRel: normalizePath(path.join(normalizedGovRootRel, WORK_PACKETS_LOGICAL_DIRNAME)),
    },
    {
      dirName: LEGACY_TASK_PACKETS_DIRNAME,
      rootAbs: path.join(normalizedGovRootAbs, LEGACY_TASK_PACKETS_DIRNAME),
      rootRel: normalizePath(path.join(normalizedGovRootRel, LEGACY_TASK_PACKETS_DIRNAME)),
    },
  ];
}

function existingWorkPacketRootCandidatesAt(govRootAbs, govRootRel) {
  return workPacketRootCandidatesAt(govRootAbs, govRootRel)
    .filter((candidate) => fs.existsSync(candidate.rootAbs));
}

function preferredWorkPacketRootCandidateAt(govRootAbs, govRootRel) {
  return existingWorkPacketRootCandidatesAt(govRootAbs, govRootRel)[0]
    || workPacketRootCandidatesAt(govRootAbs, govRootRel).at(-1);
}

function workPacketArchiveRootCandidatesAt(govRootAbs, govRootRel) {
  return workPacketRootCandidatesAt(govRootAbs, govRootRel)
    .flatMap((candidate) => ([
      {
        lifecycleClass: "SUPERSEDED",
        rootAbs: path.join(candidate.rootAbs, "_archive", "superseded"),
        rootRel: normalizePath(path.join(candidate.rootRel, "_archive", "superseded")),
      },
      {
        lifecycleClass: "VALIDATED_CLOSED",
        rootAbs: path.join(candidate.rootAbs, "_archive", "validated_closed"),
        rootRel: normalizePath(path.join(candidate.rootRel, "_archive", "validated_closed")),
      },
    ]));
}

export const WORK_PACKET_STORAGE_ROOT_ABS = preferredWorkPacketRootCandidateAt(GOV_ROOT_ABS, GOV_ROOT_REPO_REL).rootAbs;
export const WORK_PACKET_STORAGE_ROOT_REPO_REL = preferredWorkPacketRootCandidateAt(GOV_ROOT_ABS, GOV_ROOT_REPO_REL).rootRel;
export const WORK_PACKET_STUB_STORAGE_ROOT_ABS = path.join(WORK_PACKET_STORAGE_ROOT_ABS, "stubs");
export const WORK_PACKET_STUB_STORAGE_ROOT_REPO_REL = normalizePath(path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, "stubs"));
export const WORK_PACKET_ARCHIVE_ROOT_ABS = path.join(WORK_PACKET_STORAGE_ROOT_ABS, "_archive");
export const WORK_PACKET_ARCHIVE_ROOT_REPO_REL = normalizePath(path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, "_archive"));
export const WORK_PACKET_SUPERSEDED_ARCHIVE_ROOT_ABS = path.join(WORK_PACKET_ARCHIVE_ROOT_ABS, "superseded");
export const WORK_PACKET_SUPERSEDED_ARCHIVE_ROOT_REPO_REL = normalizePath(path.join(WORK_PACKET_ARCHIVE_ROOT_REPO_REL, "superseded"));
export const WORK_PACKET_VALIDATED_CLOSED_ARCHIVE_ROOT_ABS = path.join(WORK_PACKET_ARCHIVE_ROOT_ABS, "validated_closed");
export const WORK_PACKET_VALIDATED_CLOSED_ARCHIVE_ROOT_REPO_REL = normalizePath(path.join(WORK_PACKET_ARCHIVE_ROOT_REPO_REL, "validated_closed"));

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
  const entriesByWpId = new Map();
  for (const candidate of existingWorkPacketRootCandidatesAt(GOV_ROOT_ABS, GOV_ROOT_REPO_REL)) {
    for (const entry of listWorkPacketEntriesAt(candidate.rootAbs, candidate.rootRel, { skipDirNames: ["stubs", "_archive"] })) {
      if (!entriesByWpId.has(entry.wpId)) {
        entriesByWpId.set(entry.wpId, entry);
      }
    }
  }
  return [...entriesByWpId.values()].sort((left, right) =>
    left.wpId.localeCompare(right.wpId)
    || left.packetPath.localeCompare(right.packetPath)
  );
}

export function listOfficialWorkPacketPaths() {
  return listOfficialWorkPacketEntries().map((entry) => entry.packetPath);
}

export function listArchivedWorkPacketEntriesAtRepo(repoRoot, localGovRootRel = GOV_ROOT_REPO_REL) {
  const repoRootAbs = path.resolve(String(repoRoot || REPO_ROOT));
  const normalizedGovRootRel = normalizePath(localGovRootRel || GOV_ROOT_REPO_REL) || ".GOV";
  const govRootAbs = path.resolve(repoRootAbs, normalizedGovRootRel);
  const entriesByWpId = new Map();
  for (const candidate of workPacketArchiveRootCandidatesAt(govRootAbs, normalizedGovRootRel)) {
    if (!fs.existsSync(candidate.rootAbs)) continue;
    for (const entry of listWorkPacketEntriesAt(candidate.rootAbs, candidate.rootRel)) {
      if (!entriesByWpId.has(entry.wpId)) {
        entriesByWpId.set(entry.wpId, {
          ...entry,
          lifecycleClass: candidate.lifecycleClass,
        });
      }
    }
  }
  return [...entriesByWpId.values()].sort((left, right) =>
    left.wpId.localeCompare(right.wpId)
    || left.packetPath.localeCompare(right.packetPath)
  );
}

export function listArchivedWorkPacketEntries() {
  return listArchivedWorkPacketEntriesAtRepo(REPO_ROOT, GOV_ROOT_REPO_REL);
}

export function listStubWorkPacketEntries() {
  const entriesByWpId = new Map();
  for (const candidate of existingWorkPacketRootCandidatesAt(GOV_ROOT_ABS, GOV_ROOT_REPO_REL)) {
    const stubRootAbs = path.join(candidate.rootAbs, "stubs");
    const stubRootRel = normalizePath(path.join(candidate.rootRel, "stubs"));
    for (const entry of listWorkPacketEntriesAt(stubRootAbs, stubRootRel)) {
      if (!entriesByWpId.has(entry.wpId)) {
        entriesByWpId.set(entry.wpId, entry);
      }
    }
  }
  return [...entriesByWpId.values()].sort((left, right) =>
    left.wpId.localeCompare(right.wpId)
    || left.packetPath.localeCompare(right.packetPath)
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
  return resolveWorkPacketPathAtRepo(REPO_ROOT, wpId, GOV_ROOT_REPO_REL);
}

export function resolveWorkPacketPathAtRepo(repoRoot, wpId, localGovRootRel = ".GOV") {
  const repoRootAbs = path.resolve(String(repoRoot || REPO_ROOT));
  const normalizedGovRootRel = normalizePath(localGovRootRel || ".GOV") || ".GOV";
  const govRootAbs = path.resolve(repoRootAbs, normalizedGovRootRel);
  for (const candidate of workPacketRootCandidatesAt(govRootAbs, normalizedGovRootRel)) {
    const folderPath = normalizePath(path.join(candidate.rootRel, wpId, "packet.md"));
    const folderAbsPath = path.join(candidate.rootAbs, wpId, "packet.md");
    const flatPath = normalizePath(path.join(candidate.rootRel, `${wpId}.md`));
    const flatAbsPath = path.join(candidate.rootAbs, `${wpId}.md`);
    if (fs.existsSync(folderAbsPath)) {
      return {
        packetPath: folderPath,
        packetAbsPath: folderAbsPath,
        packetDir: normalizePath(path.join(candidate.rootRel, wpId)),
        packetDirAbs: path.join(candidate.rootAbs, wpId),
        isFolder: true,
      };
    }
    if (fs.existsSync(flatAbsPath)) {
      return {
        packetPath: flatPath,
        packetAbsPath: flatAbsPath,
        packetDir: candidate.rootRel,
        packetDirAbs: candidate.rootAbs,
        isFolder: false,
      };
    }
  }
  for (const archiveCandidate of workPacketArchiveRootCandidatesAt(govRootAbs, normalizedGovRootRel)) {
    const folderPath = normalizePath(path.join(archiveCandidate.rootRel, wpId, "packet.md"));
    const folderAbsPath = path.join(archiveCandidate.rootAbs, wpId, "packet.md");
    const flatPath = normalizePath(path.join(archiveCandidate.rootRel, `${wpId}.md`));
    const flatAbsPath = path.join(archiveCandidate.rootAbs, `${wpId}.md`);
    if (fs.existsSync(folderAbsPath)) {
      return {
        packetPath: folderPath,
        packetAbsPath: folderAbsPath,
        packetDir: normalizePath(path.join(archiveCandidate.rootRel, wpId)),
        packetDirAbs: path.join(archiveCandidate.rootAbs, wpId),
        isFolder: true,
        lifecycleClass: archiveCandidate.lifecycleClass,
      };
    }
    if (fs.existsSync(flatAbsPath)) {
      return {
        packetPath: flatPath,
        packetAbsPath: flatAbsPath,
        packetDir: archiveCandidate.rootRel,
        packetDirAbs: archiveCandidate.rootAbs,
        isFolder: false,
        lifecycleClass: archiveCandidate.lifecycleClass,
      };
    }
  }
  return null;
}

export function workPacketPathAtRepo(repoRoot, wpId, localGovRootRel = ".GOV") {
  return resolveWorkPacketPathAtRepo(repoRoot, wpId, localGovRootRel)?.packetPath
    || normalizePath(path.join(localGovRootRel || ".GOV", LEGACY_TASK_PACKETS_DIRNAME, `${wpId}.md`));
}

export function workPacketAbsPathAtRepo(repoRoot, wpId, localGovRootRel = ".GOV") {
  return resolveWorkPacketPathAtRepo(repoRoot, wpId, localGovRootRel)?.packetAbsPath
    || path.resolve(String(repoRoot || REPO_ROOT), workPacketPathAtRepo(repoRoot, wpId, localGovRootRel));
}

export function workPacketPath(wpId) {
  return resolveWorkPacketPath(wpId)?.packetPath || normalizePath(path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpId}.md`));
}

export function workPacketAbsPath(wpId) {
  return resolveWorkPacketPath(wpId)?.packetAbsPath || path.join(WORK_PACKET_STORAGE_ROOT_ABS, `${wpId}.md`);
}

export function ensureWorkPacketLifecycleLayout(repoRoot = REPO_ROOT, localGovRootRel = GOV_ROOT_REPO_REL) {
  const repoRootAbs = path.resolve(String(repoRoot || REPO_ROOT));
  const normalizedGovRootRel = normalizePath(localGovRootRel || GOV_ROOT_REPO_REL) || ".GOV";
  const govRootAbs = path.resolve(repoRootAbs, normalizedGovRootRel);
  const activeRoot = preferredWorkPacketRootCandidateAt(govRootAbs, normalizedGovRootRel);

  fs.mkdirSync(activeRoot.rootAbs, { recursive: true });
  fs.mkdirSync(path.join(activeRoot.rootAbs, "stubs"), { recursive: true });
  fs.mkdirSync(path.join(activeRoot.rootAbs, "_archive", "superseded"), { recursive: true });
  fs.mkdirSync(path.join(activeRoot.rootAbs, "_archive", "validated_closed"), { recursive: true });

  return {
    activeRootAbs: activeRoot.rootAbs,
    activeRootRel: activeRoot.rootRel,
    stubRootAbs: path.join(activeRoot.rootAbs, "stubs"),
    stubRootRel: normalizePath(path.join(activeRoot.rootRel, "stubs")),
    archiveRootAbs: path.join(activeRoot.rootAbs, "_archive"),
    archiveRootRel: normalizePath(path.join(activeRoot.rootRel, "_archive")),
    supersededArchiveRootAbs: path.join(activeRoot.rootAbs, "_archive", "superseded"),
    supersededArchiveRootRel: normalizePath(path.join(activeRoot.rootRel, "_archive", "superseded")),
    validatedClosedArchiveRootAbs: path.join(activeRoot.rootAbs, "_archive", "validated_closed"),
    validatedClosedArchiveRootRel: normalizePath(path.join(activeRoot.rootRel, "_archive", "validated_closed")),
  };
}

export function taskBoardPathAtRepo(repoRoot, localGovRootRel = ".GOV") {
  return path.resolve(String(repoRoot || REPO_ROOT), normalizePath(path.join(localGovRootRel || ".GOV", "roles_shared", "records", "TASK_BOARD.md")));
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
  for (const candidate of workPacketRootCandidatesAt(GOV_ROOT_ABS, GOV_ROOT_REPO_REL)) {
    const folderPath = normalizePath(path.join(candidate.rootRel, wpId, "refinement.md"));
    const folderAbsPath = path.join(candidate.rootAbs, wpId, "refinement.md");
    if (fs.existsSync(folderAbsPath)) return folderPath;
  }
  const flatPath = govRootRelPath("refinements", `${wpId}.md`);
  const flatAbsPath = govRootAbsPath("refinements", `${wpId}.md`);
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
