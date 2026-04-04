const PROOF_STATUS_VALUES = ["PASS", "FAIL"];
const PROOF_KIND_VALUES = [
  "LIVE_PREPARE_WORKTREE_HEALTH",
  "COMMITTED_TARGET_PROOF",
];
const EVIDENCE_SCHEMA_ID = "hsk.committed_validation_evidence@2";
const EVIDENCE_SCHEMA_VERSION = "committed_validation_evidence_v2";
const HISTORY_LIMIT = 24;

function normalizeText(value) {
  return String(value || "").trim();
}

function normalizeStatus(value) {
  const normalized = normalizeText(value).toUpperCase();
  return PROOF_STATUS_VALUES.includes(normalized) ? normalized : "FAIL";
}

function normalizeProofKind(value, fallback = "LIVE_PREPARE_WORKTREE_HEALTH") {
  const normalized = normalizeText(value).toUpperCase();
  return PROOF_KIND_VALUES.includes(normalized) ? normalized : fallback;
}

function clone(value) {
  return value == null ? value : JSON.parse(JSON.stringify(value));
}

function normalizeProofRecord(raw, fallbackKind = "LIVE_PREPARE_WORKTREE_HEALTH") {
  if (!raw || typeof raw !== "object") return null;
  const record = { ...raw };
  if (!normalizeText(record.wp_id) && normalizeText(record.wpId)) {
    record.wp_id = record.wpId;
  }
  return {
    proof_kind: normalizeProofKind(record.proof_kind, fallbackKind),
    wp_id: normalizeText(record.wp_id),
    status: normalizeStatus(record.status),
    live_prepare_worktree_status:
      normalizeText(record.live_prepare_worktree_status).toUpperCase() || normalizeStatus(record.status),
    committed_target_status:
      normalizeText(record.committed_target_status).toUpperCase() || normalizeStatus(record.status),
    validated_at: normalizeText(record.validated_at),
    source_truth: normalizeText(record.source_truth),
    prepare_branch: normalizeText(record.prepare_branch),
    prepare_worktree_dir: normalizeText(record.prepare_worktree_dir),
    prepare_worktree_sync_warnings: Array.isArray(record.prepare_worktree_sync_warnings)
      ? record.prepare_worktree_sync_warnings.map((entry) => normalizeText(entry)).filter(Boolean)
      : [],
    committed_validation_mode: normalizeText(record.committed_validation_mode),
    committed_validation_target: normalizeText(record.committed_validation_target),
    target_head_sha: normalizeText(record.target_head_sha),
    pre_work_status: normalizeText(record.pre_work_status).toUpperCase() || "FAIL",
    cargo_clean_required: Boolean(record.cargo_clean_required),
    cargo_clean_status: normalizeText(record.cargo_clean_status).toUpperCase() || "FAIL",
    post_work_status: normalizeText(record.post_work_status).toUpperCase() || "FAIL",
    pre_work_command: normalizeText(record.pre_work_command),
    cargo_clean_command: normalizeText(record.cargo_clean_command),
    post_work_command: normalizeText(record.post_work_command),
    pre_work_output: normalizeText(record.pre_work_output),
    cargo_clean_output: normalizeText(record.cargo_clean_output),
    post_work_output: normalizeText(record.post_work_output),
  };
}

function normalizeHistory(rawHistory = []) {
  if (!Array.isArray(rawHistory)) return [];
  return rawHistory
    .map((entry) => normalizeProofRecord(entry, normalizeProofKind(entry?.proof_kind || "", "LIVE_PREPARE_WORKTREE_HEALTH")))
    .filter(Boolean)
    .sort((left, right) => String(left.validated_at || "").localeCompare(String(right.validated_at || "")))
    .slice(-HISTORY_LIMIT);
}

function firstTruthy(...values) {
  for (const value of values) {
    if (normalizeText(value)) return value;
  }
  return "";
}

function legacyRecordFromRaw(raw) {
  if (!raw || typeof raw !== "object") return null;
  if (!("status" in raw) && !("target_head_sha" in raw) && !("prepare_worktree_dir" in raw)) return null;
  return normalizeProofRecord(raw, "LIVE_PREPARE_WORKTREE_HEALTH");
}

