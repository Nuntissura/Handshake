import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
  normalizePath,
} from "../lib/runtime-paths.mjs";

export const WP_DOSSIER_INDEX_SCHEMA_ID = "handshake.gov.wp_dossier_index";
export const WP_DOSSIER_INDEX_SCHEMA_VERSION = "wp_dossier_index_v1";
export const WP_DOSSIER_MANIFEST_SCHEMA_ID = "handshake.gov.wp_dossier_artifact_manifest";
export const WP_DOSSIER_MANIFEST_SCHEMA_VERSION = "wp_dossier_artifact_manifest_v1";

const REQUIRED_DIRS = [
  "raw",
  "raw/commands",
  "acp",
  "repomem",
  "commands",
  "bundle_failures",
];

const REQUIRED_FILES = [
  "index.json",
  "events.jsonl",
  "artifact_manifest.json",
  "workflow_postmortem.md",
];

function normalizeWpId(wpId = "") {
  const normalized = String(wpId || "").trim();
  if (!/^WP-[A-Za-z0-9_.-]+$/u.test(normalized)) {
    throw new Error(`Invalid WP id: ${wpId || "<missing>"}`);
  }
  return normalized;
}

function runtimeRel(runtimeRootAbs, targetAbs) {
  return normalizePath(path.relative(path.resolve(runtimeRootAbs), path.resolve(targetAbs)));
}

export function wpCommunicationsRootAbs(runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS) {
  return path.join(path.resolve(runtimeRootAbs), "roles_shared", "WP_COMMUNICATIONS");
}

export function wpDossiersRootAbs(runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS) {
  return path.join(path.resolve(runtimeRootAbs), "roles_shared", "WP_DOSSIERS");
}

export function wpDossierRootAbs(wpId, runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS) {
  return path.join(wpDossiersRootAbs(runtimeRootAbs), normalizeWpId(wpId));
}

export function discoverActiveWpIds(runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS) {
  const commRoot = wpCommunicationsRootAbs(runtimeRootAbs);
  if (!fs.existsSync(commRoot)) return [];
  return fs.readdirSync(commRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && /^WP-[A-Za-z0-9_.-]+$/u.test(entry.name))
    .map((entry) => entry.name)
    .sort((left, right) => left.localeCompare(right));
}

function countJsonlRows(absPath) {
  if (!fs.existsSync(absPath)) return 0;
  return fs.readFileSync(absPath, "utf8").split(/\r?\n/u).filter(Boolean).length;
}

function existingArtifactRefs(rootAbs, runtimeRootAbs) {
  const out = [];
  const scanRoots = ["raw", "acp", "repomem", "commands", "bundle_failures"];
  for (const relRoot of scanRoots) {
    const absRoot = path.join(rootAbs, relRoot);
    if (!fs.existsSync(absRoot)) continue;
    const stack = [absRoot];
    while (stack.length > 0) {
      const dir = stack.pop();
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const abs = path.join(dir, entry.name);
        if (entry.isDirectory()) {
          stack.push(abs);
        } else if (entry.isFile()) {
          out.push({
            kind: relRoot,
            path: runtimeRel(runtimeRootAbs, abs),
            bytes: fs.statSync(abs).size,
          });
        }
      }
    }
  }
  return out.sort((left, right) => left.path.localeCompare(right.path));
}

