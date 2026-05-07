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

const recipeIds = new Set(expected.entries
  .filter((entry) => entry.surface_kind === "JUST_RECIPE")
  .map((entry) => entry.surface_id));

const malformedAliases = expected.entries.filter((entry) => {
  if (entry.consolidation_status !== "KEEP_COMPATIBILITY_ALIAS_WITH_REPLACEMENT") return false;
  if (!Array.isArray(entry.alias_target_recipes) || entry.alias_target_recipes.length !== 1) return true;
  const [target] = entry.alias_target_recipes;
  if (!target || target === entry.just_recipes?.[0]) return true;
  return !recipeIds.has(`just:${target}`);
});

if (malformedAliases.length > 0) {
  violations.push(
    `Public surface consolidation has ${malformedAliases.length} compatibility alias row(s) without exactly one concrete target recipe: ${malformedAliases
      .slice(0, 10)
      .map((entry) => entry.surface_id)
      .join("; ")}`,
  );
}

const canonicalSessionControls = new Set([
  "just:session-start",
  "just:session-send",
  "just:session-cancel",
  "just:session-close",
]);

const nonCanonicalSessionControls = expected.entries.filter(
  (entry) =>
    canonicalSessionControls.has(entry.surface_id) &&
    (entry.entrypoint_status !== "CANONICAL_PUBLIC_ENTRY" || entry.consolidation_status !== "RETAIN_CANONICAL"),
);

if (nonCanonicalSessionControls.length > 0) {
  violations.push(
    `Public surface consolidation has ${nonCanonicalSessionControls.length} session control row(s) that are not retained canonical public entries: ${nonCanonicalSessionControls
      .map((entry) => `${entry.surface_id}:${entry.entrypoint_status}/${entry.consolidation_status}`)
      .join("; ")}`,
  );
}

const sessionAliasTargetErrors = expected.entries.filter((entry) => {
  if (entry.consolidation_status !== "KEEP_COMPATIBILITY_ALIAS_WITH_REPLACEMENT") return false;
  const recipeName = Array.isArray(entry.just_recipes) ? entry.just_recipes[0] : "";
  if (!/^(start|steer|cancel|close)-(activation-manager|coder|wp-validator|integration-validator)-session$/u.test(recipeName)) {
    return false;
  }
  const [target] = Array.isArray(entry.alias_target_recipes) ? entry.alias_target_recipes : [];
  return !canonicalSessionControls.has(`just:${target}`);
});

if (sessionAliasTargetErrors.length > 0) {
  violations.push(
    `Public surface consolidation has ${sessionAliasTargetErrors.length} session alias row(s) not targeting canonical session controls: ${sessionAliasTargetErrors
      .slice(0, 10)
      .map((entry) => `${entry.surface_id}->${(entry.alias_target_recipes || []).join(",") || "none"}`)
      .join("; ")}`,
  );
}

const destructiveRemovalCandidates = expected.entries.filter(
  (entry) =>
    entry.removal_gate !== "NOT_A_REMOVAL_CANDIDATE" &&
    String(entry.removal_gate || "").toUpperCase().includes("REMOVE") &&
    entry.consolidation_status !== "KEEP_COMPATIBILITY_ALIAS_WITH_REPLACEMENT",
);

if (destructiveRemovalCandidates.length > 0) {
  violations.push(
    `Public surface consolidation has ${destructiveRemovalCandidates.length} non-alias row(s) with removal-oriented gates: ${destructiveRemovalCandidates
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
