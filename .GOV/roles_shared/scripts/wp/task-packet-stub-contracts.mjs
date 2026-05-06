import fs from "node:fs/promises";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath, pathToFileURL } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, "../../../..");
const GOV_ROOT = path.join(REPO_ROOT, ".GOV");
const STUBS_DIR = path.join(GOV_ROOT, "task_packets", "stubs");
const SCHEMA_ID = "hsk.work_packet_stub_contract@1";
const SCHEMA_VERSION = "work_packet_stub_contract_v1";
const STUB_FILE_RE = /^(WP-[A-Za-z0-9._-]+)\.md$/;

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

export function buildStubContract({ wpId = "", stubText = "", stubPath = "" } = {}) {
  const projectionBody = stripProjectionHeader(stubText);
  const contractPath = stubPath.replace(/\.md$/i, ".contract.json");
  return {
    schema_id: SCHEMA_ID,
    schema_version: SCHEMA_VERSION,
    contract_authority: "PRIMARY_MACHINE_READABLE_STUB",
    execution_authority: "NON_EXECUTION_STUB",
    wp_id: parseSingleField(projectionBody, "WP_ID") || wpId,
    base_wp_id: String(parseSingleField(projectionBody, "BASE_WP_ID") || wpId).replace(/\s*\(.*/, "").trim(),
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
      status: parseSingleField(projectionBody, "STUB_STATUS") || "STUB (NOT READY FOR DEV)",
      format_version: parseSingleField(projectionBody, "STUB_FORMAT_VERSION"),
      activation_required: true,
      user_signature_required: false,
      refinement_required_before_execution: true,
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
    },
    activation_contract: {
      may_start_coder: false,
      may_start_validator: false,
      required_activation_steps: [
        "create_or_refresh_refinement",
        "obtain_user_signature",
        "create_official_work_packet_contract",
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

export async function writeStubContractForPath(stubMdAbsPath = "") {
  const stubText = await fs.readFile(stubMdAbsPath, "utf8");
  const wpId = path.basename(stubMdAbsPath).replace(/\.md$/i, "");
  const stubPath = normalizeRepoPath(stubMdAbsPath);
  const contract = buildStubContract({ wpId, stubText, stubPath });
  const contractPath = stubContractPathFor(stubMdAbsPath);
  await fs.writeFile(contractPath, stableJson(contract), "utf8");
  return contract;
}

export async function writeAllStubContracts() {
  const files = await listStubMarkdownFiles();
  const contracts = [];
  for (const file of files) {
    contracts.push(await writeStubContractForPath(file));
  }
  return contracts;
}

export async function checkAllStubContracts() {
  const files = await listStubMarkdownFiles();
  const failures = [];
  for (const file of files) {
    const expected = stableJson(buildStubContract({
      wpId: path.basename(file).replace(/\.md$/i, ""),
      stubText: await fs.readFile(file, "utf8"),
      stubPath: normalizeRepoPath(file),
    }));
    const contractPath = stubContractPathFor(file);
    let actual = "";
    try {
      actual = await fs.readFile(contractPath, "utf8");
    } catch {
      failures.push(`${normalizeRepoPath(contractPath)} missing`);
      continue;
    }
    if (actual.replace(/\r\n/g, "\n") !== expected) {
      failures.push(`${normalizeRepoPath(contractPath)} stale`);
    }
  }
  return { ok: failures.length === 0, failures, count: files.length };
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
