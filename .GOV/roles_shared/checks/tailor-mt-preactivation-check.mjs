#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { readResolvedSpecTextAtRepo, resolveSpecCurrentAtRepo } from "../scripts/lib/spec-current-lib.mjs";

const ROOT = process.cwd();
const WP_ID = "WP-KERNEL-010-Tailor-Cloth-Garment-Engine-v1";
const PACKET_DIR = path.join(ROOT, ".GOV", "task_packets", WP_ID);
const INDEX_PATH = path.join(PACKET_DIR, "_MT_INDEX.json");
const REVIEW_PATH = path.join(PACKET_DIR, "_MULTI_LENS_REVIEW.json");
const PARITY_PATH = path.join(PACKET_DIR, "_PARITY_REVIEW_V3.json");
const BUILD_READY_PATH = path.join(PACKET_DIR, `${WP_ID}.build-readiness-prework.json`);
const REFINEMENT_PREWORK_PATH = path.join(PACKET_DIR, `${WP_ID}.technical-refinement-prework.json`);
const STUB_PATH = path.join(ROOT, ".GOV", "task_packets", "stubs", `${WP_ID}.contract.json`);
const EXPECTED_TOTAL = 842;
const EXPECTED_IDS = Array.from({ length: EXPECTED_TOTAL }, (_, index) => `MT-${String(index + 1).padStart(3, "0")}`);
const EXPECTED_ID_SET = new Set(EXPECTED_IDS);
const DELTA_IDS = Array.from({ length: 103 }, (_, index) => `MT-${String(index + 740).padStart(3, "0")}`);
const RECONCILIATION_STATUS = "SPEC_V02_203_RECONCILED_BODYKIT_V2_DAG_CANDIDATE";
const ACTIVE_SPEC_VERSION = "v02.203";
const ACTIVE_SPEC_ENTRYPOINT = ".GOV/spec/master-spec-v02.203/indexed-spec-manifest.json";
const CAPABILITY_REQUIREMENT_VALUES = ["required", "accepted_exclusion", "optional", "deliberate_exceedance"];
const CAPABILITY_QUALIFICATION_VALUES = ["unimplemented", "implemented_unqualified", "qualified", "stale", "failed", "unsupported"];
const REQUIRED_MANUAL_EVIDENCE = [
  "user_manual_diff",
  "manual_self_consistency_test",
  "manual_inspection_or_no_context_operation_test",
  "hbr_int_009_diagnostic_posture_recorded",
];
const REQUIRED_INTERFACE_TARGETS = new Map([
  ["MT-740", ["tailor/capabilities"]],
  ["MT-741", ["tailor/commands"]],
  ["MT-742", ["tailor/governance/operation_contract"]],
  ["MT-743", ["tailor/diagnostics"]],
  ["MT-747", ["user_manual/tailor"]],
  ["MT-748", ["tailor/surface_registry"]],
  ["MT-749", ["tailor/interlink/mix"]],
  ["MT-750", ["surface_extension_seam", "handshake_native/src/tailor"]],
  ["MT-780", ["tailor/contract_gate"]],
  ["MT-782", ["implementation_profile"]],
  ["MT-783", ["tailor/dependencies"]],
  ["MT-794", ["bodykit/providers"]],
  ["MT-808", ["bodykit/performance/track"]],
  ["MT-821", ["tailor/interchange/profile"]],
  ["MT-831", ["tailor/module_registration"]],
  ["MT-840", ["tailor/projects_v2"]],
]);
const EXPECTED_DIAGNOSTIC_POSTURES = new Map();
const REQUIRED_DELTA_ANCHORS = new Map([
  ["MT-783", ["TAI-V2-RUN-008", "TAI-V2-QA-005"]],
  ["MT-794", ["TAI-V2-BDY-001", "TAI-V2-BDY-002"]],
  ["MT-808", ["TAI-V2-PERF-001"]],
  ["MT-821", ["TAI-V2-IO-001"]],
  ["MT-831", ["TAI-V2-UX-001"]],
  ["MT-840", ["TAI-V2-PRJ-003", "TAI-V2-PRJ-006"]],
  ["MT-842", ["TAI-V2-QA-008", "TAI-V2-QA-009"]],
]);
const failures = [];

function fail(message) {
  failures.push(message);
}

function readJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (error) {
    fail(`${path.relative(ROOT, filePath)}: ${error.message}`);
    return null;
  }
}

function sameStringArray(left, right) {
  return Array.isArray(left) && Array.isArray(right) && left.length === right.length && left.every((value, index) => value === right[index]);
}

function anchorRange(prefix, end) {
  return Array.from({ length: end }, (_, index) => `${prefix}-${String(index + 1).padStart(3, "0")}`);
}

