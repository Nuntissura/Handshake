import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { DatabaseSync } from "node:sqlite";
import {
  CLI_SESSION_TOOL,
  ROLE_SESSION_FALLBACK_MODEL,
  ROLE_MODEL_PROFILE_POLICY,
  ROLE_SESSION_PRIMARY_MODEL,
  ROLE_SESSION_REASONING_CONFIG_KEY,
  ROLE_SESSION_REASONING_CONFIG_VALUE,
  SESSION_BUSY_INGRESS_MODES,
  resolveRoleModelProfileSelection,
  roleModelProfileField,
  roleModelProfileSupportsGovernedLaunch,
  SESSION_COMMAND_KINDS,
  SESSION_COMMAND_OUTCOME_STATES,
  SESSION_COMMAND_STATUSES,
  SESSION_CONTROL_BROKER_AUTH_MODE,
  SESSION_CONTROL_BROKER_BUILD_ID,
  SESSION_CONTROL_OUTPUT_DIR,
  defaultCoderBranch,
  defaultCoderWorktreeDir,
  defaultIntegrationValidatorBranch,
  defaultIntegrationValidatorWorktreeDir,
  defaultWpValidatorBranch,
  defaultWpValidatorWorktreeDir,
  roleNextCommand,
  roleStartupCommand,
} from "./session-policy.mjs";
import {
  buildGovernedActionRequest,
  buildGovernedActionResult,
  defaultGovernedActionRuleIdForSessionCommand,
  validateGovernedActionRequestShape,
  validateGovernedActionResultShape,
} from "./session-governed-action-lib.mjs";
import { buildPhaseCheckCommand } from "../../checks/phase-check-lib.mjs";
import { buildRoleInbox } from "../lib/wp-communication-health-lib.mjs";
import {
  GOV_ROOT_ABS,
  GOV_ROOT_ENV_VAR,
  GOVERNANCE_RUNTIME_ROOT_ABS,
  listStubWorkPacketEntries,
  repoPathAbs,
  resolveWorkPacketPath,
  workPacketPath,
} from "../lib/runtime-paths.mjs";
import {
  formatProtectedWorktreeResolutionDiagnostics,
  resolveProtectedWorktree,
} from "../topology/git-topology-lib.mjs";

export const SESSION_CONTROL_REQUEST_SCHEMA_ID = "hsk.session_control_request@1";
export const SESSION_CONTROL_REQUEST_SCHEMA_VERSION = "session_control_request_v1";
export const SESSION_CONTROL_RESULT_SCHEMA_ID = "hsk.session_control_result@1";
export const SESSION_CONTROL_RESULT_SCHEMA_VERSION = "session_control_result_v1";
export const ORCHESTRATOR_MANAGED_REAL_BLOCKER_CLASSES = [
  "POLICY_CONFLICT",
  "AUTHORITY_OVERRIDE_REQUIRED",
  "OPERATOR_ARTIFACT_REQUIRED",
  "ENVIRONMENT_FAILURE",
];
export const CODEX_AUTHORITY_PATH = ".GOV/codex/Handshake_Codex_v1.4.md";

export function roleProtocolPath(role) {
  if (role === "ACTIVATION_MANAGER") return ".GOV/roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md";
  if (role === "CODER") return ".GOV/roles/coder/CODER_PROTOCOL.md";
  if (role === "MEMORY_MANAGER") return ".GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md";
  if (role === "WP_VALIDATOR") return ".GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md";
  if (role === "INTEGRATION_VALIDATOR") return ".GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md";
  if (role === "VALIDATOR") return ".GOV/roles/validator/VALIDATOR_PROTOCOL.md";
  return ".GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md";
}

export function buildRoleAuthorityString(role, wpId) {
  return `AGENTS.md + ${CODEX_AUTHORITY_PATH} + ${roleProtocolPath(role)} + startup output + ${resolveAuthorityPacketPath(wpId)}`;
}

function resolveAuthorityPacketPath(wpId) {
  const packetInfo = resolveWorkPacketPath(wpId);
  if (packetInfo?.packetPath) return packetInfo.packetPath;
  const stubInfo = listStubWorkPacketEntries().find((entry) => entry.wpId === wpId);
  if (stubInfo?.packetPath) return stubInfo.packetPath;
  return workPacketPath(wpId);
}

function nowIso() {
  return new Date().toISOString();
}

function writeJsonlEvent(outputStream, event) {
  outputStream.write(`${JSON.stringify({ timestamp: nowIso(), ...event })}\n`);
}

function resolveCliToolByName(toolName) {
  if (process.platform !== "win32") return toolName;
  const result = spawnSync("where.exe", [toolName], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
    windowsHide: true,
  });
  if (result.status !== 0) return `${toolName}.cmd`;
  const matches = result.stdout
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const exeMatch = matches.find((entry) => /\.exe$/i.test(entry));
  if (exeMatch) return exeMatch;
  return matches.find((entry) => /\.cmd$/i.test(entry)) || matches[0] || `${toolName}.cmd`;
}

function resolveCliTool() {
  return resolveCliToolByName(CLI_SESSION_TOOL);
}

const CLAUDE_CODE_CLI_TOOL = "claude";

function resolveClaudeCodeCliTool() {
  return resolveCliToolByName(CLAUDE_CODE_CLI_TOOL);
}

const OLLAMA_CLI_TOOL = "ollama";

function resolveOllamaCliTool() {
  return resolveCliToolByName(OLLAMA_CLI_TOOL);
}

export function resolveCliToolForProfile(profile) {
  if (profile.provider === "ANTHROPIC") return resolveClaudeCodeCliTool();
  if (profile.provider === "OLLAMA_LOCAL") return resolveOllamaCliTool();
  return resolveCliTool();
}

function quotePsLiteral(value) {
  return `'${String(value ?? "").replace(/'/g, "''")}'`;
}

function writeWindowsPromptRunner({ toolPath, args = [], prompt = "", prefix = "governed-cli" } = {}) {
  const promptPath = path.join(os.tmpdir(), `${prefix}-${Date.now()}-${crypto.randomUUID()}.prompt.txt`);
  const scriptPath = path.join(os.tmpdir(), `${prefix}-${Date.now()}-${crypto.randomUUID()}.ps1`);
  fs.writeFileSync(promptPath, String(prompt || ""), "utf8");
  const psArgsLines = args.map((arg) => `  ${quotePsLiteral(arg)}`).join(",\r\n");
  const script = [
    `$ErrorActionPreference = 'Stop'`,
    `$cliArgs = @(`,
    psArgsLines,
    `)`,
    `$promptText = Get-Content -Raw -LiteralPath ${quotePsLiteral(promptPath)}`,
    `$promptText | & ${quotePsLiteral(toolPath)} @cliArgs`,
  ].join("\r\n");
  fs.writeFileSync(scriptPath, script, "utf8");
  return { promptPath, scriptPath };
}

export function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

export function toRepoRelativePath(repoRoot, targetPath) {
  const repoAbs = path.resolve(repoRoot);
  const targetAbs = path.resolve(targetPath);
  const relative = normalizePath(path.relative(repoAbs, targetAbs));
  return relative || ".";
}

export function sanitizeSessionKey(value) {
  return String(value || "")
    .trim()
    .replace(/[^A-Za-z0-9._-]+/g, "_");
}

function repoScopeKey(repoRoot) {
  return crypto
    .createHash("sha256")
    .update(normalizePath(path.resolve(repoRoot)))
    .digest("hex")
    .slice(0, 24);
}

export function brokerAuthTokenFile(repoRoot) {
  return path.join(os.tmpdir(), "handshake-acp-bridge", repoScopeKey(repoRoot), "auth-token.txt");
}

export function ensureBrokerAuthToken(repoRoot) {
  const filePath = brokerAuthTokenFile(repoRoot);
  if (fs.existsSync(filePath)) {
    const existing = fs.readFileSync(filePath, "utf8").trim();
    if (existing) return existing;
  }
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  const token = crypto.randomBytes(32).toString("hex");
  fs.writeFileSync(filePath, `${token}\n`, { encoding: "utf8", mode: 0o600 });
  return token;
}

export function resolveRoleConfig(roleName, workPacketId) {
  if (roleName === "ACTIVATION_MANAGER") {
    return {
      branch: "gov_kernel",
      worktreeDir: ".",
      title: `ACTMAN ${workPacketId}`,
      startupCommand: roleStartupCommand("ACTIVATION_MANAGER"),
      nextCommand: roleNextCommand("ACTIVATION_MANAGER", workPacketId),
      focus: "pre-launch governance authoring only: refinement, approved spec enrichment, signature normalization/recording, packet hydration, microtask preparation, worktree preparation, and activation readiness",
    };
  }
  if (roleName === "CODER") {
    return {
      branch: defaultCoderBranch(workPacketId),
      worktreeDir: defaultCoderWorktreeDir(workPacketId),
      title: `CODER ${workPacketId}`,
      startupCommand: roleStartupCommand("CODER"),
      nextCommand: roleNextCommand("CODER", workPacketId),
      focus: "implementation, governance paperwork, and coder-side delegation only when the packet allows it",
    };
  }
  if (roleName === "MEMORY_MANAGER") {
    return {
      branch: "gov_kernel",
      worktreeDir: ".",
      title: `MEMORY_MANAGER ${workPacketId}`,
      startupCommand: roleStartupCommand("MEMORY_MANAGER"),
      nextCommand: roleNextCommand("MEMORY_MANAGER", workPacketId),
      focus: "governance memory hygiene: quality assessment, contradiction resolution, stale entry analysis, RGF candidate drafting, proposal writing to .GOV/roles/memory_manager/proposals/",
    };
  }
  if (roleName === "WP_VALIDATOR") {
    return {
      branch: defaultWpValidatorBranch(workPacketId),
      worktreeDir: defaultWpValidatorWorktreeDir(workPacketId),
      title: `WPVAL ${workPacketId}`,
      startupCommand: roleStartupCommand("WP_VALIDATOR"),
      nextCommand: roleNextCommand("WP_VALIDATOR", workPacketId),
      focus: "WP-scoped technical steering, bootstrap/skeleton review, and packet-scoped validation receipts (operates from the shared coder worktree and diffs against main)",
    };
  }
  if (roleName === "INTEGRATION_VALIDATOR") {
    return {
      branch: defaultIntegrationValidatorBranch(workPacketId),
      worktreeDir: defaultIntegrationValidatorWorktreeDir(workPacketId),
      title: `INTVAL ${workPacketId}`,
      startupCommand: roleStartupCommand("INTEGRATION_VALIDATOR"),
      nextCommand: roleNextCommand("INTEGRATION_VALIDATOR", workPacketId),
      focus: "final technical verdict, merge authority, sync-gov-to-main, and main/origin push (operates from handshake_main on branch main)",
    };
  }
  return null;
}

export function resolveRoleWorktreePath(repoRoot, roleConfig = {}) {
  const normalizedBranch = String(roleConfig.branch || "").trim();
  const worktreeDir = String(roleConfig.worktreeDir || "").trim();
  if (normalizedBranch === "main" || /(^|[\\/])handshake_main$/i.test(worktreeDir)) {
    const resolution = resolveProtectedWorktree("handshake_main", { repoRoot });
    return {
      absWorktreeDir: resolution.absDir,
      resolution,
      diagnostics: formatProtectedWorktreeResolutionDiagnostics(resolution),
    };
  }
  return {
    absWorktreeDir: path.resolve(repoRoot, worktreeDir || "."),
    resolution: null,
    diagnostics: [],
  };
}

function sessionCompatScript(role, scriptName) {
  return role === "INTEGRATION_VALIDATOR"
    ? `"$env:${GOV_ROOT_ENV_VAR}/roles_shared/scripts/session/${scriptName}"`
    : `.GOV/roles_shared/scripts/session/${scriptName}`;
}

function validatorCompatCommand(role, wpId, command) {
  const scriptPath = sessionCompatScript(role, "role-command-compat.mjs");
  const args = command === "validator-next" ? `${role} ${wpId}` : role;
  return `node ${scriptPath} ${command} ${args}`;
}

function repomemCompatCommand(role, subcommand, content = "", flags = "") {
  const scriptPath = sessionCompatScript(role, "repomem-compat.mjs");
  const quotedContent = content ? ` "${content.replace(/"/g, '\\"')}"` : "";
  return `node ${scriptPath} ${subcommand}${quotedContent}${flags ? ` ${flags}` : ""}`;
}

function executableStartupCommand(role, wpId, roleConfig) {
  if (role === "WP_VALIDATOR" || role === "INTEGRATION_VALIDATOR" || role === "VALIDATOR") {
    return validatorCompatCommand(role, wpId, "validator-startup");
  }
  return roleConfig.startupCommand;
}

function executableNextCommand(role, wpId, roleConfig) {
  if (role === "WP_VALIDATOR" || role === "INTEGRATION_VALIDATOR" || role === "VALIDATOR") {
    return validatorCompatCommand(role, wpId, "validator-next");
  }
  return roleConfig.nextCommand;
}

