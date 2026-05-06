import fs from "node:fs";
import path from "node:path";
import {
  GOV_ROOT_REPO_REL,
  normalizePath,
  repoPathAbs,
  resolveWorkPacketPath,
  WORK_PACKET_STORAGE_ROOT_REPO_REL,
} from "./runtime-paths.mjs";
import { parsePacketSingleField } from "./scope-surface-lib.mjs";
import {
  addOrReplaceGeneratedProjectionHeader,
  DETERMINISTIC_CONTRACT_RED_TEAM_PROFILE,
  MICRO_TASK_CONTRACT_SCHEMA_ID,
  REFINEMENT_CONTRACT_SCHEMA_ID,
  stableStringify,
  stampContractProjectionMetadata,
  stripGeneratedProjectionHeader,
  WORK_PACKET_CONTRACT_SCHEMA_ID,
} from "./packet-contract-lib.mjs";

function readTextIfExists(absPath = "") {
  try {
    if (!absPath || !fs.existsSync(absPath)) return "";
    return fs.readFileSync(absPath, "utf8");
  } catch {
    return "";
  }
}

function readJsonIfExists(absPath = "") {
  const text = readTextIfExists(absPath);
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function parseStatus(text = "") {
  return (
    (String(text || "").match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (String(text || "").match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || parsePacketSingleField(text, "STATUS")
    || parsePacketSingleField(text, "STUB_STATUS")
    || ""
  ).trim();
}

function splitList(value = "") {
  return String(value || "")
    .split(/[;,]/)
    .map((entry) => entry.trim().replace(/^`|`$/g, ""))
    .filter(Boolean);
}

function markdownProjection(pathValue, status = "LEGACY_AUTHORITY") {
  return {
    path: normalizePath(pathValue),
    status,
    source_file: null,
    source_hash: null,
    projection_hash: null,
    generated_at_utc: null,
    generator: null,
  };
}

function contractPathFor(resolved, fileName) {
  if (!resolved?.isFolder || !resolved.packetDirAbs || !resolved.packetDir) return null;
  return {
    abs: path.join(resolved.packetDirAbs, fileName),
    rel: normalizePath(path.posix.join(resolved.packetDir, fileName)),
  };
}

export function listWorkPacketIdsForContractImport() {
  const rootAbs = repoPathAbs(WORK_PACKET_STORAGE_ROOT_REPO_REL);
  if (!fs.existsSync(rootAbs)) return [];
  const ids = [];
  for (const entry of fs.readdirSync(rootAbs, { withFileTypes: true })) {
    if (entry.isDirectory() && /^WP-/.test(entry.name) && fs.existsSync(path.join(rootAbs, entry.name, "packet.md"))) {
      ids.push(entry.name);
    }
  }
  return ids.sort((left, right) => left.localeCompare(right));
}

export function buildLegacyWorkPacketContract({ wpId, packetText = "", packetPath = "", packetDir = "" } = {}) {
  const workflowLane = parsePacketSingleField(packetText, "WORKFLOW_LANE");
  const communicationDir = parsePacketSingleField(packetText, "WP_COMMUNICATION_DIR");
  return {
    schema_id: WORK_PACKET_CONTRACT_SCHEMA_ID,
    schema_version: "work_packet_contract_v1",
    contract_authority: "LEGACY_AUTHORITY",
    wp_id: parsePacketSingleField(packetText, "WP_ID") || wpId,
    base_wp_id: (parsePacketSingleField(packetText, "BASE_WP_ID") || wpId || "").replace(/\s*\(.*/, "").trim(),
    created_at_utc: parsePacketSingleField(packetText, "DATE") || null,
    updated_at_utc: null,
    source_control: {
      merge_base_sha: parsePacketSingleField(packetText, "MERGE_BASE_SHA"),
      canonical_branch: "main",
      work_branch: parsePacketSingleField(packetText, "LOCAL_BRANCH"),
      worktree_dir: parsePacketSingleField(packetText, "LOCAL_WORKTREE_DIR"),
      remote_backup_branch: parsePacketSingleField(packetText, "REMOTE_BACKUP_BRANCH"),
    },
    workflow: {
      lane: workflowLane,
      authority: workflowLane === "MANUAL_RELAY" ? "CLASSIC_ORCHESTRATOR" : "ORCHESTRATOR",
      execution_owner: parsePacketSingleField(packetText, "EXECUTION_OWNER"),
      session_start_authority: parsePacketSingleField(packetText, "SESSION_START_AUTHORITY"),
      host_preference: parsePacketSingleField(packetText, "SESSION_HOST_PREFERENCE"),
      communication_dir: communicationDir,
      thread_file: parsePacketSingleField(packetText, "WP_THREAD_FILE") || (communicationDir ? path.posix.join(communicationDir, "THREAD.md") : ""),
      runtime_status_file: parsePacketSingleField(packetText, "WP_RUNTIME_STATUS_FILE") || (communicationDir ? path.posix.join(communicationDir, "RUNTIME_STATUS.json") : ""),
      receipts_file: parsePacketSingleField(packetText, "WP_RECEIPTS_FILE") || (communicationDir ? path.posix.join(communicationDir, "RECEIPTS.jsonl") : ""),
      communication_contract: parsePacketSingleField(packetText, "COMMUNICATION_CONTRACT"),
      communication_health_gate: parsePacketSingleField(packetText, "COMMUNICATION_HEALTH_GATE"),
    },
    lifecycle: {
      status: parseStatus(packetText),
      main_containment_status: parsePacketSingleField(packetText, "MAIN_CONTAINMENT_STATUS"),
      current_main_compatibility_status: parsePacketSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_STATUS"),
      risk_tier: parsePacketSingleField(packetText, "RISK_TIER"),
      user_signature: parsePacketSingleField(packetText, "USER_SIGNATURE"),
      packet_format_version: parsePacketSingleField(packetText, "PACKET_FORMAT_VERSION"),
    },
    authority_files: {
      packet_contract: packetDir ? path.posix.join(packetDir, "packet.json") : "",
      packet_projection: normalizePath(packetPath),
      refinement_contract: packetDir ? path.posix.join(packetDir, "refinement.json") : "",
      microtask_contract_glob: packetDir ? path.posix.join(packetDir, "MT-*.json") : "",
    },
    markdown_projection: markdownProjection(packetPath),
    scope: {
      summary: parsePacketSingleField(packetText, "What"),
      why: parsePacketSingleField(packetText, "Why"),
      allowed_paths: splitList(parsePacketSingleField(packetText, "IN_SCOPE_PATHS")),
      forbidden_paths: splitList(parsePacketSingleField(packetText, "OUT_OF_SCOPE")),
      spec_anchors: splitList(parsePacketSingleField(packetText, "SPEC_ANCHOR") || parsePacketSingleField(packetText, "SPEC_ANCHOR_PRIMARY")),
      acceptance_criteria: [],
    },
    refinement: {
      contract_file: packetDir ? path.posix.join(packetDir, "refinement.json") : "",
      status: parsePacketSingleField(packetText, "REFINEMENT_ENFORCEMENT_PROFILE") ? "SIGNED_OR_LEGACY" : "UNKNOWN",
      activation_manager_required: workflowLane === "ORCHESTRATOR_MANAGED",
      enforcement_profile: parsePacketSingleField(packetText, "REFINEMENT_ENFORCEMENT_PROFILE"),
      hydration_profile: parsePacketSingleField(packetText, "PACKET_HYDRATION_PROFILE"),
    },
    microtasks: {
      contract_glob: packetDir ? path.posix.join(packetDir, "MT-*.json") : "",
      declared_ids: [],
      active_id: null,
      next_id: null,
    },
    red_team: {
      required: true,
      profile: DETERMINISTIC_CONTRACT_RED_TEAM_PROFILE,
      assumptions_to_attack: [
        "Imported legacy Markdown may contain shadow authority that the first-pass contract did not capture.",
        "Projection hash repair proves file identity, not semantic completeness.",
        "Consumers must treat LEGACY_AUTHORITY as migration debt until reviewed or regenerated.",
      ],
      minimum_controls: [
        "Prefer primary JSON once imported.",
        "Keep generated projection hashes fail-closed.",
        "Record unsupported fallback fields as RGF migration debt.",
      ],
    },
  };
}

export function readWorkPacketContract(wpId) {
  const resolved = resolveWorkPacketPath(wpId);
  if (!resolved) return { ok: false, source: "MISSING", contract: null, resolved: null, packetText: "" };
  const contractPath = contractPathFor(resolved, "packet.json");
  const contract = contractPath ? readJsonIfExists(contractPath.abs) : null;
  const packetText = readTextIfExists(resolved.packetAbsPath);
  if (contract) {
    return {
      ok: true,
      source: "PRIMARY_MACHINE_READABLE",
      contract,
      resolved,
      packetText,
      contractPath: contractPath.rel,
      contractAbsPath: contractPath.abs,
    };
  }
  return {
    ok: Boolean(packetText),
    source: "LEGACY_AUTHORITY",
    contract: buildLegacyWorkPacketContract({
      wpId,
      packetText,
      packetPath: resolved.packetPath,
      packetDir: resolved.packetDir,
    }),
    resolved,
    packetText,
    contractPath: contractPath?.rel || "",
    contractAbsPath: contractPath?.abs || "",
  };
}

export function buildWorkPacketCommunicationView(wpId) {
  const state = readWorkPacketContract(wpId);
  if (!state.ok) {
    return {
      ok: false,
      source: state.source,
      wpId,
      packetPath: "",
      packetAbsPath: "",
      packetText: "",
      contract: null,
    };
  }

  const contract = state.contract || {};
  const workflow = contract.workflow || {};
  const lifecycle = contract.lifecycle || {};
  const sourceControl = contract.source_control || {};
  const packetText = state.packetText || "";
  const communicationDir = normalizePath(workflow.communication_dir || parsePacketSingleField(packetText, "WP_COMMUNICATION_DIR"));
  const threadFile = normalizePath(
    workflow.thread_file
    || parsePacketSingleField(packetText, "WP_THREAD_FILE")
    || (communicationDir ? path.posix.join(communicationDir, "THREAD.md") : ""),
  );
  const runtimeStatusFile = normalizePath(
    workflow.runtime_status_file
    || parsePacketSingleField(packetText, "WP_RUNTIME_STATUS_FILE")
    || (communicationDir ? path.posix.join(communicationDir, "RUNTIME_STATUS.json") : ""),
  );
  const receiptsFile = normalizePath(
    workflow.receipts_file
    || parsePacketSingleField(packetText, "WP_RECEIPTS_FILE")
    || (communicationDir ? path.posix.join(communicationDir, "RECEIPTS.jsonl") : ""),
  );
  const packetPath = normalizePath(
    state.resolved?.packetPath
    || contract.markdown_projection?.path
    || contract.authority_files?.packet_projection
    || "",
  );

  return {
    ok: true,
    source: state.source,
    wpId: contract.wp_id || parsePacketSingleField(packetText, "WP_ID") || wpId,
    baseWpId: String(contract.base_wp_id || parsePacketSingleField(packetText, "BASE_WP_ID") || wpId || "").replace(/\s*\(.*/, "").trim(),
    packetPath,
    packetAbsPath: state.resolved?.packetAbsPath || (packetPath ? repoPathAbs(packetPath) : ""),
    packetText,
    contract,
    workflowLane: workflow.lane || parsePacketSingleField(packetText, "WORKFLOW_LANE"),
    packetFormatVersion: lifecycle.packet_format_version || parsePacketSingleField(packetText, "PACKET_FORMAT_VERSION"),
    communicationContract: workflow.communication_contract || parsePacketSingleField(packetText, "COMMUNICATION_CONTRACT"),
    communicationHealthGate: workflow.communication_health_gate || parsePacketSingleField(packetText, "COMMUNICATION_HEALTH_GATE"),
    communicationDir,
    threadFile,
    runtimeStatusFile,
    receiptsFile,
    executionOwner: workflow.execution_owner || parsePacketSingleField(packetText, "EXECUTION_OWNER"),
    workflowAuthority: workflow.authority || parsePacketSingleField(packetText, "WORKFLOW_AUTHORITY"),
    technicalAdvisor: workflow.technical_advisor || parsePacketSingleField(packetText, "TECHNICAL_ADVISOR"),
    technicalAuthority: workflow.technical_authority || parsePacketSingleField(packetText, "TECHNICAL_AUTHORITY"),
    mergeAuthority: workflow.merge_authority || parsePacketSingleField(packetText, "MERGE_AUTHORITY"),
    localBranch: sourceControl.work_branch || parsePacketSingleField(packetText, "LOCAL_BRANCH"),
    localWorktreeDir: sourceControl.worktree_dir || parsePacketSingleField(packetText, "LOCAL_WORKTREE_DIR"),
    agenticMode: workflow.agentic_mode || parsePacketSingleField(packetText, "AGENTIC_MODE"),
    packetStatus: lifecycle.status || parseStatus(packetText),
    mainContainmentStatus: lifecycle.main_containment_status || parsePacketSingleField(packetText, "MAIN_CONTAINMENT_STATUS"),
    currentMainCompatibilityStatus: lifecycle.current_main_compatibility_status || parsePacketSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_STATUS"),
    riskTier: lifecycle.risk_tier || parsePacketSingleField(packetText, "RISK_TIER"),
  };
}

export function buildLegacyRefinementContract({ wpId, refinementText = "", refinementPath = "", packetContractPath = "", packetDir = "" } = {}) {
  return {
    schema_id: REFINEMENT_CONTRACT_SCHEMA_ID,
    schema_version: "refinement_contract_v1",
    contract_authority: "LEGACY_AUTHORITY",
    wp_id: wpId,
    created_at_utc: parsePacketSingleField(refinementText, "DATE") || null,
    updated_at_utc: null,
    authority_files: {
      refinement_contract: packetDir ? path.posix.join(packetDir, "refinement.json") : "",
      refinement_projection: normalizePath(refinementPath),
      packet_contract: normalizePath(packetContractPath),
    },
    markdown_projection: markdownProjection(refinementPath),
    activation_manager: {
      required: false,
      status: "LEGACY_IMPORT",
      session_id: null,
      handoff_summary: null,
      readiness_status: "UNKNOWN",
    },
    refinement: {
      operator_request: parsePacketSingleField(refinementText, "OPERATOR_REQUEST") || wpId,
      user_signature: parsePacketSingleField(refinementText, "USER_SIGNATURE"),
      enforcement_profile: parsePacketSingleField(refinementText, "REFINEMENT_ENFORCEMENT_PROFILE"),
      approved_spec_enrichment: [],
      scope_edges: [],
      assumptions: [],
      non_goals: [],
      spec_anchors: splitList(parsePacketSingleField(refinementText, "SPEC_ANCHOR") || parsePacketSingleField(refinementText, "SPEC_TARGET_RESOLVED")),
      acceptance_criteria: [],
      microtask_plan: [],
    },
    red_team: {
      required: true,
      profile: DETERMINISTIC_CONTRACT_RED_TEAM_PROFILE,
      risks: ["Imported refinement contract is a legacy synthesis and may not capture all prose-only decisions."],
      minimum_controls: ["Use this contract as migration scaffold and preserve original projection hash."],
    },
  };
}

export function buildLegacyMicrotaskContract({ wpId, mtId, mtText = "", mtPath = "", packetContractPath = "", packetDir = "" } = {}) {
  const dependsOn = parsePacketSingleField(mtText, "DEPENDS_ON");
  return {
    schema_id: MICRO_TASK_CONTRACT_SCHEMA_ID,
    schema_version: "microtask_contract_v1",
    contract_authority: "LEGACY_AUTHORITY",
    wp_id: wpId,
    mt_id: parsePacketSingleField(mtText, "MT_ID") || mtId,
    created_at_utc: null,
    updated_at_utc: null,
    authority_files: {
      microtask_contract: packetDir ? path.posix.join(packetDir, `${mtId}.json`) : "",
      microtask_projection: normalizePath(mtPath),
      packet_contract: normalizePath(packetContractPath),
    },
    markdown_projection: markdownProjection(mtPath),
    lifecycle: {
      status: parsePacketSingleField(mtText, "STATUS") || "PENDING",
      depends_on: dependsOn && dependsOn !== "NONE" ? [dependsOn] : [],
      blocks: [],
      active: false,
      validator_verdict: "PENDING",
    },
    scope: {
      summary: parsePacketSingleField(mtText, "CLAUSE"),
      allowed_paths: splitList(parsePacketSingleField(mtText, "CODE_SURFACES")),
      forbidden_paths: [],
      acceptance_criteria: splitList(parsePacketSingleField(mtText, "CLAUSE")),
      proof_targets: splitList(parsePacketSingleField(mtText, "EXPECTED_TESTS")),
      risk_if_missed: parsePacketSingleField(mtText, "RISK_IF_MISSED"),
    },
    handoff: {
      coder_session: null,
      wp_validator_session: null,
      review_request_receipt_id: null,
      review_response_receipt_id: null,
    },
    red_team: {
      required: true,
      profile: DETERMINISTIC_CONTRACT_RED_TEAM_PROFILE,
      risks: ["Imported MT contract may under-capture prose-only review or proof instructions."],
      minimum_controls: ["Use contract-first identity and preserve projection hash."],
    },
  };
}

function writeImportedContractPair({ contract, contractAbsPath, projectionAbsPath, projectionRel, generator = "wp-contract-import.mjs" }) {
  const projectionBody = stripGeneratedProjectionHeader(readTextIfExists(projectionAbsPath));
  const sourceFile = normalizePath(
    contract?.authority_files?.packet_contract
    || contract?.authority_files?.refinement_contract
    || contract?.authority_files?.microtask_contract
    || contractAbsPath,
  );
  const primary = {
    ...contract,
    contract_authority: "PRIMARY_MACHINE_READABLE",
  };
  const stamped = stampContractProjectionMetadata(primary, {
    projectionPath: projectionRel,
    projectionBody,
    sourceFile,
    generator,
  });
  fs.writeFileSync(contractAbsPath, stableStringify(stamped), "utf8");
  fs.writeFileSync(projectionAbsPath, addOrReplaceGeneratedProjectionHeader(projectionBody, stamped, {
    sourceFile,
  }), "utf8");
  return stamped;
}

export function importWorkPacketContracts(wpId, { dryRun = false, repair = true } = {}) {
  const state = readWorkPacketContract(wpId);
  if (!state.ok || !state.resolved?.isFolder) {
    return { wp_id: wpId, ok: false, action: "SKIPPED", reason: "missing folder-based packet" };
  }
  const actions = [];
  const packetContractPath = contractPathFor(state.resolved, "packet.json");
  const packetExists = packetContractPath && fs.existsSync(packetContractPath.abs);
  if (!packetExists || repair) {
    actions.push({ kind: packetExists ? "REPAIR_PACKET_PROJECTION" : "IMPORT_PACKET", path: packetContractPath.rel });
    if (!dryRun) {
      writeImportedContractPair({
        contract: state.contract,
        contractAbsPath: packetContractPath.abs,
        projectionAbsPath: state.resolved.packetAbsPath,
        projectionRel: state.resolved.packetPath,
      });
    }
  }

  const refinementMdAbs = path.join(state.resolved.packetDirAbs, "refinement.md");
  if (fs.existsSync(refinementMdAbs)) {
    const refinementContractPath = contractPathFor(state.resolved, "refinement.json");
    const refinementExists = fs.existsSync(refinementContractPath.abs);
    if (!refinementExists || repair) {
      actions.push({ kind: refinementExists ? "REPAIR_REFINEMENT_PROJECTION" : "IMPORT_REFINEMENT", path: refinementContractPath.rel });
      if (!dryRun) {
        writeImportedContractPair({
          contract: buildLegacyRefinementContract({
            wpId,
            refinementText: readTextIfExists(refinementMdAbs),
            refinementPath: path.posix.join(state.resolved.packetDir, "refinement.md"),
            packetContractPath: packetContractPath.rel,
            packetDir: state.resolved.packetDir,
          }),
          contractAbsPath: refinementContractPath.abs,
          projectionAbsPath: refinementMdAbs,
          projectionRel: path.posix.join(state.resolved.packetDir, "refinement.md"),
        });
      }
    }
  }

  for (const entry of fs.readdirSync(state.resolved.packetDirAbs, { withFileTypes: true })) {
    if (!entry.isFile() || !/^MT-\d{3}\.md$/i.test(entry.name)) continue;
    const mtId = entry.name.replace(/\.md$/i, "");
    const mtMdAbs = path.join(state.resolved.packetDirAbs, entry.name);
    const mtContractPath = contractPathFor(state.resolved, `${mtId}.json`);
    const mtExists = fs.existsSync(mtContractPath.abs);
    if (mtExists && !repair) continue;
    actions.push({ kind: mtExists ? "REPAIR_MT_PROJECTION" : "IMPORT_MT", path: mtContractPath.rel });
    if (!dryRun) {
      writeImportedContractPair({
        contract: buildLegacyMicrotaskContract({
          wpId,
          mtId,
          mtText: readTextIfExists(mtMdAbs),
          mtPath: path.posix.join(state.resolved.packetDir, entry.name),
          packetContractPath: packetContractPath.rel,
          packetDir: state.resolved.packetDir,
        }),
        contractAbsPath: mtContractPath.abs,
        projectionAbsPath: mtMdAbs,
        projectionRel: path.posix.join(state.resolved.packetDir, entry.name),
      });
    }
  }

  return { wp_id: wpId, ok: true, dry_run: dryRun, actions };
}

export function contractPathsForWp(wpId) {
  const resolved = resolveWorkPacketPath(wpId);
  if (!resolved?.isFolder) return null;
  return {
    packet: contractPathFor(resolved, "packet.json")?.rel || "",
    refinement: contractPathFor(resolved, "refinement.json")?.rel || "",
    microtask_glob: path.posix.join(resolved.packetDir, "MT-*.json"),
    legacy_root: `${GOV_ROOT_REPO_REL}/task_packets/${wpId}/`,
  };
}
