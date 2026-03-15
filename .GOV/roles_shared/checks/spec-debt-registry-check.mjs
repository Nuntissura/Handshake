import { loadSpecDebtRegistry, SPEC_DEBT_REGISTRY_PATH } from "../scripts/lib/spec-debt-registry-lib.mjs";

function fail(message, details = []) {
  console.error(`[SPEC_DEBT_REGISTRY_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

const registry = loadSpecDebtRegistry();
if (registry.errors.length > 0) {
  fail(`Spec debt registry invalid: ${SPEC_DEBT_REGISTRY_PATH.replace(/\\/g, "/")}`, registry.errors);
}

console.log("spec-debt-registry-check ok");