export function selectModel(modelSelector) {
  return String(modelSelector || "").trim().toUpperCase() === "FALLBACK"
    ? ROLE_SESSION_FALLBACK_MODEL
    : ROLE_SESSION_PRIMARY_MODEL;
}

export function buildRoleEnvironmentOverrides({
  role = "",
  governanceRootAbs = GOV_ROOT_ABS,
} = {}) {
  if (String(role || "").trim().toUpperCase() !== "INTEGRATION_VALIDATOR") {
    return {};
  }
  return {
    [GOV_ROOT_ENV_VAR]: normalizePath(path.resolve(governanceRootAbs || GOV_ROOT_ABS)),
  };
}

export function loadWorkPacketContent(wpId) {
  const packetPath = resolveAuthorityPacketPath(wpId);
  const packetAbs = repoPathAbs(packetPath);
  if (!fs.existsSync(packetAbs)) return "";
  return fs.readFileSync(packetAbs, "utf8");
}

export function resolveRoleLaunchSelection({
  role,
  wpId,
  modelSelector = "PRIMARY",
  packetContent = "",
} = {}) {
  const effectivePacketContent = packetContent || loadWorkPacketContent(wpId);
  const selection = resolveRoleModelProfileSelection(role, effectivePacketContent, modelSelector);
  return {
    packetContent: effectivePacketContent,
    primaryProfileId: selection.primary_profile_id,
    selectedProfileId: selection.selected_profile_id,
    selectedProfile: selection.profile,
  };
}

export function assertRoleLaunchProfileSupported({
  role,
  wpId,
  selectedProfileId,
  selectedProfile,
} = {}) {
  if (!selectedProfileId || !selectedProfile) {
    throw new Error(
      `Missing governed role model profile for ${role}:${wpId}. Expected packet field ${roleModelProfileField(role) || "<unknown>"}.`,
    );
  }
  if (!roleModelProfileSupportsGovernedLaunch(selectedProfileId)) {
    throw new Error(
      `Role profile ${selectedProfileId} for ${role}:${wpId} is governance-declared only (tool=${selectedProfile.session_tool}, runtime_support=${selectedProfile.runtime_support}). Implement provider-specific governed launch support before using it in ACP/session-control.`,
    );
  }
  if (selectedProfile.allowed_roles && !selectedProfile.allowed_roles.includes(role)) {
    throw new Error(
      `Role profile ${selectedProfileId} is restricted to roles [${selectedProfile.allowed_roles.join(", ")}] but was assigned to ${role}:${wpId}. Choose a profile that supports this role.`,
    );
  }
  return selectedProfile;
}

const SESSION_MEMORY_TOKEN_BUDGET = 1500;
const STARTUP_MEMORY_PROMPT_TOKEN_BUDGET = 220;
const STARTUP_MEMORY_PROMPT_MAX_LINES = 8;
const STARTUP_CONVERSATION_PROMPT_TOKEN_BUDGET = 120;
const STARTUP_CONVERSATION_PROMPT_MAX_LINES = 4;

function estimateTokens(text) {
  return Math.ceil(String(text || "").length / 4);
}

export function boundPromptLines(lines, { tokenBudget = 200, maxLines = 6 } = {}) {
  const selected = [];
  let usedTokens = 0;
  for (const rawLine of Array.isArray(lines) ? lines : []) {
    const line = String(rawLine || "").trimEnd();
    if (!line) continue;
    if (selected.length >= maxLines) break;
    const lineTokens = estimateTokens(line);
    if (usedTokens + lineTokens > tokenBudget) break;
    usedTokens += lineTokens;
    selected.push(line);
  }
  return selected;
}

// Role-scoped type filters: coder gets fail log only, validator adds context, orchestrator gets everything
const ROLE_MEMORY_TYPE_FILTER = {
  CODER: new Set(["procedural"]),
  WP_VALIDATOR: new Set(["procedural", "semantic"]),
  INTEGRATION_VALIDATOR: new Set(["procedural", "semantic"]),
};

function loadSessionMemoryLines(wpId, { role = "", fileTargets = [], tokenBudget = SESSION_MEMORY_TOKEN_BUDGET } = {}) {
  try {
    const dbPath = path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "GOVERNANCE_MEMORY.db");
    if (!fs.existsSync(dbPath)) return [];
    const db = new DatabaseSync(dbPath);
    try {
      const now = Date.now();
      const allowedTypes = ROLE_MEMORY_TYPE_FILTER[role] || null; // null = all types

      const candidates = db.prepare(
        `SELECT id, memory_type, topic, summary, file_scope, importance, access_count, created_at, last_accessed_at
         FROM memory_index
         WHERE consolidated = 0 AND (wp_id = ? OR wp_id = '')
         ORDER BY importance DESC, created_at DESC LIMIT 80`
      ).all(wpId || "");
      if (candidates.length === 0) return [];

      // RGF-128: precompute file basenames for file-scope matching
      const targetBasenames = fileTargets.map(f => path.basename(f).toLowerCase()).filter(Boolean);

      for (const c of candidates) {
        // Role-scoped type filter
        if (allowedTypes && !allowedTypes.has(c.memory_type)) { c._score = -1; continue; }

        const ageMs = now - new Date(c.last_accessed_at || c.created_at).getTime();
        const ageDays = ageMs / (1000 * 60 * 60 * 24);
        const recencyBoost = Math.exp(-0.05 * ageDays);
        const accessBoost = 1 + Math.min(c.access_count, 10) * 0.05;
        // RGF-130: staleness penalty
        let stalenessFactor = 1;
        if (c.file_scope && (c.memory_type === "procedural" || c.memory_type === "semantic")) {
          const scopeFiles = c.file_scope.split(",").map(f => f.trim()).filter(Boolean);
          if (scopeFiles.length > 0) {
            const existCount = scopeFiles.filter(f => {
              try { return fs.existsSync(repoPathAbs(f)); } catch { return false; }
            }).length;
            if (existCount === 0) stalenessFactor = 0.5;
          }
        }
        // RGF-128: file-scope match boost
        let fileScopeBoost = 1;
        if (targetBasenames.length > 0 && c.file_scope) {
          const memBasenames = c.file_scope.split(",").map(f => path.basename(f.trim()).toLowerCase()).filter(Boolean);
          const hasMatch = memBasenames.some(mb => targetBasenames.some(tb => mb.includes(tb) || tb.includes(mb)));
          if (hasMatch) fileScopeBoost = 2;
        }
        c._score = (c.importance || 0.5) * recencyBoost * accessBoost * stalenessFactor * fileScopeBoost;
      }

      const scored = candidates.filter(c => c._score > 0).sort((a, b) => b._score - a._score);
      if (scored.length === 0) return [];

      // RGF-133: session diversification + RGF-139: trust scoring
      const SESSION_DIVERSITY_CAP = 3;
      const TRUST_SCORES = {
        receipt_extraction: 1.0, smoketest_extraction: 0.9, check_failure: 0.8,
        manual_capture: 0.7, "memory-capture": 0.7, session_flush: 0.5,
      };
      const sessionCounts = new Map();
      const diversified = [];
      for (const c of scored) {
        const entry = db.prepare("SELECT source_session, source_artifact, metadata FROM memory_entries WHERE index_id = ? LIMIT 1").get(c.id);
        const sess = entry?.source_session || "_unknown_";
        const count = sessionCounts.get(sess) || 0;
        if (count >= SESSION_DIVERSITY_CAP) continue;
        // RGF-139: apply trust multiplier based on source
        const source = entry?.source_artifact || "";
        let meta = {};
        try { meta = JSON.parse(entry?.metadata || "{}"); } catch {}
        const trustKey = meta.session_flush ? "session_flush"
          : meta.captured_mid_session ? "manual_capture"
          : meta.captured_from_check ? "check_failure"
          : source === "RECEIPTS.jsonl" ? "receipt_extraction"
          : source.endsWith(".md") ? "smoketest_extraction"
          : "receipt_extraction";
        c._score *= (TRUST_SCORES[trustKey] || 0.8);
        sessionCounts.set(sess, count + 1);
        diversified.push(c);
      }
      // Re-sort after trust adjustment
      diversified.sort((a, b) => b._score - a._score);

      // Fetch content for procedural memories (the actual fix recipe)
      const contentCache = new Map();
      const proceduralIds = diversified.filter(c => c.memory_type === "procedural").map(c => c.id);
      for (const id of proceduralIds) {
        const entry = db.prepare("SELECT content FROM memory_entries WHERE index_id = ? LIMIT 1").get(id);
        if (entry?.content) contentCache.set(id, entry.content);
      }

      // Build structured output grouped by type
      let tokenCount = 0;
      const patterns = []; // procedural — shown with content snippet
      const context = [];  // semantic — shown as one-liners
      const history = [];  // episodic — compressed into timeline

      for (const c of diversified) {
        if (c.memory_type === "procedural") {
          const content = contentCache.get(c.id) || "";
          const snippet = content.split("\n").slice(0, 3).join(" | ").slice(0, 200);
          const line = `- ${c.topic}${c.file_scope ? ` (${c.file_scope})` : ""}\n  → ${snippet || c.summary.slice(0, 150)}`;
          const lineTokens = estimateTokens(line);
          if (tokenCount + lineTokens > tokenBudget) continue;
          tokenCount += lineTokens;
          patterns.push({ ...c, _line: line });
        } else if (c.memory_type === "semantic") {
          const line = `- ${c.topic}: ${c.summary.slice(0, 150)}`;
          const lineTokens = estimateTokens(line);
          if (tokenCount + lineTokens > tokenBudget) continue;
          tokenCount += lineTokens;
          context.push({ ...c, _line: line });
        } else if (c.memory_type === "episodic") {
          // Compress: just the receipt kind + short outcome
          const kind = c.topic.split(/\s+/)[0] || "EVENT";
          const shortSummary = c.summary.slice(0, 80);
          const line = `${kind}: ${shortSummary}`;
          const lineTokens = estimateTokens(line);
          if (tokenCount + lineTokens > tokenBudget) continue;
          tokenCount += lineTokens;
          history.push({ ...c, _line: line });
        }
      }

      // RGF-147: load recent pre-task snapshots for this WP
      const snapshots = loadRecentSnapshots(db, { wpId, maxPerType: 1, maxTotal: 3, tokenBudget: tokenBudget - tokenCount });
      tokenCount += snapshots.tokenCount;

      const allSelected = [...patterns, ...context, ...history, ...snapshots.entries];
      if (allSelected.length === 0) return [];

      // Update access counts
      db.prepare(
        `UPDATE memory_index SET access_count = access_count + 1, last_accessed_at = ?
         WHERE id IN (${allSelected.map(() => "?").join(",")})`
      ).run(new Date().toISOString(), ...allSelected.map(s => s.id));

      // Assemble structured output
      const lines = [];
      const sectionCounts = [];
      if (patterns.length > 0) sectionCounts.push(`${patterns.length} patterns`);
      if (context.length > 0) sectionCounts.push(`${context.length} context`);
      if (history.length > 0) sectionCounts.push(`${history.length} events`);
      if (snapshots.entries.length > 0) sectionCounts.push(`${snapshots.entries.length} snapshots`);
      lines.push(`SESSION MEMORY (${sectionCounts.join(", ")}, ${tokenCount} est. tokens):`);

      if (patterns.length > 0) {
        lines.push("FAIL LOG:");
        for (const p of patterns) lines.push(p._line);
      }
      if (context.length > 0) {
        lines.push("CONTEXT:");
        for (const c of context) lines.push(c._line);
      }
      if (history.length > 0) {
        lines.push(`HISTORY: ${history.map(h => h._line).join(" → ")}`);
      }
      if (snapshots.entries.length > 0) {
        lines.push("SNAPSHOTS:");
        for (const s of snapshots.entries) lines.push(s._line);
      }

      return lines;
    } finally { try { db.close(); } catch {} }
  } catch { return []; }
}

