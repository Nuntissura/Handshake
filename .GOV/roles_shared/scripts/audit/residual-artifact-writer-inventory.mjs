import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  GOV_ROOT_REPO_REL,
  REPO_ROOT,
  normalizePath,
  repoPathAbs,
} from "../lib/runtime-paths.mjs";
import { sha256Short, stableStringify } from "../lib/packet-contract-lib.mjs";

export const RESIDUAL_ARTIFACT_WRITER_INVENTORY_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/records/RESIDUAL_ARTIFACT_WRITER_INVENTORY.json`;

const SCAN_ROOTS = [
  `${GOV_ROOT_REPO_REL}/roles`,
  `${GOV_ROOT_REPO_REL}/roles_shared/scripts`,
  `${GOV_ROOT_REPO_REL}/roles_shared/checks`,
];

const SCAN_EXTENSIONS = new Set([".mjs", ".js", ".cjs", ".ps1", ".cmd"]);

const WRITE_CALL_PATTERNS = [
  { call: "fs.writeFileSync", operation: "WRITE", re: /\bfs\.writeFileSync\s*\(/u },
  { call: "fs.appendFileSync", operation: "APPEND", re: /\bfs\.appendFileSync\s*\(/u },
  { call: "fs.writeFile", operation: "WRITE_ASYNC", re: /\bfs\.writeFile\s*\(/u },
  { call: "fs.appendFile", operation: "APPEND_ASYNC", re: /\bfs\.appendFile\s*\(/u },
  { call: "writeJsonFile", operation: "WRITE_JSON", re: /\bwriteJsonFile\s*\(/u },
  { call: "writeFile", operation: "WRITE_HELPER", re: /\bwriteFile\s*\(/u },
  { call: "Set-Content", operation: "WRITE_POWERSHELL", re: /\bSet-Content\b/iu },
  { call: "Add-Content", operation: "APPEND_POWERSHELL", re: /\bAdd-Content\b/iu },
  { call: "Out-File", operation: "WRITE_POWERSHELL", re: /\bOut-File\b/iu },
];

function relFromAbs(absPath) {
  return normalizePath(path.relative(REPO_ROOT, absPath));
}

function walkFiles(rootAbs) {
  const out = [];
  if (!fs.existsSync(rootAbs)) return out;
  for (const entry of fs.readdirSync(rootAbs, { withFileTypes: true })) {
    const abs = path.join(rootAbs, entry.name);
    if (entry.isDirectory()) {
      out.push(...walkFiles(abs));
      continue;
    }
    if (!entry.isFile() || !SCAN_EXTENSIONS.has(path.extname(entry.name))) continue;
    out.push(abs);
  }
  return out;
}

function firstWriteCall(line = "") {
  for (const pattern of WRITE_CALL_PATTERNS) {
    if (pattern.re.test(line)) return pattern;
  }
  return null;
}

function extractTargetExpression(line = "") {
  const open = line.indexOf("(");
  if (open < 0) return "";
  let depth = 0;
  let quote = "";
  for (let idx = open + 1; idx < line.length; idx += 1) {
    const ch = line[idx];
    const prev = idx > 0 ? line[idx - 1] : "";
    if (quote) {
      if (ch === quote && prev !== "\\") quote = "";
      continue;
    }
    if (ch === "\"" || ch === "'" || ch === "`") {
      quote = ch;
      continue;
    }
    if (ch === "(" || ch === "[" || ch === "{") depth += 1;
    if (ch === ")" || ch === "]" || ch === "}") depth -= 1;
    if (ch === "," && depth <= 0) return line.slice(open + 1, idx).trim();
  }
  return line.slice(open + 1).replace(/\)\s*;?\s*$/u, "").trim();
}

function ownerForPath(relPath = "") {
  const normalized = normalizePath(relPath);
  const roleMatch = normalized.match(/^\.GOV\/roles\/([^/]+)/u);
  if (roleMatch) return roleMatch[1].toUpperCase();
  if (normalized.includes("/roles_shared/")) return "SHARED";
  return "UNKNOWN";
}

