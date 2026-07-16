#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();
const WP_ID = "WP-KERNEL-010-Tailor-Cloth-Garment-Engine-v1";
const PACKET_DIR = path.join(ROOT, ".GOV", "task_packets", WP_ID);
const INDEX_PATH = path.join(PACKET_DIR, "_MT_INDEX.json");
const REVIEW_PATH = path.join(PACKET_DIR, "_MULTI_LENS_REVIEW.json");
const PARITY_PATH = path.join(PACKET_DIR, "_PARITY_REVIEW_V3.json");
const BUILD_READY_PATH = path.join(PACKET_DIR, `${WP_ID}.build-readiness-prework.json`);
const REFINEMENT_PREWORK_PATH = path.join(PACKET_DIR, `${WP_ID}.technical-refinement-prework.json`);
const STUB_PATH = path.join(ROOT, ".GOV", "task_packets", "stubs", `${WP_ID}.contract.json`);
const EXPECTED_TOTAL = 782;
const EXPECTED_IDS = Array.from({ length: EXPECTED_TOTAL }, (_, index) => `MT-${String(index + 1).padStart(3, "0")}`);
const EXPECTED_ID_SET = new Set(EXPECTED_IDS);
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

if (index.schema !== "tailor.mt_index@1") fail(`index schema is ${index.schema}`);
if (index.wp_id !== WP_ID) fail(`index wp_id is ${index.wp_id}`);
if (index.total !== EXPECTED_TOTAL) fail(`index total is ${index.total}, expected ${EXPECTED_TOTAL}`);
if (index.range !== "MT-001..MT-782") fail(`index range is ${index.range}`);
if (!Array.isArray(index.microtasks) || index.microtasks.length !== EXPECTED_TOTAL) fail(`index microtask count is ${index.microtasks?.length}`);

