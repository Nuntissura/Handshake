import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import {
  GOV_ROOT_REPO_REL,
  REPO_ROOT,
  normalizePath,
  repoPathAbs,
} from "./runtime-paths.mjs";

export const GOVERNANCE_TOPOLOGY_SCHEMA_ID = "handshake.gov.governance_topology";
export const GOVERNANCE_TOPOLOGY_SCHEMA_VERSION = "governance_topology_v1";
export const GOVERNANCE_TOPOLOGY_REPO_REL_PATH = normalizePath(path.join(
  GOV_ROOT_REPO_REL,
  "roles_shared",
  "records",
  "GOVERNANCE_TOPOLOGY.json",
));

const MAINTENANCE_REQUIRED_ROLES = [
  "ORCHESTRATOR",
  "KERNEL_BUILDER",
  "CLASSIC_ORCHESTRATOR",
  "ACTIVATION_MANAGER",
  "WP_VALIDATOR",
  "INTEGRATION_VALIDATOR",
  "VALIDATOR",
  "MEMORY_MANAGER",
];

const EXCLUDED_MAINTENANCE_ROLES = ["CODER"];

const SCAN_ROOTS = [
  "justfile",
  "orcstart.cmd",
  "kbstart.cmd",
  "AGENTS.md",
  ".github",
  normalizePath(path.join(GOV_ROOT_REPO_REL, "codex")),
  normalizePath(path.join(GOV_ROOT_REPO_REL, "operator", "scripts")),
  normalizePath(path.join(GOV_ROOT_REPO_REL, "operator", "docs")),
  normalizePath(path.join(GOV_ROOT_REPO_REL, "operator", "docs_local")),
  normalizePath(path.join(GOV_ROOT_REPO_REL, "roles")),
  normalizePath(path.join(GOV_ROOT_REPO_REL, "roles_shared", "checks")),
  normalizePath(path.join(GOV_ROOT_REPO_REL, "roles_shared", "scripts")),
  normalizePath(path.join(GOV_ROOT_REPO_REL, "roles_shared", "tests")),
  normalizePath(path.join(GOV_ROOT_REPO_REL, "roles_shared", "docs")),
  normalizePath(path.join(GOV_ROOT_REPO_REL, "roles_shared", "records")),
  normalizePath(path.join(GOV_ROOT_REPO_REL, "templates")),
];

const EXCLUDED_PATH_PARTS = new Set([
  ".git",
  "node_modules",
  "target",
  "dist",
  "build",
  "runtime",
  "task_packets",
  "work_packets",
  "refinements",
  "Audits",
  "_archive",
]);

const INVENTORY_EXTENSIONS = new Set([
  ".mjs",
  ".js",
  ".cjs",
  ".cmd",
  ".ps1",
  ".sh",
  ".json",
  ".jsonl",
  ".md",
  ".yml",
  ".yaml",
]);

const CANONICAL_RECIPE_NAMES = new Set([
  "gov-check",
  "docs-check",
  "phase-check",
  "orchestrator-startup",
  "kernel-builder-startup",
  "classic-orchestrator-startup",
  "validator-startup",
  "coder-startup",
  "memory-manager-startup",
  "role-startup-topology-check",
  "activation-manager",
  "session-start",
  "session-send",
  "session-cancel",
  "session-close",
]);

const CANONICAL_SCRIPT_BASENAMES = new Set([
  "gov-check.mjs",
  "phase-check.mjs",
  "docs-check.mjs",
]);

function sha256Hex(value) {
  return crypto.createHash("sha256").update(value).digest("hex");
}

