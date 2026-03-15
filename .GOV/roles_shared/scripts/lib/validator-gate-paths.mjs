import fs from "node:fs";
import path from "node:path";

export const SHARED_VALIDATOR_GATE_DIR = path.join(".GOV", "roles_shared", "runtime", "validator_gates");

function isJsonFile(name) {
  return typeof name === "string" && name.toLowerCase().endsWith(".json");
}

export function validatorGateFileName(wpId) {
  return `${String(wpId || "").trim()}.json`;
}

export function validatorGatePath(wpId) {
  return path.join(SHARED_VALIDATOR_GATE_DIR, validatorGateFileName(wpId));
}

export function ensureValidatorGateDir() {
  if (!fs.existsSync(SHARED_VALIDATOR_GATE_DIR)) {
    fs.mkdirSync(SHARED_VALIDATOR_GATE_DIR, { recursive: true });
  }
}

export function resolveValidatorGatePath(wpId) {
  return validatorGatePath(wpId);
}

export function listValidatorGateStateFiles() {
  const results = [];
  if (!fs.existsSync(SHARED_VALIDATOR_GATE_DIR)) return results;

  for (const entry of fs.readdirSync(SHARED_VALIDATOR_GATE_DIR, { withFileTypes: true })) {
    if (!entry.isFile() || !isJsonFile(entry.name)) continue;
    if (entry.name === "VALIDATOR_GATES.json") continue;
    results.push(path.join(SHARED_VALIDATOR_GATE_DIR, entry.name));
  }

  return results.sort((left, right) => left.localeCompare(right));
}