export function normalizeCommittedValidationEvidence(raw) {
  const legacy = legacyRecordFromRaw(raw);
  const latestRun = normalizeProofRecord(raw?.latest_run, legacy?.proof_kind || "LIVE_PREPARE_WORKTREE_HEALTH")
    || legacy
    || null;
  const latestLivePrepareWorktreeHealth =
    normalizeProofRecord(raw?.latest_live_prepare_worktree_health, "LIVE_PREPARE_WORKTREE_HEALTH")
    || latestRun;
  const latestCommittedTargetProof =
    normalizeProofRecord(raw?.latest_committed_target_proof, "COMMITTED_TARGET_PROOF")
    || (latestRun ? { ...latestRun, proof_kind: "COMMITTED_TARGET_PROOF" } : null);
  const lastSuccessfulCommittedTargetProof =
    normalizeProofRecord(raw?.last_successful_committed_target_proof, "COMMITTED_TARGET_PROOF")
    || (legacy?.status === "PASS" ? { ...legacy, proof_kind: "COMMITTED_TARGET_PROOF" } : null);

  const proofHistory = normalizeHistory(raw?.proof_history);
  const normalized = {
    schema_id: EVIDENCE_SCHEMA_ID,
    schema_version: EVIDENCE_SCHEMA_VERSION,
    wp_id: normalizeText(firstTruthy(
      raw?.wp_id,
      latestRun?.wp_id,
      latestLivePrepareWorktreeHealth?.wp_id,
      latestCommittedTargetProof?.wp_id,
      lastSuccessfulCommittedTargetProof?.wp_id,
    )),
    latest_run: latestRun ? clone(latestRun) : null,
    latest_live_prepare_worktree_health: latestLivePrepareWorktreeHealth ? clone(latestLivePrepareWorktreeHealth) : null,
    latest_committed_target_proof: latestCommittedTargetProof ? clone(latestCommittedTargetProof) : null,
    last_successful_committed_target_proof: lastSuccessfulCommittedTargetProof ? clone(lastSuccessfulCommittedTargetProof) : null,
    proof_history: proofHistory,
  };

  if (!normalized.last_successful_committed_target_proof) {
    const derived = [...proofHistory].reverse().find((entry) =>
      entry.proof_kind === "COMMITTED_TARGET_PROOF" && entry.status === "PASS");
    if (derived) normalized.last_successful_committed_target_proof = clone(derived);
  }

  return normalized;
}

function appendHistory(history, record) {
  if (!record) return history;
  return [...normalizeHistory(history), clone(record)].slice(-HISTORY_LIMIT);
}

export function recordCommittedValidationRun(existingRaw, runEvidence) {
  const existing = normalizeCommittedValidationEvidence(existingRaw);
  const normalizedRun = normalizeProofRecord(runEvidence, "LIVE_PREPARE_WORKTREE_HEALTH");
  if (!normalizedRun) return existing;

  const livePrepareStatus = normalizeStatus(
    runEvidence?.live_prepare_worktree_status || normalizedRun.live_prepare_worktree_status || normalizedRun.status,
  );
  const committedTargetStatus = normalizeStatus(
    runEvidence?.committed_target_status || normalizedRun.committed_target_status || normalizedRun.status,
  );

  const latestRun = {
    ...normalizedRun,
    status: committedTargetStatus,
    live_prepare_worktree_status: livePrepareStatus,
    committed_target_status: committedTargetStatus,
  };

  const committedTargetProof = {
    ...latestRun,
    proof_kind: "COMMITTED_TARGET_PROOF",
    status: committedTargetStatus,
  };

  const next = {
    schema_id: EVIDENCE_SCHEMA_ID,
    schema_version: EVIDENCE_SCHEMA_VERSION,
    wp_id: normalizeText(latestRun.wp_id || existing.wp_id),
    latest_run: clone(latestRun),
    latest_live_prepare_worktree_health: clone({
      ...latestRun,
      proof_kind: "LIVE_PREPARE_WORKTREE_HEALTH",
      status: livePrepareStatus,
    }),
    latest_committed_target_proof: clone(committedTargetProof),
    last_successful_committed_target_proof:
      committedTargetStatus === "PASS"
        ? clone(committedTargetProof)
        : clone(existing.last_successful_committed_target_proof),
    proof_history: appendHistory(existing.proof_history, {
      ...latestRun,
      proof_kind: "LIVE_PREPARE_WORKTREE_HEALTH",
      status: livePrepareStatus,
    }),
  };

  next.proof_history = appendHistory(next.proof_history, committedTargetProof);
  return next;
}

export function committedEvidenceForCloseout(raw) {
  const normalized = normalizeCommittedValidationEvidence(raw);
  return normalized.last_successful_committed_target_proof
    || normalized.latest_committed_target_proof
    || normalized.latest_run
    || null;
}

export function livePrepareWorktreeHealthEvidence(raw) {
  const normalized = normalizeCommittedValidationEvidence(raw);
  return normalized.latest_live_prepare_worktree_health
    || normalized.latest_run
    || null;
}

export function committedEvidenceHasDurablePass(raw) {
  const committedProof = committedEvidenceForCloseout(raw);
  return committedProof?.status === "PASS";
}