function stableStringify(value) {
  if (Array.isArray(value)) {
    return `[${value.map((entry) => stableStringify(entry)).join(",")}]`;
  }
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

function repoRelFromAbs(absPath) {
  return normalizePath(path.relative(REPO_ROOT, path.resolve(absPath)));
}

function pathParts(repoRelPath) {
  return normalizePath(repoRelPath).split("/").filter(Boolean);
}

function shouldSkipDir(repoRelPath) {
  return pathParts(repoRelPath).some((part) => EXCLUDED_PATH_PARTS.has(part));
}

function shouldInventoryFile(repoRelPath) {
  const normalized = normalizePath(repoRelPath);
  if (normalized === "justfile" || normalized === "AGENTS.md") return true;
  if (shouldSkipDir(normalized)) return false;
  return INVENTORY_EXTENSIONS.has(path.extname(normalized));
}

function walkSurfaceFiles(startRelPath) {
  const startAbs = repoPathAbs(startRelPath);
  if (!fs.existsSync(startAbs)) return [];
  const stat = fs.statSync(startAbs);
  if (stat.isFile()) {
    const relPath = repoRelFromAbs(startAbs);
    return shouldInventoryFile(relPath) ? [relPath] : [];
  }
  if (!stat.isDirectory()) return [];

  const out = [];
  const visit = (dirAbs) => {
    const dirRel = repoRelFromAbs(dirAbs);
    if (shouldSkipDir(dirRel)) return;
    for (const dirent of fs.readdirSync(dirAbs, { withFileTypes: true })) {
      const childAbs = path.join(dirAbs, dirent.name);
      const childRel = repoRelFromAbs(childAbs);
      if (dirent.isDirectory()) {
        visit(childAbs);
      } else if (dirent.isFile() && shouldInventoryFile(childRel)) {
        out.push(childRel);
      }
    }
  };
  visit(startAbs);
  return out;
}

function collectSurfaceFilePaths() {
  return [...new Set(SCAN_ROOTS.flatMap((root) => walkSurfaceFiles(root)))]
    .sort((left, right) => left.localeCompare(right));
}

function readTextIfExists(repoRelPath) {
  const absPath = repoPathAbs(repoRelPath);
  return fs.existsSync(absPath) ? fs.readFileSync(absPath, "utf8") : "";
}

function sourceHashFor(repoRelPath) {
  if (normalizePath(repoRelPath) === GOVERNANCE_TOPOLOGY_REPO_REL_PATH) {
    return null;
  }
  const absPath = repoPathAbs(repoRelPath);
  if (!fs.existsSync(absPath)) return null;
  return `sha256:${sha256Hex(fs.readFileSync(absPath))}`;
}

export function parseJustRecipes(justfileText = readTextIfExists("justfile")) {
  const lines = String(justfileText || "").replace(/\r\n/g, "\n").split("\n");
  const recipes = [];
  let active = null;

  const closeActive = () => {
    if (!active) return;
    active.body = active.bodyLines.join("\n").trimEnd();
    active.body_hash = `sha256:${sha256Hex(`${active.name}\n${active.header}\n${active.body}`)}`;
    delete active.bodyLines;
    recipes.push(active);
    active = null;
  };

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#") || trimmed.startsWith("set ") || trimmed.includes(":=")) {
      if (active && (/^\s/.test(line) || !trimmed || trimmed.startsWith("#"))) {
        active.bodyLines.push(line);
      }
      continue;
    }
    const match = line.match(/^([A-Za-z0-9_][A-Za-z0-9_-]*)(?:\s+[^:]*)?:\s*(?:#.*)?$/);
    if (match && !/^\s/.test(line)) {
      closeActive();
      active = {
        name: match[1],
        header: line.trimEnd(),
        line: index + 1,
        bodyLines: [],
      };
      continue;
    }
    if (active) {
      active.bodyLines.push(line);
    }
  }
  closeActive();
  return recipes.sort((left, right) => left.name.localeCompare(right.name));
}

function recipeReferencesPath(recipe, repoRelPath) {
  const normalized = normalizePath(repoRelPath);
  const withoutGovRoot = normalized.startsWith(`${GOV_ROOT_REPO_REL}/`)
    ? normalized.slice(`${GOV_ROOT_REPO_REL}/`.length)
    : normalized;
  const body = normalizePath(`${recipe.header}\n${recipe.body || ""}`);
  return body.includes(normalized)
    || body.includes(withoutGovRoot)
    || body.includes(`{{GOV_ROOT}}/${withoutGovRoot}`)
    || body.includes(`"{{GOV_ROOT}}/${withoutGovRoot}"`);
}

function recipeNamesByFile(recipes, filePaths) {
  const byPath = new Map(filePaths.map((filePath) => [filePath, []]));
  for (const recipe of recipes) {
    for (const filePath of filePaths) {
      if (recipeReferencesPath(recipe, filePath)) {
        byPath.get(filePath).push(recipe.name);
      }
    }
  }
  for (const names of byPath.values()) {
    names.sort((left, right) => left.localeCompare(right));
  }
  return byPath;
}

function ownerRoleForPath(repoRelPath) {
  const parts = pathParts(repoRelPath);
  const rolesIndex = parts.indexOf("roles");
  if (rolesIndex >= 0 && parts[rolesIndex + 1]) {
    const rolePart = parts[rolesIndex + 1].toUpperCase();
    if (rolePart === "README.MD") return "SHARED";
    return rolePart;
  }
  if (parts.includes("roles_shared")) return "SHARED";
  if (parts.includes("operator")) return "OPERATOR";
  if (parts.includes("codex")) return "CODEX";
  if (parts.includes(".github")) return "GITHUB";
  if (repoRelPath === "justfile") return "SHARED";
  if (repoRelPath === "AGENTS.md") return "AGENTS";
  return "SHARED";
}

function authorityBoundaryForPath(repoRelPath) {
  const owner = ownerRoleForPath(repoRelPath);
  if (owner === "SHARED") return "REPO_GOVERNANCE_SHARED";
  if (owner === "CODEX" || owner === "AGENTS") return "ROOT_AGENT_CONTRACT";
  if (owner === "GITHUB") return "GITHUB_AUTOMATION";
  if (owner === "OPERATOR") return "OPERATOR_INTERFACE";
  return `ROLE_${owner}`;
}

function phaseForText(value = "") {
  const text = String(value || "").toLowerCase();
  if (text.includes("startup") || text.includes("preflight") || text.includes("role-startup")) return "STARTUP";
  if (text.includes("handoff")) return "HANDOFF";
  if (text.includes("verdict") || text.includes("validation") || text.includes("validator")) return "VALIDATION";
  if (text.includes("closeout") || text.includes("merge")) return "CLOSEOUT";
  if (text.includes("session") || text.includes("broker") || text.includes("launch")) return "SESSION_CONTROL";
  if (text.includes("topology") || text.includes("worktree") || text.includes("backup") || text.includes("sync")) return "TOPOLOGY";
  if (text.includes("/wp/") || text.includes("wp-") || text.includes("packet") || text.includes("mt-")) return "WORK_PACKET";
  if (text.includes("memory") || text.includes("repomem")) return "MEMORY";
  if (text.includes("audit") || text.includes("dossier") || text.includes("smoketest")) return "AUDIT";
  if (text.includes("spec")) return "SPEC";
  if (text.includes("check")) return "CHECK";
  return "GENERAL";
}

function surfaceKindForFile(repoRelPath) {
  const normalized = normalizePath(repoRelPath);
  const ext = path.extname(normalized).toLowerCase();
  const base = path.basename(normalized);
  if (normalized === "justfile") return "JUSTFILE";
  if (base === "AGENTS.md") return "AGENTS_CONTRACT";
  if (["orcstart.cmd", "kbstart.cmd"].includes(normalized)) return "GOVERNANCE_SCRIPT";
  if (/PROTOCOL\.md$/i.test(base) || /\/agentic\/AGENTIC_PROTOCOL\.md$/i.test(normalized)) return "ROLE_PROTOCOL";
  if (normalized.includes("/checks/") && ext === ".mjs") return "GOVERNANCE_CHECK";
  if (normalized.includes("/tests/") && ext === ".mjs") return "GOVERNANCE_TEST";
  if (normalized.includes("/scripts/") && [".mjs", ".js", ".cjs", ".cmd", ".ps1", ".sh"].includes(ext)) return "GOVERNANCE_SCRIPT";
  if (normalized.includes("/lib/") && [".mjs", ".js", ".cjs"].includes(ext)) return "GOVERNANCE_LIBRARY";
  if (normalized.includes("/templates/")) return "GOVERNANCE_TEMPLATE";
  if (normalized.includes("/records/") && [".json", ".jsonl"].includes(ext)) return "MACHINE_READABLE_RECORD";
  if (normalized.includes("/records/")) return "GOVERNANCE_RECORD";
  if (normalized.includes("/docs") || ext === ".md") return "GOVERNANCE_DOC";
  if ([".json", ".jsonl", ".yml", ".yaml"].includes(ext)) return "MACHINE_READABLE_CONTRACT";
  return "GOVERNANCE_FILE";
}

function sideEffectClassForText(value = "", kind = "") {
  const text = String(value || "").toLowerCase();
  if (kind.includes("CHECK") || kind.includes("TEST") || text.includes("check") || text.includes("status") || text.includes("report") || text.includes("list") || text.includes("resolve") || text.includes("scan") || text.includes("search") || text.includes("brief") || text.includes("next")) {
    return "READ_ONLY_OR_DIAGNOSTIC";
  }
  if (text.includes("delete") || text.includes("cleanup") || text.includes("reset") || text.includes("retire") || text.includes("worktree") || text.includes("git push") || text.includes("git commit") || text.includes("sync-gov-to-main")) {
    return "MUTATES_GIT_OR_WORKTREE";
  }
  if (text.includes("append") || text.includes("record") || text.includes("create") || text.includes("generate") || text.includes("install") || text.includes("launch") || text.includes("start") || text.includes("cancel") || text.includes("close") || text.includes("capture") || text.includes("compact") || text.includes("migrate") || text.includes("sync")) {
    return "MUTATES_GOVERNANCE_OR_RUNTIME";
  }
  return "READ_ONLY_OR_DIAGNOSTIC";
}

function replacementBundleFor({ repoRelPath = "", recipe = null, phase = "", kind = "" } = {}) {
  const key = String(recipe?.name || repoRelPath || "").toLowerCase();
  if (recipe?.name === "gov-check" || key.includes("gov-check")) return "just gov-check";
  if (recipe?.name === "phase-check" || key.includes("phase-check")) return "just phase-check";
  if (phase === "STARTUP") return "role startup recipes plus just gov-check";
  if (["HANDOFF", "VALIDATION", "CLOSEOUT", "WORK_PACKET", "SESSION_CONTROL"].includes(phase)) return "just phase-check";
  if (phase === "TOPOLOGY" || kind.includes("CHECK")) return "just gov-check";
  if (phase === "MEMORY") return "just memory-refresh plus just gov-check";
  return "just gov-check";
}

function publicExposureForFile({ repoRelPath, recipeNames = [] } = {}) {
  if (repoRelPath === "justfile") return "PUBLIC_JUSTFILE";
  if (recipeNames.length > 0) return "PUBLIC_VIA_JUST_RECIPE";
  if (surfaceKindForFile(repoRelPath).includes("CHECK")) return "GOV_CHECK_SUBSTEP_OR_DIRECT_CHECK";
  return "INTERNAL_PATH";
}

function entrypointStatusForFile({ repoRelPath, kind, recipeNames = [] } = {}) {
  const base = path.basename(repoRelPath);
  if (CANONICAL_SCRIPT_BASENAMES.has(base)) return "CANONICAL_BUNDLE_OR_CHECKPOINT";
  if (/bundle-check\.mjs$/i.test(base)) return "CANONICAL_SUB_BUNDLE";
  if (kind === "GOVERNANCE_LIBRARY") return "INTERNAL_HELPER";
  if (kind === "GOVERNANCE_TEST") return "INTERNAL_TEST";
  if (kind === "ROLE_PROTOCOL" || kind === "AGENTS_CONTRACT") return "CONTRACT_AUTHORITY";
  if (repoRelPath === GOVERNANCE_TOPOLOGY_REPO_REL_PATH) return "MACHINE_PROJECTION";
  if (recipeNames.length > 0) return "PUBLIC_LEAF_BASELINED";
  return "INTERNAL_HELPER";
}

export function justCallTargetsForRecipe(recipe) {
  return [...String(recipe?.body || "").matchAll(/^\s*@?just\s+([A-Za-z0-9_][A-Za-z0-9_-]*)\b/gm)]
    .map((match) => match[1])
    .filter(Boolean);
}

export function justCallInvocationsForRecipe(recipe) {
  return String(recipe?.body || "")
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => /^@?just\s+[A-Za-z0-9_][A-Za-z0-9_-]*\b/u.test(line))
    .map((line) => line.replace(/^@?just\s+/u, "just "));
}