export function buildWpDossierIndex(wpId, runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS) {
  const normalizedWpId = normalizeWpId(wpId);
  const root = wpDossierRootAbs(normalizedWpId, runtimeRootAbs);
  const bundleFailuresJsonl = path.join(root, "bundle_failures", "phase_bundle_failures.jsonl");
  return {
    schema_id: WP_DOSSIER_INDEX_SCHEMA_ID,
    schema_version: WP_DOSSIER_INDEX_SCHEMA_VERSION,
    wp_id: normalizedWpId,
    updated_at_utc: new Date().toISOString(),
    root,
    contract: {
      storage: "external_governance_runtime_root",
      raw_archive_policy: "dump_all_logs_for_posterity",
      model_entrypoint: "index.json",
      terminal_narrative: "workflow_postmortem.md",
      no_git_storage: true,
    },
    paths: {
      index_json: runtimeRel(runtimeRootAbs, path.join(root, "index.json")),
      events_jsonl: runtimeRel(runtimeRootAbs, path.join(root, "events.jsonl")),
      artifact_manifest_json: runtimeRel(runtimeRootAbs, path.join(root, "artifact_manifest.json")),
      workflow_postmortem_md: runtimeRel(runtimeRootAbs, path.join(root, "workflow_postmortem.md")),
      raw_dir: runtimeRel(runtimeRootAbs, path.join(root, "raw")),
      acp_dir: runtimeRel(runtimeRootAbs, path.join(root, "acp")),
      repomem_dir: runtimeRel(runtimeRootAbs, path.join(root, "repomem")),
      commands_dir: runtimeRel(runtimeRootAbs, path.join(root, "commands")),
      bundle_failures_dir: runtimeRel(runtimeRootAbs, path.join(root, "bundle_failures")),
      bundle_failures_jsonl: runtimeRel(runtimeRootAbs, bundleFailuresJsonl),
    },
    counts: {
      bundle_failure_entries: countJsonlRows(bundleFailuresJsonl),
      artifact_refs: existingArtifactRefs(root, runtimeRootAbs).length,
    },
  };
}

export function initializeWpDossier(wpId, {
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
  eventType = "DOSSIER_INITIALIZED",
} = {}) {
  const normalizedWpId = normalizeWpId(wpId);
  const root = wpDossierRootAbs(normalizedWpId, runtimeRootAbs);
  const existed = fs.existsSync(path.join(root, "index.json"));
  for (const rel of REQUIRED_DIRS) {
    fs.mkdirSync(path.join(root, rel), { recursive: true });
  }

  const manifestPath = path.join(root, "artifact_manifest.json");
  const manifest = {
    schema_id: WP_DOSSIER_MANIFEST_SCHEMA_ID,
    schema_version: WP_DOSSIER_MANIFEST_SCHEMA_VERSION,
    wp_id: normalizedWpId,
    updated_at_utc: new Date().toISOString(),
    artifacts: existingArtifactRefs(root, runtimeRootAbs),
  };
  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");

  const postmortemPath = path.join(root, "workflow_postmortem.md");
  if (!fs.existsSync(postmortemPath)) {
    fs.writeFileSync(
      postmortemPath,
      [
        `# Workflow Postmortem: ${normalizedWpId}`,
        "",
        "Orchestrator-owned terminal narrative. Raw ACP, repomem, command, and bundle logs remain in this dossier tree for posterity.",
        "",
      ].join("\n"),
      "utf8",
    );
  }

  const eventsPath = path.join(root, "events.jsonl");
  if (!fs.existsSync(eventsPath)) fs.writeFileSync(eventsPath, "", "utf8");
  if (!existed) {
    fs.appendFileSync(eventsPath, `${JSON.stringify({
      schema_id: "handshake.gov.wp_dossier_event",
      schema_version: "wp_dossier_event_v1",
      timestamp: new Date().toISOString(),
      wp_id: normalizedWpId,
      event_type: eventType,
    })}\n`, "utf8");
  }

  const indexPath = path.join(root, "index.json");
  fs.writeFileSync(indexPath, `${JSON.stringify(buildWpDossierIndex(normalizedWpId, runtimeRootAbs), null, 2)}\n`, "utf8");
  return {
    wp_id: normalizedWpId,
    root,
    index_path: indexPath,
    created: !existed,
  };
}

export function initializeActiveWpDossiers({
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
  wpIds = [],
} = {}) {
  const ids = wpIds.length > 0 ? wpIds.map((wpId) => normalizeWpId(wpId)) : discoverActiveWpIds(runtimeRootAbs);
  return ids.map((wpId) => initializeWpDossier(wpId, { runtimeRootAbs }));
}