// RGF-147: load recent pre-task snapshots from governance memory
// Returns { entries: [{ id, _line }], tokenCount }
function loadRecentSnapshots(db, { wpId = "", maxPerType = 1, maxTotal = 3, tokenBudget = 300 } = {}) {
  const result = { entries: [], tokenCount: 0 };
  try {
    // Check if snapshot_type column exists
    const columns = db.prepare("PRAGMA table_info(memory_index)").all();
    if (!columns.some(c => c.name === "snapshot_type")) return result;

    let sql = `SELECT id, snapshot_type, topic, summary, wp_id, created_at
               FROM memory_index
               WHERE snapshot_type != '' AND consolidated = 0`;
    const params = [];
    if (wpId) { sql += " AND (wp_id = ? OR wp_id = '')"; params.push(wpId); }
    sql += " ORDER BY created_at DESC LIMIT 20";
    const rows = db.prepare(sql).all(...params);
    if (rows.length === 0) return result;

    // Keep only the most recent per snapshot_type, up to maxTotal
    const seenTypes = new Map();
    for (const row of rows) {
      if (result.entries.length >= maxTotal) break;
      const typeCount = seenTypes.get(row.snapshot_type) || 0;
      if (typeCount >= maxPerType) continue;
      const age = Math.round((Date.now() - new Date(row.created_at).getTime()) / 3600000);
      const ageLabel = age < 1 ? "<1h ago" : age < 24 ? `${age}h ago` : `${Math.round(age / 24)}d ago`;
      const line = `- [${row.snapshot_type}] ${row.wp_id || "cross-WP"} (${ageLabel}): ${row.summary.slice(0, 120)}`;
      const lineTokens = estimateTokens(line);
      if (result.tokenCount + lineTokens > tokenBudget) continue;
      result.tokenCount += lineTokens;
      seenTypes.set(row.snapshot_type, typeCount + 1);
      result.entries.push({ id: row.id, _line: line });
    }
  } catch { /* best-effort */ }
  return result;
}

// RGF-125: Orchestrator memory injection — cross-WP, broader budget, different scoring
const ORCHESTRATOR_MEMORY_TOKEN_BUDGET = 2000;

function loadOrchestratorMemoryLines() {
  try {
    const dbPath = path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "GOVERNANCE_MEMORY.db");
    if (!fs.existsSync(dbPath)) return [];
    const db = new DatabaseSync(dbPath);
    try {
      const now = Date.now();
      // Cross-WP: no wp_id filter — orchestrator needs governance-wide context
      const candidates = db.prepare(
        `SELECT id, memory_type, topic, summary, file_scope, wp_id, importance, access_count, created_at, last_accessed_at
         FROM memory_index
         WHERE consolidated = 0
         ORDER BY importance DESC, created_at DESC LIMIT 80`
      ).all();
      if (candidates.length === 0) return [];

      for (const c of candidates) {
        const ageMs = now - new Date(c.last_accessed_at || c.created_at).getTime();
        const ageDays = ageMs / (1000 * 60 * 60 * 24);
        const recencyBoost = Math.exp(-0.05 * ageDays);
        const accessBoost = 1 + Math.min(c.access_count, 10) * 0.05;
        // Type priority: semantic > procedural > episodic (orchestrator cares about patterns)
        const typePriority = c.memory_type === "semantic" ? 1.5
          : c.memory_type === "procedural" ? 1.2
          : 1.0;
        // Systemic boost: memories accessed many times are governance-relevant patterns
        const systemicBoost = c.access_count >= 5 ? 1.3 : 1.0;
        c._score = (c.importance || 0.5) * recencyBoost * accessBoost * typePriority * systemicBoost;
      }
      candidates.sort((a, b) => b._score - a._score);

      let tokenCount = 0;
      const selected = [];
      for (const c of candidates) {
        const wpTag = c.wp_id ? ` [${c.wp_id}]` : "";
        const line = `- [${c.memory_type}]${wpTag} ${c.topic}: ${c.summary}`;
        const lineTokens = estimateTokens(line);
        if (tokenCount + lineTokens > ORCHESTRATOR_MEMORY_TOKEN_BUDGET) break;
        tokenCount += lineTokens;
        selected.push({ ...c, _line: line });
      }
      if (selected.length === 0 && tokenCount === 0) {
        // RGF-147: even if no regular memories, try to load snapshots
        const snapshotsOnly = loadRecentSnapshots(db, { wpId: "", maxPerType: 1, maxTotal: 3, tokenBudget: ORCHESTRATOR_MEMORY_TOKEN_BUDGET });
        if (snapshotsOnly.entries.length === 0) return [];
        db.prepare(
          `UPDATE memory_index SET access_count = access_count + 1, last_accessed_at = ?
           WHERE id IN (${snapshotsOnly.entries.map(() => "?").join(",")})`
        ).run(new Date().toISOString(), ...snapshotsOnly.entries.map(s => s.id));
        return [
          `GOVERNANCE MEMORY (${snapshotsOnly.entries.length} snapshots, ${snapshotsOnly.tokenCount} est. tokens, cross-WP):`,
          "SNAPSHOTS:",
          ...snapshotsOnly.entries.map(s => s._line),
        ];
      }

      // RGF-147: load recent pre-task snapshots (cross-WP for orchestrator)
      const snapshots = loadRecentSnapshots(db, { wpId: "", maxPerType: 1, maxTotal: 3, tokenBudget: ORCHESTRATOR_MEMORY_TOKEN_BUDGET - tokenCount });
      tokenCount += snapshots.tokenCount;
      const allEntries = [...selected, ...snapshots.entries];

      db.prepare(
        `UPDATE memory_index SET access_count = access_count + 1, last_accessed_at = ?
         WHERE id IN (${allEntries.map(() => "?").join(",")})`
      ).run(new Date().toISOString(), ...allEntries.map(s => s.id));

      const lines = [
        `GOVERNANCE MEMORY (${selected.length} entries${snapshots.entries.length > 0 ? `, ${snapshots.entries.length} snapshots` : ""}, ${tokenCount} est. tokens, cross-WP):`,
        ...selected.map(s => s._line),
      ];
      if (snapshots.entries.length > 0) {
        lines.push("SNAPSHOTS:");
        for (const s of snapshots.entries) lines.push(s._line);
      }
      lines.push(`Use \`just memory-search "<query>"\` to retrieve full content for any entry.`);
      return lines;
    } finally { try { db.close(); } catch {} }
  } catch { return []; }
}

// ---------------------------------------------------------------------------
// Conversation context injection — cross-session conversational memory
// ---------------------------------------------------------------------------

function loadConversationContext() {
  try {
    const dbPath = path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "GOVERNANCE_MEMORY.db");
    if (!fs.existsSync(dbPath)) return [];
    const db = new DatabaseSync(dbPath);
    try {
      // Check if conversation_log table exists
      const tables = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='conversation_log'").get();
      if (!tables) return [];

      const MAX_TOKENS = 600;
      const lines = [];
      let tokenCount = 0;

      // Find last closed session
      const lastClose = db.prepare(
        "SELECT session_id FROM conversation_log WHERE checkpoint_type = 'SESSION_CLOSE' ORDER BY timestamp_utc DESC LIMIT 1"
      ).get();

      let lastSessionEntries = [];
      if (lastClose) {
        lastSessionEntries = db.prepare(
          "SELECT * FROM conversation_log WHERE session_id = ? ORDER BY timestamp_utc ASC"
        ).all(lastClose.session_id);
      } else {
        // No closed session — try the second-most-recent SESSION_OPEN
        const opens = db.prepare(
          "SELECT session_id FROM conversation_log WHERE checkpoint_type = 'SESSION_OPEN' ORDER BY timestamp_utc DESC LIMIT 2"
        ).all();
        if (opens.length >= 2) {
          lastSessionEntries = db.prepare(
            "SELECT * FROM conversation_log WHERE session_id = ? ORDER BY timestamp_utc ASC"
          ).all(opens[1].session_id);
        }
      }

      if (lastSessionEntries.length > 0) {
        const sessionDate = lastSessionEntries[0].timestamp_utc?.slice(0, 10) || "unknown";
        const sessionRole = lastSessionEntries[0].role || "unknown";
        lines.push(`CONVERSATION CONTEXT (prior session ${sessionDate}, ${sessionRole}):`);
        tokenCount += 15;

        // Prioritize high-value checkpoints
        const priority = { SESSION_OPEN: 0, INSIGHT: 1, RESEARCH_CLOSE: 2, SESSION_CLOSE: 3, PRE_TASK: 4 };
        const sorted = [...lastSessionEntries].sort((a, b) =>
          (priority[a.checkpoint_type] ?? 99) - (priority[b.checkpoint_type] ?? 99)
        );

        for (const entry of sorted) {
          if (tokenCount >= MAX_TOKENS) break;
          const line = `- [${entry.checkpoint_type}] ${entry.topic}${entry.wp_id ? ` (${entry.wp_id})` : ""}`;
          const lineTokens = Math.ceil(line.length / 4);
          if (tokenCount + lineTokens > MAX_TOKENS) break;
          lines.push(line);
          tokenCount += lineTokens;

          if (entry.decisions) {
            const decLine = `  Decisions: ${entry.decisions.slice(0, 150)}`;
            const decTokens = Math.ceil(decLine.length / 4);
            if (tokenCount + decTokens <= MAX_TOKENS) {
              lines.push(decLine);
              tokenCount += decTokens;
            }
          }
        }
      }

      // Also show recent insights from older sessions (last 7 days, deduped from last session)
      const sevenDaysAgo = new Date(Date.now() - 7 * 86400000).toISOString();
      const lastSessionId = lastSessionEntries[0]?.session_id || "";
      const recentInsights = db.prepare(
        `SELECT * FROM conversation_log
         WHERE checkpoint_type = 'INSIGHT'
           AND timestamp_utc >= ?
           AND session_id != ?
         ORDER BY timestamp_utc DESC LIMIT 5`
      ).all(sevenDaysAgo, lastSessionId);

      if (recentInsights.length > 0 && tokenCount < MAX_TOKENS) {
        lines.push("RECENT INSIGHTS (past 7 days):");
        tokenCount += 8;
        for (const entry of recentInsights) {
          if (tokenCount >= MAX_TOKENS) break;
          const date = entry.timestamp_utc?.slice(0, 10) || "?";
          const line = `- [${date}] ${entry.topic}`;
          const lineTokens = Math.ceil(line.length / 4);
          if (tokenCount + lineTokens > MAX_TOKENS) break;
          lines.push(line);
          tokenCount += lineTokens;
        }
      }

      if (lines.length > 0) {
        lines.push(`Use \`just repomem log\` for full conversation history.`);
      }

      return lines;
    } finally { try { db.close(); } catch {} }
  } catch { return []; }
}

export function buildStartupInjectionLines({
  role = "",
  wpId = "",
  startupMemoryLines = null,
  conversationContextLines = null,
} = {}) {
  const resolvedStartupMemoryLines = startupMemoryLines ?? boundPromptLines(
    role === "ORCHESTRATOR"
      ? loadOrchestratorMemoryLines()
      : loadSessionMemoryLines(wpId, { role, tokenBudget: STARTUP_MEMORY_PROMPT_TOKEN_BUDGET }),
    {
      tokenBudget: STARTUP_MEMORY_PROMPT_TOKEN_BUDGET,
      maxLines: STARTUP_MEMORY_PROMPT_MAX_LINES,
    },
  );
  const resolvedConversationLines = conversationContextLines ?? boundPromptLines(
    loadConversationContext(),
    {
      tokenBudget: STARTUP_CONVERSATION_PROMPT_TOKEN_BUDGET,
      maxLines: STARTUP_CONVERSATION_PROMPT_MAX_LINES,
    },
  );

  if (resolvedStartupMemoryLines.length === 0 && resolvedConversationLines.length === 0) {
    return [];
  }

  return [
    "MEMORY INJECTION (BOUNDED): recent fail/context lines are included below to reduce repeated mistakes. Treat them as hints; packet, code, and live runtime truth win.",
    ...resolvedStartupMemoryLines,
    ...resolvedConversationLines,
  ];
}

function roleRepomemOpenCommand(role, wpId) {
  const normalizedRole = String(role || "").trim().toUpperCase();
  if (normalizedRole === "MEMORY_MANAGER") {
    return repomemCompatCommand(
      normalizedRole,
      "open",
      `Start governed Memory Manager hygiene session for ${wpId}; inspect memory quality, emit backed proposals, and close with decisions.`,
      "--role MEMORY_MANAGER",
    );
  }
  return repomemCompatCommand(
    normalizedRole,
    "open",
    `Start governed ${normalizedRole} session for ${wpId}; capture durable decisions, failures, concerns, and findings for closeout import.`,
    `--role ${normalizedRole} --wp ${wpId}`,
  );
}

