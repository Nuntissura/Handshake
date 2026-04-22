import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  defaultIntegrationValidatorWorktreeDir,
  packetRequiresMergeContainmentTruth,
} from "../session/session-policy.mjs";
import { REPO_ROOT } from "./runtime-paths.mjs";
import {
  inferExecutionCloseoutMode,
  inferValidationVerdictFromPublication,
  materializeRuntimeAuthorityView,
  readExecutionPublicationView,
} from "./wp-execution-state-lib.mjs";

const SHA_RE = /^[0-9a-f]{7,40}$/i;
const RFC3339_UTC_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/;

export const MAIN_CONTAINMENT_STATUS_VALUES = [
  "NOT_STARTED",
  "MERGE_PENDING",
  "CONTAINED_IN_MAIN",
  "NOT_REQUIRED",
];

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function replaceSingleField(packetText, label, nextValue) {
  const re = new RegExp(`^(\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*)(.+)\\s*$`, "mi");
  if (!re.test(String(packetText || ""))) {
    throw new Error(`Missing packet field: ${label}`);
  }
  return String(packetText || "").replace(re, `$1${nextValue}`);
}

function replaceStatusField(packetText, nextStatus) {
  const candidates = [
    /^(\s*-\s*\*\*Status:\*\*\s*)(.+?)\s*$/mi,
    /^(\s*\*\*Status:\*\*\s*)(.+?)\s*$/mi,
    /^(\s*Status:\s*)(.+?)\s*$/mi,
  ];
  for (const candidate of candidates) {
    if (candidate.test(String(packetText || ""))) {
      return String(packetText || "").replace(candidate, (_, prefix) => `${prefix}${nextStatus}`);
    }
  }
  throw new Error("Missing packet status field");
}

function parseStatus(text) {
  const candidates = [
    /^\s*-\s*\*\*Status:\*\*\s*(.+?)\s*$/mi,
    /^\s*\*\*Status:\*\*\s*(.+?)\s*$/mi,
    /^\s*Status:\s*(.+?)\s*$/mi,
  ];
  for (const candidate of candidates) {
    const match = String(text || "").match(candidate);
    if (match) {
      return String(match[1] || "").trim();
    }
  }
  return "";
}

function extractSectionAfterHeading(text, heading) {
  const lines = String(text || "").split(/\r?\n/);
  const headingRe = new RegExp(`^##\\s+${heading}\\b`, "i");
  const startIndex = lines.findIndex((line) => headingRe.test(line));
  if (startIndex === -1) return "";

  let endIndex = lines.length;
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    if (/^##\s+\S/.test(lines[index])) {
      endIndex = index;
      break;
    }
  }
  return lines.slice(startIndex + 1, endIndex).join("\n");
}

function normalizeNoneLike(value) {
  const raw = String(value || "").trim();
  if (!raw) return "NONE";
  if (/^(NONE|N\/A|NA|NULL)$/i.test(raw)) return "NONE";
  return raw;
}

function normalizeValidationVerdict(value) {
  return String(value || "").trim().toUpperCase();
}