export function isSimpleJustCompatibilityAlias(recipe) {
  const meaningfulLines = String(recipe?.body || "")
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"));
  return meaningfulLines.length === 1
    && /^@?just\s+[A-Za-z0-9_][A-Za-z0-9_-]*\b/u.test(meaningfulLines[0]);
}

function entrypointStatusForRecipe(recipe) {
  if (CANONICAL_RECIPE_NAMES.has(recipe.name)) return "CANONICAL_PUBLIC_ENTRY";
  if (isSimpleJustCompatibilityAlias(recipe)) return "COMPATIBILITY_ALIAS";
  return "PUBLIC_RECIPE_BASELINED";
}

function debugArtifactFor({ phase = "", kind = "" } = {}) {
  if (phase === "WORK_PACKET" || phase === "SESSION_CONTROL" || phase === "CLOSEOUT" || phase === "HANDOFF" || phase === "VALIDATION") {
    return "gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-{ID}/check_details.jsonl";
  }
  if (kind.includes("CHECK") || kind.includes("TEST")) {
    return "gov_runtime/check_details.jsonl";
  }
  return "gov_runtime/roles_shared/failure_dossiers/phase_bundle_failures.jsonl";
}

function buildFileSurface(repoRelPath, recipeNames = []) {
  const kind = surfaceKindForFile(repoRelPath);
  const phase = phaseForText(repoRelPath);
  return {
    surface_id: `file:${repoRelPath}`,
    path: repoRelPath,
    surface_kind: kind,
    owner_role: ownerRoleForPath(repoRelPath),
    authority_boundary: authorityBoundaryForPath(repoRelPath),
    phase,
    side_effect_class: sideEffectClassForText(repoRelPath, kind),
    public_exposure: publicExposureForFile({ repoRelPath, recipeNames }),
    just_recipes: recipeNames,
    entrypoint_status: entrypointStatusForFile({ repoRelPath, kind, recipeNames }),
    replacement_bundle: replacementBundleFor({ repoRelPath, phase, kind }),
    primary_debug_artifact: debugArtifactFor({ phase, kind }),
    contract_authority: repoRelPath === GOVERNANCE_TOPOLOGY_REPO_REL_PATH ? "CX-912" : "CX-912_TOPOLOGY_INVENTORY",
    validation_coverage: kind.includes("CHECK") || repoRelPath === GOVERNANCE_TOPOLOGY_REPO_REL_PATH
      ? ["just gov-check", "governance-topology-check"]
      : ["governance-topology-check"],
    source_hash: sourceHashFor(repoRelPath),
    last_verified_at_utc: null,
  };
}

