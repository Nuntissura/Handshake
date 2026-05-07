import fs from "node:fs";
import crypto from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  GOV_ROOT_REPO_REL,
  repoPathAbs,
} from "../lib/runtime-paths.mjs";
import {
  buildGovernanceTopology,
  isSimpleJustCompatibilityAlias,
  justCallTargetsForRecipe,
  parseJustRecipes,
} from "../lib/governance-topology-lib.mjs";
import { sha256Short, stableStringify } from "../lib/packet-contract-lib.mjs";

export const PUBLIC_SURFACE_CONSOLIDATION_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/records/PUBLIC_SURFACE_CONSOLIDATION.json`;

function isPublicLikeSurface(surface = {}) {
  return String(surface.public_exposure || "").includes("PUBLIC")
    || String(surface.entrypoint_status || "").includes("PUBLIC")
    || String(surface.entrypoint_status || "").includes("COMPATIBILITY");
}

function consolidationStatus(surface = {}) {
  const status = String(surface.entrypoint_status || "");
  const kind = String(surface.surface_kind || "");
  const exposure = String(surface.public_exposure || "");
  if (exposure === "PUBLIC_JUSTFILE" || kind === "JUSTFILE") {
    return "RETAIN_CANONICAL";
  }
  if (status === "CANONICAL_PUBLIC_ENTRY" || status === "CANONICAL_BUNDLE_OR_CHECKPOINT" || status === "CANONICAL_SUB_BUNDLE") {
    return "RETAIN_CANONICAL";
  }
  if (status === "COMPATIBILITY_ALIAS") return "KEEP_COMPATIBILITY_ALIAS_WITH_REPLACEMENT";
  if (status === "PUBLIC_RECIPE_BASELINED" || status === "PUBLIC_LEAF_BASELINED") return "CONSOLIDATE_WHEN_TOUCHED";
  if (status === "CONTRACT_AUTHORITY" || kind === "ROLE_PROTOCOL" || kind === "AGENTS_CONTRACT") return "RETAIN_CONTRACT_AUTHORITY";
  return "TRACK_PUBLIC_SURFACE";
}

function removalGate(surface = {}) {
  const status = consolidationStatus(surface);
  if (status === "RETAIN_CANONICAL" || status === "RETAIN_CONTRACT_AUTHORITY") {
    return "NOT_A_REMOVAL_CANDIDATE";
  }
  if (status === "KEEP_COMPATIBILITY_ALIAS_WITH_REPLACEMENT") {
    return "MAY_REMOVE_ONLY_AFTER_REPLACEMENT_IS_TRACKED_USABLE_AND_OPERATOR_ERGONOMICS_ARE_PRESERVED";
  }
  if (status === "CONSOLIDATE_WHEN_TOUCHED") {
    return "WHEN_TOUCHED_EXTEND_REPLACEMENT_BUNDLE_OR_RECORD_EXCEPTION_BEFORE_KEEPING_PUBLIC_LEAF";
  }
  return "REQUIRES_EXPLICIT_TOPOLOGY_EXCEPTION";
}

function publicEntry(surface = {}, recipeByName = new Map()) {
  const recipeName = surface.surface_kind === "JUST_RECIPE" && Array.isArray(surface.just_recipes)
    ? surface.just_recipes[0]
    : null;
  const recipe = recipeName ? recipeByName.get(recipeName) : null;
  const aliasTargetRecipes = recipe && isSimpleJustCompatibilityAlias(recipe)
    ? justCallTargetsForRecipe(recipe)
    : [];
  return {
    surface_id: surface.surface_id,
    path: surface.path,
    surface_kind: surface.surface_kind,
    owner_role: surface.owner_role,
    authority_boundary: surface.authority_boundary,
    phase: surface.phase,
    side_effect_class: surface.side_effect_class,
    public_exposure: surface.public_exposure,
    entrypoint_status: surface.entrypoint_status,
    just_recipes: Array.isArray(surface.just_recipes) ? surface.just_recipes : [],
    replacement_bundle: surface.replacement_bundle,
    primary_debug_artifact: surface.primary_debug_artifact,
    validation_coverage: Array.isArray(surface.validation_coverage) ? surface.validation_coverage : [],
    consolidation_status: consolidationStatus(surface),
    removal_gate: removalGate(surface),
    alias_target_recipes: aliasTargetRecipes,
    alias_target_count: aliasTargetRecipes.length,
  };
}

function countBy(entries, key) {
  const out = {};
  for (const entry of entries) {
    const value = String(entry[key] || "<missing>");
    out[value] = (out[value] || 0) + 1;
  }
  return Object.fromEntries(Object.entries(out).sort(([left], [right]) => left.localeCompare(right)));
}

function groupKey(entry) {
  return [
    entry.owner_role,
    entry.phase,
    entry.side_effect_class,
    entry.replacement_bundle,
  ].join("|");
}

function buildGroups(entries = []) {
  const groups = new Map();
  for (const entry of entries) {
    if (!["CONSOLIDATE_WHEN_TOUCHED", "KEEP_COMPATIBILITY_ALIAS_WITH_REPLACEMENT"].includes(entry.consolidation_status)) continue;
    const key = groupKey(entry);
    if (!groups.has(key)) {
      groups.set(key, {
        group_id: `public-surface-group:${sha256Short(key)}`,
        owner_role: entry.owner_role,
        phase: entry.phase,
        side_effect_class: entry.side_effect_class,
        replacement_bundle: entry.replacement_bundle,
        policy: "Prefer extending the replacement bundle over preserving separate public leaves.",
        surfaces: [],
      });
    }
    groups.get(key).surfaces.push(entry.surface_id);
  }
  return [...groups.values()]
    .map((group) => ({
      ...group,
      count: group.surfaces.length,
      surfaces: group.surfaces.sort((left, right) => left.localeCompare(right)),
    }))
    .sort((left, right) =>
      right.count - left.count
      || left.owner_role.localeCompare(right.owner_role)
      || left.phase.localeCompare(right.phase)
      || left.replacement_bundle.localeCompare(right.replacement_bundle)
    );
}

function sourceTopologyProjectionHash(entries = []) {
  const sourceProjection = entries.map((entry) => ({
    authority_boundary: entry.authority_boundary,
    entrypoint_status: entry.entrypoint_status,
    alias_target_recipes: entry.alias_target_recipes,
    just_recipes: entry.just_recipes,
    owner_role: entry.owner_role,
    path: entry.path,
    phase: entry.phase,
    primary_debug_artifact: entry.primary_debug_artifact,
    public_exposure: entry.public_exposure,
    replacement_bundle: entry.replacement_bundle,
    side_effect_class: entry.side_effect_class,
    surface_id: entry.surface_id,
    surface_kind: entry.surface_kind,
    validation_coverage: entry.validation_coverage,
  }));
  return `sha256:${crypto
    .createHash("sha256")
    .update(stableStringify({
      projection_scope: "public_surface_entries_excluding_generated_ledger_self_reference",
      schema_version: "public_surface_source_projection_v1",
      entries: sourceProjection,
    }))
    .digest("hex")}`;
}

export function buildPublicSurfaceConsolidation(topology = buildGovernanceTopology()) {
  const recipeByName = new Map(parseJustRecipes().map((recipe) => [recipe.name, recipe]));
  const entries = (topology.surfaces || [])
    .filter(isPublicLikeSurface)
    .map((surface) => publicEntry(surface, recipeByName))
    .sort((left, right) => left.surface_id.localeCompare(right.surface_id));
  const groups = buildGroups(entries);
  const totals = {
    public_surfaces: entries.length,
    canonical: entries.filter((entry) => entry.consolidation_status === "RETAIN_CANONICAL").length,
    contract_authority: entries.filter((entry) => entry.consolidation_status === "RETAIN_CONTRACT_AUTHORITY").length,
    compatibility_aliases: entries.filter((entry) => entry.consolidation_status === "KEEP_COMPATIBILITY_ALIAS_WITH_REPLACEMENT").length,
    consolidate_when_touched: entries.filter((entry) => entry.consolidation_status === "CONSOLIDATE_WHEN_TOUCHED").length,
    tracked_public_surfaces: entries.filter((entry) => entry.consolidation_status === "TRACK_PUBLIC_SURFACE").length,
    consolidation_groups: groups.length,
  };
  return {
    schema_id: "handshake.gov.public_surface_consolidation",
    schema_version: "public_surface_consolidation_v1",
    generated_by: "public-surface-consolidation.mjs",
    generated_at_utc: null,
    source_topology_projection_hash: sourceTopologyProjectionHash(entries),
    policy: {
      rgf_id: "RGF-300",
      purpose: "Execute public Justfile and leaf-surface reduction without deleting live operator surfaces prematurely.",
      default_action: "Keep commands usable, but route touched public leaves through canonical bundles or record explicit exceptions.",
      no_destructive_deletion: true,
      public_surface_rule: "No new public governance surface may exist without topology and consolidation metadata.",
    },
    totals,
    counts: {
      by_status: countBy(entries, "consolidation_status"),
      by_entrypoint_status: countBy(entries, "entrypoint_status"),
      by_owner_role: countBy(entries, "owner_role"),
      by_phase: countBy(entries, "phase"),
      by_side_effect_class: countBy(entries, "side_effect_class"),
    },
    consolidation_groups: groups,
    entries,
    consolidation_hash: `sha256:${sha256Short(stableStringify({ totals, groups, entries }))}`,
  };
}

export function writePublicSurfaceConsolidation() {
  const record = buildPublicSurfaceConsolidation();
  const absPath = repoPathAbs(PUBLIC_SURFACE_CONSOLIDATION_PATH);
  fs.mkdirSync(path.dirname(absPath), { recursive: true });
  fs.writeFileSync(absPath, stableStringify(record), "utf8");
  return record;
}

function main() {
  const sync = process.argv.includes("--sync");
  const record = sync ? writePublicSurfaceConsolidation() : buildPublicSurfaceConsolidation();
  console.log(`public-surface-consolidation ${sync ? "synced" : "ok"}: ${record.totals.public_surfaces} public surface(s), ${record.totals.consolidate_when_touched} consolidate-when-touched`);
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (invokedPath && path.resolve(fileURLToPath(import.meta.url)) === invokedPath) {
  main();
}