export function buildStartupPrompt({
  role,
  wpId,
  roleConfig,
  selectedModel,
  selectedProfileId = "",
  selectedProfile = null,
  startupMemoryLines = null,
  conversationContextLines = null,
}) {
  const isClaudeCodeProfile = selectedProfile && selectedProfile.provider === "ANTHROPIC";
  const modelProfileLine = selectedProfileId && selectedProfile
    ? `MODEL PROFILE: ${selectedProfileId} (${selectedProfile.provider}, tool=${selectedProfile.session_tool}, runtime_support=${selectedProfile.runtime_support}, claim_model=${selectedProfile.claim_model}, reasoning=${selectedProfile.reasoning_strength}${selectedProfile.reasoning_policy_note ? `, policy=${selectedProfile.reasoning_policy_note}` : ""}).`
    : `MODEL PROFILE POLICY: ${ROLE_MODEL_PROFILE_POLICY} (legacy/default packet fields may omit explicit per-role profile ids).`;
  const repomemOpenCommand = roleRepomemOpenCommand(role, wpId);
  const commonLines = [
    `ROLE LOCK: You are the ${role}. Do not change roles unless explicitly reassigned.`,
    `WP_ID: ${wpId}`,
    `WORKTREE: ${roleConfig.worktreeDir}`,
    `BRANCH: ${roleConfig.branch}`,
    modelProfileLine,
    ...(isClaudeCodeProfile
      ? [
        `MODEL POLICY: selected ${selectedModel} with ${selectedProfile.launch_reasoning_config_key}=${selectedProfile.launch_reasoning_config_value}. This session is locked to ${selectedModel}; do not use sonnet, haiku, or any model other than opus. Do not set --model or ANTHROPIC_MODEL to anything other than ${selectedProfile.launch_model}.`,
        `REPO POLICY: this is a Claude Code governed session. Do not reference Codex model aliases or OpenAI model conventions.`,
      ]
      : [
        `MODEL POLICY: selected ${selectedModel} with ${selectedProfile?.launch_reasoning_config_key || ROLE_SESSION_REASONING_CONFIG_KEY}=${selectedProfile?.launch_reasoning_config_value || ROLE_SESSION_REASONING_CONFIG_VALUE}. Repo defaults are primary ${ROLE_SESSION_PRIMARY_MODEL} and fallback ${ROLE_SESSION_FALLBACK_MODEL}; packet-declared profiles override those defaults for this lane.`,
        `REPO POLICY: do not switch to Codex model aliases for repo-governed sessions.`,
      ]
    ),
    ...(isClaudeCodeProfile
      ? [
        `AGENT GOVERNANCE (HARD): You MAY use the Agent tool with subagent_type="Explore" or subagent_type="Plan" for read-only research, codebase inspection, and reporting. You MUST NOT delegate product code writes (Edit, Write, NotebookEdit) or governance decisions to any agent or subagent. All agent/subagent output is UNTRUSTED — you (the primary ${selectedModel} model) must independently verify any finding before it becomes truth or drives a code change. Subagents lack the reasoning strength and context depth to hold governance rules, WP topology, and signed scope simultaneously; treat their output as draft research only.`,
        `AGENT MODEL LOCK (HARD): Never configure subagents to use a model other than the governing session model. The --model flag and ANTHROPIC_MODEL env var are set at session level; do not override them in agent invocations.`,
      ]
      : [`SESSION ISOLATION: do not spawn or use helper agents/subagents inside this governed role lane.`]
    ),
    `MINIMAL LIVE READ SET (MANDATORY): After startup and assignment, work from startup output + active packet + active WP thread/notifications + .GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md when command choice is unclear.`,
    `CANONICAL_CONTEXT_DIGEST: if live authority/context feels fragmented, use ${role === "ACTIVATION_MANAGER" ? `just activation-manager next ${wpId}` : `just active-lane-brief ${role} ${wpId}`} instead of rereading ${role === "ACTIVATION_MANAGER" ? "refinement/packet/task-board/runtime" : "packet/runtime/task-board/session"} surfaces separately.`,
    `ANTI-REDISCOVERY RULE: Do not keep rereading large governance protocols, rerunning just --list, or repeating path/source-of-truth checks after context is already stable. If you need that repeated rereading, report ambiguity instead of silently paying for it.`,
    role === "MEMORY_MANAGER"
      ? `REPOMEM EXCEPTION: Memory Manager is a packetless hygiene lane, not a normal WP repomem coverage target. Use this lane's synthetic receipts and, if mutation is needed, open memory with: ${repomemOpenCommand}.`
      : `SESSION_OPEN (MANDATORY): Before any governed mutation or role-owned WP action, run ${repomemOpenCommand}. Capture decisions, failures, concerns, discoveries, and escalations with role-bound \`just repomem ... --wp ${wpId}\`; closeout imports those checkpoints mechanically into the Workflow Dossier.`,
    `POST-SIGNATURE RELAPSE GUARD (MANDATORY): For WORKFLOW_LANE=ORCHESTRATOR_MANAGED after signature/prepare, do not ask the Operator for routine approval, proceed, or checkpoint actions. If a real blocker exists, route it back to the Orchestrator and name exactly one BLOCKER_CLASS: ${ORCHESTRATOR_MANAGED_REAL_BLOCKER_CLASSES.join(", ")}.`,
  ];

  let roleLines;
  if (role === "ACTIVATION_MANAGER") {
    roleLines = [
      `AFTER STARTUP: Wait for Operator or Orchestrator instruction. Do not start downstream launch, workflow status sync, or product work without a specific task.`,
      `AUTHORITY: ${buildRoleAuthorityString(role, wpId)}`,
      `FOCUS: pre-launch governance authoring only in wt-gov-kernel on branch gov_kernel.`,
      `WORKFLOW SPLIT (MANDATORY): For \`WORKFLOW_LANE=ORCHESTRATOR_MANAGED\`, you are the mandatory governed pre-launch authoring lane and temporary worker. You must own the heavy pre-launch reasoning, hand back \`ACTIVATION_READINESS\` to the Orchestrator, and then self-close. For \`MANUAL_RELAY\`, pre-launch belongs to \`CLASSIC_ORCHESTRATOR\`; do not invent a second manual authority lane.`,
      `REFINEMENT STANDARD (HARD): your refinement and spec-enrichment work must match or exceed the old Orchestrator pre-launch quality bar. Own the full research, primitive-index, matrix, appendix, and force-multiplier follow-through instead of treating refinement as a lightweight summary.`,
      `RESEARCH APPLICABILITY RULE (HARD): for internal, repo-governed, or product-governance mirror WPs already grounded in the current Master Spec plus local product/runtime code, prefer local-spec/local-code truth first and mark external research sections NOT_APPLICABLE when honest. Never perform empty, generic, or off-topic web searches just to fill refinement headings.`,
      `CONVERGENCE RULE (HARD): once you have the core spec/runtime evidence for the assigned WP, switch into updating the named target refinement/spec artifact immediately. Do not broad-scan unrelated .GOV/refinements or .GOV/task_packets for examples. If structure help is genuinely needed, read at most 2 directly analogous artifacts, then write the target artifact.`,
      `WINDOWS EDIT LIMIT RULE (HARD): when a refinement/spec artifact already exists under a long Windows worktree path, do not attempt a monolithic whole-file rewrite that can trip os error 206. Prefer bounded in-place section edits or chunked apply_patch updates against the existing file.`,
      `BLOCKER-FIRST REPAIR RULE (HARD): when the gate reports a named blocker list for a partially filled refinement/spec artifact, repair only those blocker-named lines or sections first. Do not read excerpt-heavy tail sections, exact spec-anchor windows, or large later blocks until the gate specifically requires them.`,
      `STUB DISCOVERY RULE (HARD): when refinement, enrichment, primitive-index upkeep, or matrix expansion exposes new high-ROI items or unknown capabilities, create or update stub backlog entries instead of silently dropping them.`,
      `MODEL PROFILE RULE: Activation Manager launch defaults to the governed repo profile when packet fields are absent because this lane may run before packet hydration is complete.`,
      `COMMAND SURFACE RULE: use the canonical \`just activation-manager <action>\` surface for role-local repair/reference work and the shared refinement/signature/packet-prep commands the Orchestrator explicitly delegates. Shared implementation does not change authority ownership.`,
      `FILE-FIRST HANDOFF RULE (HARD): write the refinement/spec artifact, run the checks on that file, and return only the file path plus a compact REFINEMENT_HANDOFF_SUMMARY. Do not paste the full refinement/spec text by default.`,
      `REFINEMENT_HANDOFF_SUMMARY (HARD): include REFINEMENT_PATH, REFINEMENT_CHECK, ENRICHMENT_NEEDED, NEW_STUBS_CREATED_OR_UPDATED, NEW_FEATURES_OR_CAPABILITIES_DISCOVERED, MAJOR_TECH_UPGRADE_ADVICE, REVIEW_FOCUS, and NEXT_ORCHESTRATOR_ACTION.`,
      `REFINEMENT_CHECK RULE (HARD): REFINEMENT_CHECK must come from the real refinement checker on the written file. Placeholder scans, ASCII-only scans, and diff sanity checks do not count as pass truth by themselves.`,
      `UPGRADE DISCIPLINE (HARD): only surface MAJOR_TECH_UPGRADE_ADVICE when the refinement found a material implementation upgrade with clear ROI. Do not recommend replacing entrenched integrated technologies or techniques for marginal gains.`,
      `EXCERPT FALLBACK RULE (HARD): only if the Orchestrator explicitly requests sections or anchors should you paste excerpts back, and then only in bounded chunks. Safe default: 4 blocks.`,
      `SIGNATURE ROUND-TRIP (MANDATORY): once the refinement/spec bundle is review-ready, stop and ask the Orchestrator for operator approval evidence, the one-time signature, and the selected Coder-A..Z owner. After the Orchestrator returns that bundle, continue packet, microtask, worktree, backup, and readiness work.`,
      `PRIMARY ARTIFACT (MANDATORY): before asking the Orchestrator to continue, write or refresh \`just activation-manager readiness ${wpId} --write\` and treat the resulting \`ACTIVATION_READINESS\` block as the handoff truth.`,
      `REPAIR LOOP (MANDATORY): if the Orchestrator patches a governance bug or rejects readiness, apply only the bounded remediation requested. If the Orchestrator relaunches you fresh, accept the fresh session instead of forcing stale-context continuation.`,
      `HARD BOUNDARIES: no product code edits; no coder or validator launch or steering; no operator-approval authority; no final workflow-status truth promotion.`,
      `MANUAL-LANE GUARD: if the active workflow is manual, keep pre-launch work under the Orchestrator and use this role only when explicitly assigned for bounded repair/reference work.`,
    ];
  } else if (role === "MEMORY_MANAGER") {
    roleLines = [
      `AFTER STARTUP: Wait for Orchestrator instruction. Do not invent governance work outside the current memory-hygiene session scope.`,
      `AUTHORITY: ${buildRoleAuthorityString(role, wpId)}`,
      `FOCUS: governance memory hygiene only — report review, DB inspection, stale/contradictory memory judgment, and orchestrator-facing proposal/flag drafting.`,
      `SYNTHETIC-WP RULE: ${wpId} is a synthetic packetless governed lane. Do not expect an official packet or packet-derived runtime projection. Use the hygiene report, proposal backup files, and the synthetic WP communication ledger as the live truth surface.`,
      `RECEIPT EMISSION (MANDATORY): when you create an orchestrator-facing proposal, flag, or RGF candidate, emit the matching governed receipt and keep the markdown backup file in \`.GOV/roles/memory_manager/proposals/\`. Commands: \`just memory-manager-proposal ${wpId} <your-session> "<summary>" "<backup_ref>"\`, \`just memory-manager-flag-receipt ${wpId} <your-session> "<summary>" "<backup_ref>"\`, \`just memory-manager-rgf-candidate ${wpId} <your-session> "<summary>" "<backup_ref>"\`.`,
      `RECEIPT DISCIPLINE: the receipt summary must match the actual backup artifact you wrote. Use the backup file path as \`backup_ref\`; do not emit MEMORY_* receipts without the corresponding report/proposal evidence unless no file is honestly needed.`,
      `CLOSEOUT DISCIPLINE: when the review work is actually complete, run \`just repomem close "<session summary>" --decisions "<key decisions without shell metacharacter tricks>"\` before stopping. Completion for this lane is signaled by the governed \`SESSION_COMPLETION\` notification after your turn settles; explicit ACP \`CLOSE_SESSION\` remains orchestrator-owned.`,
      `ORCHESTRATOR VISIBILITY: MEMORY_* receipts route to ORCHESTRATOR through the synthetic WP communication lane. Use \`just check-notifications ${wpId} MEMORY_MANAGER <your-session>\` only if the Orchestrator later targets this role with follow-up guidance.`,
      `BOUNDARIES: do not edit protocols, codex, AGENTS.md, product code, or the governance task board directly.`,
    ];
  } else if (role === "CODER") {
    const startupMeshCommand = buildPhaseCheckCommand({
      phase: "STARTUP",
      wpId,
      role: "CODER",
      session: "<your-session>",
    });
    roleLines = [
      `AFTER STARTUP: Wait for Operator or Orchestrator instruction. Do not create a WP, choose a task, or start implementation without an assigned packet.`,
      `AUTHORITY: ${buildRoleAuthorityString(role, wpId)}`,
      `FOCUS: only the assigned WP in the assigned WP worktree.`,
      `GOVERNANCE NOISE RULE: the worktree .GOV tree is a live shared governance junction. Treat it as read-only context except for the assigned packet and declared MT files, which you may update for coder-owned status/evidence through the junction without committing .GOV on the feature branch. Use \`just coder-next ${wpId}\` as the filtered resume surface, and do not treat raw .GOV git noise as WP scope evidence.`,
      `FLOW: \`MANUAL_RELAY\` = \`just phase-check STARTUP ${wpId} CODER\` -> skeleton approval when required -> implementation -> \`just phase-check HANDOFF ${wpId} CODER\` -> Validator handoff. \`ORCHESTRATOR_MANAGED\` = \`just phase-check STARTUP ${wpId} CODER\` -> validator-owned bootstrap/skeleton checkpoint -> implementation with bounded overlap review -> \`just phase-check HANDOFF ${wpId} CODER\` -> Validator handoff; no routine Operator approvals after signature.`,
      `BRANCH RULE: never merge \`main\`; only use the assigned WP backup branch when the packet allows it.`,
      `DIRECT COMMUNICATION (MANDATORY): Use the structured direct-review helpers, not generic thread traffic, for the required coder <-> WP validator lane. Respond to validator kickoff with \`just wp-coder-intent ${wpId} <your-session> <validator-session> "<summary>" <correlation_id>\`, use that kickoff/intent loop for bootstrap/skeleton/data-shape review, and publish review-ready handoff with \`just wp-coder-handoff ${wpId} <your-session> <validator-session> "<summary>"\`. Use \`just wp-thread-append ${wpId} CODER <your-session> "<message>" @wpval\` only for soft coordination that is not part of the required contract.`,
      `STARTUP MESH (MANDATORY): Before the first WP-specific bootstrap or code change, run \`${startupMeshCommand}\` and do not proceed until the startup communication mesh reports ready.`,
      `EARLY GATE (HARD): After every governed \`CODER_INTENT\`, wait for explicit WP-validator clearance before implementation hardens or full \`CODER_HANDOFF\` is legal. If runtime is waiting on \`WP_VALIDATOR_INTENT_CHECKPOINT\`, keep the lane in early review instead of treating it like implementation-ready state.`,
      `MICROTASK OVERLAP (BOUNDED): For a completed narrow slice, you may open \`REVIEW_REQUEST\` to \`WP_VALIDATOR\` with \`microtask_json.review_mode=OVERLAP\` while you advance one next declared microtask, but the unresolved overlap queue is capped at 1 and full \`CODER_HANDOFF\` remains blocked until those overlap review items are drained.`,
      `CONTRACT GATE (HARD): Before claiming validator-ready handoff, \`just wp-communication-health-check ${wpId} KICKOFF\` must pass.`,
      `HANDOFF QUALITY (MANDATORY): Before requesting validation, you MUST produce a WEAK_SPOTS section listing the least-proven requirement and the riskiest file/boundary. "Done, tests pass" is not an acceptable handoff. See .GOV/roles/coder/docs/CODER_RUBRIC_V2.md (live law).`,
      `NOTIFICATIONS (MANDATORY): After startup, run \`just check-notifications ${wpId} CODER <your-session>\` to see only the notifications targeted to your governed session. After reading, run \`just ack-notifications ${wpId} CODER <your-session>\` to clear them. Check again before each handoff.`,
      `INBOX (MANDATORY): Before starting any new microtask and after completing one, check your inbox for pending obligations. If a validator STEER or rejection is pending for a prior MT, you MUST remediate that MT before starting new work. The bounded overlap limit (1 MT ahead) is hard — unresolved remediation debt blocks all new MT starts.`,
      `REMINDER: the Orchestrator remains workflow authority; only the Integration Validator can own merge-to-main authority.`,
    ];
  } else if (role === "WP_VALIDATOR") {
    const startupMeshCommand = buildPhaseCheckCommand({
      phase: "STARTUP",
      wpId,
      role: "WP_VALIDATOR",
      session: "<your-session>",
    });
    const contractGateCommand = buildPhaseCheckCommand({
      phase: "VERDICT",
      wpId,
      role: "WP_VALIDATOR",
    });
    roleLines = [
      `AFTER STARTUP: Wait for Operator or Orchestrator instruction. Do not start validation, cleanup, merge, or status sync without a specific task.`,
      `AUTHORITY: ${buildRoleAuthorityString(role, wpId)}`,
      `FOCUS: validate evidence in the assigned WP worktree, not intent.`,
      `FLOW: run the required gates, map requirements to file:line evidence, append the validation report, then report findings.`,
      `ORCHESTRATOR-MANAGED RULE: do not ask the Operator for routine approval, proceed, or checkpoint actions after signature/prepare. Route any real blocker back to the Orchestrator with one BLOCKER_CLASS from ${ORCHESTRATOR_MANAGED_REAL_BLOCKER_CLASSES.join(", ")}.`,
      `DIRECT COMMUNICATION (MANDATORY): Use the structured direct-review helpers, not generic thread traffic, for the required WP validator <-> coder lane. Publish kickoff with \`just wp-validator-kickoff ${wpId} <your-session> <coder-session> "<summary>"\`, use that kickoff to judge bootstrap/skeleton/micro-task direction early, and publish the review with \`just wp-validator-review ${wpId} <your-session> <coder-session> "<summary>" <correlation_id>\`. Use \`just wp-thread-append ${wpId} WP_VALIDATOR <your-session> "<message>" @coder\` only for soft coordination that is not part of the required contract.`,
      `STARTUP MESH (MANDATORY): Before bootstrap steering or verdict work, run \`${startupMeshCommand}\` and do not proceed until the startup communication mesh reports ready.`,
      `EARLY STEERING (MANDATORY): You own the governed bootstrap/skeleton checkpoint. After \`CODER_INTENT\`, either clear the plan, narrow it, or reject it; do not let the lane drift into hard implementation or full handoff on coder say-so alone.`,
      `MICROTASK OVERLAP (BOUNDED): While coder is the main projected actor, you may still review unresolved overlap microtask items in parallel. Keep that queue bounded to 1, reply explicitly with repair/clearance truth, and require the queue to drain before final coder handoff is accepted.`,
      `WORKTREE SYNC (MANDATORY): You share the coder \`feat/${wpId}\` branch and \`wtc-*\` worktree surface for this lane. Keep that shared review surface current instead of creating extra validator-only branches or worktrees.`,
      `CONTRACT GATE (HARD): Before PASS clearance, \`${contractGateCommand}\` must pass.`,
      `ANTI-GAMING (MANDATORY): Do not trust passing tests alone. Do not trust coder summaries alone. Build your own review target from packet scope, exact spec clauses, and diff against main. See .GOV/roles/validator/docs/VALIDATOR_ANTI_GAMING_RUBRIC.md (live law).`,
      `SPEC EVIDENCE (MANDATORY): Every PASS verdict MUST include a spec_clause_map with file:line citations for each packet requirement. You MUST identify at least one spec requirement you verified is NOT fully implemented (negative proof) to demonstrate independent code reading.`,
      `NOTIFICATIONS (MANDATORY): After startup, run \`just check-notifications ${wpId} WP_VALIDATOR <your-session>\` to see only the notifications targeted to your governed session. After reading, run \`just ack-notifications ${wpId} WP_VALIDATOR <your-session>\` to clear them. Check again before each verdict.`,
      `INBOX (MANDATORY): Before starting any new review and after completing one, check your inbox for pending obligations. If a coder remediation resubmission is pending for a prior MT, review that before starting new work. Unresolved review requests must be drained before verdict.`,
      `REMINDER: status sync is not a validation verdict.`,
    ];
  } else if (role === "INTEGRATION_VALIDATOR") {
    const startupMeshCommand = buildPhaseCheckCommand({
      phase: "STARTUP",
      wpId,
      role: "INTEGRATION_VALIDATOR",
      session: "<your-session>",
    });
    const contractGateCommand = buildPhaseCheckCommand({
      phase: "VERDICT",
      wpId,
      role: "INTEGRATION_VALIDATOR",
    });
    roleLines = [
      `AFTER STARTUP: Wait for Operator or Orchestrator instruction. Do not start validation, cleanup, merge, or status sync without a specific task.`,
      `AUTHORITY: ${buildRoleAuthorityString(role, wpId)}`,
      `FOCUS: validate evidence in the assigned WP worktree, not intent. You own final technical verdict and merge-to-main authority.`,
      `GOVERNANCE ROOT (HARD): even though you operate from handshake_main on branch main, live governance authority must resolve through ${GOV_ROOT_ENV_VAR} to wt-gov-kernel/.GOV. Do not use handshake_main/.GOV as the live source of truth for orchestrator-managed work.`,
      `KERNEL GOVERNANCE USAGE (MANDATORY): Do not manually grep, browse, or rebuild authority from handshake_main/.GOV. Use \`just integration-validator-context-brief ${wpId}\`, \`just active-lane-brief INTEGRATION_VALIDATOR ${wpId}\`, and commands that resolve governance through ${GOV_ROOT_ENV_VAR}.`,
      `FINAL-LANE STARTUP ORDER (HARD): Before any repo search, packet rediscovery, or broad .GOV inspection, complete \`${roleStartupCommand("INTEGRATION_VALIDATOR")}\` -> \`${roleNextCommand("INTEGRATION_VALIDATOR", wpId)}\` -> \`just integration-validator-context-brief ${wpId}\`, then treat the emitted \`packet_read_path\` and \`prepare_worktree_dir\` as the authoritative readable pointers.`,
      `FLOW: run the required gates, map requirements to file:line evidence, append the validation report, then close or merge validated work.`,
      `ORCHESTRATOR-MANAGED RULE: do not ask the Operator for routine approval, proceed, or checkpoint actions after signature/prepare. Route any real blocker back to the Orchestrator with one BLOCKER_CLASS from ${ORCHESTRATOR_MANAGED_REAL_BLOCKER_CLASSES.join(", ")}.`,
      `VERDICT COMMUNICATION (MANDATORY): The Integration Validator does NOT communicate directly with the Coder. Judge the complete work product against the master spec independently. On PASS: write verdict in packet, run validator-gate-append/commit, update task board, merge to main, run sync-gov-to-main. On FAIL: write a structured remediation report in the packet with specific fix instructions, then report to the Orchestrator via \`just wp-receipt-append ${wpId} INTEGRATION_VALIDATOR <your-session> STATUS "<FAIL summary with remediation instructions given>"\`. The Orchestrator handles relaunching the coder with remediation context. See .GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md for the full FAIL remediation flow.`,
      `STARTUP MESH (MANDATORY): Before entering the final review lane, run \`${startupMeshCommand}\` and do not proceed until the startup communication mesh reports ready.`,
      `FINAL-LANE CONTEXT (MANDATORY): Use \`just integration-validator-context-brief ${wpId}\` as the canonical authority/path/context bundle for this lane instead of rebuilding branch/worktree/session/main-compatibility truth manually.`,
      `CONTRACT GATE (HARD): Before PASS clearance, \`${contractGateCommand}\` must pass.`,
      `FINAL AUTHORITY (MANDATORY): Do not let WP validator evidence stand in for your own direct review. Final merge-ready authority for orchestrator-managed WPs belongs to this lane unless the packet explicitly says otherwise.`,
      `ANTI-GAMING (MANDATORY): Do not trust passing tests alone. Do not trust coder summaries alone. Do not trust WP validator summaries alone. Build your own review target from packet scope, exact spec clauses, and diff against main. See .GOV/roles/validator/docs/VALIDATOR_ANTI_GAMING_RUBRIC.md (live law).`,
      `SPEC EVIDENCE (MANDATORY): Every PASS verdict MUST include a spec_clause_map with file:line citations for each packet requirement. You MUST identify at least one spec requirement you verified is NOT fully implemented (negative proof) to demonstrate independent code reading.`,
      `NOTIFICATIONS (MANDATORY): After startup, run \`just check-notifications ${wpId} INTEGRATION_VALIDATOR <your-session>\` to see only the notifications targeted to your governed session. After reading, run \`just ack-notifications ${wpId} INTEGRATION_VALIDATOR <your-session>\` to clear them. Check again before each verdict.`,
      `REMINDER: status sync is not a validation verdict. The Orchestrator remains workflow authority; only you can own merge-to-main authority.`,
    ];
  } else {
    roleLines = [
      `AUTHORITY: ${buildRoleAuthorityString(role, wpId)}`,
      `FOCUS: ${roleConfig.focus}.`,
    ];
  }

  const startupInjectionLines = buildStartupInjectionLines({
    role,
    wpId,
    startupMemoryLines,
    conversationContextLines,
  });

  const startupCommands = [
    executableStartupCommand(role, wpId, roleConfig),
    executableNextCommand(role, wpId, roleConfig),
    ...(role === "INTEGRATION_VALIDATOR" ? [`just integration-validator-context-brief ${wpId}`] : []),
  ];

  // Inject runtime inbox state so the model sees concrete pending obligations.
  const inboxLines = [];
  if (role === "CODER" || role === "WP_VALIDATOR" || role === "INTEGRATION_VALIDATOR") {
    try {
      const runtimeStatusPath = repoPathAbs(
        `${GOV_ROOT_ABS.replace(/\\/g, "/").replace(/.*\.GOV$/i, ".GOV")}/../gov_runtime/roles_shared/WP_COMMUNICATIONS/${wpId}/RUNTIME_STATUS.json`
          .replace(/^\.GOV\/\.\.\//, "")
      );
      // Try the canonical comm path from the packet if available.
      const packetInfo = resolveWorkPacketPath(wpId);
      let runtimeStatus = null;
      if (packetInfo?.packetAbsPath && fs.existsSync(packetInfo.packetAbsPath)) {
        const packetText = fs.readFileSync(packetInfo.packetAbsPath, "utf8");
        const rtField = (packetText.match(/^\s*-\s*\**WP_RUNTIME_STATUS_FILE\**\s*:\s*(.+)/mi) || [])[1]?.trim();
        if (rtField) {
          const rtAbs = repoPathAbs(rtField);
          if (fs.existsSync(rtAbs)) {
            runtimeStatus = JSON.parse(fs.readFileSync(rtAbs, "utf8"));
          }
        }
      }
      if (runtimeStatus) {
        const inbox = buildRoleInbox(role, runtimeStatus);
        if (inbox.items.length > 0) {
          inboxLines.push(`INBOX STATE (${inbox.items.length} pending):`);
          inboxLines.push(`  NEXT_ACTION: ${inbox.next_action}`);
          for (const item of inbox.items.slice(0, 5)) {
            inboxLines.push(`  - [${item.kind}]${item.mt ? ` ${item.mt}` : ""}: ${item.summary}`);
          }
          inboxLines.push(`  You MUST address these before starting new work.`);
        }
      }
    } catch {
      // Non-fatal: inbox injection is best-effort at startup.
    }
  }

  const bootLines = [
    `Execute only this startup bootstrap now, in order, before any other work:`,
    ...startupCommands.map((command, index) => `${index + 1}. ${command}`),
    `After those commands, report only the resulting lifecycle/gate state, blockers, and next required command(s).`,
    `Do not run follow-on tests, validation, implementation, edits, or merge actions in this START_SESSION turn.`,
    `Stop after reporting and wait for a later SEND_PROMPT from the Orchestrator.`,
  ];

  return [...commonLines, ...roleLines, ...inboxLines, ...startupInjectionLines, ...bootLines].join("\n");
}

export function buildSteeringPrompt({ role, wpId, roleConfig = null }) {
  const resolvedRoleConfig = roleConfig || resolveRoleConfig(role, wpId);
  if (!resolvedRoleConfig) {
    throw new Error(`Unknown role for steering prompt: ${role}`);
  }
  const repomemOpenCommand = roleRepomemOpenCommand(role, wpId);
  const executableNext = executableNextCommand(role, wpId, resolvedRoleConfig);
  if (role === "MEMORY_MANAGER") {
    return [
      `RESUME GOVERNED ${role} lane for ${wpId}.`,
      `AUTHORITY: ${buildRoleAuthorityString(role, wpId)}`,
      `Use gov_runtime/roles_shared/MEMORY_HYGIENE_REPORT.md + .GOV/roles/memory_manager/proposals/ + synthetic WP communication files under WP_COMMUNICATIONS/${wpId} as the live truth surface. There is no official packet for this lane.`,
      `REPOMEM EXCEPTION: this synthetic hygiene lane is not a normal WP coverage target; if mutation is needed and no Memory Manager session is open, run ${repomemOpenCommand}.`,
      `Run in order:`,
      `1. ${executableNext}`,
      `2. Inspect any existing backup proposal files before drafting new MEMORY_* receipts so you do not duplicate findings.`,
      `3. Emit only the single next truthful MEMORY_PROPOSAL / MEMORY_FLAG / MEMORY_RGF_CANDIDATE receipt(s) backed by real written evidence.`,
      `4. If this steer completes the review, run \`just repomem close "<session summary>" --decisions "<key decisions>"\` and then stop. The governed control lane will emit \`SESSION_COMPLETION\` when the turn settles; do not invent your own session-retirement mechanism.`,
      `Report only maintenance findings, emitted receipt kinds, blockers, and next required command(s).`,
      `Do not request routine Operator approval or treat this like a packet-based implementation lane.`,
    ].join("\n");
  }
  const orderedCommands = role === "ACTIVATION_MANAGER"
    ? [
      resolvedRoleConfig.nextCommand,
      `just activation-manager readiness ${wpId} --write`,
    ]
    : role === "INTEGRATION_VALIDATOR"
      ? [
        `just integration-validator-context-brief ${wpId}`,
        executableNext,
        `just check-notifications ${wpId} ${role} <your-session>`,
      ]
      : [
        executableNext,
        `just check-notifications ${wpId} ${role} <your-session>`,
      ];
  return [
    `RESUME GOVERNED ${role} lane for ${wpId}.`,
    `AUTHORITY: ${buildRoleAuthorityString(role, wpId)}`,
    `SESSION_OPEN GATE: before any governed mutation in this turn, ensure the role-bound session is open with ${repomemOpenCommand}. Use \`just repomem decision|error|concern|insight ... --wp ${wpId}\` for durable run notes instead of live dossier narration.`,
    `Use ${role === "ACTIVATION_MANAGER" ? "refinement + packet (if present) + activation readiness artifact + current runtime/session projection" : "packet + active WP thread/notifications + current runtime projection"} as the live truth surface. Do not reread large governance documents if context is already stable.`,
    `If route/context feels fragmented, use ${role === "ACTIVATION_MANAGER" ? `just activation-manager next ${wpId}` : `just active-lane-brief ${role} ${wpId}`} instead of rediscovering ${role === "ACTIVATION_MANAGER" ? "refinement/packet/runtime truth" : "packet/runtime/session truth"} manually.`,
    role === "INTEGRATION_VALIDATOR"
      ? `KERNEL GOVERNANCE RULE: operate from handshake_main for product truth, but use ${GOV_ROOT_ENV_VAR} and just integration-validator-context-brief ${wpId} for live governance truth. Do not manually inspect handshake_main/.GOV as authoritative context.`
      : null,
    role === "INTEGRATION_VALIDATOR"
      ? `FIRST READ RULE: before any repo-wide search or packet rediscovery, open \`just integration-validator-context-brief ${wpId}\` and use its \`packet_read_path\` / \`prepare_worktree_dir\` output as the authoritative readable path bundle.`
      : null,
    role === "ACTIVATION_MANAGER"
      ? `WORKFLOW SPLIT (MANDATORY): in orchestrator-managed workflow you are the mandatory temporary pre-launch worker and governed pre-launch authoring lane; in manual workflow, pre-launch belongs to CLASSIC_ORCHESTRATOR. Do not convert this role into a second manual authority lane.`
      : null,
    role === "ACTIVATION_MANAGER"
      ? `WINDOWS EDIT LIMIT RULE (HARD): when updating an existing refinement/spec artifact under a long Windows worktree path, avoid monolithic whole-file rewrite attempts that can fail with os error 206. Use bounded in-place section edits or chunked apply_patch updates instead.`
      : null,
    role === "ACTIVATION_MANAGER"
      ? `BLOCKER-FIRST REPAIR RULE (HARD): when activation-manager next already named the current blockers for a partially filled refinement/spec artifact, repair only those blocker sections first and rerun the gate. Do not reread excerpt-heavy tail sections or exact spec-anchor windows until the gate actually asks for them.`
      : null,
    role === "ACTIVATION_MANAGER"
      ? `FILE-FIRST HANDOFF RULE (HARD): return the written refinement/spec file path plus a compact REFINEMENT_HANDOFF_SUMMARY. Do not paste the full refinement/spec text unless the Orchestrator explicitly requests excerpts.`
      : null,
    role === "ACTIVATION_MANAGER"
      ? `REFINEMENT_HANDOFF_SUMMARY (HARD): include REFINEMENT_PATH, REFINEMENT_CHECK, ENRICHMENT_NEEDED, NEW_STUBS_CREATED_OR_UPDATED, NEW_FEATURES_OR_CAPABILITIES_DISCOVERED, MAJOR_TECH_UPGRADE_ADVICE, REVIEW_FOCUS, and NEXT_ORCHESTRATOR_ACTION.`
      : null,
    role === "ACTIVATION_MANAGER"
      ? `REFINEMENT_CHECK RULE (HARD): REFINEMENT_CHECK must come from the real refinement checker on the written file. Placeholder scans, ASCII-only scans, and diff sanity checks do not count as pass truth by themselves.`
      : null,
    role === "ACTIVATION_MANAGER"
      ? `UPGRADE DISCIPLINE (HARD): report MAJOR_TECH_UPGRADE_ADVICE only when the refinement found a material implementation upgrade with clear ROI. Do not recommend replacing entrenched integrated technologies or techniques for marginal gains.`
      : null,
    role === "ACTIVATION_MANAGER"
      ? `EXCERPT FALLBACK RULE (HARD): if excerpts are explicitly requested, return only the requested sections or anchors in bounded chunks. Safe default: 4 blocks.`
      : null,
    role === "ACTIVATION_MANAGER"
      ? `RESEARCH APPLICABILITY RULE (HARD): when the WP is an internal or product-governance mirror change already anchored in current spec plus local code/runtime truth, keep research local-first and mark external research NOT_APPLICABLE if that is the honest answer. Do not wander into off-topic web searches.`
      : null,
    role === "ACTIVATION_MANAGER"
      ? `CONVERGENCE RULE (HARD): after you have enough local evidence for the assigned WP, update the named target refinement/spec artifact immediately. Do not broad-scan unrelated .GOV/refinements or .GOV/task_packets for examples; if structure help is truly needed, inspect at most 2 directly analogous artifacts, then write the target artifact.`
      : null,
    role === "ACTIVATION_MANAGER"
      ? `REPAIR LOOP (MANDATORY): if the Orchestrator patched governance or rejected readiness, perform only the bounded repair requested, or stop cleanly for fresh-session relaunch.`
      : null,
    `Run in order:`,
    ...orderedCommands.map((command, index) => `${index + 1}. ${command}`),
    role === "ACTIVATION_MANAGER"
      ? `${orderedCommands.length + 1}. Refresh only the single next activation artifact or repair needed to reach a truthful ACTIVATION_READINESS outcome.`
      : `${orderedCommands.length + 1}. If you consume any pending notification, acknowledge it with your actor session id using just ack-notifications ${wpId} ${role} <your-session>.`,
    `Then perform only the single next governed action implied by the active ${role === "ACTIVATION_MANAGER" ? "activation state" : "receipts/notifications and runtime projection"}.`,
    `Report only lifecycle/gate state, blockers, and next required command(s).`,
    `Do not request routine Operator approval on an orchestrator-managed lane.`,
    // Adversarial validator review prompt [RGF-99 / CX-503J]
    role === "WP_VALIDATOR"
      ? `ADVERSARIAL REVIEW [CX-503J]: After confirming code compiles and tests pass, actively try to break it. Look for race conditions, input validation gaps, error handling omissions, capability escalation paths, spec requirements the coder missed, and edge cases not covered by tests. Your job is not to confirm the code works — it is to find where it does not. Never trust self-reports.`
      : null,
    // Auto-relay communication instructions per role [CX-503C]
    role === "WP_VALIDATOR"
      ? `AUTO-RELAY: When you finish reviewing a microtask, send your response back to the coder via: just wp-review-response ${wpId} WP_VALIDATOR WP_VALIDATOR:${wpId} CODER CODER:${wpId} '<MT-NNN PASS or STEER: findings>'. This triggers the auto-relay to dispatch your response to the coder session.`
      : null,
    role === "CODER"
      ? `AUTO-RELAY: After committing each microtask, a git hook will automatically fire wp-review-request to the validator. If the hook does not fire, run manually: just wp-review-request ${wpId} CODER CODER:${wpId} WP_VALIDATOR WP_VALIDATOR:${wpId} '<MT-NNN complete: summary>'. Then STOP and wait for the validator's response.`
      : null,
  ].filter(Boolean).join("\n");
}

export function buildSessionControlRequest({
  commandId = "",
  commandKind,
  wpId,
  role,
  sessionKey,
  localBranch,
  localWorktreeDir,
  absWorktreeDir,
  selectedModel,
  selectedProfileId = "",
  prompt,
  threadId = "",
  summary = "",
  outputJsonlFile,
  environmentOverrides = null,
  targetCommandId = "",
  createdByRole = "ORCHESTRATOR",
  reasoningConfigKey = ROLE_SESSION_REASONING_CONFIG_KEY,
  reasoningConfigValue = ROLE_SESSION_REASONING_CONFIG_VALUE,
  busyIngressMode = "",
  governedAction = null,
}) {
  const COMMAND_KIND = String(commandKind || "").trim().toUpperCase();
  if (!SESSION_COMMAND_KINDS.includes(COMMAND_KIND)) {
    throw new Error(`Unknown SESSION_COMMAND kind: ${COMMAND_KIND}`);
  }
  const normalizedBusyIngressMode = String(
    busyIngressMode
    || (COMMAND_KIND === "SEND_PROMPT" ? "ENQUEUE_ON_BUSY" : "REJECT"),
  ).trim().toUpperCase();
  if (!SESSION_BUSY_INGRESS_MODES.includes(normalizedBusyIngressMode)) {
    throw new Error(`Unknown SESSION_COMMAND busy_ingress_mode: ${normalizedBusyIngressMode}`);
  }
  const nextCommandId = commandId || crypto.randomUUID();
  const createdAt = nowIso();
  const outputJsonlPath = normalizePath(outputJsonlFile);
  const governedActionRequest = buildGovernedActionRequest({
    actionId: governedAction?.action_id || nextCommandId,
    ruleId: governedAction?.rule_id || defaultGovernedActionRuleIdForSessionCommand(COMMAND_KIND, governedAction?.action_kind),
    actionKind: governedAction?.action_kind || "EXTERNAL_EXECUTE",
    commandKind: COMMAND_KIND,
    commandId: nextCommandId,
    sessionKey,
    wpId,
    role,
    createdByRole,
    targetCommandId,
    summary: governedAction?.summary ?? summary,
    reasonCode: governedAction?.reason_code || COMMAND_KIND,
    metadata: governedAction?.metadata || { output_jsonl_file: outputJsonlPath },
    requestedAt: createdAt,
  });
  return {
    schema_id: SESSION_CONTROL_REQUEST_SCHEMA_ID,
    schema_version: SESSION_CONTROL_REQUEST_SCHEMA_VERSION,
    command_id: nextCommandId,
    created_at: createdAt,
    command_kind: COMMAND_KIND,
    created_by_role: createdByRole,
    session_key: sessionKey,
    wp_id: wpId,
    role,
    session_thread_id: threadId,
    local_branch: normalizePath(localBranch),
    local_worktree_dir: normalizePath(localWorktreeDir),
    selected_model: selectedModel,
    selected_profile_id: selectedProfileId,
    reasoning_config_key: reasoningConfigKey,
    reasoning_config_value: reasoningConfigValue,
    prompt,
    summary,
    output_jsonl_file: outputJsonlPath,
    busy_ingress_mode: normalizedBusyIngressMode,
    environment_overrides: environmentOverrides && typeof environmentOverrides === "object"
      ? Object.fromEntries(
        Object.entries(environmentOverrides)
          .map(([key, value]) => [String(key || "").trim(), String(value ?? "").trim()])
          .filter(([key, value]) => key && value),
      )
      : {},
    target_command_id: targetCommandId,
    governed_action: governedActionRequest,
  };
}

export function buildSessionControlResult({
  commandId,
  commandKind,
  sessionKey,
  wpId,
  role,
  status,
  threadId = "",
  summary = "",
  outputJsonlFile = "",
  lastAgentMessage = "",
  error = "",
  durationMs = 0,
  targetCommandId = "",
  cancelStatus = "",
  outcomeState = "",
  brokerBuildId = SESSION_CONTROL_BROKER_BUILD_ID,
  governedAction = null,
}) {
  const STATUS = String(status || "").trim().toUpperCase();
  if (!SESSION_COMMAND_STATUSES.includes(STATUS)) {
    throw new Error(`Unknown SESSION_COMMAND status: ${STATUS}`);
  }
  const COMMAND_KIND = String(commandKind || "").trim().toUpperCase();
  const OUTCOME_STATE = String(outcomeState || classifySessionControlOutcomeState({
    status: STATUS,
    commandKind: COMMAND_KIND,
    error,
    summary,
    cancelStatus,
  })).trim().toUpperCase();
  if (!SESSION_COMMAND_OUTCOME_STATES.includes(OUTCOME_STATE)) {
    throw new Error(`Unknown SESSION_COMMAND outcome_state: ${OUTCOME_STATE}`);
  }
  const processedAt = nowIso();
  const outputJsonlPath = normalizePath(outputJsonlFile);
  const governedActionResult = buildGovernedActionResult({
    actionId: governedAction?.action_id || commandId,
    ruleId: governedAction?.rule_id || defaultGovernedActionRuleIdForSessionCommand(COMMAND_KIND, governedAction?.action_kind),
    actionKind: governedAction?.action_kind || "EXTERNAL_EXECUTE",
    commandKind: COMMAND_KIND,
    commandId,
    sessionKey,
    wpId,
    role,
    status: STATUS,
    outcomeState: OUTCOME_STATE,
    targetCommandId,
    summary: summary || governedAction?.summary || "",
    error,
    metadata: governedAction?.metadata || { output_jsonl_file: outputJsonlPath },
    processedAt,
  });
  return {
    schema_id: SESSION_CONTROL_RESULT_SCHEMA_ID,
    schema_version: SESSION_CONTROL_RESULT_SCHEMA_VERSION,
    command_id: commandId,
    processed_at: processedAt,
    command_kind: COMMAND_KIND,
    session_key: sessionKey,
    wp_id: wpId,
    role,
    status: STATUS,
    outcome_state: OUTCOME_STATE,
    thread_id: threadId,
    summary,
    output_jsonl_file: outputJsonlPath,
    last_agent_message: lastAgentMessage,
    error,
    duration_ms: durationMs,
    target_command_id: targetCommandId,
    cancel_status: cancelStatus,
    broker_build_id: brokerBuildId,
    governed_action: governedActionResult,
  };
}

export function classifySessionControlOutcomeState({
  status = "",
  commandKind = "",
  error = "",
  summary = "",
  cancelStatus = "",
} = {}) {
  const STATUS = String(status || "").trim().toUpperCase();
  const COMMAND_KIND = String(commandKind || "").trim().toUpperCase();
  const CANCEL_STATUS = String(cancelStatus || "").trim().toUpperCase();
  const detail = `${String(error || "").trim()} ${String(summary || "").trim()}`.trim();

  if (STATUS === "RUNNING") return "ACCEPTED_RUNNING";
  if (STATUS === "QUEUED") return "ACCEPTED_QUEUED";
  if (STATUS === "COMPLETED") {
    if (COMMAND_KIND === "START_SESSION" && /already has steerable thread|already ready/i.test(detail)) {
      return "ALREADY_READY";
    }
    return "SETTLED";
  }
  if (COMMAND_KIND === "CANCEL_SESSION" && (CANCEL_STATUS === "CANCELLATION_REQUESTED" || CANCEL_STATUS === "NOT_RUNNING")) {
    return "SETTLED";
  }
  if (COMMAND_KIND === "START_SESSION" && /already has thread|already has steerable thread|already ready/i.test(detail)) {
    return "ALREADY_READY";
  }
  if (COMMAND_KIND === "SEND_PROMPT" && /no steerable thread id is registered/i.test(detail)) {
    return "REQUIRES_START";
  }
  if (/concurrent governed run already active/i.test(detail)) {
    return "BUSY_ACTIVE_RUN";
  }
  if (
    /recovered orphaned governed request/i.test(detail)
    || /no active broker run or settled result remained/i.test(detail)
    || /broker restarted while the governed run was active/i.test(detail)
    || /broker dispatch failed/i.test(detail)
    || /build mismatch while active runs exist/i.test(detail)
    || /stale broker could not be stopped/i.test(detail)
    || /could not be stopped before restart/i.test(detail)
  ) {
    return "REQUIRES_RECOVERY";
  }
  return "FAILED";
}

export function isAcceptedSessionControlOutcomeState(outcomeState = "") {
  const normalized = String(outcomeState || "").trim().toUpperCase();
  return normalized === "ACCEPTED_RUNNING"
    || normalized === "ACCEPTED_QUEUED"
    || normalized === "ACCEPTED_PENDING";
}

export function defaultSessionOutputFile(repoRoot, sessionKey, commandId) {
  const safeSessionKey = sanitizeSessionKey(sessionKey);
  return normalizePath(path.join(SESSION_CONTROL_OUTPUT_DIR, safeSessionKey, `${commandId}.jsonl`));
}

export async function runCodexThreadCommand({
  absWorktreeDir,
  selectedModel,
  profile = null,
  prompt,
  outputFile,
  threadId = "",
  environmentOverrides = null,
  onEvent = null,
  onSpawn = null,
}) {
  const outputPath = path.resolve(outputFile);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  const outputStream = fs.createWriteStream(outputPath, { flags: "a" });
  const startedAt = Date.now();
  const cliToolPath = resolveCliTool();
  const childEnvironment = {
    ...process.env,
    ...(environmentOverrides && typeof environmentOverrides === "object" ? environmentOverrides : {}),
  };
  const reasoningConfigKey = profile?.launch_reasoning_config_key || ROLE_SESSION_REASONING_CONFIG_KEY;
  const reasoningConfigValue = profile?.launch_reasoning_config_value || ROLE_SESSION_REASONING_CONFIG_VALUE;

  const args = threadId
    ? [
      "exec",
      "resume",
      threadId,
      "--json",
      "-c",
      'sandbox_mode="danger-full-access"',
      "-m",
      selectedModel,
      "-c",
      `${reasoningConfigKey}=\"${reasoningConfigValue}\"`,
      "-",
    ]
    : [
      "exec",
      "--json",
      "-c",
      'sandbox_mode="danger-full-access"',
      "-m",
      selectedModel,
      "-c",
      `${reasoningConfigKey}=\"${reasoningConfigValue}\"`,
      "-C",
      absWorktreeDir,
      "-",
    ];

  return await new Promise((resolve) => {
    const windowsRunner = process.platform === "win32"
      ? writeWindowsPromptRunner({
        toolPath: cliToolPath,
        args,
        prompt,
        prefix: "handshake-codex-governed",
      })
      : null;
    const child = process.platform === "win32"
      ? spawn("powershell.exe", [
        "-NoLogo",
        "-NonInteractive",
        "-File",
        windowsRunner.scriptPath,
      ], {
        cwd: absWorktreeDir,
        env: childEnvironment,
        shell: false,
        windowsHide: true,
        stdio: ["ignore", "pipe", "pipe"],
      })
      : spawn(cliToolPath, args, {
        cwd: absWorktreeDir,
        env: childEnvironment,
        shell: false,
        stdio: ["pipe", "pipe", "pipe"],
      });

    if (typeof onSpawn === "function") onSpawn(child);
    if (process.platform !== "win32") {
      child.stdin.end(String(prompt || ""));
    }

    let stderr = "";
    let stdoutBuffer = "";
    let observedThreadId = threadId || "";
    let lastAgentMessage = "";
    let settled = false;

    const finish = (result) => {
      if (settled) return;
      settled = true;
      outputStream.end();
      if (windowsRunner) {
        try { fs.unlinkSync(windowsRunner.scriptPath); } catch {}
        try { fs.unlinkSync(windowsRunner.promptPath); } catch {}
      }
      resolve(result);
    };

    const handleLine = (line) => {
      if (!line) return;
      try {
        const event = JSON.parse(line);
        const normalized = { timestamp: nowIso(), ...event };
        writeJsonlEvent(outputStream, event);
        if (typeof onEvent === "function") onEvent(normalized);
        if (normalized.type === "thread.started" && normalized.thread_id) {
          observedThreadId = normalized.thread_id;
        }
        if (normalized.type === "item.completed" && normalized.item?.type === "agent_message") {
          lastAgentMessage = normalized.item.text || lastAgentMessage;
        }
      } catch {
        const rawEvent = { type: "stdout.raw", text: line };
        writeJsonlEvent(outputStream, rawEvent);
        if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...rawEvent });
      }
    };

    child.stdout.on("data", (chunk) => {
      stdoutBuffer += chunk.toString("utf8");
      const lines = stdoutBuffer.split(/\r?\n/);
      stdoutBuffer = lines.pop() || "";
      for (const line of lines) handleLine(line.trim());
    });

    child.stderr.on("data", (chunk) => {
      const text = chunk.toString("utf8");
      stderr += text;
      const event = { type: "stderr", text };
      writeJsonlEvent(outputStream, event);
      if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...event });
    });

    child.on("error", (error) => {
      stderr += error.message;
      const event = { type: "spawn.error", message: error.message };
      writeJsonlEvent(outputStream, event);
      if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...event });
      finish({
        ok: false,
        exitCode: 1,
        threadId: observedThreadId,
        lastAgentMessage,
        stderr: stderr.trim(),
        durationMs: Date.now() - startedAt,
        outputFile: outputPath,
      });
    });

    child.on("close", (code) => {
      if (stdoutBuffer.trim()) handleLine(stdoutBuffer.trim());
      const closedEvent = { type: "process.closed", exit_code: code ?? 1 };
      writeJsonlEvent(outputStream, closedEvent);
      if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...closedEvent });
      finish({
        ok: code === 0,
        exitCode: code ?? 1,
        threadId: observedThreadId,
        lastAgentMessage,
        stderr: stderr.trim(),
        durationMs: Date.now() - startedAt,
        outputFile: outputPath,
      });
    });
  });
}

