import fsSync from "node:fs";
import fs from "node:fs/promises";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath, pathToFileURL } from "node:url";
import { MACHINE_READABLE_ARTIFACT_POLICY } from "../lib/packet-contract-lib.mjs";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, "../../../..");
const GOV_ROOT = path.join(REPO_ROOT, ".GOV");
const STUBS_DIR = path.join(GOV_ROOT, "task_packets", "stubs");
const SCHEMA_ID = "hsk.work_packet_stub_contract@1";
const SCHEMA_VERSION = "work_packet_stub_contract_v1";
const STUB_FILE_RE = /^(WP-[A-Za-z0-9._-]+)\.md$/;
const STUB_CONTRACT_FILE_RE = /^(WP-[A-Za-z0-9._-]+)\.contract\.json$/;

function stableJson(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function hashText(text) {
  return crypto.createHash("sha256").update(text, "utf8").digest("hex");
}

function normalizeRepoPath(absPath) {
  return path.relative(REPO_ROOT, absPath).split(path.sep).join("/");
}

function stripProjectionHeader(text = "") {
  return String(text || "").replace(/^<!--\s*GENERATED_PROJECTION[\s\S]*?-->\s*/i, "");
}

function parseSingleField(text = "", label = "") {
  const escaped = String(label).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = String(text || "").match(new RegExp(`^\\s*-\\s*${escaped}\\s*:\\s*(.+)\\s*$`, "mi"));
  return match ? match[1].trim() : "";
}

function splitList(value = "") {
  return String(value || "")
    .split(/[;,]/)
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function parseIntegerField(text = "", label = "") {
  const value = parseSingleField(text, label);
  if (!value) return null;
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : null;
}

function parseBooleanField(text = "", label = "") {
  const value = parseSingleField(text, label);
  if (!value) return null;
  if (/^(true|yes|1)$/i.test(value)) return true;
  if (/^(false|no|0)$/i.test(value)) return false;
  return null;
}

function parseIntentSection(text = "", heading = "") {
  const lines = String(text || "").split(/\r?\n/);
  const start = lines.findIndex((line) => line.trim().toUpperCase() === `## ${heading}`.toUpperCase());
  if (start === -1) return "";
  let end = lines.length;
  for (let index = start + 1; index < lines.length; index += 1) {
    if (/^##\s+/.test(lines[index])) {
      end = index;
      break;
    }
  }
  return lines.slice(start + 1, end).join("\n").trim();
}

async function exists(absPath) {
  try {
    await fs.access(absPath);
    return true;
  } catch {
    return false;
  }
}

function stubContractPathFor(stubMdAbsPath = "") {
  return stubMdAbsPath.replace(/\.md$/i, ".contract.json");
}

function stubMarkdownPathFor(stubContractAbsPath = "") {
  return stubContractAbsPath.replace(/\.contract\.json$/i, ".md");
}

export function stubContractPathFromMarkdownPath(stubMdPath = "") {
  return String(stubMdPath || "").replace(/\.md$/i, ".contract.json");
}

export function stubMarkdownPathFromContractPath(stubContractPath = "") {
  return String(stubContractPath || "").replace(/\.contract\.json$/i, ".md");
}

// Accepts either a stub .md path (legacy) or a stub .contract.json path
// (JSON-primary). Caller doesn't need to know the surface kind — both resolve
// to the same .contract.json on disk.
export function readStubContractForMarkdownPath(stubPath = "") {
  const inputPath = String(stubPath || "");
  const contractPath = /\.contract\.json$/i.test(inputPath)
    ? inputPath
    : stubContractPathFromMarkdownPath(inputPath);
  const absPath = path.isAbsolute(contractPath) ? contractPath : path.resolve(REPO_ROOT, contractPath);
  try {
    const contract = JSON.parse(fsSync.readFileSync(absPath, "utf8"));
    return { ok: true, source: "PRIMARY_MACHINE_READABLE_STUB", contract, contractPath };
  } catch {
    return { ok: false, source: "MISSING", contract: null, contractPath };
  }
}

export function buildStubContract({ wpId = "", stubText = "", stubPath = "" } = {}) {
  const projectionBody = stripProjectionHeader(stubText);
  const contractPath = stubPath.replace(/\.md$/i, ".contract.json");
  const lifecycleStatus = parseSingleField(projectionBody, "STUB_STATUS") || "STUB (NOT READY FOR DEV)";
  const activePacket = parseSingleField(projectionBody, "ACTIVE_PACKET");
  const activePacketContract = parseSingleField(projectionBody, "ACTIVE_PACKET_CONTRACT");
  const activatedAtUtc = parseSingleField(projectionBody, "ACTIVATED_AT");
  const activationSignature = parseSingleField(projectionBody, "ACTIVATION_SIGNATURE");
  const activatedToPacket = /^ACTIVATED_TO_PACKET$/i.test(lifecycleStatus);
  const draftMicrotaskSuitePath = parseSingleField(projectionBody, "DRAFT_MICROTASK_SUITE_PATH");
  const draftMicrotaskCount = parseIntegerField(projectionBody, "DRAFT_MICROTASK_COUNT");
  const officialMicrotasksGenerated = parseBooleanField(projectionBody, "OFFICIAL_MICROTASKS_GENERATED");
  const draftMicrotaskActivationDestinationPattern =
    parseSingleField(projectionBody, "DRAFT_MICROTASK_ACTIVATION_DESTINATION_PATTERN") ||
    ".GOV/task_packets/<WP_ID>/MT-*.{md,json}";
  const hasDraftMicrotaskSuite = Boolean(draftMicrotaskSuitePath || draftMicrotaskCount !== null);
  return {
    schema_id: SCHEMA_ID,
    schema_version: SCHEMA_VERSION,
    contract_authority: "PRIMARY_MACHINE_READABLE_STUB",
    artifact_policy: MACHINE_READABLE_ARTIFACT_POLICY,
    execution_authority: activatedToPacket ? "SUPERSEDED_BY_ACTIVE_PACKET" : "NON_EXECUTION_STUB",
    wp_id: parseSingleField(projectionBody, "WP_ID") || wpId,
    base_wp_id: String(parseSingleField(projectionBody, "BASE_WP_ID") || wpId).replace(/\s*\(.*/, "").trim(),
    ...(activatedToPacket && activePacketContract ? { superseded_by: activePacketContract } : {}),
    ...(activatedToPacket && activatedAtUtc ? { activated_at_utc: activatedAtUtc } : {}),
    ...(activatedToPacket && activationSignature ? { activation_signature: activationSignature } : {}),
    created_at_utc: parseSingleField(projectionBody, "CREATED_AT") || null,
    source_files: {
      stub_contract: contractPath,
      markdown_projection: stubPath,
    },
    markdown_projection: {
      path: stubPath,
      status: "LEGACY_PROJECTION_IN_SYNC",
      projection_sha256: hashText(projectionBody),
      generated_by: "task-packet-stub-contracts.mjs",
    },
    lifecycle: {
      status: lifecycleStatus,
      format_version: parseSingleField(projectionBody, "STUB_FORMAT_VERSION"),
      activation_required: !activatedToPacket,
      user_signature_required: false,
      refinement_required_before_execution: !activatedToPacket,
      ...(activatedToPacket && activePacket ? { active_packet: activePacket } : {}),
    },
    build_order: {
      domain: parseSingleField(projectionBody, "BUILD_ORDER_DOMAIN"),
      tech_blocker: parseSingleField(projectionBody, "BUILD_ORDER_TECH_BLOCKER"),
      value_tier: parseSingleField(projectionBody, "BUILD_ORDER_VALUE_TIER"),
      risk_tier: parseSingleField(projectionBody, "BUILD_ORDER_RISK_TIER"),
      depends_on: splitList(parseSingleField(projectionBody, "BUILD_ORDER_DEPENDS_ON")),
      blocks: splitList(parseSingleField(projectionBody, "BUILD_ORDER_BLOCKS")),
    },
    spec_trace: {
      spec_target: parseSingleField(projectionBody, "SPEC_TARGET"),
      roadmap_pointer: parseSingleField(projectionBody, "ROADMAP_POINTER"),
      roadmap_add_coverage: parseSingleField(projectionBody, "ROADMAP_ADD_COVERAGE"),
    },
    session_policy: {
      session_start_authority: parseSingleField(projectionBody, "SESSION_START_AUTHORITY"),
      session_host_preference: parseSingleField(projectionBody, "SESSION_HOST_PREFERENCE"),
      session_launch_policy: parseSingleField(projectionBody, "SESSION_LAUNCH_POLICY"),
      role_model_profile_policy: parseSingleField(projectionBody, "ROLE_MODEL_PROFILE_POLICY"),
      planned_execution_owner_range: parseSingleField(projectionBody, "PLANNED_EXECUTION_OWNER_RANGE"),
    },
    draft_scope: {
      intent: parseIntentSection(projectionBody, "INTENT (DRAFT)"),
      scope_sketch: parseIntentSection(projectionBody, "SCOPE_SKETCH (DRAFT)"),
      acceptance_criteria: parseIntentSection(projectionBody, "ACCEPTANCE_CRITERIA (DRAFT)"),
      risks_unknowns: parseIntentSection(projectionBody, "RISKS / UNKNOWNs (DRAFT)"),
      governance_artifact_stance: parseIntentSection(projectionBody, "MACHINE_READABLE_GOVERNANCE_ARTIFACT_STANCE"),
    },
    ...(hasDraftMicrotaskSuite
      ? {
          draft_microtasks: {
            source: "STUB_MARKDOWN_HANDOFF",
            suite_path: draftMicrotaskSuitePath,
            count: draftMicrotaskCount,
            official_microtasks_generated: officialMicrotasksGenerated === true,
            activation_destination_pattern: draftMicrotaskActivationDestinationPattern,
            activation_required_before_official_generation: true,
            activation_rule:
              "convert_draft_suite_after_refinement_user_signature_and_official_packet_creation",
          },
        }
      : {}),
    activation_contract: {
      may_start_coder: false,
      may_start_validator: false,
      ...(activatedToPacket ? { status: "COMPLETED_BY_ACTIVE_PACKET" } : {}),
      ...(activatedToPacket && activePacketContract ? { completed_packet_contract: activePacketContract } : {}),
      required_activation_steps: [
        "create_or_refresh_refinement",
        "obtain_user_signature",
        "create_official_work_packet_contract",
        ...(hasDraftMicrotaskSuite ? ["convert_draft_microtask_suite_to_official_microtask_contracts"] : []),
        "move_task_board_entry_from_stub_to_ready_for_dev",
      ],
    },
    red_team: {
      required: true,
      profile: "DETERMINISTIC_CONTRACT_MIGRATION_V1",
      assumptions_to_attack: [
        "A stub is planning metadata, not execution authority.",
        "Stub Markdown may contain stale draft scope until activated through a signed packet.",
        "Apps must not treat stub contracts as runnable work packets.",
      ],
      minimum_controls: [
        "Keep execution_authority=NON_EXECUTION_STUB.",
        "Require official packet contract before Coder or Validator launch.",
        "Fail drift checks when stub Markdown changes without contract regeneration.",
      ],
    },
  };
}

async function listStubMarkdownFiles() {
  if (!(await exists(STUBS_DIR))) return [];
  const entries = await fs.readdir(STUBS_DIR, { withFileTypes: true });
  return entries
    .filter((entry) => entry.isFile() && STUB_FILE_RE.test(entry.name))
    .map((entry) => path.join(STUBS_DIR, entry.name))
    .sort((left, right) => normalizeRepoPath(left).localeCompare(normalizeRepoPath(right), "en", { numeric: true }));
}

export async function listStubContractFiles() {
  if (!(await exists(STUBS_DIR))) return [];
  const entries = await fs.readdir(STUBS_DIR, { withFileTypes: true });
  return entries
    .filter((entry) => entry.isFile() && STUB_CONTRACT_FILE_RE.test(entry.name))
    .map((entry) => path.join(STUBS_DIR, entry.name))
    .sort((left, right) => normalizeRepoPath(left).localeCompare(normalizeRepoPath(right), "en", { numeric: true }));
}

// Minimum schema check for hand-authored .contract.json stubs that have no
// .md sibling. Validates the fields downstream tools (build-order-sync,
// task-board, traceability) need to safely discover and order the stub.
// Returns array of human-readable failure strings; empty array means OK.
export function validateHandAuthoredStubContract(contract = {}, contractPath = "") {
  const failures = [];
  const ref = contractPath ? `${contractPath}: ` : "";
  if (!contract || typeof contract !== "object") {
    failures.push(`${ref}contract is not an object`);
    return failures;
  }
  if (contract.schema_id !== SCHEMA_ID) {
    failures.push(`${ref}schema_id must equal "${SCHEMA_ID}" (got "${contract.schema_id}")`);
  }
  if (contract.contract_authority !== "PRIMARY_MACHINE_READABLE_STUB") {
    failures.push(`${ref}contract_authority must equal "PRIMARY_MACHINE_READABLE_STUB"`);
  }
  const policy = contract.artifact_policy || {};
  if (policy.authority_surface !== "MACHINE_CONTRACT") {
    failures.push(`${ref}artifact_policy.authority_surface must equal "MACHINE_CONTRACT"`);
  }
  if (typeof contract.wp_id !== "string" || !contract.wp_id.startsWith("WP-")) {
    failures.push(`${ref}wp_id must be a non-empty string starting with "WP-"`);
  }
  if (typeof contract.base_wp_id !== "string" || !contract.base_wp_id.startsWith("WP-")) {
    failures.push(`${ref}base_wp_id must be a non-empty string starting with "WP-"`);
  }
  const lifecycle = contract.lifecycle || {};
  if (typeof lifecycle.status !== "string" || lifecycle.status.trim() === "") {
    failures.push(`${ref}lifecycle.status must be a non-empty string`);
  }
  const buildOrder = contract.build_order || {};
  for (const field of ["domain", "tech_blocker", "value_tier", "risk_tier"]) {
    if (typeof buildOrder[field] !== "string" || buildOrder[field].trim() === "") {
      failures.push(`${ref}build_order.${field} must be a non-empty string`);
    }
  }
  for (const field of ["depends_on", "blocks"]) {
    if (!Array.isArray(buildOrder[field])) {
      failures.push(`${ref}build_order.${field} must be an array`);
    }
  }
  const specTrace = contract.spec_trace || {};
  if (typeof specTrace.roadmap_pointer !== "string" || specTrace.roadmap_pointer.trim() === "") {
    failures.push(`${ref}spec_trace.roadmap_pointer must be a non-empty string`);
  }
  const activation = contract.activation_contract || {};
  if (activation.may_start_coder !== false) {
    failures.push(`${ref}activation_contract.may_start_coder must be false (stubs cannot launch coder)`);
  }
  if (activation.may_start_validator !== false) {
    failures.push(`${ref}activation_contract.may_start_validator must be false (stubs cannot launch validator)`);
  }
  return failures;
}

export async function writeStubContractForPath(stubMdAbsPath = "") {
  const stubText = await fs.readFile(stubMdAbsPath, "utf8");
  const wpId = path.basename(stubMdAbsPath).replace(/\.md$/i, "");
  const stubPath = normalizeRepoPath(stubMdAbsPath);
  const contract = buildStubContract({ wpId, stubText, stubPath });
  const contractPath = stubContractPathFor(stubMdAbsPath);
  await fs.writeFile(contractPath, stableJson(contract), "utf8");
  return contract;
}

// Discovery walks .contract.json files (new no-.md policy). For each contract:
// - If a .md sibling exists, it's a legacy stub: regenerate the contract from
//   the .md source so the projection stays in sync (existing behavior).
// - If no .md sibling exists, the contract is hand-authored authority: leave
//   it alone. The hand-authored content survives across writeAll runs.
export async function writeAllStubContracts() {
  const contractFiles = await listStubContractFiles();
  const contracts = [];
  const seenContractPaths = new Set();
  for (const contractAbsPath of contractFiles) {
    const mdAbsPath = stubMarkdownPathFor(contractAbsPath);
    if (await exists(mdAbsPath)) {
      const contract = await writeStubContractForPath(mdAbsPath);
      contracts.push(contract);
    } else {
      // Hand-authored .json-only stub. Read and return as-is so callers see it
      // in the count, but don't overwrite.
      try {
        const raw = await fs.readFile(contractAbsPath, "utf8");
        contracts.push(JSON.parse(raw));
      } catch {
        // Unreadable .contract.json — let the check phase report it.
      }
    }
    seenContractPaths.add(contractAbsPath);
  }
  // Backstop: pick up any legacy .md stubs whose .contract.json hasn't been
  // generated yet (first-time materialization).
  const mdFiles = await listStubMarkdownFiles();
  for (const mdAbsPath of mdFiles) {
    const contractAbsPath = stubContractPathFor(mdAbsPath);
    if (!seenContractPaths.has(contractAbsPath)) {
      const contract = await writeStubContractForPath(mdAbsPath);
      contracts.push(contract);
    }
  }
  return contracts;
}

// Check walks .contract.json files (new no-.md policy). For each contract:
// - If a .md sibling exists, derive expected from .md and require an exact
//   match with the on-disk contract (existing legacy drift check).
// - If no .md sibling exists, validate the hand-authored contract has the
//   required schema fields directly.
// Backstop: catches legacy .md stubs that lack a generated contract.
export async function checkAllStubContracts() {
  const contractFiles = await listStubContractFiles();
  const failures = [];
  let checkedCount = 0;
  const seenContractPaths = new Set();
  for (const contractAbsPath of contractFiles) {
    seenContractPaths.add(contractAbsPath);
    const mdAbsPath = stubMarkdownPathFor(contractAbsPath);
    if (await exists(mdAbsPath)) {
      const expected = stableJson(buildStubContract({
        wpId: path.basename(mdAbsPath).replace(/\.md$/i, ""),
        stubText: await fs.readFile(mdAbsPath, "utf8"),
        stubPath: normalizeRepoPath(mdAbsPath),
      }));
      let actual = "";
      try {
        actual = await fs.readFile(contractAbsPath, "utf8");
      } catch {
        failures.push(`${normalizeRepoPath(contractAbsPath)} missing`);
        continue;
      }
      if (actual.replace(/\r\n/g, "\n") !== expected) {
        failures.push(`${normalizeRepoPath(contractAbsPath)} stale`);
      }
      checkedCount += 1;
    } else {
      let contract;
      try {
        contract = JSON.parse(await fs.readFile(contractAbsPath, "utf8"));
      } catch (error) {
        failures.push(`${normalizeRepoPath(contractAbsPath)} unreadable: ${error.message}`);
        continue;
      }
      const schemaFailures = validateHandAuthoredStubContract(
        contract,
        normalizeRepoPath(contractAbsPath),
      );
      for (const failure of schemaFailures) failures.push(failure);
      checkedCount += 1;
    }
  }
  // Backstop: catch legacy .md stubs with no generated .contract.json.
  const mdFiles = await listStubMarkdownFiles();
  for (const mdAbsPath of mdFiles) {
    const contractAbsPath = stubContractPathFor(mdAbsPath);
    if (seenContractPaths.has(contractAbsPath)) continue;
    failures.push(`${normalizeRepoPath(contractAbsPath)} missing`);
  }
  return { ok: failures.length === 0, failures, count: checkedCount };
}

function parseArgs(argv = []) {
  return {
    all: argv.includes("--all"),
    check: argv.includes("--check"),
    json: argv.includes("--json"),
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.check) {
    const result = await checkAllStubContracts();
    if (!result.ok) {
      console.error(`task-packet-stub-contracts check failed: ${result.failures.length} stale/missing contract(s)`);
      for (const failure of result.failures.slice(0, 20)) {
        console.error(`- ${failure}`);
      }
      process.exit(1);
    }
    console.log(`task-packet-stub-contracts ok: ${result.count} stub contract(s)`);
    return;
  }
  if (!args.all) {
    console.error("Usage: node .GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs --all [--json] | --check");
    process.exit(1);
  }
  const contracts = await writeAllStubContracts();
  if (args.json) {
    process.stdout.write(stableJson({ count: contracts.length, contracts }));
  } else {
    console.log(`task-packet-stub-contracts wrote ${contracts.length} contract(s)`);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main().catch((error) => {
    console.error(`task-packet-stub-contracts failed: ${error.message}`);
    process.exit(1);
  });
}