function buildRecipeSurface(recipe) {
  const kind = "JUST_RECIPE";
  const phase = phaseForText(`${recipe.name}\n${recipe.body || ""}`);
  return {
    surface_id: `just:${recipe.name}`,
    path: `justfile#${recipe.name}`,
    surface_kind: kind,
    owner_role: "SHARED",
    authority_boundary: "PUBLIC_JUST_SURFACE",
    phase,
    side_effect_class: sideEffectClassForText(`${recipe.name}\n${recipe.body || ""}`, kind),
    public_exposure: "PUBLIC_JUST_RECIPE",
    just_recipes: [recipe.name],
    entrypoint_status: entrypointStatusForRecipe(recipe),
    replacement_bundle: replacementBundleFor({ recipe, phase, kind }),
    primary_debug_artifact: debugArtifactFor({ phase, kind }),
    contract_authority: "CX-912_PUBLIC_SURFACE_CONTRACT",
    validation_coverage: ["governance-topology-check"],
    source_hash: recipe.body_hash,
    last_verified_at_utc: null,
  };
}

function countBy(items, keyFn) {
  const counts = {};
  for (const item of items) {
    const key = keyFn(item);
    counts[key] = (counts[key] || 0) + 1;
  }
  return Object.fromEntries(Object.entries(counts).sort(([left], [right]) => left.localeCompare(right)));
}