export async function runClaudeCodeCommand({
  absWorktreeDir,
  selectedModel,
  profile = null,
  prompt,
  outputFile,
  sessionId = "",
  environmentOverrides = null,
  onEvent = null,
  onSpawn = null,
}) {
  const outputPath = path.resolve(outputFile);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  const outputStream = fs.createWriteStream(outputPath, { flags: "a" });
  const startedAt = Date.now();
  const cliToolPath = resolveClaudeCodeCliTool();
  const childEnvironment = {
    ...process.env,
    ...(environmentOverrides && typeof environmentOverrides === "object" ? environmentOverrides : {}),
  };
  const effort = profile?.launch_reasoning_config_value || ROLE_SESSION_REASONING_CONFIG_VALUE;

  const baseArgs = [
    "-p",
    "--model", selectedModel,
    "--effort", effort,
    "--output-format", "stream-json",
    "--verbose",
    "--dangerously-skip-permissions",
  ];

  const args = sessionId
    ? [...baseArgs, "--resume", sessionId, prompt]
    : [...baseArgs, prompt];

  return await new Promise((resolve) => {
    const child = spawn(cliToolPath, args, {
      cwd: absWorktreeDir,
      env: childEnvironment,
      shell: false,
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });

    if (typeof onSpawn === "function") onSpawn(child);

    let stderr = "";
    let stdoutBuffer = "";
    let observedSessionId = sessionId || "";
    let lastAgentMessage = "";
    let observedModelUsage = {};
    let settled = false;

    const finish = (result) => {
      if (settled) return;
      settled = true;
      outputStream.end();
      resolve(result);
    };

    const handleLine = (line) => {
      if (!line) return;
      try {
        const event = JSON.parse(line);
        const normalized = { timestamp: nowIso(), ...event };
        writeJsonlEvent(outputStream, event);
        if (typeof onEvent === "function") onEvent(normalized);

        if (normalized.type === "result") {
          if (normalized.session_id) observedSessionId = normalized.session_id;
          if (normalized.result) lastAgentMessage = normalized.result;
          if (normalized.modelUsage) observedModelUsage = normalized.modelUsage;
        }
        if (normalized.type === "assistant" && normalized.message?.content) {
          const textParts = normalized.message.content.filter((p) => p.type === "text");
          if (textParts.length > 0) lastAgentMessage = textParts.map((p) => p.text).join("\n");
        }
      } catch {
        const rawEvent = { type: "stdout.raw", text: line };
        writeJsonlEvent(outputStream, rawEvent);
        if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...rawEvent });
      }
    };

    child.stdout.on("data", (chunk) => {
      stdoutBuffer += chunk.toString("utf8");
      const lines = stdoutBuffer.split(/\r?\n/);
      stdoutBuffer = lines.pop() || "";
      for (const line of lines) handleLine(line.trim());
    });

    child.stderr.on("data", (chunk) => {
      const text = chunk.toString("utf8");
      stderr += text;
      const event = { type: "stderr", text };
      writeJsonlEvent(outputStream, event);
      if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...event });
    });

    child.on("error", (error) => {
      stderr += error.message;
      const event = { type: "spawn.error", message: error.message };
      writeJsonlEvent(outputStream, event);
      if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...event });
      finish({
        ok: false,
        exitCode: 1,
        threadId: observedSessionId,
        lastAgentMessage,
        stderr: stderr.trim(),
        durationMs: Date.now() - startedAt,
        outputFile: outputPath,
        modelUsage: observedModelUsage,
      });
    });

    child.on("close", (code) => {
      if (stdoutBuffer.trim()) handleLine(stdoutBuffer.trim());

      const modelKeys = Object.keys(observedModelUsage);
      const modelMismatch = modelKeys.length > 0 && !modelKeys.includes(selectedModel);
      if (modelMismatch) {
        const violation = `MODEL_LOCK_VIOLATION: expected only ${selectedModel} but observed ${modelKeys.join(", ")}`;
        stderr += `\n${violation}`;
        const event = { type: "model.lock.violation", expected: selectedModel, observed: modelKeys };
        writeJsonlEvent(outputStream, event);
        if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...event });
      }

      const closedEvent = { type: "process.closed", exit_code: code ?? 1 };
      writeJsonlEvent(outputStream, closedEvent);
      if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...closedEvent });
      finish({
        ok: code === 0 && !modelMismatch,
        exitCode: code ?? 1,
        threadId: observedSessionId,
        lastAgentMessage,
        stderr: stderr.trim(),
        durationMs: Date.now() - startedAt,
        outputFile: outputPath,
        modelUsage: observedModelUsage,
      });
    });
  });
}

