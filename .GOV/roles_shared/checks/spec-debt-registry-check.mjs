import { loadSpecDebtRegistry, SPEC_DEBT_REGISTRY_PATH } from "../scripts/lib/spec-debt-registry-lib.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("spec-debt-registry-check.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("spec-debt-registry-check.mjs", message, { role: "SHARED", details });
}

const registry = loadSpecDebtRegistry();
if (registry.errors.length > 0) {
  fail(`Spec debt registry invalid: ${SPEC_DEBT_REGISTRY_PATH.replace(/\\/g, "/")}`, registry.errors);
}

console.log("spec-debt-registry-check ok");

