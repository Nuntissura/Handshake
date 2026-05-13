import fs from "node:fs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import { repoPathAbs } from "../scripts/lib/runtime-paths.mjs";
import { stableStringify } from "../scripts/lib/packet-contract-lib.mjs";
import {
  RESIDUAL_ARTIFACT_WRITER_INVENTORY_PATH,
  buildResidualArtifactWriterInventory,
} from "../scripts/audit/residual-artifact-writer-inventory.mjs";

registerFailCaptureHook("residual-artifact-writer-inventory-check.mjs", { role: "SHARED" });

const inventoryAbsPath = repoPathAbs(RESIDUAL_ARTIFACT_WRITER_INVENTORY_PATH);
const expected = buildResidualArtifactWriterInventory();
const expectedText = stableStringify(expected);
const violations = [];

if (!fs.existsSync(inventoryAbsPath)) {
  violations.push(`${RESIDUAL_ARTIFACT_WRITER_INVENTORY_PATH}: missing; run node .GOV/roles_shared/scripts/audit/residual-artifact-writer-inventory.mjs --sync`);
} else {
  const actualText = fs.readFileSync(inventoryAbsPath, "utf8");
  if (actualText !== expectedText) {
    violations.push(`${RESIDUAL_ARTIFACT_WRITER_INVENTORY_PATH}: stale; run node .GOV/roles_shared/scripts/audit/residual-artifact-writer-inventory.mjs --sync`);
  }
}

const unclassified = expected.entries.filter((entry) => entry.authority_class === "UNCLASSIFIED");
if (unclassified.length > 0) {
  violations.push(`Residual artifact writer inventory has ${unclassified.length} unclassified writer(s): ${unclassified.slice(0, 10).map((entry) => `${entry.path}:${entry.line}`).join("; ")}`);
}

if (violations.length > 0) {
  failWithMemory("residual-artifact-writer-inventory-check.mjs", "Residual artifact writer inventory drift detected", {
    role: "SHARED",
    details: violations,
  });
}

console.log(`residual-artifact-writer-inventory-check ok (${expected.totals.entries} writer(s), ${expected.totals.migration_candidates} migration candidate(s))`);
