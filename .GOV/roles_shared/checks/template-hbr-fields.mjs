import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  buildDefaultHbrAcceptanceMatrix,
  buildDefaultHbrContext,
  buildDefaultHbrObligations,
  formatPacketAcceptanceMatrixSection,
} from "../scripts/lib/packet-closure-monitor-lib.mjs";

const PILLARS = ["INT", "SWARM", "VIS", "QUIET", "MAN"];

function defaultRepoRoot() {
  const injected = String(process.env.HANDSHAKE_ACTIVE_REPO_ROOT || "").trim();
  if (injected) return path.resolve(injected);
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

function parseArgs(argv) {
  const args = { repoRoot: defaultRepoRoot() };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--repo-root") {
      const value = argv[index + 1];
      if (!value) throw new Error("--repo-root requires a path");
      args.repoRoot = path.resolve(value);
      index += 1;
      continue;
    }
    throw new Error(`unknown argument: ${arg}`);
  }
  return args;
}

function templatePath(repoRoot, name) {
  return path.join(repoRoot, ".GOV", "templates", name);
}

function relativeTemplatePath(name) {
  return `.GOV/templates/${name}`;
}

function readText(repoRoot, name, failures) {
  const filePath = templatePath(repoRoot, name);
  try {
    return fs.readFileSync(filePath, "utf8");
  } catch (error) {
    failures.push(`${relativeTemplatePath(name)} unreadable: ${error.message}`);
    return null;
  }
}

function readJson(repoRoot, name, failures) {
  const text = readText(repoRoot, name, failures);
  if (text === null) return null;
  const jsonText = text.charCodeAt(0) === 0xFEFF ? text.slice(1) : text;
  try {
    return JSON.parse(jsonText);
  } catch (error) {
    failures.push(`${relativeTemplatePath(name)} invalid JSON: ${error.message}`);
    return null;
  }
}

