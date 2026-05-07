import fs from "node:fs";

import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import { stableStringify } from "../scripts/lib/packet-contract-lib.mjs";
import { repoPathAbs } from "../scripts/lib/runtime-paths.mjs";
import {
  PUBLIC_SURFACE_CONSOLIDATION_PATH,
  buildPublicSurfaceConsolidation,
} from "../scripts/topology/public-surface-consolidation.mjs";

registerFailCaptureHook("public-surface-consolidation-check.mjs", { role: "SHARED" });

const expected = buildPublicSurfaceConsolidation();
const expectedText = stableStringify(expected);
const absPath = repoPathAbs(PUBLIC_SURFACE_CONSOLIDATION_PATH);
const violations = [];

if (!fs.existsSync(absPath)) {
  violations.push(
    `${PUBLIC_SURFACE_CONSOLIDATION_PATH}: missing; run node .GOV/roles_shared/scripts/topology/public-surface-consolidation.mjs --sync`,
  );
} else {
  const actualText = fs.readFileSync(absPath, "utf8");
  if (actualText !== expectedText) {
    violations.push(
      `${PUBLIC_SURFACE_CONSOLIDATION_PATH}: stale; run node .GOV/roles_shared/scripts/topology/public-surface-consolidation.mjs --sync`,
    );
  }
}

const missingMetadata = expected.entries.filter(
  (entry) =>
    !entry.replacement_bundle ||
    !entry.primary_debug_artifact ||
    !Array.isArray(entry.validation_coverage) ||
    entry.validation_coverage.length === 0,
);

if (missingMetadata.length > 0) {
  violations.push(
    `Public surface consolidation has ${missingMetadata.length} row(s) missing required metadata: ${missingMetadata
      .slice(0, 10)
      .map((entry) => entry.surface_id)
      .join("; ")}`,
  );
}

const newPublicNoPolicy = expected.entries.filter(
  (entry) => entry.consolidation_status === "TRACK_PUBLIC_SURFACE",
);

if (newPublicNoPolicy.length > 0) {
  violations.push(
    `Public surface consolidation has ${newPublicNoPolicy.length} tracked public surface(s) without concrete consolidation policy: ${newPublicNoPolicy
      .slice(0, 10)
      .map((entry) => entry.surface_id)
      .join("; ")}`,
  );
}

if (violations.length > 0) {
  failWithMemory(
    "public-surface-consolidation-check.mjs",
    "Public surface consolidation drift detected",
    { role: "SHARED", details: violations },
  );
}

console.log(
  `public-surface-consolidation-check ok (${expected.totals.public_surfaces} public surface(s), ${expected.totals.consolidate_when_touched} consolidate-when-touched)`,
);
