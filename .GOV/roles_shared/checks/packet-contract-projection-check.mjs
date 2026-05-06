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

const violations = [];
const contracts = scanContractFiles(repoPathAbs(TASK_PACKETS_DIR));

for (const contractAbs of contracts) {
  const contractRel = relFromAbs(contractAbs);
  let contract = null;
  try {
    contract = readJson(contractAbs);
  } catch (error) {
    violations.push(error.message);
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

console.log(`packet-contract-projection-check ok (${contracts.length} contract(s))`);