export async function runOllamaCommand({
  absWorktreeDir,
  selectedModel,
  prompt,
  outputFile,
  environmentOverrides = null,
  onEvent = null,
  onSpawn = null,
}) {
  const outputPath = path.resolve(outputFile);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  const outputStream = fs.createWriteStream(outputPath, { flags: "a" });
  const startedAt = Date.now();
  const cliToolPath = resolveOllamaCliTool();
  const childEnvironment = {
    ...process.env,
    ...(environmentOverrides && typeof environmentOverrides === "object" ? environmentOverrides : {}),
  };

  const args = ["run", selectedModel];

  return await new Promise((resolve) => {
    const windowsRunner = process.platform === "win32"
      ? writeWindowsPromptRunner({
        toolPath: cliToolPath,
        args,
        prompt,
        prefix: "handshake-ollama-governed",
      })
      : null;
    const child = process.platform === "win32"
      ? spawn("powershell.exe", [
        "-NoLogo",
        "-NonInteractive",
        "-File",
        windowsRunner.scriptPath,
      ], {
        cwd: absWorktreeDir,
        env: childEnvironment,
        shell: false,
        windowsHide: true,
        stdio: ["ignore", "pipe", "pipe"],
      })
      : spawn(cliToolPath, args, {
        cwd: absWorktreeDir,
        env: childEnvironment,
        shell: false,
        stdio: ["pipe", "pipe", "pipe"],
      });

    if (typeof onSpawn === "function") onSpawn(child);
    if (process.platform !== "win32") {
      child.stdin.end(String(prompt || ""));
    }

    let stderr = "";
    let stdoutBuffer = "";
    let lastAgentMessage = "";
    let settled = false;

    const finish = (result) => {
      if (settled) return;
      settled = true;
      outputStream.end();
      if (windowsRunner) {
        try { fs.unlinkSync(windowsRunner.scriptPath); } catch {}
        try { fs.unlinkSync(windowsRunner.promptPath); } catch {}
      }
      resolve(result);
    };

    const handleLine = (line) => {
      if (!line) return;
      const event = { type: "ollama.output", text: line };
      writeJsonlEvent(outputStream, event);
      if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...event });
      lastAgentMessage = line;
    };

    child.stdout.on("data", (chunk) => {
      stdoutBuffer += chunk.toString("utf8");
      const lines = stdoutBuffer.split(/\r?\n/);
      stdoutBuffer = lines.pop() || "";
      for (const line of lines) handleLine(line);
    });

    child.stderr.on("data", (chunk) => {
      const text = chunk.toString("utf8");
      stderr += text;
      const event = { type: "stderr", text };
      writeJsonlEvent(outputStream, event);
      if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...event });
    });

    child.on("error", (error) => {
      stderr += error.message;
      const event = { type: "spawn.error", message: error.message };
      writeJsonlEvent(outputStream, event);
      if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...event });
      finish({
        ok: false,
        exitCode: 1,
        threadId: "",
        lastAgentMessage,
        stderr: stderr.trim(),
        durationMs: Date.now() - startedAt,
        outputFile: outputPath,
      });
    });

    child.on("close", (code) => {
      if (stdoutBuffer.trim()) handleLine(stdoutBuffer.trim());
      const closedEvent = { type: "process.closed", exit_code: code ?? 1 };
      writeJsonlEvent(outputStream, closedEvent);
      if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...closedEvent });
      finish({
        ok: code === 0,
        exitCode: code ?? 1,
        threadId: "",
        lastAgentMessage,
        stderr: stderr.trim(),
        durationMs: Date.now() - startedAt,
        outputFile: outputPath,
      });
    });
  });
}