function buildPublicLeafGroups(surfaces) {
  const groups = new Map();
  for (const surface of surfaces) {
    if (!String(surface.entrypoint_status || "").includes("BASELINED")) continue;
    const key = [
      surface.owner_role,
      surface.phase,
      surface.side_effect_class,
      surface.replacement_bundle,
    ].join("|");
    if (!groups.has(key)) {
      groups.set(key, {
        owner_role: surface.owner_role,
        phase: surface.phase,
        side_effect_class: surface.side_effect_class,
        replacement_bundle: surface.replacement_bundle,
        policy_status: "CONSOLIDATE_OR_JUSTIFY_WHEN_TOUCHED",
        surfaces: [],
      });
    }
    groups.get(key).surfaces.push(surface.surface_id);
  }
  return [...groups.values()]
    .map((group) => ({
      ...group,
      surfaces: group.surfaces.sort((left, right) => left.localeCompare(right)),
      count: group.surfaces.length,
    }))
    .sort((left, right) =>
      left.owner_role.localeCompare(right.owner_role)
      || left.phase.localeCompare(right.phase)
      || left.side_effect_class.localeCompare(right.side_effect_class)
      || left.replacement_bundle.localeCompare(right.replacement_bundle)
    );
}

function isBlank(value) {
  return value === undefined
    || value === null
    || String(value).trim() === ""
    || (Array.isArray(value) && value.length === 0);
}

function hasSha256(value) {
  return /^sha256:[0-9a-f]{64}$/u.test(String(value || ""));
}

function hasTopologySafeSourceHash(surface) {
  if (surface.path === GOVERNANCE_TOPOLOGY_REPO_REL_PATH && surface.source_hash === null) return true;
  return hasSha256(surface.source_hash);
}

function isPublicSurface(surface) {
  return String(surface.public_exposure || "").includes("PUBLIC")
    || String(surface.entrypoint_status || "").includes("PUBLIC")
    || String(surface.entrypoint_status || "").includes("COMPATIBILITY");
}

