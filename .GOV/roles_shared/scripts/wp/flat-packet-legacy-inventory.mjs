import fs from "node:fs/promises";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath, pathToFileURL } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, "../../../..");
const GOV_ROOT = path.join(REPO_ROOT, ".GOV");
const TASK_PACKETS_DIR = path.join(GOV_ROOT, "task_packets");
const STUBS_DIR = path.join(TASK_PACKETS_DIR, "stubs");
const DEFAULT_INVENTORY_PATH = path.join(
  GOV_ROOT,
  "roles_shared",
  "records",
  "FLAT_PACKET_LEGACY_INVENTORY.json",
);

const SCHEMA_VERSION = "FLAT_PACKET_LEGACY_INVENTORY.v1";
const WP_FILE_RE = /^(WP-[A-Za-z0-9._-]+)\.md$/;

function toRepoPath(absPath) {
  return path.relative(REPO_ROOT, absPath).split(path.sep).join("/");
}

function stableJson(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function hashText(text) {
  return crypto.createHash("sha256").update(text, "utf8").digest("hex");
}

function sortKey(entry) {
  const numeric = entry.wp_id.match(/^WP-(\d+)$/);
  const idPart = numeric ? String(Number(numeric[1])).padStart(8, "0") : entry.wp_id;
  return `${idPart}:${entry.artifact_kind}:${entry.path}`;
}

async function exists(absPath) {
  try {
    await fs.access(absPath);
    return true;
  } catch {
    return false;
  }
}

async function listFlatPacketMarkdown(dir, kind) {
  if (!(await exists(dir))) {
    return [];
  }

  const entries = await fs.readdir(dir, { withFileTypes: true });
  const artifacts = [];

  for (const entry of entries) {
    if (!entry.isFile()) {
      continue;
    }

    const match = entry.name.match(WP_FILE_RE);
    if (!match) {
      continue;
    }

    const wpId = match[1];
    const absPath = path.join(dir, entry.name);
    const text = await fs.readFile(absPath, "utf8");
    const pairedContractAbs = path.join(TASK_PACKETS_DIR, wpId, "packet.json");
    const pairedContract = (await exists(pairedContractAbs)) ? toRepoPath(pairedContractAbs) : null;

    artifacts.push({
      wp_id: wpId,
      artifact_kind: kind,
      path: toRepoPath(absPath),
      sha256: hashText(text),
      bytes: Buffer.byteLength(text, "utf8"),
      paired_folder_contract: pairedContract,
      authority_status: authorityStatus(kind, pairedContract),
      disposition: disposition(kind, pairedContract),
      migration_refactor_id: "RGF-289",
    });
  }

  return artifacts;
}

function authorityStatus(kind, pairedContract) {
  if (kind === "FLAT_PACKET_STUB") {
    return "STUB_NOT_EXECUTION_AUTHORITY";
  }
  if (pairedContract) {
    return "LEGACY_MARKDOWN_SUPERSEDED_BY_FOLDER_CONTRACT_PENDING_ARCHIVE_DECISION";
  }
  return "LEGACY_MARKDOWN_AUTHORITY_PENDING_CONTRACT_MIGRATION";
}

function disposition(kind, pairedContract) {
  if (kind === "FLAT_PACKET_STUB") {
    return "CLASSIFY_THEN_IMPORT_OR_SUPERSEDE_STUB_WITHOUT_DUPLICATE_AUTHORITY";
  }
  if (pairedContract) {
    return "VERIFY_FOLDER_CONTRACT_THEN_ARCHIVE_OR_FREEZE_FLAT_REFERENCE";
  }
  return "IMPORT_TO_TYPED_CONTRACT_OR_FREEZE_AS_EXPLICIT_LEGACY_AUTHORITY";
}

export async function buildFlatPacketLegacyInventory() {
  const official = await listFlatPacketMarkdown(TASK_PACKETS_DIR, "FLAT_OFFICIAL_PACKET");
  const stubs = await listFlatPacketMarkdown(STUBS_DIR, "FLAT_PACKET_STUB");
  const artifacts = [...official, ...stubs].sort((a, b) => sortKey(a).localeCompare(sortKey(b)));

  const counts = {
    total: artifacts.length,
    flat_official_packets: artifacts.filter((entry) => entry.artifact_kind === "FLAT_OFFICIAL_PACKET").length,
    flat_packet_stubs: artifacts.filter((entry) => entry.artifact_kind === "FLAT_PACKET_STUB").length,
    paired_folder_contracts: artifacts.filter((entry) => entry.paired_folder_contract).length,
    unpaired_legacy_authority: artifacts.filter(
      (entry) => entry.authority_status === "LEGACY_MARKDOWN_AUTHORITY_PENDING_CONTRACT_MIGRATION",
    ).length,
  };

  const inventoryBody = {
    schema_version: SCHEMA_VERSION,
    inventory_kind: "FLAT_PACKET_LEGACY_MIGRATION_INPUTS",
    policy_ref: "RGF-289",
    source_roots: [".GOV/task_packets/WP-*.md", ".GOV/task_packets/stubs/WP-*.md"],
    deterministic_order: "wp_id_numeric_then_artifact_kind_then_path",
    counts,
    artifacts,
  };

  return {
    ...inventoryBody,
    artifacts_sha256: hashText(stableJson(artifacts)),
  };
}

export async function writeFlatPacketLegacyInventory(inventoryPath = DEFAULT_INVENTORY_PATH) {
  const inventory = await buildFlatPacketLegacyInventory();
  await fs.mkdir(path.dirname(inventoryPath), { recursive: true });
  await fs.writeFile(inventoryPath, stableJson(inventory), "utf8");
  return inventory;
}

export async function checkFlatPacketLegacyInventory(inventoryPath = DEFAULT_INVENTORY_PATH) {
  const expected = stableJson(await buildFlatPacketLegacyInventory());
  let actual;
  try {
    actual = await fs.readFile(inventoryPath, "utf8");
  } catch (error) {
    throw new Error(`missing inventory ${toRepoPath(inventoryPath)}: ${error.message}`);
  }

  if (actual.replace(/\r\n/g, "\n") !== expected) {
    throw new Error(`stale inventory ${toRepoPath(inventoryPath)}; run just flat-packet-legacy-inventory`);
  }

  return JSON.parse(actual);
}

function parseArgs(argv) {
  const args = {
    check: false,
    dryRun: false,
    json: false,
    output: DEFAULT_INVENTORY_PATH,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--check") {
      args.check = true;
    } else if (arg === "--dry-run") {
      args.dryRun = true;
    } else if (arg === "--json") {
      args.json = true;
    } else if (arg === "--output") {
      const value = argv[index + 1];
      if (!value) {
        throw new Error("--output requires a path");
      }
      args.output = path.resolve(REPO_ROOT, value);
      index += 1;
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }

  return args;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));

  if (args.check) {
    const inventory = await checkFlatPacketLegacyInventory(args.output);
    console.log(`flat-packet-legacy-inventory ok: ${inventory.counts.total} artifact(s)`);
    return;
  }

  const inventory = args.dryRun
    ? await buildFlatPacketLegacyInventory()
    : await writeFlatPacketLegacyInventory(args.output);

  if (args.json) {
    process.stdout.write(stableJson(inventory));
    return;
  }

  const verb = args.dryRun ? "would write" : "wrote";
  console.log(
    `flat-packet-legacy-inventory ${verb} ${toRepoPath(args.output)}: ` +
      `${inventory.counts.flat_official_packets} flat official, ` +
      `${inventory.counts.flat_packet_stubs} stubs, ` +
      `${inventory.counts.unpaired_legacy_authority} unpaired legacy authority`,
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main().catch((error) => {
    console.error(`flat-packet-legacy-inventory failed: ${error.message}`);
    process.exit(1);
  });
}