export async function runGovernedRoleCommand({
  profile,
  absWorktreeDir,
  selectedModel,
  prompt,
  outputFile,
  threadId = "",
  environmentOverrides = null,
  onEvent = null,
  onSpawn = null,
}) {
  if (profile.provider === "ANTHROPIC") {
    return runClaudeCodeCommand({
      absWorktreeDir,
      selectedModel,
      profile,
      prompt,
      outputFile,
      sessionId: threadId,
      environmentOverrides,
      onEvent,
      onSpawn,
    });
  }
  if (profile.provider === "OLLAMA_LOCAL") {
    return runOllamaCommand({
      absWorktreeDir,
      selectedModel,
      prompt,
      outputFile,
      environmentOverrides,
      onEvent,
      onSpawn,
    });
  }
  return runCodexThreadCommand({
    absWorktreeDir,
    selectedModel,
    profile,
    prompt,
    outputFile,
    threadId,
    environmentOverrides,
    onEvent,
    onSpawn,
  });
}

export function validateSessionControlRequestShape(request) {
  const errors = [];
  const commandKind = String(request?.command_kind || "").trim().toUpperCase();
  if (!request || typeof request !== "object") return ["request must be an object"];
  if (request.schema_id !== SESSION_CONTROL_REQUEST_SCHEMA_ID) errors.push(`schema_id must be ${SESSION_CONTROL_REQUEST_SCHEMA_ID}`);
  if (request.schema_version !== SESSION_CONTROL_REQUEST_SCHEMA_VERSION) errors.push(`schema_version must be ${SESSION_CONTROL_REQUEST_SCHEMA_VERSION}`);
  if (!SESSION_COMMAND_KINDS.includes(commandKind)) errors.push("command_kind is invalid");
  if (!request.command_id) errors.push("command_id is required");
  if (request.created_by_role !== "ORCHESTRATOR") errors.push("created_by_role must be ORCHESTRATOR");
  if (!request.session_key) errors.push("session_key is required");
  if (!request.wp_id) errors.push("wp_id is required");
  if (!request.role) errors.push("role is required");
  if (!["CANCEL_SESSION", "CLOSE_SESSION"].includes(commandKind) && !request.prompt) errors.push("prompt is required");
  if (!request.output_jsonl_file) errors.push("output_jsonl_file is required");
  if (
    "busy_ingress_mode" in request
    && !SESSION_BUSY_INGRESS_MODES.includes(String(request.busy_ingress_mode || "").trim().toUpperCase())
  ) {
    errors.push(`busy_ingress_mode must be one of ${SESSION_BUSY_INGRESS_MODES.join(" | ")}`);
  }
  if ("environment_overrides" in request) {
    if (!request.environment_overrides || typeof request.environment_overrides !== "object" || Array.isArray(request.environment_overrides)) {
      errors.push("environment_overrides must be an object when present");
    } else {
      for (const [key, value] of Object.entries(request.environment_overrides)) {
        if (!String(key || "").trim()) errors.push("environment_overrides keys must be non-empty strings");
        if (!String(value ?? "").trim()) errors.push(`environment_overrides.${key} must be a non-empty string`);
      }
    }
  }
  if (commandKind === "CANCEL_SESSION" && !request.target_command_id) {
    errors.push("target_command_id is required for CANCEL_SESSION");
  }
  errors.push(...validateGovernedActionRequestShape(request.governed_action, { allowMissing: true }));
  return errors;
}