const REQUIRED_SPEC_ANCHORS = [
  ...anchorRange("TAI-PRO", 6),
  ...anchorRange("TAI-PHY", 7),
  ...anchorRange("TAI-ACT", 10),
  ...anchorRange("TAI-CLP", 7),
  ...anchorRange("TAI-BKP", 11),
  ...anchorRange("TAI-INT", 6),
  ...anchorRange("TAI-UX", 5),
  ...anchorRange("TAI-QA", 7),
  ...anchorRange("TAI-GATE", 5),
  ...anchorRange("TAI-V2-RUN", 10),
  ...anchorRange("TAI-V2-BDY", 12),
  "TAI-V2-BDY-002A",
  ...anchorRange("TAI-V2-PERF", 10),
  ...anchorRange("TAI-V2-IO", 10),
  ...anchorRange("TAI-V2-PRJ", 10),
  ...anchorRange("TAI-V2-ACT", 7),
  ...anchorRange("TAI-V2-UX", 12),
  ...anchorRange("TAI-V2-QA", 9),
  ...anchorRange("TAI-V2-RES", 3),
];

let resolvedSpec = null;
let activeSpecText = "";
try {
  resolvedSpec = resolveSpecCurrentAtRepo(ROOT, { allowLegacy: false });
  activeSpecText = readResolvedSpecTextAtRepo(ROOT, resolvedSpec);
} catch (error) {
  fail(`active spec resolution/health failed: ${error.message}`);
}

for (const requiredPath of [INDEX_PATH, REVIEW_PATH, PARITY_PATH, BUILD_READY_PATH, REFINEMENT_PREWORK_PATH, STUB_PATH]) {
  if (!fs.existsSync(requiredPath)) fail(`missing required artifact ${path.relative(ROOT, requiredPath)}`);
}

const index = readJson(INDEX_PATH);
const review = readJson(REVIEW_PATH);
const parity = readJson(PARITY_PATH);
const buildReady = readJson(BUILD_READY_PATH);
const refinement = readJson(REFINEMENT_PREWORK_PATH);
const stub = readJson(STUB_PATH);

if (!index || !review || !parity || !buildReady || !refinement || !stub) {
  console.error(failures.join("\n"));
  process.exit(1);
}

if (resolvedSpec && activeSpecText) {
  if (resolvedSpec.versionTag !== ACTIVE_SPEC_VERSION) fail(`active spec version is ${resolvedSpec.versionTag}, expected ${ACTIVE_SPEC_VERSION}`);
  if (resolvedSpec.specEntryPointPath !== ACTIVE_SPEC_ENTRYPOINT) fail(`active spec entrypoint is ${resolvedSpec.specEntryPointPath}`);
  for (const anchor of REQUIRED_SPEC_ANCHORS) {
    if (!activeSpecText.includes(`[${anchor}]`)) fail(`active v02.203 spec is missing exact anchor [${anchor}]`);
  }
  if (!activeSpecText.includes("ReferenceCpuF64") || !activeSpecText.includes("InteractiveGpuXpbd") || !activeSpecText.includes("FinalCpuBarrierF64")) {
    fail("active v02.203 spec does not preserve all three named Tailor solver lanes");
  }
  if (!/reference oracle is not a production-performance tier/i.test(activeSpecText) || !/two production tiers/i.test(activeSpecText)) fail("active v02.203 spec does not preserve the reference-oracle/two-production-tier distinction");
  const capabilitySpecStart = activeSpecText.indexOf("[TAI-PRO-001]");
  const capabilitySpecEnd = activeSpecText.indexOf("[TAI-PRO-002]", capabilitySpecStart);
  const capabilitySpec = capabilitySpecStart >= 0 && capabilitySpecEnd > capabilitySpecStart ? activeSpecText.slice(capabilitySpecStart, capabilitySpecEnd) : "";
  if (!/requirement_status:\s*required\s*\|\s*accepted_exclusion\s*\|\s*optional\s*\|\s*deliberate_exceedance/i.test(capabilitySpec)) fail("active v02.201 TAI-PRO-001 requirement_status enum is missing or drifted");
  if (!/qualification_status:\s*unimplemented\s*\|\s*implemented_unqualified\s*\|\s*qualified\s*\|\s*stale\s*\|\s*failed\s*\|\s*unsupported/i.test(capabilitySpec)) fail("active v02.201 TAI-PRO-001 qualification_status enum is missing or drifted");
  if (/requirement_class/.test(capabilitySpec)) fail("active v02.201 TAI-PRO-001 retains legacy requirement_class");
}

if (index.schema !== "tailor.mt_index@1") fail(`index schema is ${index.schema}`);
if (index.wp_id !== WP_ID) fail(`index wp_id is ${index.wp_id}`);
if (index.total !== EXPECTED_TOTAL) fail(`index total is ${index.total}, expected ${EXPECTED_TOTAL}`);
if (index.range !== "MT-001..MT-842") fail(`index range is ${index.range}`);
if (!Array.isArray(index.microtasks) || index.microtasks.length !== EXPECTED_TOTAL) fail(`index microtask count is ${index.microtasks?.length}`);

const indexIds = index.microtasks?.map((item) => item.mt_id) || [];
if (!sameStringArray(indexIds, EXPECTED_IDS)) fail("index ids are not exactly MT-001..MT-842 in order");
if (new Set(indexIds).size !== EXPECTED_TOTAL) fail("index contains duplicate MT ids");

