import fs from "node:fs";
import path from "node:path";
import {
  GOV_ROOT_REPO_REL,
  normalizePath,
  repoPathAbs,
  resolveRefinementPath,
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

function normalizeListValues(values = []) {
  return Array.from(new Set(
    (Array.isArray(values) ? values : splitList(values))
      .map((entry) => String(entry || "").trim().replace(/^`|`$/g, ""))
      .filter(Boolean),
  ));
}

function formatProjectionField(label, value) {
  const normalized = String(value ?? "").trim();
  return normalized ? `- ${label}: ${normalized}` : "";
}

function contractOrPacketField(value, packetText, label) {
  const normalized = String(value ?? "").trim();
  return normalized || parsePacketSingleField(packetText, label);
}

function formatProjectionList(label, values = []) {
  const normalizedValues = normalizeListValues(values);
  if (normalizedValues.length === 0) return "";
  return [
    `- ${label}:`,
    ...normalizedValues.map((entry) => `  - ${entry}`),
  ].join("\n");
}

function appendIfPresent(lines, value) {
  const normalized = String(value || "").trimEnd();
  if (normalized) lines.push(normalized);
}

function normalizeBaseWpId(value = "", wpId = "") {
  const raw = String(value || "").replace(/\s*\(.*/, "").trim();
  const candidate = raw || wpId || "";
  if (candidate === wpId && /-v\d+$/i.test(candidate)) return candidate.replace(/-v\d+$/i, "");
  if (candidate === wpId && /-\d{8}$/.test(candidate)) return candidate.replace(/-\d{8}$/, "");
  return candidate;
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

function folderPacketPathsForWp(wpId) {
  const packetDir = normalizePath(path.posix.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, wpId));
  const packetDirAbs = repoPathAbs(packetDir);
  return {
    packetDir,
    packetDirAbs,
    packetPath: normalizePath(path.posix.join(packetDir, "packet.md")),
    packetAbsPath: path.join(packetDirAbs, "packet.md"),
    packetContractPath: normalizePath(path.posix.join(packetDir, "packet.json")),
    packetContractAbsPath: path.join(packetDirAbs, "packet.json"),
  };
}

export function listWorkPacketIdsForContractImport() {
  const rootAbs = repoPathAbs(WORK_PACKET_STORAGE_ROOT_REPO_REL);
  if (!fs.existsSync(rootAbs)) return [];
  const ids = new Set();
  for (const entry of fs.readdirSync(rootAbs, { withFileTypes: true })) {
    if (entry.isDirectory() && /^WP-/.test(entry.name) && fs.existsSync(path.join(rootAbs, entry.name, "packet.md"))) {
      ids.add(entry.name);
    } else if (entry.isFile() && /^WP-[A-Za-z0-9._-]+\.md$/.test(entry.name)) {
      ids.add(entry.name.replace(/\.md$/i, ""));
    }
  }
  return [...ids].sort((left, right) => left.localeCompare(right, "en", { numeric: true }));
}

export function buildLegacyWorkPacketContract({ wpId, packetText = "", packetPath = "", packetDir = "" } = {}) {
  const workflowLane = parsePacketSingleField(packetText, "WORKFLOW_LANE");
  const communicationDir = parsePacketSingleField(packetText, "WP_COMMUNICATION_DIR");
  return {
    schema_id: WORK_PACKET_CONTRACT_SCHEMA_ID,
    schema_version: "work_packet_contract_v1",
    contract_authority: "LEGACY_AUTHORITY",
    wp_id: parsePacketSingleField(packetText, "WP_ID") || wpId,
    base_wp_id: normalizeBaseWpId(parsePacketSingleField(packetText, "BASE_WP_ID"), wpId),
    created_at_utc: parsePacketSingleField(packetText, "DATE") || null,
    updated_at_utc: null,
    source_control: {
      merge_base_sha: parsePacketSingleField(packetText, "MERGE_BASE_SHA"),
      canonical_branch: "main",
      work_branch: parsePacketSingleField(packetText, "LOCAL_BRANCH"),
      worktree_dir: parsePacketSingleField(packetText, "LOCAL_WORKTREE_DIR"),
      remote_backup_branch: parsePacketSingleField(packetText, "REMOTE_BACKUP_BRANCH"),
      remote_backup_url: parsePacketSingleField(packetText, "REMOTE_BACKUP_URL"),
      backup_push_status: parsePacketSingleField(packetText, "BACKUP_PUSH_STATUS"),
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
      notifications_file: parsePacketSingleField(packetText, "WP_NOTIFICATIONS_FILE") || (communicationDir ? path.posix.join(communicationDir, "NOTIFICATIONS.jsonl") : ""),
      communication_contract: parsePacketSingleField(packetText, "COMMUNICATION_CONTRACT"),
      communication_health_gate: parsePacketSingleField(packetText, "COMMUNICATION_HEALTH_GATE"),
    },
    lifecycle: {
      status: parseStatus(packetText),
      main_containment_status: parsePacketSingleField(packetText, "MAIN_CONTAINMENT_STATUS"),
      merged_main_commit: parsePacketSingleField(packetText, "MERGED_MAIN_COMMIT"),
      main_containment_verified_at_utc: parsePacketSingleField(packetText, "MAIN_CONTAINMENT_VERIFIED_AT_UTC"),
      current_main_compatibility_status: parsePacketSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_STATUS"),
      current_main_compatibility_baseline_sha: parsePacketSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA"),
      current_main_compatibility_verified_at_utc: parsePacketSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC"),
      packet_widening_decision: parsePacketSingleField(packetText, "PACKET_WIDENING_DECISION"),
      packet_widening_evidence: parsePacketSingleField(packetText, "PACKET_WIDENING_EVIDENCE"),
      current_wp_status: parsePacketSingleField(packetText, "CURRENT_WP_STATUS"),
      risk_tier: parsePacketSingleField(packetText, "RISK_TIER"),
      user_signature: parsePacketSingleField(packetText, "USER_SIGNATURE"),
      packet_format_version: parsePacketSingleField(packetText, "PACKET_FORMAT_VERSION"),
      wp_validator_of_record: parsePacketSingleField(packetText, "WP_VALIDATOR_OF_RECORD"),
      integration_validator_of_record: parsePacketSingleField(packetText, "INTEGRATION_VALIDATOR_OF_RECORD"),
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

export function updateWorkPacketLifecycleContract({
  wpId,
  lifecyclePatch = {},
  projectionText = "",
  generator = "work-packet-lifecycle-writer",
} = {}) {
  const state = readWorkPacketContract(wpId);
  if (!state.ok || !state.resolved?.packetAbsPath) {
    return {
      updated: false,
      reason: "packet_missing",
      packetText: projectionText || "",
      contract: null,
      contractPath: "",
      contractAbsPath: "",
    };
  }
  if (!state.resolved?.isFolder || !state.resolved?.packetDirAbs || !state.resolved?.packetDir) {
    return {
      updated: false,
      reason: "flat_legacy_packet_has_no_primary_contract_path",
      packetText: projectionText || state.packetText || "",
      contract: state.contract,
      contractPath: "",
      contractAbsPath: "",
    };
  }

  const contractAbsPath = state.contractAbsPath || path.join(state.resolved.packetDirAbs, "packet.json");
  const contractPath = state.contractPath || path.posix.join(state.resolved.packetDir, "packet.json");
  const projectionRel = state.resolved.packetPath;
  const projectionAbs = state.resolved.packetAbsPath;
  const projectionBody = stripGeneratedProjectionHeader(projectionText || state.packetText || "");
  const sourceFile = normalizePath(
    state.contract?.authority_files?.packet_contract
    || contractPath,
  );
  const nextContract = {
    ...(state.contract || {}),
    contract_authority: "PRIMARY_MACHINE_READABLE",
    lifecycle: {
      ...(state.contract?.lifecycle || {}),
      ...(lifecyclePatch || {}),
    },
  };
  nextContract.authority_files = {
    ...(nextContract.authority_files || {}),
    packet_contract: contractPath,
    packet_projection: projectionRel,
  };
  const stamped = stampContractProjectionMetadata(nextContract, {
    projectionPath: projectionRel,
    projectionBody,
    sourceFile,
    generator,
  });
  const nextProjectionText = addOrReplaceGeneratedProjectionHeader(projectionBody, stamped, {
    sourceFile,
  });
  fs.writeFileSync(contractAbsPath, stableStringify(stamped), "utf8");
  fs.writeFileSync(projectionAbs, nextProjectionText, "utf8");
  return {
    updated: true,
    reason: "updated_primary_contract_and_projection",
    packetText: nextProjectionText,
    contract: stamped,
    contractPath,
    contractAbsPath,
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
  const notificationsFile = normalizePath(
    workflow.notifications_file
    || parsePacketSingleField(packetText, "WP_NOTIFICATIONS_FILE")
    || (communicationDir ? path.posix.join(communicationDir, "NOTIFICATIONS.jsonl") : ""),
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
    baseWpId: normalizeBaseWpId(contract.base_wp_id || parsePacketSingleField(packetText, "BASE_WP_ID"), wpId),
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
    notificationsFile,
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
    currentMainCompatibilityBaselineSha: lifecycle.current_main_compatibility_baseline_sha || parsePacketSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA"),
    currentMainCompatibilityVerifiedAtUtc: lifecycle.current_main_compatibility_verified_at_utc || parsePacketSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC"),
    packetWideningDecision: lifecycle.packet_widening_decision || parsePacketSingleField(packetText, "PACKET_WIDENING_DECISION"),
    packetWideningEvidence: lifecycle.packet_widening_evidence || parsePacketSingleField(packetText, "PACKET_WIDENING_EVIDENCE"),
    riskTier: lifecycle.risk_tier || parsePacketSingleField(packetText, "RISK_TIER"),
    mainContainmentVerifiedAtUtc: lifecycle.main_containment_verified_at_utc || parsePacketSingleField(packetText, "MAIN_CONTAINMENT_VERIFIED_AT_UTC"),
    mergedMainCommit: lifecycle.merged_main_commit || parsePacketSingleField(packetText, "MERGED_MAIN_COMMIT"),
    wpValidatorOfRecord: lifecycle.wp_validator_of_record || parsePacketSingleField(packetText, "WP_VALIDATOR_OF_RECORD"),
    integrationValidatorOfRecord: lifecycle.integration_validator_of_record || parsePacketSingleField(packetText, "INTEGRATION_VALIDATOR_OF_RECORD"),
  };
}

export function buildContractDerivedPacketProjectionText({ contract = null, packetText = "", source = "" } = {}) {
  const normalizedContract = contract && typeof contract === "object" ? contract : {};
  const workflow = normalizedContract.workflow || {};
  const lifecycle = normalizedContract.lifecycle || {};
  const sourceControl = normalizedContract.source_control || {};
  const scope = normalizedContract.scope || {};
  const authorityFiles = normalizedContract.authority_files || {};
  const projection = normalizedContract.markdown_projection || {};
  const lines = [
    `# Contract-Derived Packet Projection: ${normalizedContract.wp_id || "<unknown>"}`,
    "",
    "<!-- CONTRACT_DERIVED_PACKET_PROJECTION: generated in-memory for legacy evaluator compatibility; do not persist as authority. -->",
    "",
  ];

  appendIfPresent(lines, formatProjectionField("WP_ID", normalizedContract.wp_id));
  appendIfPresent(lines, formatProjectionField("BASE_WP_ID", normalizedContract.base_wp_id));
  appendIfPresent(lines, formatProjectionField("Status", contractOrPacketField(lifecycle.status, packetText, "Status")));
  appendIfPresent(lines, formatProjectionField("WORKFLOW_LANE", contractOrPacketField(workflow.lane, packetText, "WORKFLOW_LANE")));
  appendIfPresent(lines, formatProjectionField("WORKFLOW_AUTHORITY", contractOrPacketField(workflow.authority, packetText, "WORKFLOW_AUTHORITY")));
  appendIfPresent(lines, formatProjectionField("EXECUTION_OWNER", contractOrPacketField(workflow.execution_owner, packetText, "EXECUTION_OWNER")));
  appendIfPresent(lines, formatProjectionField("SESSION_START_AUTHORITY", contractOrPacketField(workflow.session_start_authority, packetText, "SESSION_START_AUTHORITY")));
  appendIfPresent(lines, formatProjectionField("WP_COMMUNICATION_DIR", contractOrPacketField(workflow.communication_dir, packetText, "WP_COMMUNICATION_DIR")));
  appendIfPresent(lines, formatProjectionField("WP_THREAD_FILE", contractOrPacketField(workflow.thread_file, packetText, "WP_THREAD_FILE")));
  appendIfPresent(lines, formatProjectionField("WP_RUNTIME_STATUS_FILE", contractOrPacketField(workflow.runtime_status_file, packetText, "WP_RUNTIME_STATUS_FILE")));
  appendIfPresent(lines, formatProjectionField("WP_RECEIPTS_FILE", contractOrPacketField(workflow.receipts_file, packetText, "WP_RECEIPTS_FILE")));
  appendIfPresent(lines, formatProjectionField("WP_NOTIFICATIONS_FILE", contractOrPacketField(workflow.notifications_file, packetText, "WP_NOTIFICATIONS_FILE")));
  appendIfPresent(lines, formatProjectionField("COMMUNICATION_CONTRACT", contractOrPacketField(workflow.communication_contract, packetText, "COMMUNICATION_CONTRACT")));
  appendIfPresent(lines, formatProjectionField("COMMUNICATION_HEALTH_GATE", contractOrPacketField(workflow.communication_health_gate, packetText, "COMMUNICATION_HEALTH_GATE")));
  appendIfPresent(lines, formatProjectionField("LOCAL_BRANCH", contractOrPacketField(sourceControl.work_branch, packetText, "LOCAL_BRANCH")));
  appendIfPresent(lines, formatProjectionField("LOCAL_WORKTREE_DIR", contractOrPacketField(sourceControl.worktree_dir, packetText, "LOCAL_WORKTREE_DIR")));
  appendIfPresent(lines, formatProjectionField("REMOTE_BACKUP_BRANCH", contractOrPacketField(sourceControl.remote_backup_branch, packetText, "REMOTE_BACKUP_BRANCH")));
  appendIfPresent(lines, formatProjectionField("REMOTE_BACKUP_URL", contractOrPacketField(sourceControl.remote_backup_url, packetText, "REMOTE_BACKUP_URL")));
  appendIfPresent(lines, formatProjectionField("BACKUP_PUSH_STATUS", contractOrPacketField(sourceControl.backup_push_status, packetText, "BACKUP_PUSH_STATUS")));
  appendIfPresent(lines, formatProjectionField("USER_SIGNATURE", contractOrPacketField(lifecycle.user_signature, packetText, "USER_SIGNATURE")));
  appendIfPresent(lines, formatProjectionField("PACKET_FORMAT_VERSION", contractOrPacketField(lifecycle.packet_format_version, packetText, "PACKET_FORMAT_VERSION")));
  appendIfPresent(lines, formatProjectionField("WP_VALIDATOR_OF_RECORD", contractOrPacketField(lifecycle.wp_validator_of_record, packetText, "WP_VALIDATOR_OF_RECORD")));
  appendIfPresent(lines, formatProjectionField("INTEGRATION_VALIDATOR_OF_RECORD", contractOrPacketField(lifecycle.integration_validator_of_record, packetText, "INTEGRATION_VALIDATOR_OF_RECORD")));
  appendIfPresent(lines, formatProjectionField("MAIN_CONTAINMENT_STATUS", contractOrPacketField(lifecycle.main_containment_status, packetText, "MAIN_CONTAINMENT_STATUS")));
  appendIfPresent(lines, formatProjectionField("MERGED_MAIN_COMMIT", contractOrPacketField(lifecycle.merged_main_commit, packetText, "MERGED_MAIN_COMMIT")));
  appendIfPresent(lines, formatProjectionField("MAIN_CONTAINMENT_VERIFIED_AT_UTC", contractOrPacketField(lifecycle.main_containment_verified_at_utc, packetText, "MAIN_CONTAINMENT_VERIFIED_AT_UTC")));
  appendIfPresent(lines, formatProjectionField("CURRENT_MAIN_COMPATIBILITY_STATUS", contractOrPacketField(lifecycle.current_main_compatibility_status, packetText, "CURRENT_MAIN_COMPATIBILITY_STATUS")));
  appendIfPresent(lines, formatProjectionField("CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA", contractOrPacketField(lifecycle.current_main_compatibility_baseline_sha, packetText, "CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA")));
  appendIfPresent(lines, formatProjectionField("CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC", contractOrPacketField(lifecycle.current_main_compatibility_verified_at_utc, packetText, "CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC")));
  appendIfPresent(lines, formatProjectionField("PACKET_WIDENING_DECISION", contractOrPacketField(lifecycle.packet_widening_decision, packetText, "PACKET_WIDENING_DECISION")));
  appendIfPresent(lines, formatProjectionField("PACKET_WIDENING_EVIDENCE", contractOrPacketField(lifecycle.packet_widening_evidence, packetText, "PACKET_WIDENING_EVIDENCE")));
  appendIfPresent(lines, formatProjectionField("CURRENT_WP_STATUS", contractOrPacketField(lifecycle.current_wp_status, packetText, "CURRENT_WP_STATUS")));
  appendIfPresent(lines, formatProjectionField("RISK_TIER", contractOrPacketField(lifecycle.risk_tier, packetText, "RISK_TIER")));
  appendIfPresent(lines, formatProjectionField("AUTHORITATIVE_CONTRACT_FILE", authorityFiles.packet_contract));
  appendIfPresent(lines, formatProjectionField("MARKDOWN_PROJECTION_FILE", projection.path || authorityFiles.packet_projection));
  appendIfPresent(lines, formatProjectionField("CONTRACT_SOURCE", source || normalizedContract.contract_authority || ""));
  appendIfPresent(lines, formatProjectionList("IN_SCOPE_PATHS", scope.allowed_paths || []));
  appendIfPresent(lines, formatProjectionList("OUT_OF_SCOPE", scope.forbidden_paths || []));
  appendIfPresent(lines, formatProjectionList("SPEC_ANCHOR", scope.spec_anchors || []));
  appendIfPresent(lines, formatProjectionList("ACCEPTANCE_CRITERIA", scope.acceptance_criteria || []));

  const legacyText = stripGeneratedProjectionHeader(packetText);
  if (legacyText.trim()) {
    lines.push("");
    lines.push("## LEGACY_PACKET_PROJECTION_FALLBACK");
    lines.push("");
    lines.push(legacyText.trimEnd());
  }

  return `${lines.filter((line) => line !== null && line !== undefined).join("\n")}\n`;
}

export function buildWorkPacketEvaluatorView(wpId) {
  const communicationView = buildWorkPacketCommunicationView(wpId);
  if (!communicationView.ok) return communicationView;
  const packetText = buildContractDerivedPacketProjectionText({
    contract: communicationView.contract,
    packetText: communicationView.packetText,
    source: communicationView.source,
  });
  return {
    ...communicationView,
    packetText,
    legacyPacketText: communicationView.packetText,
    contractDerivedPacketText: packetText,
    contractSource: communicationView.source,
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
      user_approval_evidence: parsePacketSingleField(refinementText, "USER_APPROVAL_EVIDENCE"),
      user_review_status: parsePacketSingleField(refinementText, "USER_REVIEW_STATUS"),
      enrichment_needed: parsePacketSingleField(refinementText, "ENRICHMENT_NEEDED"),
      clearly_covers_verdict: parsePacketSingleField(refinementText, "CLEARLY_COVERS_VERDICT"),
      stub_wp_ids: parsePacketSingleField(refinementText, "STUB_WP_IDS"),
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

export function readWorkPacketRefinementContract(wpId) {
  const packetState = readWorkPacketContract(wpId);
  const packetContractPath = packetState.contractPath || "";
  const folderRefinementPath = packetState.resolved?.isFolder
    ? {
        rel: path.posix.join(packetState.resolved.packetDir, "refinement.md"),
        abs: path.join(packetState.resolved.packetDirAbs, "refinement.md"),
      }
    : null;
  const contractPath = packetState.resolved?.isFolder ? contractPathFor(packetState.resolved, "refinement.json") : null;
  const contract = contractPath ? readJsonIfExists(contractPath.abs) : null;
  const legacyRel = folderRefinementPath && fs.existsSync(folderRefinementPath.abs)
    ? folderRefinementPath.rel
    : (resolveRefinementPath(wpId) || path.posix.join(GOV_ROOT_REPO_REL, "refinements", `${wpId}.md`));
  const legacyAbs = folderRefinementPath && fs.existsSync(folderRefinementPath.abs)
    ? folderRefinementPath.abs
    : repoPathAbs(legacyRel);
  const refinementText = readTextIfExists(legacyAbs);

  if (contract) {
    return {
      ok: true,
      source: "PRIMARY_MACHINE_READABLE",
      contract,
      refinementText,
      refinementPath: contract.markdown_projection?.path || legacyRel,
      refinementAbsPath: legacyAbs,
      contractPath: contractPath.rel,
      contractAbsPath: contractPath.abs,
      packetState,
    };
  }

  if (!refinementText) {
    return {
      ok: false,
      source: "MISSING",
      contract: null,
      refinementText: "",
      refinementPath: legacyRel,
      refinementAbsPath: legacyAbs,
      contractPath: contractPath?.rel || "",
      contractAbsPath: contractPath?.abs || "",
      packetState,
    };
  }

  return {
    ok: true,
    source: "LEGACY_AUTHORITY",
    contract: buildLegacyRefinementContract({
      wpId,
      refinementText,
      refinementPath: legacyRel,
      packetContractPath,
      packetDir: packetState.resolved?.packetDir || "",
    }),
    refinementText,
    refinementPath: legacyRel,
    refinementAbsPath: legacyAbs,
    contractPath: contractPath?.rel || "",
    contractAbsPath: contractPath?.abs || "",
    packetState,
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
  if (!state.ok) {
    return { wp_id: wpId, ok: false, action: "SKIPPED", reason: "missing packet" };
  }
  if (!state.resolved?.isFolder) {
    const folderPaths = folderPacketPathsForWp(wpId);
    const packetExists = fs.existsSync(folderPaths.packetContractAbsPath);
    const actions = [
      { kind: packetExists ? "REPAIR_IMPORTED_FLAT_PACKET_PROJECTION" : "IMPORT_FLAT_PACKET_TO_FOLDER_CONTRACT", path: folderPaths.packetContractPath },
      { kind: "FREEZE_FLAT_LEGACY_REFERENCE", path: state.resolved?.packetPath || "" },
    ];
    if (!dryRun) {
      fs.mkdirSync(folderPaths.packetDirAbs, { recursive: true });
      fs.writeFileSync(folderPaths.packetAbsPath, stripGeneratedProjectionHeader(state.packetText), "utf8");
      writeImportedContractPair({
        contract: buildLegacyWorkPacketContract({
          wpId,
          packetText: state.packetText,
          packetPath: folderPaths.packetPath,
          packetDir: folderPaths.packetDir,
        }),
        contractAbsPath: folderPaths.packetContractAbsPath,
        projectionAbsPath: folderPaths.packetAbsPath,
        projectionRel: folderPaths.packetPath,
      });
    }
    return { wp_id: wpId, ok: true, dry_run: dryRun, actions };
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