export function validateGovernanceTopologyInventory(topology = buildGovernanceTopology()) {
  const errors = [];
  if (!topology || typeof topology !== "object" || Array.isArray(topology)) {
    return ["governance topology must be an object"];
  }
  if (!Array.isArray(topology.surfaces)) {
    return ["governance topology surfaces must be an array"];
  }

  const requiredFields = [
    "surface_id",
    "path",
    "surface_kind",
    "owner_role",
    "authority_boundary",
    "phase",
    "side_effect_class",
    "public_exposure",
    "just_recipes",
    "entrypoint_status",
    "replacement_bundle",
    "primary_debug_artifact",
    "contract_authority",
    "validation_coverage",
    "source_hash",
  ];
  const seenSurfaceIds = new Set();
  const seenPaths = new Map();
  const scriptLikeKinds = new Set([
    "GOVERNANCE_SCRIPT",
    "GOVERNANCE_LIBRARY",
    "GOVERNANCE_CHECK",
    "GOVERNANCE_TEST",
  ]);

  for (const surface of topology.surfaces) {
    const label = surface?.surface_id || surface?.path || "<surface>";
    if (!surface || typeof surface !== "object" || Array.isArray(surface)) {
      errors.push(`${label}: surface must be an object`);
      continue;
    }
    for (const field of requiredFields) {
      if (field === "source_hash" && surface.path === GOVERNANCE_TOPOLOGY_REPO_REL_PATH && surface.source_hash === null) continue;
      if (field === "just_recipes") {
        if (!Array.isArray(surface.just_recipes)) errors.push(`${label}: just_recipes must be an array`);
        continue;
      }
      if (isBlank(surface[field])) errors.push(`${label}: missing required topology field ${field}`);
    }
    if (seenSurfaceIds.has(surface.surface_id)) {
      errors.push(`${label}: duplicate surface_id`);
    }
    seenSurfaceIds.add(surface.surface_id);

    if (String(surface.surface_id || "").startsWith("file:")) {
      if (seenPaths.has(surface.path)) {
        errors.push(`${label}: duplicate file path with ${seenPaths.get(surface.path)}`);
      }
      seenPaths.set(surface.path, label);
      if (!fs.existsSync(repoPathAbs(surface.path))) {
        errors.push(`${label}: file path missing on disk (${surface.path})`);
      }
    }

    if (String(surface.owner_role || "").includes(".")) {
      errors.push(`${label}: owner_role must be a role/authority token, found ${surface.owner_role}`);
    }
    if (["UNKNOWN", "<missing>"].includes(String(surface.owner_role || "").toUpperCase())) {
      errors.push(`${label}: owner_role must be classified`);
    }
    if (["UNKNOWN", "<missing>"].includes(String(surface.phase || "").toUpperCase())) {
      errors.push(`${label}: phase must be classified`);
    }
    if (!hasTopologySafeSourceHash(surface)) {
      errors.push(`${label}: source_hash must be sha256:<64hex> except the topology self-row null policy`);
    }
    if (!Array.isArray(surface.validation_coverage) || surface.validation_coverage.length === 0) {
      errors.push(`${label}: validation_coverage must be a non-empty array`);
    }
    if (!Array.isArray(surface.just_recipes)) {
      errors.push(`${label}: just_recipes must be an array`);
    }
    if (surface.public_exposure === "PUBLIC_VIA_JUST_RECIPE" && (!Array.isArray(surface.just_recipes) || surface.just_recipes.length === 0)) {
      errors.push(`${label}: PUBLIC_VIA_JUST_RECIPE requires at least one just recipe`);
    }
    if (surface.surface_kind === "JUST_RECIPE" && !String(surface.path || "").startsWith("justfile#")) {
      errors.push(`${label}: JUST_RECIPE path must be justfile#<recipe>`);
    }
    if (isPublicSurface(surface) && isBlank(surface.replacement_bundle)) {
      errors.push(`${label}: public surfaces require replacement_bundle metadata`);
    }
    if (scriptLikeKinds.has(surface.surface_kind)) {
      for (const field of ["owner_role", "phase", "side_effect_class", "primary_debug_artifact", "entrypoint_status"]) {
        if (isBlank(surface[field])) errors.push(`${label}: script inventory metadata missing ${field}`);
      }
    }
  }

  const surfaceCount = topology.surfaces.length;
  const summaryCount = Number(topology.public_surface_summary?.total_surfaces ?? -1);
  if (summaryCount !== surfaceCount) {
    errors.push(`public_surface_summary.total_surfaces drift (expected ${surfaceCount}, found ${summaryCount})`);
  }

  return [...new Set(errors)];
}