export function validateSessionControlResultShape(result) {
  const errors = [];
  const commandKind = String(result?.command_kind || "").trim().toUpperCase();
  const outcomeState = String(result?.outcome_state || "").trim().toUpperCase();
  if (!result || typeof result !== "object") return ["result must be an object"];
  if (result.schema_id !== SESSION_CONTROL_RESULT_SCHEMA_ID) errors.push(`schema_id must be ${SESSION_CONTROL_RESULT_SCHEMA_ID}`);
  if (result.schema_version !== SESSION_CONTROL_RESULT_SCHEMA_VERSION) errors.push(`schema_version must be ${SESSION_CONTROL_RESULT_SCHEMA_VERSION}`);
  if (!SESSION_COMMAND_KINDS.includes(commandKind)) errors.push("command_kind is invalid");
  if (!SESSION_COMMAND_STATUSES.includes(String(result.status || "").trim().toUpperCase())) errors.push("status is invalid");
  if ("outcome_state" in result && !SESSION_COMMAND_OUTCOME_STATES.includes(outcomeState)) errors.push("outcome_state is invalid");
  if (!result.command_id) errors.push("command_id is required");
  if (!result.session_key) errors.push("session_key is required");
  if (!result.wp_id) errors.push("wp_id is required");
  if (!result.role) errors.push("role is required");
  if ("broker_build_id" in result && !String(result.broker_build_id || "").trim()) {
    errors.push("broker_build_id must be a non-empty string when present");
  }
  if (commandKind === "CANCEL_SESSION" && !result.target_command_id) {
    errors.push("target_command_id is required for CANCEL_SESSION");
  }
  errors.push(...validateGovernedActionResultShape(result.governed_action, { allowMissing: true }));
  return errors;
}