function isPlainObject(value) {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

function getPath(value, dottedPath) {
  return dottedPath.split(".").reduce((current, segment) => (
    current === undefined || current === null ? undefined : current[segment]
  ), value);
}

function requireIncludes(failures, templateName, text, needle) {
  if (!text.includes(needle)) {
    failures.push(`${relativeTemplatePath(templateName)} missing ${needle}`);
  }
}

function requireEmptyArrayPath(failures, templateName, object, dottedPath) {
  const value = getPath(object, dottedPath);
  if (!Array.isArray(value)) {
    failures.push(`${relativeTemplatePath(templateName)} missing array ${dottedPath}`);
    return;
  }
  if (value.length !== 0) {
    failures.push(`${relativeTemplatePath(templateName)} expected ${dottedPath} to default to an empty array`);
  }
}

function requireValuePath(failures, templateName, object, dottedPath, expectedValue) {
  const value = getPath(object, dottedPath);
  if (value !== expectedValue) {
    failures.push(`${relativeTemplatePath(templateName)} expected ${dottedPath} === ${JSON.stringify(expectedValue)}`);
  }
}

function requireRepoFileIncludes(failures, repoRoot, relativePath, needles) {
  const filePath = path.join(repoRoot, ...relativePath.split("/"));
  let text = "";
  try {
    text = fs.readFileSync(filePath, "utf8");
  } catch (error) {
    failures.push(`${relativePath} unreadable: ${error.message}`);
    return;
  }
  for (const needle of needles) {
    if (!text.includes(needle)) {
      failures.push(`${relativePath} missing ${needle}`);
    }
  }
}

function requirePillarText(failures, templateName, text) {
  requireIncludes(failures, templateName, text, "hbr_pillar_review");
  for (const pillar of PILLARS) {
    const pattern = new RegExp(`\\b${pillar}\\b[\\s\\S]{0,160}\\bapplicable\\b[\\s\\S]{0,160}\\bevidence_path\\b`);
    if (!pattern.test(text)) {
      failures.push(`${relativeTemplatePath(templateName)} missing ${pillar} hbr_pillar_review applicable/evidence_path fields`);
    }
  }
}

function requirePillarJson(failures, templateName, object) {
  const review = getPath(object, "refinement.hbr_pillar_review");
  if (!isPlainObject(review)) {
    failures.push(`${relativeTemplatePath(templateName)} missing object refinement.hbr_pillar_review`);
    return;
  }
  for (const pillar of PILLARS) {
    const pillarReview = review[pillar];
    if (!isPlainObject(pillarReview)) {
      failures.push(`${relativeTemplatePath(templateName)} missing object refinement.hbr_pillar_review.${pillar}`);
      continue;
    }
    if (!Object.hasOwn(pillarReview, "applicable")) {
      failures.push(`${relativeTemplatePath(templateName)} missing refinement.hbr_pillar_review.${pillar}.applicable`);
    } else if (pillarReview.applicable !== null) {
      failures.push(`${relativeTemplatePath(templateName)} expected refinement.hbr_pillar_review.${pillar}.applicable === null`);
    }
    if (!Object.hasOwn(pillarReview, "evidence_path")) {
      failures.push(`${relativeTemplatePath(templateName)} missing refinement.hbr_pillar_review.${pillar}.evidence_path`);
    } else if (pillarReview.evidence_path !== null) {
      failures.push(`${relativeTemplatePath(templateName)} expected refinement.hbr_pillar_review.${pillar}.evidence_path === null`);
    }
  }
}

function requireGeneratedDefaults(failures, repoRoot) {
  const hbrContext = buildDefaultHbrContext();
  if (!Array.isArray(hbrContext.tags_declared) || hbrContext.tags_declared.length !== 0) {
    failures.push("buildDefaultHbrContext().tags_declared must default to []");
  }
  if (!Array.isArray(hbrContext.not_applicable_overrides) || hbrContext.not_applicable_overrides.length !== 0) {
    failures.push("buildDefaultHbrContext().not_applicable_overrides must default to []");
  }

  const acceptanceMatrix = buildDefaultHbrAcceptanceMatrix();
  if (acceptanceMatrix.schema_version !== 1) {
    failures.push("buildDefaultHbrAcceptanceMatrix().schema_version must default to 1");
  }
  if (!Array.isArray(acceptanceMatrix.hbr) || acceptanceMatrix.hbr.length !== 0) {
    failures.push("buildDefaultHbrAcceptanceMatrix().hbr must default to []");
  }
  if (!Array.isArray(acceptanceMatrix.hbr_not_applicable) || acceptanceMatrix.hbr_not_applicable.length !== 0) {
    failures.push("buildDefaultHbrAcceptanceMatrix().hbr_not_applicable must default to []");
  }

  const hbrObligations = buildDefaultHbrObligations();
  if (!Array.isArray(hbrObligations) || hbrObligations.length !== 0) {
    failures.push("buildDefaultHbrObligations() must default to []");
  }

  const generatedPacketSection = formatPacketAcceptanceMatrixSection([]);
  for (const needle of [
    "hbr.tags_declared: []",
    "hbr.not_applicable_overrides: []",
    "acceptance_matrix.schema_version: 1",
    "acceptance_matrix.hbr: []",
    "acceptance_matrix.hbr_not_applicable: []",
  ]) {
    if (!generatedPacketSection.includes(needle)) {
      failures.push(`formatPacketAcceptanceMatrixSection([]) missing ${needle}`);
    }
  }

  const generatorNeedles = [
    "buildDefaultHbrContext()",
    "buildDefaultHbrAcceptanceMatrix()",
    "buildDefaultHbrObligations()",
  ];
  requireRepoFileIncludes(failures, repoRoot, ".GOV/roles/orchestrator/scripts/create-task-packet.mjs", generatorNeedles);
  requireRepoFileIncludes(failures, repoRoot, ".GOV/roles_shared/scripts/wp/atelier-lens-consolidation-packet-generator.mjs", generatorNeedles);
}

export function validateTemplates(repoRoot = defaultRepoRoot()) {
  const failures = [];

  const taskPacketText = readText(repoRoot, "TASK_PACKET_TEMPLATE.md", failures);
  if (taskPacketText !== null) {
    for (const needle of [
      "hbr.tags_declared",
      "hbr.not_applicable_overrides",
      "hbr.tags_declared: []",
      "hbr.not_applicable_overrides: []",
      "acceptance_matrix.hbr",
      "acceptance_matrix.hbr_not_applicable",
      "acceptance_matrix.hbr: []",
      "acceptance_matrix.hbr_not_applicable: []",
    ]) {
      requireIncludes(failures, "TASK_PACKET_TEMPLATE.md", taskPacketText, needle);
    }
  }

  const workPacketContract = readJson(repoRoot, "WORK_PACKET_CONTRACT_TEMPLATE.json", failures);
  if (workPacketContract !== null) {
    requireEmptyArrayPath(failures, "WORK_PACKET_CONTRACT_TEMPLATE.json", workPacketContract, "hbr.tags_declared");
    requireEmptyArrayPath(failures, "WORK_PACKET_CONTRACT_TEMPLATE.json", workPacketContract, "hbr.not_applicable_overrides");
    requireValuePath(failures, "WORK_PACKET_CONTRACT_TEMPLATE.json", workPacketContract, "acceptance_matrix.schema_version", 1);
    requireEmptyArrayPath(failures, "WORK_PACKET_CONTRACT_TEMPLATE.json", workPacketContract, "acceptance_matrix.hbr");
    requireEmptyArrayPath(failures, "WORK_PACKET_CONTRACT_TEMPLATE.json", workPacketContract, "acceptance_matrix.hbr_not_applicable");
  }

  const refinementText = readText(repoRoot, "REFINEMENT_TEMPLATE.md", failures);
  if (refinementText !== null) {
    requirePillarText(failures, "REFINEMENT_TEMPLATE.md", refinementText);
  }

  const refinementContract = readJson(repoRoot, "REFINEMENT_CONTRACT_TEMPLATE.json", failures);
  if (refinementContract !== null) {
    requirePillarJson(failures, "REFINEMENT_CONTRACT_TEMPLATE.json", refinementContract);
    requireEmptyArrayPath(failures, "REFINEMENT_CONTRACT_TEMPLATE.json", refinementContract, "refinement.microtask_plan_item_defaults.hbr_obligations");
  }

  const microtaskContract = readJson(repoRoot, "MICRO_TASK_CONTRACT_TEMPLATE.json", failures);
  if (microtaskContract !== null) {
    requireEmptyArrayPath(failures, "MICRO_TASK_CONTRACT_TEMPLATE.json", microtaskContract, "hbr_obligations");
  }

  requireGeneratedDefaults(failures, repoRoot);

  return failures;
}

export function runCli(argv = process.argv.slice(2)) {
  let args;
  try {
    args = parseArgs(argv);
  } catch (error) {
    console.error(JSON.stringify({
      check: "template-hbr-fields",
      verdict: "ERROR",
      failures: [error instanceof Error ? error.message : String(error)],
    }, null, 2));
    return 3;
  }

  const failures = validateTemplates(args.repoRoot);
  if (failures.length > 0) {
    console.error(JSON.stringify({
      check: "template-hbr-fields",
      verdict: "FAIL",
      failures,
    }, null, 2));
    return 1;
  }

  console.log("template-hbr-fields ok (TASK_PACKET_TEMPLATE.md, WORK_PACKET_CONTRACT_TEMPLATE.json, REFINEMENT_TEMPLATE.md, REFINEMENT_CONTRACT_TEMPLATE.json, MICRO_TASK_CONTRACT_TEMPLATE.json; hbr_obligations covered by MICRO_TASK_CONTRACT_TEMPLATE.json)");
  return 0;
}

function isInvokedAsMain() {
  if (!process.argv[1]) return false;
  const invoked = fs.realpathSync.native(path.resolve(process.argv[1]));
  const current = fs.realpathSync.native(fileURLToPath(import.meta.url));
  return invoked === current;
}

if (isInvokedAsMain()) {
  process.exitCode = runCli();
}