export function buildGovernanceTopology() {
  const filePaths = collectSurfaceFilePaths();
  const recipes = parseJustRecipes();
  const recipesByFile = recipeNamesByFile(recipes, filePaths);
  const fileSurfaces = filePaths.map((filePath) => buildFileSurface(filePath, recipesByFile.get(filePath) || []));
  const recipeSurfaces = recipes.map((recipe) => buildRecipeSurface(recipe));
  const surfaces = [...fileSurfaces, ...recipeSurfaces]
    .sort((left, right) => left.surface_id.localeCompare(right.surface_id));

  const phaseBundles = [
    {
      bundle_id: "gov-check",
      runner: "just gov-check",
      implementation: normalizePath(path.join(GOV_ROOT_REPO_REL, "roles_shared", "checks", "gov-check.mjs")),
      phases: ["SPEC", "WORK_PACKET", "SESSION_CONTROL", "TOPOLOGY", "MEMORY", "GOVERNANCE_STRUCTURE"],
      supports_flags: ["--list", "--dry-run", "--json", "--sync-topology"],
      failure_dossier: "gov_runtime/roles_shared/failure_dossiers/phase_bundle_failures.jsonl",
    },
    {
      bundle_id: "phase-check",
      runner: "just phase-check <PHASE> WP-{ID} [ROLE] [SESSION]",
      implementation: normalizePath(path.join(GOV_ROOT_REPO_REL, "roles_shared", "checks", "phase-check.mjs")),
      phases: ["STARTUP", "HANDOFF", "VERDICT", "CLOSEOUT"],
      supports_flags: ["--verbose", "--range", "--rev", "--sync-mode"],
      failure_dossier: "gov_runtime/roles_shared/failure_dossiers/phase_bundle_failures.jsonl",
    },
  ];

  const publicSurfaceSummary = {
    total_surfaces: surfaces.length,
    counts_by_kind: countBy(surfaces, (surface) => surface.surface_kind),
    counts_by_owner_role: countBy(surfaces, (surface) => surface.owner_role),
    counts_by_entrypoint_status: countBy(surfaces, (surface) => surface.entrypoint_status),
    public_recipe_count: recipeSurfaces.length,
    public_leaf_groups: buildPublicLeafGroups(surfaces),
  };

  const topology = {
    schema_id: GOVERNANCE_TOPOLOGY_SCHEMA_ID,
    schema_version: GOVERNANCE_TOPOLOGY_SCHEMA_VERSION,
    status: "GENERATED_DETERMINISTIC_BASELINE",
    authority: {
      source_codex_clause: "CX-912",
      implementation_owner: "ORCHESTRATOR",
      maintenance_required_roles: MAINTENANCE_REQUIRED_ROLES,
      excluded_roles: EXCLUDED_MAINTENANCE_ROLES,
    },
    update_contract: {
      direct_update_required_when: [
        "adding a governance script, check, test, public Just recipe, workflow artifact, role protocol, phase bundle, topology surface, session/runtime authority surface, or machine-readable governance contract",
        "renaming or relocating a governance surface",
        "retiring a leaf script or compatibility wrapper",
        "changing the owner, phase, side-effect class, debug artifact, or public exposure of a governance surface",
      ],
      proposal_required_when_direct_write_is_forbidden: true,
      proposal_fields: [
        "surface_path",
        "surface_kind",
        "owner_role",
        "phase",
        "side_effect_class",
        "public_exposure",
        "replacement_bundle",
        "primary_debug_artifact",
        "reason",
      ],
    },
    projection_contract: {
      deterministic: true,
      self_path: GOVERNANCE_TOPOLOGY_REPO_REL_PATH,
      self_source_hash_policy: "null_to_avoid_self-referential_projection_hash",
      check: "governance-topology-check",
      sync_command: "just gov-check --sync-topology",
    },
    script_inventory_reconciliation_contract: {
      rgf_id: "RGF-298",
      check: "governance-topology-check",
      enforced_fields: [
        "surface_id",
        "path",
        "surface_kind",
        "owner_role",
        "authority_boundary",
        "phase",
        "side_effect_class",
        "public_exposure",
        "entrypoint_status",
        "replacement_bundle",
        "primary_debug_artifact",
        "contract_authority",
        "validation_coverage",
        "source_hash",
      ],
      coverage_scope: [
        "governance scripts",
        "governance checks",
        "governance tests",
        "governance libraries",
        "public Just recipes",
        "role protocols",
        "machine-readable records",
      ],
      fail_closed_on: [
        "duplicate surface ids",
        "duplicate file rows",
        "missing files",
        "missing required metadata",
        "unclassified owner roles",
        "invalid source hashes",
        "public recipe exposure without recipe linkage",
      ],
    },
    phase_checkpoint_bundles: phaseBundles,
    leaf_script_sunset_policy: {
      default_status_for_existing_public_leaves: "PUBLIC_LEAF_BASELINED",
      when_touched: "replace with canonical bundle, convert to internal helper, or record explicit compatibility justification in this ledger",
      compatibility_alias_status: "COMPATIBILITY_ALIAS",
      removal_gate: "must remain represented until replacement bundle is tracked and usable",
    },
    failure_dossier_contract: {
      schema_id: "handshake.gov.phase_bundle_failure_dossier",
      schema_version: "phase_bundle_failure_dossier_v1",
      jsonl_path: "gov_runtime/roles_shared/failure_dossiers/phase_bundle_failures.jsonl",
      markdown_projection_path: "gov_runtime/roles_shared/failure_dossiers/phase_bundle_failures.md",
      wp_dossier_root_template: "gov_runtime/roles_shared/WP_DOSSIERS/WP-{ID}/",
      wp_dossier_index: "gov_runtime/roles_shared/WP_DOSSIERS/WP-{ID}/index.json",
      wp_dossier_events: "gov_runtime/roles_shared/WP_DOSSIERS/WP-{ID}/events.jsonl",
      wp_dossier_artifact_manifest: "gov_runtime/roles_shared/WP_DOSSIERS/WP-{ID}/artifact_manifest.json",
      wp_dossier_workflow_postmortem: "gov_runtime/roles_shared/WP_DOSSIERS/WP-{ID}/workflow_postmortem.md",
      raw_archive_policy: "dump_all_logs_for_posterity",
      model_entrypoint_policy: "models/tools read index.json first, then follow artifact refs to raw logs when needed",
      required_fields: [
        "run_id",
        "timestamp",
        "wp_id",
        "phase",
        "bundle",
        "substep_id",
        "command",
        "owner_role",
        "side_effect_class",
        "cwd",
        "env_summary",
        "exit_code",
        "duration_ms",
        "stdout_artifact",
        "stderr_artifact",
        "artifact_refs",
        "wp_dossier_root",
        "wp_dossier_index",
        "workflow_postmortem_ref",
        "debug_artifact",
        "invariant",
        "suspected_cause_category",
        "remediation_hint",
        "related_topology_rows",
        "topology_row_ids",
        "memory_capture_status",
      ],
    },
    implementation_plan: [
      {
        rgf_id: "RGF-291",
        title: "Script Surface Inventory and Public Entry Contract",
        status: "IMPLEMENTED",
        deliverable: "deterministic topology ledger projection plus gov-check enforcement",
      },
      {
        rgf_id: "RGF-292",
        title: "Phase Checkpoint Bundle Runner",
        status: "IMPLEMENTED",
        deliverable: "gov-check exposes list/dry-run/json/sync topology flags and records bundle substep metadata",
      },
      {
        rgf_id: "RGF-293",
        title: "Justfile Public Surface Consolidation",
        status: "IMPLEMENTED_BASELINE",
        deliverable: "all public Just recipes are inventoried and classified for canonical, alias, or baselined leaf handling",
      },
      {
        rgf_id: "RGF-294",
        title: "Leaf Script Sunset/Compatibility Wrapper Policy",
        status: "IMPLEMENTED_BASELINE",
        deliverable: "baselined leaves have replacement bundles and must be consolidated or justified when touched",
      },
      {
        rgf_id: "RGF-295",
        title: "Structured Failure Dossier for Phase Bundles",
        status: "IMPLEMENTED",
        deliverable: "phase bundle failures append JSONL dossier entries with markdown projection and output artifacts",
      },
    ],
    public_surface_summary: publicSurfaceSummary,
    surfaces,
  };

  topology.projection_hash = `sha256:${sha256Hex(stableStringify({
    schema_version: topology.schema_version,
    status: topology.status,
    authority: topology.authority,
    update_contract: topology.update_contract,
    projection_contract: topology.projection_contract,
    script_inventory_reconciliation_contract: topology.script_inventory_reconciliation_contract,
    phase_checkpoint_bundles: topology.phase_checkpoint_bundles,
    leaf_script_sunset_policy: topology.leaf_script_sunset_policy,
    failure_dossier_contract: topology.failure_dossier_contract,
    implementation_plan: topology.implementation_plan,
    public_surface_summary: topology.public_surface_summary,
    surfaces: topology.surfaces,
  }))}`;

  return topology;
}

