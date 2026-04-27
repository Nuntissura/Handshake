import fs from "node:fs";
import path from "node:path";
import { GOVERNANCE_RUNTIME_ROOT_ABS } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  evaluateIntelligentReviewStaleness,
  INTELLIGENT_REVIEW_STALENESS_DAYS,
} from "./memory-manager-policy.mjs";

export const INTELLIGENT_REVIEW_LAST_RUN_PATH = path.join(
  GOVERNANCE_RUNTIME_ROOT_ABS,
  "roles_shared",
  "INTELLIGENT_REVIEW_LAST_RUN.json",
);

export function readIntelligentReviewLastRunMarker(filePath = INTELLIGENT_REVIEW_LAST_RUN_PATH) {
  try {
    if (!fs.existsSync(filePath)) return null;
    const raw = fs.readFileSync(filePath, "utf8");
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return null;
    return parsed;
  } catch {
    return null;
  }
}

export function buildIntelligentReviewCadenceStatus({
  filePath = INTELLIGENT_REVIEW_LAST_RUN_PATH,
  now = Date.now(),
} = {}) {
  const marker = readIntelligentReviewLastRunMarker(filePath);
  const lastRunIso = marker?.timestamp_utc || "";
  const evaluation = evaluateIntelligentReviewStaleness({ lastRunIso, now });
  const status = evaluation.status === "FRESH" ? "FRESH" : "DEBT";
  return {
    schema_id: "hsk.intelligent_review_cadence@1",
    status,
    evaluation_status: evaluation.status,
    reason: evaluation.reason,
    last_intelligent_review_iso: evaluation.last_intelligent_review_iso,
    days_since_intelligent_review: evaluation.days_since_intelligent_review,
    staleness_gate_days: INTELLIGENT_REVIEW_STALENESS_DAYS,
    marker_present: marker !== null,
    last_run_session_id: marker?.session_id || null,
  };
}