function classifyWriter({ relPath = "", line = "", context = "", operation = "", targetExpression = "" } = {}) {
  const normalizedPath = normalizePath(relPath);
  const haystack = `${line}\n${context}\n${targetExpression}`.toLowerCase();

  if (normalizedPath.includes("/tests/")) {
    return {
      artifact_class: "TEST_FIXTURE",
      authority_class: "TEST_ONLY",
      risk_class: "LOW",
      migration_status: "EXEMPT_TEST_FIXTURE",
      rationale: "Test fixture writes are not runtime governance authority.",
    };
  }

  if (normalizedPath.endsWith("/residual-artifact-writer-inventory.mjs")) {
    return {
      artifact_class: "WRITER_INVENTORY_JSON",
      authority_class: "PRIMARY_MACHINE_READABLE_RECORD",
      risk_class: "LOW",
      migration_status: "CANONICAL",
      rationale: "This script writes the deterministic residual writer inventory.",
    };
  }

  if (haystack.includes("patchabspath") || haystack.includes(".patch")) {
    return {
      artifact_class: "SIGNED_SCOPE_PATCH_ARTIFACT",
      authority_class: "EVIDENCE_ARTIFACT",
      risk_class: "MEDIUM",
      migration_status: "KEEP_EVIDENCE_ARTIFACT",
      rationale: "Patch artifacts are evidence payloads referenced by packet lifecycle truth.",
    };
  }

  if (haystack.includes("contractabspath") || haystack.includes("contractpath") || haystack.includes("packet.json") || haystack.includes("refinement.json") || haystack.includes("mt-") && haystack.includes(".json")) {
    return {
      artifact_class: "PRIMARY_CONTRACT_JSON",
      authority_class: "PRIMARY_MACHINE_READABLE_ARTIFACT",
      risk_class: "LOW",
      migration_status: "CANONICAL_CONTRACT_WRITER",
      rationale: "The writer targets a primary JSON contract or contract-adjacent structured file.",
    };
  }

  if (haystack.includes("projectionabs") || haystack.includes("projectionpath")) {
    return {
      artifact_class: "GENERATED_MARKDOWN_PROJECTION",
      authority_class: "CONTRACT_SYNC_PROJECTION_WRITER",
      risk_class: "MEDIUM",
      migration_status: "KEEP_WITH_PROJECTION_HASH_CHECK",
      rationale: "Projection writers are allowed when paired with contract metadata and projection drift checks.",
    };
  }

  if (normalizedPath.endsWith("/work-packet-contract-read-lib.mjs") && haystack.includes("folderpaths.packetabspath")) {
    return {
      artifact_class: "PACKET_CONTRACT_IMPORT_PROJECTION",
      authority_class: "CONTRACT_SYNC_PROJECTION_WRITER",
      risk_class: "MEDIUM",
      migration_status: "PROVEN_CONTRACT_SYNC_HELPER",
      rationale: "The import helper writes packet.md as a generated projection paired with packet.json contract metadata.",
    };
  }

  if (normalizedPath.endsWith("/work-packet-contract-read-lib.mjs") && haystack.includes("fallbackabspath")) {
    return {
      artifact_class: "LEGACY_PACKET_MARKDOWN_FALLBACK",
      authority_class: "EXPLICIT_LEGACY_COMPATIBILITY_FALLBACK",
      risk_class: "MEDIUM",
      migration_status: "PROVEN_LEGACY_FALLBACK_BOUNDARY",
      rationale: "The shared contract writer falls back to Markdown only when a primary folder contract cannot be resolved; this is the governed legacy boundary.",
    };
  }

  if (haystack.includes("fallbackabspath") || haystack.includes("packetabspath") || haystack.includes("packet.md") || haystack.includes("nextpackettext")) {
    return {
      artifact_class: "WORK_PACKET_MARKDOWN_PROJECTION",
      authority_class: "COMPATIBILITY_PROJECTION_FALLBACK",
      risk_class: "HIGH",
      migration_status: "MIGRATE_OR_PROVE_CONTRACT_SYNC",
      rationale: "Packet Markdown projection writes can bypass packet.json unless routed through a contract sync helper.",
    };
  }

  if (haystack.includes("refinement.md")) {
    return {
      artifact_class: "REFINEMENT_MARKDOWN_PROJECTION",
      authority_class: "COMPATIBILITY_PROJECTION_FALLBACK",
      risk_class: "HIGH",
      migration_status: "MIGRATE_OR_PROVE_CONTRACT_SYNC",
      rationale: "Refinement Markdown projection writes must remain subordinate to refinement.json.",
    };
  }

  if (haystack.includes("mt-") && haystack.includes(".md")) {
    return {
      artifact_class: "MICROTASK_MARKDOWN_PROJECTION",
      authority_class: "COMPATIBILITY_PROJECTION_FALLBACK",
      risk_class: "HIGH",
      migration_status: "MIGRATE_OR_PROVE_CONTRACT_SYNC",
      rationale: "MT Markdown projection writes must remain subordinate to MT-*.json.",
    };
  }

  if (haystack.includes("threadabspath") || haystack.includes("notificationsabspath") || haystack.includes("cursorabspath") || haystack.includes("wp-communication") || haystack.includes("runtime_status.json") || haystack.includes("runtimestatusabspath")) {
    return {
      artifact_class: "WP_COMMUNICATION_RUNTIME_ARTIFACT",
      authority_class: "EXTERNAL_RUNTIME_STATE",
      risk_class: "MEDIUM",
      migration_status: "STRUCTURED_RUNTIME_AUTHORITY",
      rationale: "WP communication files are external runtime authority or append-only coordination projections.",
    };
  }

  if (haystack.includes("receipts") || haystack.includes(".jsonl") || operation.includes("APPEND")) {
    return {
      artifact_class: "APPEND_ONLY_RUNTIME_LOG",
      authority_class: "EXTERNAL_RUNTIME_LEDGER",
      risk_class: "MEDIUM",
      migration_status: "STRUCTURED_APPEND_ONLY_AUTHORITY",
      rationale: "Append-only runtime ledgers remain structured machine-readable artifacts.",
    };
  }

  if (haystack.includes("dossier")) {
    return {
      artifact_class: "WORKFLOW_DOSSIER_PROJECTION",
      authority_class: "GENERATED_OPERATOR_PROJECTION",
      risk_class: "MEDIUM",
      migration_status: "KEEP_GENERATED_PROJECTION",
      rationale: "Workflow dossier Markdown is a generated diagnostic projection, not primary packet authority.",
    };
  }

  if (haystack.includes("audit")) {
    return {
      artifact_class: "AUDIT_MARKDOWN_PROJECTION",
      authority_class: "GENERATED_OPERATOR_PROJECTION",
      risk_class: "MEDIUM",
      migration_status: "KEEP_GENERATED_PROJECTION",
      rationale: "Audit skeleton Markdown is generated diagnostic material and should be paired with structured evidence where applicable.",
    };
  }

  if (haystack.includes("check-result") || haystack.includes("gate-output") || haystack.includes("artifactpath") || haystack.includes("perfile")) {
    return {
      artifact_class: "CHECK_OR_GATE_ARTIFACT",
      authority_class: "DIAGNOSTIC_ARTIFACT",
      risk_class: "MEDIUM",
      migration_status: "KEEP_DIAGNOSTIC_ARTIFACT",
      rationale: "Check and gate outputs are diagnostic artifacts rather than packet/refinement/MT authority.",
    };
  }

  if (haystack.includes("gatestate") || haystack.includes("validator_gate")) {
    return {
      artifact_class: "VALIDATOR_GATE_STATE_JSON",
      authority_class: "STRUCTURED_RUNTIME_AUTHORITY",
      risk_class: "MEDIUM",
      migration_status: "STRUCTURED_RUNTIME_AUTHORITY",
      rationale: "Validator gate state is already structured JSON.",
    };
  }

  if (operation === "WRITE_JSON" || haystack.includes(".json")) {
    return {
      artifact_class: "STRUCTURED_JSON_ARTIFACT",
      authority_class: "MACHINE_READABLE_ARTIFACT",
      risk_class: "LOW",
      migration_status: "STRUCTURED_AUTHORITY_OR_PROJECTION",
      rationale: "The write target appears to be structured JSON rather than editable Markdown.",
    };
  }

  if (normalizedPath.includes("/dev/")) {
    return {
      artifact_class: "DEVELOPMENT_SCAFFOLD_OUTPUT",
      authority_class: "SCAFFOLD_GENERATOR_OUTPUT",
      risk_class: "LOW",
      migration_status: "OUT_OF_PACKET_AUTHORITY",
      rationale: "Development scaffold generators write product/workflow files outside packet projection authority.",
    };
  }

  if (normalizedPath.includes("/memory/") || normalizedPath.includes("/memory_manager/")) {
    return {
      artifact_class: "GOVERNANCE_MEMORY_ARTIFACT",
      authority_class: "MEMORY_RUNTIME_ARTIFACT",
      risk_class: "LOW",
      migration_status: "OUT_OF_PACKET_AUTHORITY",
      rationale: "Governance memory files are runtime memory artifacts, not packet/refinement/MT projections.",
    };
  }

  if (normalizedPath.includes("/session/") || normalizedPath.includes("launch-cli-session") || normalizedPath.includes("orchestrator-rescue")) {
    return {
      artifact_class: "SESSION_CONTROL_ARTIFACT",
      authority_class: "SESSION_RUNTIME_ARTIFACT",
      risk_class: "MEDIUM",
      migration_status: "OUT_OF_PACKET_AUTHORITY",
      rationale: "Session control writers manage launch/runtime artifacts outside packet projection authority.",
    };
  }

  if (normalizedPath.includes("/topology/") || normalizedPath.includes("governance-topology") || normalizedPath.includes("governance-snapshot")) {
    return {
      artifact_class: "TOPOLOGY_OR_SNAPSHOT_ARTIFACT",
      authority_class: "STRUCTURED_GOVERNANCE_RECORD",
      risk_class: "LOW",
      migration_status: "STRUCTURED_AUTHORITY_OR_PROJECTION",
      rationale: "Topology and snapshot writers are governed record/projection surfaces.",
    };
  }

  if (normalizedPath.includes("build-order") || normalizedPath.includes("task-board") || normalizedPath.includes("wp-traceability") || normalizedPath.includes("spec-debt")) {
    return {
      artifact_class: "GOVERNANCE_RECORD_ARTIFACT",
      authority_class: "GOVERNANCE_LEDGER_OR_PROJECTION",
      risk_class: "MEDIUM",
      migration_status: "TRACK_FOR_MACHINE_READABLE_MIGRATION",
      rationale: "Governance records remain tracked ledgers/projections and should migrate to structured authority where feasible.",
    };
  }

  if (haystack.includes("settingspath") || haystack.includes("scriptpath") || haystack.includes("pspath") || haystack.includes("tokenfile") || haystack.includes("manifestpath")) {
    return {
      artifact_class: "TOOLING_CONTROL_ARTIFACT",
      authority_class: "TOOLING_RUNTIME_ARTIFACT",
      risk_class: "LOW",
      migration_status: "OUT_OF_PACKET_AUTHORITY",
      rationale: "Tooling control artifacts are outside packet/refinement/MT projection authority.",
    };
  }

  if (haystack.includes(".md")) {
    return {
      artifact_class: "MARKDOWN_PROJECTION",
      authority_class: "PROJECTION_OR_LEGACY_BRIDGE",
      risk_class: "MEDIUM",
      migration_status: "CLASSIFY_BEFORE_MIGRATION",
      rationale: "Markdown writes require explicit classification before they can remain as generated projections.",
    };
  }

  return {
    artifact_class: "GENERIC_GOVERNANCE_ARTIFACT_WRITE",
    authority_class: "CLASSIFIED_GENERIC_GOVERNANCE_WRITER",
    risk_class: "MEDIUM",
    migration_status: "TRACK_FOR_FUTURE_REFINEMENT",
    rationale: "The write call is tracked but target naming is too generic for a more specific automated class.",
  };
}