export function renderGovernanceTopologyJson(topology = buildGovernanceTopology()) {
  return `${JSON.stringify(topology, null, 2)}\n`;
}

export function readGovernanceTopology() {
  const absPath = repoPathAbs(GOVERNANCE_TOPOLOGY_REPO_REL_PATH);
  if (!fs.existsSync(absPath)) return null;
  return JSON.parse(fs.readFileSync(absPath, "utf8"));
}

export function writeGovernanceTopology(topology = buildGovernanceTopology()) {
  const absPath = repoPathAbs(GOVERNANCE_TOPOLOGY_REPO_REL_PATH);
  fs.mkdirSync(path.dirname(absPath), { recursive: true });
  fs.writeFileSync(absPath, renderGovernanceTopologyJson(topology), "utf8");
  return absPath;
}

export function compareGovernanceTopologyProjection(existing = readGovernanceTopology(), expected = buildGovernanceTopology()) {
  if (!existing) {
    return {
      ok: false,
      errors: [`Missing ${GOVERNANCE_TOPOLOGY_REPO_REL_PATH}`],
      expected,
    };
  }
  const existingText = `${JSON.stringify(existing, null, 2)}\n`;
  const expectedText = renderGovernanceTopologyJson(expected);
  const errors = [];
  if (existing.schema_id !== GOVERNANCE_TOPOLOGY_SCHEMA_ID) {
    errors.push(`schema_id must be ${GOVERNANCE_TOPOLOGY_SCHEMA_ID}`);
  }
  if (existing.schema_version !== GOVERNANCE_TOPOLOGY_SCHEMA_VERSION) {
    errors.push(`schema_version must be ${GOVERNANCE_TOPOLOGY_SCHEMA_VERSION}`);
  }
  if (existing.projection_hash !== expected.projection_hash) {
    errors.push(`${GOVERNANCE_TOPOLOGY_REPO_REL_PATH} projection_hash is stale; run just gov-check --sync-topology`);
  }
  if (existingText !== expectedText) {
    errors.push(`${GOVERNANCE_TOPOLOGY_REPO_REL_PATH} projection is stale; run just gov-check --sync-topology`);
  }
  errors.push(...validateGovernanceTopologyInventory(expected));
  return {
    ok: errors.length === 0,
    errors: [...new Set(errors)],
    expected,
  };
}
