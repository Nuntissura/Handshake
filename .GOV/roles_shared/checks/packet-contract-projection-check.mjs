import fs from "node:fs";
import path from "node:path";
import {
  GOV_ROOT_REPO_REL,
  REPO_ROOT,
  normalizePath,
  repoPathAbs,
} from "../scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import { validateContractProjectionPair } from "../scripts/lib/packet-contract-lib.mjs";

registerFailCaptureHook("packet-contract-projection-check.mjs", { role: "SHARED" });

const TASK_PACKETS_DIR = `${GOV_ROOT_REPO_REL}/task_packets`;
const CONTRACT_RE = /^(packet|refinement|MT-\d{3})\.json$/i;

function relFromAbs(absPath) {
  return normalizePath(path.relative(REPO_ROOT, absPath));
}

function readJson(absPath) {
  try {
    return JSON.parse(fs.readFileSync(absPath, "utf8"));
  } catch (error) {
    throw new Error(`${relFromAbs(absPath)}: invalid JSON contract (${error?.message || error})`);
  }
}

function scanContractFiles(rootAbs) {
  const out = [];
  if (!fs.existsSync(rootAbs)) return out;
  for (const entry of fs.readdirSync(rootAbs, { withFileTypes: true })) {
    if (!entry.isDirectory() || !/^WP-/.test(entry.name)) continue;
    const wpDirAbs = path.join(rootAbs, entry.name);
    for (const child of fs.readdirSync(wpDirAbs, { withFileTypes: true })) {
      if (!child.isFile() || !CONTRACT_RE.test(child.name)) continue;
      out.push(path.join(wpDirAbs, child.name));
    }
  }
  return out.sort((left, right) => relFromAbs(left).localeCompare(relFromAbs(right)));
}

function isJsonFirstProjectionOptOut(contract) {
  const policy = contract?.artifact_policy || {};
  const projection = contract?.markdown_projection || {};
  const authoritySurface = String(policy.authority_surface || "").trim().toUpperCase();
  const creation = String(policy.projection_creation || "").trim().toUpperCase();
  const projectionStatus = String(projection.status || "").trim().toUpperCase();
  const sourceHash = String(projection.source_hash || "").trim().toUpperCase();
  const projectionHash = String(projection.projection_hash || "").trim().toUpperCase();
  const generator = String(projection.generator || "").trim().toUpperCase();

  const machineContract = authoritySurface === "MACHINE_CONTRACT";
  const modelMarkdownDenied = policy.model_created_markdown_authority_allowed === false;
  const operatorFacingDenied = policy.operator_facing_authority === false;
  const onDemandOnly = creation === "ON_OPERATOR_REQUEST_ONLY";
  const notGeneratedByDefault = projectionStatus === "NOT_GENERATED_BY_DEFAULT"
    || sourceHash === "NOT_GENERATED_BY_DEFAULT"
    || projectionHash === "NOT_GENERATED_BY_DEFAULT"
    || generator === "PENDING_DEMAND";

  return machineContract
    && modelMarkdownDenied
    && operatorFacingDenied
    && (onDemandOnly || notGeneratedByDefault);
}

const violations = [];
const contracts = scanContractFiles(repoPathAbs(TASK_PACKETS_DIR));
let skippedJsonFirst = 0;

for (const contractAbs of contracts) {
  const contractRel = relFromAbs(contractAbs);
  let contract = null;
  try {
    contract = readJson(contractAbs);
  } catch (error) {
    violations.push(error.message);
    continue;
  }

  if (isJsonFirstProjectionOptOut(contract)) {
    skippedJsonFirst += 1;
    continue;
  }

  const projectionRel = normalizePath(contract?.markdown_projection?.path || "");
  if (!projectionRel) {
    violations.push(`${contractRel}: markdown_projection.path is required`);
    continue;
  }

  const projectionAbs = path.isAbsolute(projectionRel) ? projectionRel : repoPathAbs(projectionRel);
  if (!fs.existsSync(projectionAbs)) {
    violations.push(`${contractRel}: projection missing on disk (${projectionRel})`);
    continue;
  }

  const projectionText = fs.readFileSync(projectionAbs, "utf8");
  violations.push(...validateContractProjectionPair({
    contract,
    projectionText,
    contractPath: contractRel,
    projectionPath: projectionRel,
  }));
}

if (violations.length > 0) {
  failWithMemory("packet-contract-projection-check.mjs", "Generated contract projection drift detected", {
    role: "SHARED",
    details: violations,
  });
}

const suffix = skippedJsonFirst > 0 ? `, ${skippedJsonFirst} json-first projection opt-out(s)` : "";
console.log(`packet-contract-projection-check ok (${contracts.length} contract(s)${suffix})`);