export function buildResidualArtifactWriterInventory() {
  const files = SCAN_ROOTS.flatMap((root) => walkFiles(repoPathAbs(root)))
    .sort((left, right) => relFromAbs(left).localeCompare(relFromAbs(right)));
  const entries = [];

  for (const absPath of files) {
    const relPath = relFromAbs(absPath);
    const text = fs.readFileSync(absPath, "utf8");
    const lines = text.split(/\r?\n/u);
    const sourceHash = sha256Short(text);
    for (let idx = 0; idx < lines.length; idx += 1) {
      const rawLine = lines[idx];
      const writeCall = firstWriteCall(rawLine);
      if (!writeCall) continue;
      const targetExpression = extractTargetExpression(rawLine);
      const context = [
        lines[idx - 2] || "",
        lines[idx - 1] || "",
        rawLine,
        lines[idx + 1] || "",
        lines[idx + 2] || "",
      ].join("\n");
      const classification = classifyWriter({
        relPath,
        line: rawLine,
        context,
        operation: writeCall.operation,
        targetExpression,
      });
      entries.push({
        id: `artifact-writer:${sha256Short(`${relPath}:${idx + 1}:${rawLine.trim()}`)}`,
        path: relPath,
        line: idx + 1,
        owner_role: ownerForPath(relPath),
        write_call: writeCall.call,
        operation: writeCall.operation,
        target_expression: targetExpression,
        source_hash: sourceHash,
        ...classification,
      });
    }
  }

  const totals = {
    entries: entries.length,
    high_risk: entries.filter((entry) => entry.risk_class === "HIGH").length,
    medium_risk: entries.filter((entry) => entry.risk_class === "MEDIUM").length,
    low_risk: entries.filter((entry) => entry.risk_class === "LOW").length,
    migration_candidates: entries.filter((entry) => String(entry.migration_status || "").includes("MIGRATE")).length,
    unclassified: entries.filter((entry) => entry.authority_class === "UNCLASSIFIED").length,
  };

  return {
    schema_id: "hsk.residual_artifact_writer_inventory@1",
    generated_by: "residual-artifact-writer-inventory.mjs",
    generated_at_utc: null,
    source_roots: SCAN_ROOTS,
    policy: {
      purpose: "Deterministically inventory governance artifact writers before migrating residual Markdown/projection writes.",
      high_risk_rule: "Packet, refinement, and MT Markdown writes must be migrated to primary contract writers or carry an explicit contract-sync proof.",
      deletion_rule: "This inventory does not authorize deleting scripts or artifacts.",
    },
    totals,
    entries,
    inventory_hash: `sha256:${sha256Short(stableStringify(entries))}`,
  };
}

export function writeResidualArtifactWriterInventory() {
  const inventory = buildResidualArtifactWriterInventory();
  const outputAbsPath = repoPathAbs(RESIDUAL_ARTIFACT_WRITER_INVENTORY_PATH);
  fs.mkdirSync(path.dirname(outputAbsPath), { recursive: true });
  fs.writeFileSync(outputAbsPath, stableStringify(inventory), "utf8");
  return inventory;
}

function main() {
  const shouldSync = process.argv.includes("--sync");
  const inventory = shouldSync
    ? writeResidualArtifactWriterInventory()
    : buildResidualArtifactWriterInventory();
  console.log(`residual-artifact-writer-inventory ${shouldSync ? "synced" : "ok"}: ${inventory.totals.entries} writer(s), ${inventory.totals.migration_candidates} migration candidate(s)`);
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (invokedPath && path.resolve(fileURLToPath(import.meta.url)) === invokedPath) {
  main();
}