const contracts = new Map();
for (const id of EXPECTED_IDS) {
  const filePath = path.join(PACKET_DIR, `${id}.json`);
  if (!fs.existsSync(filePath)) {
    fail(`${id}: contract file missing`);
    continue;
  }
  const mt = readJson(filePath);
  if (!mt) continue;
  contracts.set(id, mt);

  if (mt.schema_id !== "hsk.microtask_contract@1") fail(`${id}: wrong schema_id ${mt.schema_id}`);
  if (mt.schema_version !== "microtask_contract_v1") fail(`${id}: wrong schema_version ${mt.schema_version}`);
  if (mt.wp_id !== WP_ID || mt.mt_id !== id) fail(`${id}: identity mismatch ${mt.wp_id}/${mt.mt_id}`);
  if (mt.artifact_policy?.projection_creation !== "ON_OPERATOR_REQUEST_ONLY") fail(`${id}: JSON-first projection policy drift`);
  if (mt.markdown_projection?.path !== "NOT_GENERATED_BY_DEFAULT") fail(`${id}: projection path must be opt-out before activation`);
  if (mt.markdown_projection?.status !== "NOT_GENERATED_BY_DEFAULT") fail(`${id}: projection status must be opt-out before activation`);
  if (mt.markdown_projection?.source_hash !== "NOT_GENERATED_BY_DEFAULT") fail(`${id}: projection source hash must be opt-out before activation`);
  if (mt.markdown_projection?.projection_hash !== "NOT_GENERATED_BY_DEFAULT") fail(`${id}: projection hash must be opt-out before activation`);
  if (mt.markdown_projection?.generator !== "PENDING_DEMAND") fail(`${id}: projection generator must remain demand-gated`);
  if (mt.lifecycle?.status !== "PENDING" || mt.lifecycle?.active !== false || mt.lifecycle?.validator_verdict !== "PENDING") fail(`${id}: pre-activation lifecycle drift`);
  if (!Array.isArray(mt.lifecycle?.depends_on)) fail(`${id}: depends_on missing`);
  for (const dependency of mt.lifecycle?.depends_on || []) {
    if (!EXPECTED_ID_SET.has(dependency)) fail(`${id}: missing dependency ${dependency}`);
    if (dependency === id) fail(`${id}: self dependency`);
  }

  if (!mt.scope?.summary || !Array.isArray(mt.scope?.allowed_paths) || mt.scope.allowed_paths.length === 0) fail(`${id}: scope summary/allowed_paths missing`);
  if (!Array.isArray(mt.scope?.acceptance_criteria) || mt.scope.acceptance_criteria.length < 3) fail(`${id}: fewer than three acceptance criteria`);
  if (!Array.isArray(mt.scope?.proof_targets) || mt.scope.proof_targets.length === 0) fail(`${id}: proof_targets missing`);
  if (!mt.scope?.risk_if_missed) fail(`${id}: risk_if_missed missing`);
  for (const allowedPath of mt.scope?.allowed_paths || []) {
    const normalized = String(allowedPath).replaceAll("\\", "/");
    if (/^\.GOV\/(?:fixtures|testdata)(?:\/|$)/i.test(normalized) || (/^\.GOV\//i.test(normalized) && /\/(?:fixtures|testdata)(?:\/|$)/i.test(normalized))) {
      fail(`${id}: runtime/test fixture is placed under governance instead of a product testdata root: ${allowedPath}`);
    }
  }
  for (const target of REQUIRED_INTERFACE_TARGETS.get(id) || []) {
    const normalizedPaths = mt.scope.allowed_paths.map((item) => String(item).replaceAll("\\", "/").toLowerCase());
    if (!normalizedPaths.some((item) => item.includes(target.toLowerCase()))) fail(`${id}: canonical interface target is not owned by an allowed product path: ${target}`);
  }

  if (!mt.user_manual_obligation || !Array.isArray(mt.user_manual_obligation.target_entries)) fail(`${id}: user manual obligation missing`);
  if (mt.user_manual_obligation?.required) {
    if (mt.user_manual_obligation.target_entries.length === 0) fail(`${id}: required manual targets empty`);
    for (const target of mt.user_manual_obligation.target_entries) {
      if (/MT-\d{3}/.test(target)) fail(`${id}: synthetic MT-number manual target ${target}`);
      if (/ModelManual/i.test(target)) fail(`${id}: legacy ModelManual is a primary target ${target}`);
      if (!target.startsWith("Tailor UserManual > ")) fail(`${id}: non-canonical manual target ${target}`);
      const taskLabel = target.replace(/^Tailor UserManual >\s*/, "").trim();
      if (/^(?:overview|commands?|diagnostics?|reference|model operation|tailor)$/i.test(taskLabel)) fail(`${id}: manual target is not task-specific: ${target}`);
    }
    for (const evidence of REQUIRED_MANUAL_EVIDENCE) {
      if (!mt.user_manual_obligation.expected_evidence?.includes(evidence)) fail(`${id}: required manual evidence is missing ${evidence}`);
    }
  } else {
    if (!mt.user_manual_obligation?.not_applicable_reason) fail(`${id}: non-required manual obligation lacks a bounded not-applicable reason`);
    if (mt.user_manual_obligation?.target_entries?.length !== 0) fail(`${id}: non-required manual obligation still owns manual targets`);
  }

  if (!Array.isArray(mt.hbr_obligations) || mt.hbr_obligations.length === 0) fail(`${id}: hbr_obligations empty`);
  const validHbr = new Set(["HBR-INT", "HBR-SWARM", "HBR-VIS", "HBR-QUIET", "HBR-MAN", "HBR-STOP", "HBR-PRIV"]);
  for (const obligation of mt.hbr_obligations || []) if (!validHbr.has(obligation)) fail(`${id}: invalid HBR obligation ${obligation}`);
  if (!mt.hbr_obligations?.includes("HBR-PRIV")) fail(`${id}: HBR-PRIV obligation missing`);
  if (mt.resource_privacy_obligation?.required !== true) fail(`${id}: resource_privacy_obligation missing or not required`);
  for (const row of ["HBR-PRIV-001", "HBR-PRIV-002", "HBR-PRIV-003", "HBR-PRIV-004", "HBR-PRIV-005", "HBR-PRIV-006", "HBR-PRIV-007", "HBR-PRIV-008"]) {
    if (!mt.resource_privacy_obligation?.hbr_rows?.includes(row)) fail(`${id}: resource privacy row missing ${row}`);
  }

  const tiers = mt.hbr_int_009_tier_obligations;
  if (!Array.isArray(tiers) || tiers.length !== 3) fail(`${id}: expected exactly three diagnostic tiers`);
  const tierNames = new Set((tiers || []).map((item) => item.tier));
  for (const requiredTier of ["flight_recorder", "internal_diagnostics", "palmistry"]) if (!tierNames.has(requiredTier)) fail(`${id}: missing diagnostic tier ${requiredTier}`);
  for (const tier of tiers || []) {
    if (!new Set(["DIRECT", "INHERITED", "NOT_APPLICABLE"]).has(tier.posture)) fail(`${id}: invalid/deferred diagnostic posture ${tier.tier}=${tier.posture}`);
    if (!tier.reason) fail(`${id}: missing diagnostic reason for ${tier.tier}`);
    if (/defer(?:red)?|decide later|future MT|to be determined|TBD/i.test(tier.reason || "")) fail(`${id}: diagnostic reason defers classification for ${tier.tier}`);
  }
  const expectedPostures = EXPECTED_DIAGNOSTIC_POSTURES.get(id);
  if (expectedPostures) {
    const actualPostures = Object.fromEntries((tiers || []).map((item) => [item.tier, item.posture]));
    for (const [tierName, expectedPosture] of Object.entries(expectedPostures)) {
      if (actualPostures[tierName] !== expectedPosture) fail(`${id}: ${tierName} posture is ${actualPostures[tierName]}, expected ${expectedPosture} for its operation class`);
    }
  }

  const gui = mt.gui_obligation;
  if (!gui) fail(`${id}: gui_obligation missing`);
  if (gui?.gui_creation_required) {
    const paths = mt.scope.allowed_paths.join(" ").toLowerCase();
    if (!paths.includes("handshake_native") && !paths.includes("src/frontend/")) fail(`${id}: GUI creation without native frontend allowed path`);
    if (!gui.operator_surface_required || !gui.argus_required) fail(`${id}: GUI creation missing operator/Argus requirement`);
    if (!Array.isArray(gui.surfaces) || gui.surfaces.length === 0) fail(`${id}: GUI surfaces empty`);
    if (!Array.isArray(gui.argus_targets) || gui.argus_targets.length === 0) fail(`${id}: Argus targets empty`);
    for (const target of gui.argus_targets || []) {
      if (/^every control and visible state/i.test(target)) fail(`${id}: wildcard-only Argus target remains`);
    }
    if (!mt.scope.proof_targets.some((target) => /handshake-native|argus/i.test(target))) fail(`${id}: GUI creation lacks native/Argus proof target`);
  } else if (!gui?.trace_projection_required_when_non_ui) {
    fail(`${id}: non-UI contract lacks TraceProjection obligation`);
  }

  if (!mt.pre_activation_reconciliation || mt.pre_activation_reconciliation.dependency_graph_status !== RECONCILIATION_STATUS) {
    fail(`${id}: pre-activation reconciliation status not hardened`);
  }

  if (id === "MT-740") {
    const capabilityContract = [mt.scope.summary, ...(mt.scope.acceptance_criteria || []), ...(mt.scope.proof_targets || [])].join("\n");
    if (!/requirement_status/.test(capabilityContract) || !/qualification_status/.test(capabilityContract)) {
      fail("MT-740: capability contract does not expose independent requirement and qualification dimensions");
    }
    if (/requirement_class/.test(capabilityContract)) fail("MT-740: legacy requirement_class remains; canonical field is requirement_status");
    if (/\b(?:REQUIRED|ACCEPTED_EXCLUSION|OPTIONAL|DELIBERATE_EXCEEDANCE|UNIMPLEMENTED|IMPLEMENTED_UNQUALIFIED|QUALIFIED|STALE|FAILED|UNSUPPORTED)\b/.test(capabilityContract)) fail("MT-740: uppercase legacy capability enum remains");
    for (const value of [...CAPABILITY_REQUIREMENT_VALUES, ...CAPABILITY_QUALIFICATION_VALUES]) {
      if (!capabilityContract.includes(value)) fail(`MT-740: canonical capability enum value is missing ${value}`);
    }
    if (!/neither[^\n]*(?:infer|derived)|must not[^\n]*(?:infer|derived)/i.test(capabilityContract)) {
      fail("MT-740: capability contract does not prohibit deriving one capability dimension from the other");
    }
  }

  if (id === "MT-842") {
    if (!gui?.gui_creation_required || !gui?.operator_surface_required || !gui?.argus_required) fail("MT-842: final BodyKit-v2 lifecycle lacks direct GUI/Argus proof ownership");
    if (!mt.scope.allowed_paths.some((item) => /handshake_native/i.test(item))) fail("MT-842: final BodyKit-v2 lifecycle lacks a native GUI product path");
    const finalProof = [...(mt.scope.acceptance_criteria || []), ...(mt.scope.proof_targets || [])].join("\n");
    for (const proofTerm of ["Argus", "AccessKit", "accessibility", "recovery", "renderer-console", "no-context", "GUI-only", "backend-only"]) {
      if (!finalProof.toLowerCase().includes(proofTerm.toLowerCase())) fail(`MT-842: final proof omits ${proofTerm}`);
    }
  }

  if (id === "MT-749") {
    const mixContract = [mt.scope.summary, ...(mt.scope.acceptance_criteria || []), ...(mt.scope.proof_targets || [])].join("\n");
    for (const requiredTerm of ["TAI-INT-006", "MT-710", "dependency_unavailable"]) {
      if (!mixContract.includes(requiredTerm)) fail(`MT-749: PoseKit containment contract omits ${requiredTerm}`);
    }
  }

  const indexRow = index.microtasks.find((item) => item.mt_id === id);
  if (!indexRow) fail(`${id}: missing index row`);
  else {
    if (indexRow.status !== mt.lifecycle.status) fail(`${id}: index status drift`);
    if (indexRow.summary !== mt.scope.summary) fail(`${id}: index summary drift`);
    if (!sameStringArray(indexRow.depends_on, mt.lifecycle.depends_on)) fail(`${id}: index dependency drift`);
  }
}

const visiting = new Set();
const visited = new Set();
const stack = [];
function visit(id) {
  if (visited.has(id)) return;
  if (visiting.has(id)) {
    const start = stack.indexOf(id);
    fail(`dependency cycle: ${[...stack.slice(start), id].join(" -> ")}`);
    return;
  }
  visiting.add(id);
  stack.push(id);
  for (const dependency of contracts.get(id)?.lifecycle?.depends_on || []) visit(dependency);
  stack.pop();
  visiting.delete(id);
  visited.add(id);
}
for (const id of EXPECTED_IDS) visit(id);

function dependsTransitively(id, target, seen = new Set()) {
  if (seen.has(id)) return false;
  seen.add(id);
  for (const dependency of contracts.get(id)?.lifecycle?.depends_on || []) {
    if (dependency === target || dependsTransitively(dependency, target, seen)) return true;
  }
  return false;
}

for (const [id, mt] of contracts) {
  if (mt.gui_obligation?.gui_creation_required && id !== "MT-750") {
    if (!dependsTransitively(id, "MT-748")) fail(`${id}: GUI producer does not consume the canonical Tailor surface registry (MT-748)`);
    if (!dependsTransitively(id, "MT-750")) fail(`${id}: GUI producer does not consume the native shell seam gate (MT-750)`);
  }
}
if (!dependsTransitively("MT-749", "MT-710")) fail("MT-749: PoseKit containment proof is not reachable transitively from MT-710");

const countedGroups = {};
for (const item of index.microtasks) countedGroups[item.group] = (countedGroups[item.group] || 0) + 1;
if (JSON.stringify(countedGroups) !== JSON.stringify(index.by_group)) fail("index by_group counts drift from rows");

if (review.schema !== "tailor.multi_lens_review@1") fail(`review schema is ${review.schema}`);
if (!Array.isArray(review.per_mt) || review.per_mt.length !== EXPECTED_TOTAL) fail(`review per_mt count is ${review.per_mt?.length}`);
const reviewIds = review.per_mt?.map((item) => item.mt_id) || [];
if (!sameStringArray(reviewIds, EXPECTED_IDS) || new Set(reviewIds).size !== EXPECTED_TOTAL) fail("review does not account for every MT exactly once in order");
const universalReviewLenses = ["feature_scope", "backend_parallel_agents", "authority_artifacts_events", "diagnostics_palmistry", "user_manual_no_context", "flight_recorder", "resource_privacy", "recovery_accessibility_quiet", "dependency_dag"];
for (const item of review.per_mt || []) {
  if (!Array.isArray(item.lenses_applied) || item.lenses_applied.length < universalReviewLenses.length) {
    fail(`${item.mt_id}: multi-lens review does not record the universal review set`);
    continue;
  }
  for (const lens of universalReviewLenses) {
    if (!item.lenses_applied.includes(lens)) fail(`${item.mt_id}: multi-lens review omitted ${lens}`);
  }
  if (item.gui_required && !item.lenses_applied.includes("professional_ui_gui")) fail(`${item.mt_id}: GUI MT lacks professional_ui_gui review`);
  const mt = contracts.get(item.mt_id);
  if (!mt) continue;
  if (item.group !== index.microtasks.find((row) => row.mt_id === item.mt_id)?.group) fail(`${item.mt_id}: review group drift`);
  if (item.acceptance_criteria_count !== mt.scope.acceptance_criteria.length) fail(`${item.mt_id}: review acceptance-criteria count drift`);
  if (item.proof_target_count !== mt.scope.proof_targets.length) fail(`${item.mt_id}: review proof-target count drift`);
  if (item.gui_required !== mt.gui_obligation.gui_creation_required) fail(`${item.mt_id}: review GUI classification drift`);
  if (!sameStringArray(item.gui_targets, mt.gui_obligation.argus_targets)) fail(`${item.mt_id}: review GUI/Argus targets drift`);
  if (!sameStringArray(item.manual_targets, mt.user_manual_obligation.target_entries)) fail(`${item.mt_id}: review manual targets drift`);
  if (!sameStringArray(item.hbr_obligations, mt.hbr_obligations)) fail(`${item.mt_id}: review HBR obligations drift`);
  if (!sameStringArray(item.dependencies, mt.lifecycle.depends_on)) fail(`${item.mt_id}: review dependencies drift`);
  const currentPostures = Object.fromEntries(mt.hbr_int_009_tier_obligations.map((tier) => [tier.tier, tier.posture]));
  if (JSON.stringify(item.diagnostic_postures) !== JSON.stringify(currentPostures)) fail(`${item.mt_id}: review diagnostic posture drift`);
}
if (review.review_authority !== "ADVISORY_PRE_ACTIVATION_ONLY_NO_VALIDATOR_VERDICT") fail("review improperly claims validator authority");
if (review.hardening_summary?.diagnostic_deferrals_remaining !== 0 || review.hardening_summary?.synthetic_mt_number_manual_targets_remaining !== 0 || review.hardening_summary?.empty_hbr_obligations_remaining !== 0) fail("review summary records unresolved systemic contract defects");

if (parity.schema !== "tailor.parity_review@3") fail(`parity schema is ${parity.schema}`);
if (!String(parity.baseline?.marvelous_designer).includes("NOT_INSPECTED")) fail("Marvelous local binary is presented as inspected without evidence");
if (!String(parity.claim_law).includes("runtime") || !String(parity.claim_law).includes("QUALIFIED")) fail("parity claim law is not proof-gated");

if (buildReady.status !== "SPEC_V02_203_RECONCILED_CONTRACT_CANDIDATE_PENDING_PACKET_AND_ACTIVATION") fail(`build-readiness status is ${buildReady.status}`);
if (buildReady.implementation_authority !== false || buildReady.activation_changed !== false || buildReady.active_spec_changed !== true) fail("build-readiness prework misstates non-execution or completed spec transition");
if (buildReady.active_spec_version !== ACTIVE_SPEC_VERSION || buildReady.active_spec_entrypoint !== ACTIVE_SPEC_ENTRYPOINT) fail("build-readiness active spec identity drift");
if (!String(buildReady.remaining_external_input).includes("NOT_INSPECTED")) fail("build-readiness prework hides unresolved Marvelous reference input");

if (refinement.status !== "OPERATOR_APPROVED_SPEC_V02_203_RECORDED_NON_EXECUTION_REFINEMENT" || refinement.execution_authority !== false) fail("technical refinement status/authority does not match active-v02.203 non-execution truth");
if (!Array.isArray(refinement.microtask_plan) || refinement.microtask_plan.length !== EXPECTED_TOTAL) fail(`technical refinement microtask plan count is ${refinement.microtask_plan?.length}`);
if (!Array.isArray(refinement.approved_spec_enrichment) || refinement.approved_spec_enrichment.length === 0) fail("technical refinement does not record the applied v02.203 enrichment");
if (!Array.isArray(refinement.proposed_spec_enrichment) || refinement.proposed_spec_enrichment.length !== 0) fail("technical refinement still presents the applied v02.203 enrichment as proposed");
const refinementText = JSON.stringify(refinement);
for (const lane of ["ReferenceCpuF64", "InteractiveGpuXpbd", "FinalCpuBarrierF64"]) if (!refinementText.includes(lane)) fail(`technical refinement omits solver lane ${lane}`);
if (!/non-production ReferenceCpuF64/i.test(refinementText) || !/two production tiers/i.test(refinementText)) fail("technical refinement does not distinguish the CPU oracle from the two production tiers");
if (refinement.capability_dimension_contract?.requirement_field !== "requirement_status" || refinement.capability_dimension_contract?.qualification_field !== "qualification_status") fail("technical refinement capability dimension field names drift from v02.203");
if (!sameStringArray(refinement.capability_dimension_contract?.requirement_values, CAPABILITY_REQUIREMENT_VALUES)) fail("technical refinement requirement_status enum drifts from v02.203");
if (!sameStringArray(refinement.capability_dimension_contract?.qualification_values, CAPABILITY_QUALIFICATION_VALUES)) fail("technical refinement qualification_status enum drifts from v02.203");
if (!String(refinement.capability_dimension_contract?.independence_rule).includes("neither may be inferred")) fail("technical refinement does not keep capability dimensions independent");

const deltaKeys = Object.keys(refinement.spec_delta_map || {});
if (!sameStringArray(deltaKeys, DELTA_IDS)) fail("technical refinement spec_delta_map is not exactly MT-740..MT-842 in order");
for (const id of DELTA_IDS) {
  const delta = refinement.spec_delta_map?.[id];
  if (!delta || !Array.isArray(delta.anchors) || delta.anchors.length === 0) {
    fail(`${id}: spec_delta_map anchors missing`);
    continue;
  }
  for (const anchor of delta.anchors) {
    if (!REQUIRED_SPEC_ANCHORS.includes(anchor)) fail(`${id}: spec_delta_map uses unknown v02.203 anchor ${anchor}`);
    if (activeSpecText && !activeSpecText.includes(`[${anchor}]`)) fail(`${id}: spec_delta_map anchor is absent from the active spec ${anchor}`);
  }
  for (const requiredAnchor of REQUIRED_DELTA_ANCHORS.get(id) || []) {
    if (!delta.anchors.includes(requiredAnchor)) fail(`${id}: spec_delta_map omits required hardening anchor ${requiredAnchor}`);
  }
  if (!delta.manual_target?.startsWith("Tailor UserManual > ")) fail(`${id}: spec_delta_map task manual target is missing/non-canonical`);
  if (!contracts.get(id)?.user_manual_obligation?.target_entries?.includes(delta.manual_target)) fail(`${id}: spec_delta_map manual target drifts from the MT contract`);
}

const remeshLaw = refinement.legacy_law_reconciliation?.["TAI-PHY-007"];
if (!sameStringArray(remeshLaw?.owner_mts, ["MT-683", "MT-684"]) || remeshLaw?.status !== RECONCILIATION_STATUS) fail("technical refinement does not map TAI-PHY-007 exactly to reconciled MT-683/MT-684");
for (const id of ["MT-683", "MT-684"]) {
  const contractText = JSON.stringify(contracts.get(id)?.scope || {});
  for (const term of ["TAI-PHY-007", "topology", "transfer", "rollback", "atomic"]) {
    if (!contractText.toLowerCase().includes(term.toLowerCase())) fail(`${id}: TAI-PHY-007 transaction/transfer/rollback contract omits ${term}`);
  }
}

for (const gateName of ["wp_kernel_012_native_shell", "wp_ckc_posekit"]) {
  const gate = refinement.dependency_gates?.[gateName];
  if (!gate?.status || !gate.status.includes("BLOCKED_UNTIL_CONTAINED_IN_MAIN")) fail(`technical refinement dependency gate is not fail-closed: ${gateName}`);
  if (!gate?.authority_path || !fs.existsSync(path.join(ROOT, gate.authority_path))) fail(`technical refinement dependency authority path is missing: ${gateName}`);
  if (!Array.isArray(gate?.proof_required) || gate.proof_required.length < 4) fail(`technical refinement dependency proof is incomplete: ${gateName}`);
}
const wp12Packet = readJson(path.join(ROOT, refinement.dependency_gates.wp_kernel_012_native_shell.authority_path));
if (wp12Packet) {
  if (wp12Packet.lifecycle?.status !== "In Progress" || wp12Packet.lifecycle?.main_containment_status !== "NOT_STARTED" || wp12Packet.lifecycle?.current_main_compatibility_status !== "NOT_RUN") fail("WP-KERNEL-012 dependency truth changed; reconcile the Tailor gate before claiming build readiness");
  if (wp12Packet.lifecycle?.activation_status !== "ACTIVATED_IN_PROGRESS") fail("WP-KERNEL-012 activation state is not represented by the Tailor dependency gate");
}
const poseKitPacket = readJson(path.join(ROOT, refinement.dependency_gates.wp_ckc_posekit.authority_path));
if (poseKitPacket) {
  if (poseKitPacket.lifecycle?.main_containment_status !== "NOT_STARTED" || poseKitPacket.lifecycle?.current_main_compatibility_status !== "NOT_RUN" || poseKitPacket.lifecycle?.merged_main_commit !== "NONE") fail("CKC PoseKit containment truth changed; reconcile the Tailor gate before claiming build readiness");
  if (poseKitPacket.wp_validation_v2?.whole_wp_technical_judgment !== "PASS_AT_HEAD" || poseKitPacket.wp_validation_v2?.merge !== "WITHHELD") fail("CKC PoseKit technical-pass/merge-withheld evidence drifted");
}
if (refinement.absorbed_preworks?.status !== "ABSORBED_AND_SUPERSEDED_FOR_CURRENT_BUILD_READINESS_PLANNING_RETAINED_AS_PROVENANCE") fail("technical refinement does not explicitly absorb/supersede older preworks");
for (const retainedPath of refinement.absorbed_preworks?.absorbed_retained_files || []) {
  if (!fs.existsSync(path.join(ROOT, retainedPath))) fail(`absorbed prework was not retained: ${retainedPath}`);
}

if (stub.execution_authority !== "NON_EXECUTION_STUB") fail("stub execution authority changed");
if (stub.lifecycle?.status !== "SPEC_V02_203_APPLIED_BODYKIT_V2_REFINEMENT_APPROVED_HELD_FOR_PACKET_ACTIVATION_AND_DEPENDENCY_GATES") fail(`stub lifecycle does not reflect active-v02.203 held-candidate truth: ${stub.lifecycle?.status}`);
if (stub.lifecycle?.user_signature_required !== true) fail("stub no longer requires a separate activation signature");
if (stub.microtasks?.total !== EXPECTED_TOTAL || stub.activation_status?.microtasks?.total !== EXPECTED_TOTAL) fail("stub MT totals drift");
if (stub.professional_production_hardening?.implementation_authority !== false || stub.professional_production_hardening?.activation_changed !== false) fail("stub hardening block exceeds prep authority");
if (stub.professional_production_hardening?.active_spec_changed !== true || stub.professional_production_hardening?.active_spec_version !== ACTIVE_SPEC_VERSION) fail("stub hides the completed v02.203 active-spec transition");
if (stub.spec_trace?.active_bundle_at_stub_time !== ACTIVE_SPEC_ENTRYPOINT) fail("stub spec trace does not point to active v02.203");
if (stub.native_shell_toolkit_integration?.shell_packet !== "WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1") fail("stub names a stale native-shell packet");
if (!String(stub.native_shell_toolkit_integration?.pane_type).includes("UNVERIFIED")) fail("stub falsely presents the Tailor pane as live before WP-KERNEL-012 containment proof");
if (!String(stub.draft_scope?.historical_draft_scope_status).includes("SUPERSEDED_BY_ACTIVE_v02_203")) fail("stub does not quarantine its retained historical draft scope, acceptance, and render claims");
if (!String(stub.draft_scope?.current_scope_correction).includes("native final-quality rendering")) fail("stub does not record Tailor native final-render ownership");
if (stub.activation_status?.spec_enrichment?.version !== ACTIVE_SPEC_VERSION || stub.activation_status?.spec_enrichment?.status !== "DONE_OPERATOR_APPROVED_ACTIVE_AND_VALIDATED") fail("stub activation status records stale spec-enrichment truth");
if (!String(stub.activation_status?.spec_enrichment?.refinement_approval_evidence).includes("Operator approved")) fail("stub refinement approval evidence is missing or drifted");
if (stub.activation_status?.spec_enrichment?.master_spec_signature_recorded !== true) fail("stub does not record the governed Master Spec signature event");
if (stub.activation_status?.spec_enrichment?.activation_user_signature_consumed !== false || stub.activation_status?.spec_enrichment?.activation_user_signature !== null) fail("stub falsely consumes or records a WP activation signature");

if (fs.existsSync(path.join(PACKET_DIR, "packet.json"))) fail("official packet.json exists during held pre-activation prep");
if (fs.existsSync(path.join(PACKET_DIR, "refinement.json"))) fail("official refinement.json exists during held non-execution prep");

if (failures.length > 0) {
  console.error(`tailor-mt-preactivation-check FAILED (${failures.length})`);
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log(JSON.stringify({
  check: "tailor-mt-preactivation-check",
  status: "PASS",
  authority: "PRE_ACTIVATION_CONTRACT_CHECK_ONLY_NO_WP_VALIDATOR_VERDICT",
  wp_id: WP_ID,
  active_spec_version: resolvedSpec.versionTag,
  active_spec_entrypoint: resolvedSpec.specEntryPointPath,
  active_spec_required_anchor_count: REQUIRED_SPEC_ANCHORS.length,
  mt_total: EXPECTED_TOTAL,
  reconciled_mt_total: [...contracts.values()].filter((mt) => mt.pre_activation_reconciliation.dependency_graph_status === RECONCILIATION_STATUS).length,
  spec_delta_rows: Object.keys(refinement.spec_delta_map).length,
  min_acceptance_criteria: Math.min(...[...contracts.values()].map((mt) => mt.scope.acceptance_criteria.length)),
  gui_mt_count: [...contracts.values()].filter((mt) => mt.gui_obligation.gui_creation_required).length,
  final_gui_argus_proof_owned_by_mt_842: contracts.get("MT-842").gui_obligation.gui_creation_required && contracts.get("MT-842").gui_obligation.argus_required,
  manual_obligations_classified: contracts.size,
  task_specific_manual_targets: [...contracts.values()].filter((mt) => mt.user_manual_obligation.required && mt.user_manual_obligation.target_entries.length > 0).length,
  manual_not_applicable_contracts: [...contracts.values()].filter((mt) => !mt.user_manual_obligation.required).length,
  governance_runtime_fixture_violations: 0,
  diagnostic_deferrals: 0,
  dependency_cycles: 0,
  reviewed_once: review.per_mt.length
}, null, 2));