function readJson(absPath) {
  try {
    return JSON.parse(fs.readFileSync(absPath, "utf8"));
  } catch (error) {
    throw new Error(`${absPath}: invalid JSON (${error?.message || error})`);
  }
}

export function validateWpDossier(wpId, runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS) {
  const normalizedWpId = normalizeWpId(wpId);
  const root = wpDossierRootAbs(normalizedWpId, runtimeRootAbs);
  const errors = [];
  for (const rel of REQUIRED_DIRS) {
    const abs = path.join(root, rel);
    if (!fs.existsSync(abs) || !fs.statSync(abs).isDirectory()) errors.push(`${runtimeRel(runtimeRootAbs, abs)}: missing required dossier directory`);
  }
  for (const rel of REQUIRED_FILES) {
    const abs = path.join(root, rel);
    if (!fs.existsSync(abs) || !fs.statSync(abs).isFile()) errors.push(`${runtimeRel(runtimeRootAbs, abs)}: missing required dossier file`);
  }
  const indexPath = path.join(root, "index.json");
  if (fs.existsSync(indexPath)) {
    const index = readJson(indexPath);
    if (index.schema_id !== WP_DOSSIER_INDEX_SCHEMA_ID) errors.push(`${runtimeRel(runtimeRootAbs, indexPath)}: invalid schema_id`);
    if (index.wp_id !== normalizedWpId) errors.push(`${runtimeRel(runtimeRootAbs, indexPath)}: wp_id must be ${normalizedWpId}`);
    if (index.contract?.raw_archive_policy !== "dump_all_logs_for_posterity") errors.push(`${runtimeRel(runtimeRootAbs, indexPath)}: raw_archive_policy must dump all logs`);
    if (index.contract?.model_entrypoint !== "index.json") errors.push(`${runtimeRel(runtimeRootAbs, indexPath)}: model_entrypoint must be index.json`);
  }
  const manifestPath = path.join(root, "artifact_manifest.json");
  if (fs.existsSync(manifestPath)) {
    const manifest = readJson(manifestPath);
    if (manifest.schema_id !== WP_DOSSIER_MANIFEST_SCHEMA_ID) errors.push(`${runtimeRel(runtimeRootAbs, manifestPath)}: invalid schema_id`);
    if (!Array.isArray(manifest.artifacts)) errors.push(`${runtimeRel(runtimeRootAbs, manifestPath)}: artifacts must be an array`);
  }
  return errors;
}

export function validateActiveWpDossiers(runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS) {
  const activeWpIds = discoverActiveWpIds(runtimeRootAbs);
  const errors = [];
  for (const wpId of activeWpIds) {
    errors.push(...validateWpDossier(wpId, runtimeRootAbs));
  }
  return {
    activeWpIds,
    errors,
  };
}

function main() {
  const sync = process.argv.includes("--sync");
  const explicitWpIds = process.argv.slice(2).filter((arg) => /^WP-[A-Za-z0-9_.-]+$/u.test(arg));
  if (sync) {
    const initialized = initializeActiveWpDossiers({ wpIds: explicitWpIds });
    console.log(`wp-dossier-runtime synced: ${initialized.length} dossier(s)`);
    for (const item of initialized) {
      console.log(`- ${item.wp_id}: ${item.index_path}`);
    }
    return;
  }
  const result = validateActiveWpDossiers();
  if (result.errors.length > 0) {
    console.error(`wp-dossier-runtime invalid: ${result.errors.length} error(s)`);
    for (const error of result.errors) console.error(`- ${error}`);
    process.exit(1);
  }
  console.log(`wp-dossier-runtime ok: ${result.activeWpIds.length} active WP dossier(s)`);
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (invokedPath && path.resolve(fileURLToPath(import.meta.url)) === invokedPath) {
  main();
}