const VALIDATION_VERDICT_RE = /^(?:\s*#{1,6}\s+|\s*-\s*|\s*)Verdict\s*:\s*(.+)\s*$/im;
const VALIDATION_REPORT_BOUNDARY_RE = /^\s*###\s+\S/;
const VALIDATION_REPORT_TIMESTAMP_ROLE_HEADING_RE =
  /^\s*###\s+(\d{4}-\d{2}-\d{2}(?:T[^|\s]+)?)\s+\|\s+([A-Z_]+)\b(?:.*?\|\s+session=(.+))?\s*$/i;
const VALIDATION_REPORT_ROLE_BRACKET_HEADING_RE =
  /^\s*###\s+([A-Z_]+)\s+\[(\d{4}-\d{2}-\d{2}(?:T[^\]]+)?)\]\s*$/i;
const VALIDATION_REPORT_DATE_LINE_RE =
  /^\s*(?:-\s*)?(?:DATE|RECORDED_AT_UTC|REPORTED_AT_UTC)\s*:\s*(.+?)\s*$/im;
const VALIDATION_REPORT_ROLE_LINE_RE =
  /^\s*(?:-\s*)?(?:VALIDATOR_ROLE|ACTOR_ROLE|ROLE)\s*:\s*([A-Z_]+)\b.*$/im;
const VALIDATION_REPORT_SESSION_LINE_RE =
  /^\s*(?:-\s*)?(?:ACTOR_SESSION|VALIDATOR_SESSION|SESSION)\s*:\s*(.+?)\s*$/im;
const VALIDATION_REPORT_VALIDATOR_BULLET_RE =
  /^\s*-\s*Validator\s*:\s*([A-Z_]+)(?:\s*\((.*?)\))?\s*$/im;
const VALIDATION_REPORT_INLINE_SESSION_RE = /\bsession\s+([A-Za-z0-9:_-]+)/i;

function parseValidationReportMetadata(entry) {
  const heading = String(entry?.heading || "");
  const body = String((entry?.bodyLines || []).join("\n") || "");
  const timestampRoleHeadingMatch = heading.match(VALIDATION_REPORT_TIMESTAMP_ROLE_HEADING_RE);
  const roleBracketHeadingMatch = heading.match(VALIDATION_REPORT_ROLE_BRACKET_HEADING_RE);
  const validatorBulletMatch = body.match(VALIDATION_REPORT_VALIDATOR_BULLET_RE);
  const inlineSessionMatch = String(validatorBulletMatch?.[2] || "").match(VALIDATION_REPORT_INLINE_SESSION_RE);
  const headingTimestamp = timestampRoleHeadingMatch?.[1] || roleBracketHeadingMatch?.[2] || "";
  const headingRoleToken = timestampRoleHeadingMatch?.[2] || roleBracketHeadingMatch?.[1] || "";
  const normalizedHeadingRole = String(headingRoleToken).replace(/_REPORT(?:_[A-Z_]+)?$/i, "");

  return {
    timestampUtc: String(
      headingTimestamp
      || body.match(VALIDATION_REPORT_DATE_LINE_RE)?.[1]
      || "",
    ).trim(),
    actorRole: normalizeValidationVerdict(
      body.match(VALIDATION_REPORT_ROLE_LINE_RE)?.[1]
      || validatorBulletMatch?.[1]
      || normalizedHeadingRole
      || "",
    ),
    actorSession: String(
      timestampRoleHeadingMatch?.[3]
      || body.match(VALIDATION_REPORT_SESSION_LINE_RE)?.[1]
      || inlineSessionMatch?.[1]
      || "",
    ).trim(),
  };
}

function validationReportHasMeaningfulContent(entry) {
  if (String(entry?.heading || "").trim()) return true;
  const body = String((entry?.bodyLines || []).join("\n") || "");
  return VALIDATION_VERDICT_RE.test(body)
    || VALIDATION_REPORT_DATE_LINE_RE.test(body)
    || VALIDATION_REPORT_ROLE_LINE_RE.test(body)
    || VALIDATION_REPORT_SESSION_LINE_RE.test(body)
    || VALIDATION_REPORT_VALIDATOR_BULLET_RE.test(body);
}

function extractValidationReportEntries(packetText) {
  const section = extractSectionAfterHeading(packetText, "VALIDATION_REPORTS");
  const lines = String(section || "").split(/\r?\n/);
  const entries = [];
  let current = null;

  for (const line of lines) {
    if (VALIDATION_REPORT_BOUNDARY_RE.test(line)) {
      if (current) entries.push(current);
      current = {
        heading: line,
        bodyLines: [],
      };
      continue;
    }

    if (!current) {
      current = {
        heading: "",
        timestampUtc: "",
        actorRole: "",
        actorSession: "",
        bodyLines: [],
      };
    }

    current.bodyLines.push(line);
  }

  if (current) entries.push(current);
  return entries.filter((entry) => validationReportHasMeaningfulContent(entry));
}

function validationVerdictFromEntry(entry) {
  const matches = [...String((entry?.bodyLines || []).join("\n") || "").matchAll(new RegExp(VALIDATION_VERDICT_RE.source, "gim"))];
  if (matches.length === 0) return "";
  return normalizeValidationVerdict(matches.at(-1)?.[1] || "");
}

function preferredPublishedVerdict(status = "") {
  const normalizedStatus = String(status || "").trim().toUpperCase();
  if (normalizedStatus === "VALIDATED (PASS)" || normalizedStatus === "DONE") return "PASS";
  if (normalizedStatus === "VALIDATED (FAIL)") return "FAIL";
  if (normalizedStatus === "VALIDATED (OUTDATED_ONLY)") return "OUTDATED_ONLY";
  if (normalizedStatus === "VALIDATED (ABANDONED)") return "ABANDONED";
  return "";
}

function normalizeSettlementBlockerKey(value) {
  const raw = String(value || "").trim();
  if (!raw) return "";
  return raw.toUpperCase().replace(/[^A-Z0-9]+/g, "_").replace(/^_+|_+$/g, "");
}

function parseValidationVerdict(packetText) {
  return parseValidationVerdictRecord(packetText).verdict;
}

export function parseValidationVerdictRecord(packetText) {
  const entries = extractValidationReportEntries(packetText);
  const preferredVerdict = preferredPublishedVerdict(parseStatus(packetText));

  const buildRecord = (entry, index) => {
    const metadata = parseValidationReportMetadata(entry);
    return {
    verdict: validationVerdictFromEntry(entry),
    timestampUtc: metadata.timestampUtc,
    actorRole: metadata.actorRole,
    actorSession: metadata.actorSession,
    evidencePointer: `VALIDATION_REPORTS[${index + 1}]`,
    reportIndex: index + 1,
    };
  };

  if (preferredVerdict) {
    for (let index = entries.length - 1; index >= 0; index -= 1) {
      const verdict = validationVerdictFromEntry(entries[index]);
      if (verdict !== preferredVerdict) continue;
      return buildRecord(entries[index], index);
    }
  }

  for (let index = entries.length - 1; index >= 0; index -= 1) {
    const verdict = validationVerdictFromEntry(entries[index]);
    if (!verdict) continue;
    return buildRecord(entries[index], index);
  }

  return {
    verdict: "",
    timestampUtc: "",
    actorRole: "",
    actorSession: "",
    evidencePointer: "",
    reportIndex: null,
  };
}

export function readVerdictSettlementTruth({
  packetText = "",
  runtimeStatus = {},
  taskBoardStatus = "",
  blockingKeys = [],
} = {}) {
  const packetStatus = parseStatus(packetText);
  const publication = readExecutionPublicationView({
    runtimeStatus,
    packetStatus,
    taskBoardStatus,
  });
  const verdictRecord = parseValidationVerdictRecord(packetText);
  const verdictOfRecord = inferValidationVerdictFromPublication({
    packetStatus: publication.packet_status || packetStatus,
    taskBoardStatus: publication.task_board_status || taskBoardStatus,
    fallbackVerdict: verdictRecord.verdict,
  });
  const publicationCloseoutMode = inferExecutionCloseoutMode({
    packetStatus: publication.packet_status || packetStatus,
    taskBoardStatus: publication.task_board_status || taskBoardStatus,
    mainContainmentStatus: publication.runtime?.main_containment_status || "",
    fallbackVerdict: "",
  });
  const settlementRecorded = Boolean(publicationCloseoutMode?.mode);
  const normalizedBlockingKeys = [...new Set(
    (Array.isArray(blockingKeys) ? blockingKeys : [])
      .map((value) => normalizeSettlementBlockerKey(value))
      .filter(Boolean),
  )];
  const settlementBlockers = !verdictOfRecord || settlementRecorded
    ? []
    : (
        normalizedBlockingKeys.length > 0
          ? normalizedBlockingKeys
          : ["TERMINAL_PUBLICATION_PENDING"]
      );

  return {
    verdictOfRecord,
    verdictRecordedAtUtc: verdictRecord.timestampUtc || "",
    verdictActorRole: verdictRecord.actorRole || "",
    verdictActorSession: verdictRecord.actorSession || "",
    verdictEvidencePointer: verdictRecord.evidencePointer || "",
    closeoutMode: publicationCloseoutMode?.mode || "",
    publicationPacketStatus: publication.packet_status || "",
    publicationTaskBoardStatus: publication.task_board_status || "",
    settlementState: !verdictOfRecord
      ? "VERDICT_PENDING"
      : settlementRecorded
        ? "SETTLED"
        : "SETTLEMENT_DEBT",
    settlementBlockers,
    terminalPublicationRecorded: settlementRecorded,
  };
}

function resolveIntegrationValidatorWorktreeAbs(packetText, repoRoot) {
  const declared = parseSingleField(packetText, "INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR")
    || defaultIntegrationValidatorWorktreeDir("");
  if (!declared) return "";
  return path.isAbsolute(declared)
    ? path.resolve(declared)
    : path.resolve(repoRoot || REPO_ROOT, declared);
}

function resolveCoderWorktreeAbs(packetText, repoRoot) {
  const declared = parseSingleField(packetText, "LOCAL_WORKTREE_DIR");
  if (!declared) return "";
  return path.isAbsolute(declared)
    ? path.resolve(declared)
    : path.resolve(repoRoot || REPO_ROOT, declared);
}

function defaultMainContainmentVerifier({ mergedMainCommit, integrationWorktreeAbs }) {
  if (!integrationWorktreeAbs || !fs.existsSync(integrationWorktreeAbs)) {
    return {
      ok: false,
      reason: `integration main worktree missing (${integrationWorktreeAbs || "<missing>"})`,
    };
  }

  const branchResult = spawnSync("git", ["-C", integrationWorktreeAbs, "rev-parse", "--abbrev-ref", "HEAD"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (branchResult.status !== 0) {
    return {
      ok: false,
      reason: `cannot read branch from ${integrationWorktreeAbs}`,
    };
  }
  const branch = String(branchResult.stdout || "").trim();
  if (branch !== "main") {
    return {
      ok: false,
      reason: `integration worktree is on ${branch || "<unknown>"} instead of main`,
    };
  }

  const containsResult = spawnSync(
    "git",
    ["-C", integrationWorktreeAbs, "merge-base", "--is-ancestor", mergedMainCommit, "main"],
    {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    },
  );

  return {
    ok: containsResult.status === 0,
    reason: containsResult.status === 0
      ? "contained in main"
      : `commit ${mergedMainCommit} is not an ancestor of local main in ${integrationWorktreeAbs}`,
  };
}

function defaultMergePendingWorktreeVerifier({ localBranch, coderWorktreeAbs }) {
  if (!localBranch) {
    return {
      ok: false,
      reason: "LOCAL_BRANCH is missing",
    };
  }
  if (!coderWorktreeAbs || !fs.existsSync(coderWorktreeAbs)) {
    return {
      ok: false,
      reason: `declared coder worktree missing (${coderWorktreeAbs || "<missing>"})`,
    };
  }

  const branchResult = spawnSync("git", ["-C", coderWorktreeAbs, "rev-parse", "--abbrev-ref", "HEAD"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (branchResult.status !== 0) {
    return {
      ok: false,
      reason: `cannot read branch from ${coderWorktreeAbs}`,
    };
  }
  const branch = String(branchResult.stdout || "").trim();
  if (branch !== localBranch) {
    return {
      ok: false,
      reason: `declared coder worktree is on ${branch || "<unknown>"} instead of ${localBranch}`,
    };
  }

  return {
    ok: true,
    reason: "coder worktree remains available for merge containment",
  };
}

export function parseMergeProgressionTruth(packetText) {
  const validationVerdictRecord = parseValidationVerdictRecord(packetText);
  return {
    packetFormatVersion: parseSingleField(packetText, "PACKET_FORMAT_VERSION"),
    status: parseStatus(packetText),
    validationVerdict: validationVerdictRecord.verdict,
    validationVerdictRecord,
    mainContainmentStatus: String(parseSingleField(packetText, "MAIN_CONTAINMENT_STATUS") || "").trim().toUpperCase(),
    mergedMainCommit: normalizeNoneLike(parseSingleField(packetText, "MERGED_MAIN_COMMIT")),
    mainContainmentVerifiedAtUtc: normalizeNoneLike(parseSingleField(packetText, "MAIN_CONTAINMENT_VERIFIED_AT_UTC")),
    runtimeStatusPath: parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE"),
  };
}

export function updateMergeProgressionTruth(packetText, {
  status,
  mainContainmentStatus,
  mergedMainCommit,
  mainContainmentVerifiedAtUtc,
} = {}) {
  let nextText = String(packetText || "");
  if (status != null) {
    nextText = replaceStatusField(nextText, status);
  }
  if (mainContainmentStatus != null) {
    nextText = replaceSingleField(nextText, "MAIN_CONTAINMENT_STATUS", mainContainmentStatus);
  }
  if (mergedMainCommit != null) {
    nextText = replaceSingleField(nextText, "MERGED_MAIN_COMMIT", mergedMainCommit);
  }
  if (mainContainmentVerifiedAtUtc != null) {
    nextText = replaceSingleField(nextText, "MAIN_CONTAINMENT_VERIFIED_AT_UTC", mainContainmentVerifiedAtUtc);
  }
  return nextText;
}

export function validateMergeProgressionTruth(
  packetText,
  {
    packetPath = "",
    repoRoot = REPO_ROOT,
    runtimeStatusData = undefined,
    mainContainmentVerifier = null,
    mergePendingWorktreeVerifier = null,
  } = {},
) {
  const parsed = parseMergeProgressionTruth(packetText);
  const errors = [];

  if (!packetRequiresMergeContainmentTruth(parsed.packetFormatVersion)) {
    return { errors, parsed };
  }

  if (!MAIN_CONTAINMENT_STATUS_VALUES.includes(parsed.mainContainmentStatus)) {
    errors.push(
      `MAIN_CONTAINMENT_STATUS must be ${MAIN_CONTAINMENT_STATUS_VALUES.join(" | ")} (got ${parsed.mainContainmentStatus || "<missing>"})`,
    );
    return { errors, parsed };
  }

  const mergedMainCommitIsNone = parsed.mergedMainCommit === "NONE";
  const verifiedAtIsNone = parsed.mainContainmentVerifiedAtUtc === "NONE";

  if (!mergedMainCommitIsNone && !SHA_RE.test(parsed.mergedMainCommit)) {
    errors.push(`MERGED_MAIN_COMMIT must be a git SHA or NONE (got ${parsed.mergedMainCommit})`);
  }
  if (!verifiedAtIsNone && !RFC3339_UTC_RE.test(parsed.mainContainmentVerifiedAtUtc)) {
    errors.push(
      `MAIN_CONTAINMENT_VERIFIED_AT_UTC must be RFC3339 UTC or N/A/NONE (got ${parsed.mainContainmentVerifiedAtUtc})`,
    );
  }

  if (/^Validated\s*\(\s*PASS\s*\)$/i.test(parsed.status)) {
    if (parsed.validationVerdict !== "PASS") {
      errors.push("Validated (PASS) requires VALIDATION_REPORTS top-level Verdict: PASS");
    }
    if (parsed.mainContainmentStatus !== "CONTAINED_IN_MAIN") {
      errors.push("Validated (PASS) requires MAIN_CONTAINMENT_STATUS=CONTAINED_IN_MAIN");
    }
    if (mergedMainCommitIsNone) {
      errors.push("Validated (PASS) requires MERGED_MAIN_COMMIT to record the merged main SHA");
    }
    if (verifiedAtIsNone) {
      errors.push("Validated (PASS) requires MAIN_CONTAINMENT_VERIFIED_AT_UTC");
    }
    if (!mergedMainCommitIsNone && SHA_RE.test(parsed.mergedMainCommit)) {
      const integrationWorktreeAbs = resolveIntegrationValidatorWorktreeAbs(packetText, repoRoot);
      const verifier = mainContainmentVerifier || defaultMainContainmentVerifier;
      const containment = verifier({
        mergedMainCommit: parsed.mergedMainCommit,
        integrationWorktreeAbs,
        packetPath,
        repoRoot,
      });
      if (!containment.ok) {
        errors.push(`Validated (PASS) requires main containment proof: ${containment.reason}`);
      }
    }
  } else if (/^Validated\s*\(\s*FAIL\s*\)$/i.test(parsed.status)) {
    if (parsed.validationVerdict !== "FAIL") {
      errors.push("Validated (FAIL) requires VALIDATION_REPORTS top-level Verdict: FAIL");
    }
    if (parsed.mainContainmentStatus !== "NOT_REQUIRED") {
      errors.push("Validated (FAIL) requires MAIN_CONTAINMENT_STATUS=NOT_REQUIRED");
    }
    if (!mergedMainCommitIsNone) {
      errors.push("Validated (FAIL) must not record MERGED_MAIN_COMMIT");
    }
    if (!verifiedAtIsNone) {
      errors.push("Validated (FAIL) must not record MAIN_CONTAINMENT_VERIFIED_AT_UTC");
    }
  } else if (/^Validated\s*\(\s*OUTDATED_ONLY\s*\)$/i.test(parsed.status)) {
    if (parsed.validationVerdict !== "OUTDATED_ONLY") {
      errors.push("Validated (OUTDATED_ONLY) requires VALIDATION_REPORTS top-level Verdict: OUTDATED_ONLY");
    }
    if (parsed.mainContainmentStatus !== "NOT_REQUIRED") {
      errors.push("Validated (OUTDATED_ONLY) requires MAIN_CONTAINMENT_STATUS=NOT_REQUIRED");
    }
    if (!mergedMainCommitIsNone) {
      errors.push("Validated (OUTDATED_ONLY) must not record MERGED_MAIN_COMMIT");
    }
    if (!verifiedAtIsNone) {
      errors.push("Validated (OUTDATED_ONLY) must not record MAIN_CONTAINMENT_VERIFIED_AT_UTC");
    }
  } else if (/^Validated\s*\(\s*ABANDONED\s*\)$/i.test(parsed.status)) {
    if (parsed.validationVerdict !== "ABANDONED") {
      errors.push("Validated (ABANDONED) requires VALIDATION_REPORTS top-level Verdict: ABANDONED");
    }
    if (parsed.mainContainmentStatus !== "NOT_REQUIRED") {
      errors.push("Validated (ABANDONED) requires MAIN_CONTAINMENT_STATUS=NOT_REQUIRED");
    }
    if (!mergedMainCommitIsNone) {
      errors.push("Validated (ABANDONED) must not record MERGED_MAIN_COMMIT");
    }
    if (!verifiedAtIsNone) {
      errors.push("Validated (ABANDONED) must not record MAIN_CONTAINMENT_VERIFIED_AT_UTC");
    }
  } else if (/^Done(?:\s*\(Historical\))?$/i.test(parsed.status)) {
    if (parsed.validationVerdict !== "PASS") {
      errors.push("Done now means merge-pending PASS closure and requires VALIDATION_REPORTS top-level Verdict: PASS");
    }
    if (parsed.mainContainmentStatus !== "MERGE_PENDING") {
      errors.push("Done requires MAIN_CONTAINMENT_STATUS=MERGE_PENDING");
    }
    if (!mergedMainCommitIsNone) {
      errors.push("Done / MERGE_PENDING must not record MERGED_MAIN_COMMIT before main containment exists");
    }
    if (!verifiedAtIsNone) {
      errors.push("Done / MERGE_PENDING must not record MAIN_CONTAINMENT_VERIFIED_AT_UTC before main containment exists");
    }
    const localBranch = parseSingleField(packetText, "LOCAL_BRANCH");
    const coderWorktreeAbs = resolveCoderWorktreeAbs(packetText, repoRoot);
    const verifier = mergePendingWorktreeVerifier || defaultMergePendingWorktreeVerifier;
    const mergePending = verifier({
      localBranch,
      coderWorktreeAbs,
      packetPath,
      repoRoot,
    });
    if (!mergePending.ok) {
      errors.push(`Done / MERGE_PENDING requires an active coder worktree path until main containment exists: ${mergePending.reason}`);
    }
  } else {
    if (parsed.mainContainmentStatus !== "NOT_STARTED") {
      errors.push(`${parsed.status || "<missing status>"} requires MAIN_CONTAINMENT_STATUS=NOT_STARTED`);
    }
    if (!mergedMainCommitIsNone) {
      errors.push(`${parsed.status || "<missing status>"} must not record MERGED_MAIN_COMMIT`);
    }
    if (!verifiedAtIsNone) {
      errors.push(`${parsed.status || "<missing status>"} must not record MAIN_CONTAINMENT_VERIFIED_AT_UTC`);
    }
  }

  if (parsed.mainContainmentStatus === "CONTAINED_IN_MAIN" && !/^Validated\s*\(\s*PASS\s*\)$/i.test(parsed.status)) {
    errors.push("MAIN_CONTAINMENT_STATUS=CONTAINED_IN_MAIN is only legal for Status: Validated (PASS)");
  }
  if (parsed.mainContainmentStatus === "MERGE_PENDING" && !/^Done(?:\s*\(Historical\))?$/i.test(parsed.status)) {
    errors.push("MAIN_CONTAINMENT_STATUS=MERGE_PENDING is only legal for Status: Done");
  }
  if (parsed.mainContainmentStatus === "NOT_REQUIRED" && !/^Validated\s*\(\s*(FAIL|OUTDATED_ONLY|ABANDONED)\s*\)$/i.test(parsed.status)) {
    errors.push("MAIN_CONTAINMENT_STATUS=NOT_REQUIRED is only legal for Status: Validated (FAIL|OUTDATED_ONLY|ABANDONED)");
  }

  const runtimePath = String(parsed.runtimeStatusPath || "").trim();
  let runtime = runtimeStatusData;
  if (runtime === undefined && runtimePath) {
    const runtimeAbs = path.isAbsolute(runtimePath) ? runtimePath : path.resolve(repoRoot, runtimePath);
    if (!fs.existsSync(runtimeAbs)) {
      errors.push(`WP runtime status surface is missing: ${runtimePath}`);
      runtime = null;
    } else {
      try {
        runtime = materializeRuntimeAuthorityView(JSON.parse(fs.readFileSync(runtimeAbs, "utf8")));
      } catch (error) {
        errors.push(`WP runtime status surface is unreadable: ${runtimePath} (${error.message})`);
        runtime = null;
      }
    }
  }

  if (runtime && typeof runtime === "object") {
    runtime = materializeRuntimeAuthorityView(runtime);
    const runtimePacketStatus = String(runtime.current_packet_status || "").trim();
    const runtimeContainmentStatus = String(runtime.main_containment_status || "").trim().toUpperCase();
    const runtimeMergedMainCommit = runtime.merged_main_commit === null ? "NONE" : normalizeNoneLike(runtime.merged_main_commit);
    const runtimeVerifiedAt = runtime.main_containment_verified_at_utc === null
      ? "NONE"
      : normalizeNoneLike(runtime.main_containment_verified_at_utc);

    if (!runtimePacketStatus) {
      errors.push("WP runtime status surface must record current_packet_status");
    } else if (runtimePacketStatus !== parsed.status) {
      errors.push(
        `WP runtime status current_packet_status (${runtimePacketStatus}) must match packet Status (${parsed.status})`,
      );
    }
    if (!runtimeContainmentStatus) {
      errors.push("WP runtime status surface must record main_containment_status");
    } else if (runtimeContainmentStatus !== parsed.mainContainmentStatus) {
      errors.push(
        `WP runtime status main_containment_status (${runtimeContainmentStatus}) must match packet MAIN_CONTAINMENT_STATUS (${parsed.mainContainmentStatus})`,
      );
    }
    if (runtimeMergedMainCommit !== parsed.mergedMainCommit) {
      errors.push(
        `WP runtime status merged_main_commit (${runtimeMergedMainCommit}) must match packet MERGED_MAIN_COMMIT (${parsed.mergedMainCommit})`,
      );
    }
    if (runtimeVerifiedAt !== parsed.mainContainmentVerifiedAtUtc) {
      errors.push(
        `WP runtime status main_containment_verified_at_utc (${runtimeVerifiedAt}) must match packet MAIN_CONTAINMENT_VERIFIED_AT_UTC (${parsed.mainContainmentVerifiedAtUtc})`,
      );
    }
  }

  return { errors, parsed };
}