const indexIds = index.microtasks?.map((item) => item.mt_id) || [];
if (!sameStringArray(indexIds, EXPECTED_IDS)) fail("index ids are not exactly MT-001..MT-782 in order");
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

  if (!mt.user_manual_obligation || !Array.isArray(mt.user_manual_obligation.target_entries)) fail(`${id}: user manual obligation missing`);
  if (mt.user_manual_obligation?.required) {
    if (mt.user_manual_obligation.target_entries.length === 0) fail(`${id}: required manual targets empty`);
    for (const target of mt.user_manual_obligation.target_entries) {
      if (/MT-\d{3}/.test(target)) fail(`${id}: synthetic MT-number manual target ${target}`);
      if (/ModelManual/i.test(target)) fail(`${id}: legacy ModelManual is a primary target ${target}`);
      if (!target.startsWith("Tailor UserManual > ")) fail(`${id}: non-canonical manual target ${target}`);
    }
  }

  if (!Array.isArray(mt.hbr_obligations) || mt.hbr_obligations.length === 0) fail(`${id}: hbr_obligations empty`);
  const validHbr = new Set(["HBR-INT", "HBR-SWARM", "HBR-VIS", "HBR-QUIET", "HBR-MAN", "HBR-STOP"]);
  for (const obligation of mt.hbr_obligations || []) if (!validHbr.has(obligation)) fail(`${id}: invalid HBR obligation ${obligation}`);

  const tiers = mt.hbr_int_009_tier_obligations;
  if (!Array.isArray(tiers) || tiers.length !== 3) fail(`${id}: expected exactly three diagnostic tiers`);
  const tierNames = new Set((tiers || []).map((item) => item.tier));
  for (const requiredTier of ["flight_recorder", "internal_diagnostics", "palmistry"]) if (!tierNames.has(requiredTier)) fail(`${id}: missing diagnostic tier ${requiredTier}`);
  for (const tier of tiers || []) {
    if (!new Set(["DIRECT", "INHERITED", "NOT_APPLICABLE"]).has(tier.posture)) fail(`${id}: invalid/deferred diagnostic posture ${tier.tier}=${tier.posture}`);
    if (!tier.reason) fail(`${id}: missing diagnostic reason for ${tier.tier}`);
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

  if (!mt.pre_activation_reconciliation || mt.pre_activation_reconciliation.dependency_graph_status !== "PRE_ACTIVATION_BUILD_READY_DAG_CANDIDATE_PENDING_SIGNED_REFINEMENT" && id <= "MT-739") {
    fail(`${id}: pre-activation reconciliation status not hardened`);
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

const countedGroups = {};
for (const item of index.microtasks) countedGroups[item.group] = (countedGroups[item.group] || 0) + 1;
if (JSON.stringify(countedGroups) !== JSON.stringify(index.by_group)) fail("index by_group counts drift from rows");

if (review.schema !== "tailor.multi_lens_review@1") fail(`review schema is ${review.schema}`);
if (!Array.isArray(review.per_mt) || review.per_mt.length !== EXPECTED_TOTAL) fail(`review per_mt count is ${review.per_mt?.length}`);
const reviewIds = review.per_mt?.map((item) => item.mt_id) || [];
if (!sameStringArray(reviewIds, EXPECTED_IDS) || new Set(reviewIds).size !== EXPECTED_TOTAL) fail("review does not account for every MT exactly once in order");
const universalReviewLenses = ["feature_scope", "backend_parallel_agents", "authority_artifacts_events", "diagnostics_palmistry", "user_manual_no_context", "flight_recorder", "recovery_accessibility_quiet", "dependency_dag"];
for (const item of review.per_mt || []) {
  if (!Array.isArray(item.lenses_applied) || item.lenses_applied.length < universalReviewLenses.length) {
    fail(`${item.mt_id}: multi-lens review does not record the universal review set`);
    continue;
  }
  for (const lens of universalReviewLenses) {
    if (!item.lenses_applied.includes(lens)) fail(`${item.mt_id}: multi-lens review omitted ${lens}`);
  }
  if (item.gui_required && !item.lenses_applied.includes("professional_ui_gui")) fail(`${item.mt_id}: GUI MT lacks professional_ui_gui review`);
}
if (review.review_authority !== "ADVISORY_PRE_ACTIVATION_ONLY_NO_VALIDATOR_VERDICT") fail("review improperly claims validator authority");
if (review.hardening_summary?.diagnostic_deferrals_remaining !== 0 || review.hardening_summary?.synthetic_mt_number_manual_targets_remaining !== 0 || review.hardening_summary?.empty_hbr_obligations_remaining !== 0) fail("review summary records unresolved systemic contract defects");

if (parity.schema !== "tailor.parity_review@3") fail(`parity schema is ${parity.schema}`);
if (!String(parity.baseline?.marvelous_designer).includes("NOT_INSPECTED")) fail("Marvelous local binary is presented as inspected without evidence");
if (!String(parity.claim_law).includes("runtime") || !String(parity.claim_law).includes("QUALIFIED")) fail("parity claim law is not proof-gated");

if (buildReady.status !== "CONTRACT_BUILD_READY_PENDING_SIGNATURE_SPEC_AND_ACTIVATION") fail(`build-readiness status is ${buildReady.status}`);
if (buildReady.implementation_authority !== false || buildReady.activation_changed !== false || buildReady.active_spec_changed !== false) fail("build-readiness prework exceeds non-execution authority");
if (!String(buildReady.remaining_external_input).includes("NOT_INSPECTED")) fail("build-readiness prework hides unresolved Marvelous reference input");

if (refinement.status !== "CANDIDATE_COMPLETE_PENDING_UNIQUE_OPERATOR_SIGNATURE" || refinement.execution_authority !== false) fail("technical refinement prework exceeds unsigned candidate authority");
if (!Array.isArray(refinement.microtask_plan) || refinement.microtask_plan.length !== EXPECTED_TOTAL) fail(`technical refinement microtask plan count is ${refinement.microtask_plan?.length}`);
if (refinement.approved_spec_enrichment?.length !== 0) fail("unsigned candidate records approved spec enrichment");
if (!Array.isArray(refinement.proposed_spec_enrichment) || refinement.proposed_spec_enrichment.length === 0) fail("technical refinement lacks proposed spec enrichment");

if (stub.execution_authority !== "NON_EXECUTION_STUB") fail("stub execution authority changed");
if (stub.lifecycle?.status !== "READY_FOR_REFINEMENT") fail(`stub lifecycle changed to ${stub.lifecycle?.status}`);
if (stub.microtasks?.total !== EXPECTED_TOTAL || stub.activation_status?.microtasks?.total !== EXPECTED_TOTAL) fail("stub MT totals drift");
if (stub.professional_production_hardening?.implementation_authority !== false || stub.professional_production_hardening?.activation_changed !== false || stub.professional_production_hardening?.active_spec_changed !== false) fail("stub hardening block exceeds prep authority");

if (fs.existsSync(path.join(PACKET_DIR, "packet.json"))) fail("official packet.json exists during held pre-activation prep");
if (fs.existsSync(path.join(PACKET_DIR, "refinement.json"))) fail("official refinement.json exists during unsigned prep");

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
  mt_total: EXPECTED_TOTAL,
  min_acceptance_criteria: Math.min(...[...contracts.values()].map((mt) => mt.scope.acceptance_criteria.length)),
  gui_mt_count: [...contracts.values()].filter((mt) => mt.gui_obligation.gui_creation_required).length,
  diagnostic_deferrals: 0,
  dependency_cycles: 0,
  reviewed_once: review.per_mt.length
}, null, 2));
