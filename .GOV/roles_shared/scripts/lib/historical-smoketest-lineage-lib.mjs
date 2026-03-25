const HISTORICAL_CLASSIFICATION_VALUES = new Set([
  "FAILED_HISTORICAL_CLOSURE",
]);

const LIVE_SMOKETEST_STATUS_VALUES = new Set([
  "LIVE_SMOKETEST_BASELINE_PENDING",
  "LIVE_SMOKETEST_BASELINE_RECOVERED",
]);

const TASK_BOARD_BASELINE_TOKEN = "FAILED_HISTORICAL_SMOKETEST_BASELINE";

function packetIdFromPath(packetPath) {
  const normalized = String(packetPath || "").replace(/\\/g, "/").trim();
  const match = normalized.match(/\/(WP-[^/]+)\/packet\.md$/) || normalized.match(/\/(WP-[^/]+)\.md$/);
  return match ? match[1] : "";
}

function extractSection(text, heading) {
  const lines = String(text || "").split(/\r?\n/);
  const headingRe = new RegExp(`^##\\s+${heading}\\s*$`, "i");
  const startIndex = lines.findIndex((line) => headingRe.test(line.trim()));
  if (startIndex === -1) return "";

  let endIndex = lines.length;
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    if (/^##\s+\S/.test(lines[index].trim())) {
      endIndex = index;
      break;
    }
  }
  return lines.slice(startIndex + 1, endIndex).join("\n");
}

function canonicalizeHeader(value) {
  return String(value || "")
    .trim()
    .toUpperCase()
    .replace(/[^A-Z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

function parseMarkdownTable(sectionText) {
  const lines = String(sectionText || "")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.startsWith("|"));
  if (lines.length < 2) return [];

  const headers = lines[0]
    .split("|")
    .slice(1, -1)
    .map(canonicalizeHeader);
  const rows = [];

  for (let index = 2; index < lines.length; index += 1) {
    const parts = lines[index].split("|").slice(1, -1).map((part) => part.trim());
    if (parts.length !== headers.length) continue;
    const row = {};
    headers.forEach((header, headerIndex) => {
      row[header] = parts[headerIndex];
    });
    rows.push(row);
  }

  return rows;
}

export function parseTraceabilityRegistryRows(registryText) {
  const section = extractSection(registryText, "Registry \\(Phase 1\\)");
  return parseMarkdownTable(section).map((row) => ({
    baseWpId: row.BASE_WP_ID || "",
    activePacketPath: row.ACTIVE_PACKET || "",
    activePacketId: packetIdFromPath(row.ACTIVE_PACKET || ""),
    taskBoardProjection: row.TASK_BOARD || "",
    notes: row.NOTES || "",
  }));
}

export function parseHistoricalSmoketestLineageRows(registryText) {
  const section = extractSection(registryText, "Historical Failure \\+ Live Smoketest Lineage");
  return parseMarkdownTable(section).map((row) => ({
    baseWpId: row.BASE_WP_ID || "",
    historicalFailedPacket: row.HISTORICAL_FAILED_PACKET || "",
    historicalClassification: row.HISTORICAL_CLASSIFICATION || "",
    liveSmoketestStatus: row.LIVE_SMOKETEST_STATUS || "",
    activeRecoveryPacket: row.ACTIVE_RECOVERY_PACKET || "",
    driverAudit: row.DRIVER_AUDIT || "",
    latestSmoketestReview: row.LATEST_SMOKETEST_REVIEW || "",
  }));
}

export function parseTaskBoardHistoricalBaselines(taskBoardText) {
  const section = extractSection(taskBoardText, "Historical Failed Closures Used As Live Smoketest Baselines");
  const entries = [];
  const lines = String(section || "").split(/\r?\n/);
  const entryRe = /^-\s+\*\*\[(WP-[^\]]+)\]\*\*\s+-\s+\[([A-Z_]+)\]\s+-\s+base_wp_id:\s+(\S+)\s+-\s+active_recovery:\s+(\S+)\s+-\s+live_status:\s+([A-Z_]+)\s*$/;

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed.startsWith("-")) continue;
    if (trimmed === "---") continue;
    const match = trimmed.match(entryRe);
    if (!match) {
      entries.push({ malformed: true, raw: trimmed });
      continue;
    }
    entries.push({
      historicalPacket: match[1],
      token: match[2],
      baseWpId: match[3],
      activeRecoveryPacket: match[4],
      liveSmoketestStatus: match[5],
    });
  }

  return entries;
}

export function parseTaskBoardSupersededIds(taskBoardText) {
  const section = extractSection(taskBoardText, "Superseded \\(Archive\\)");
  const ids = new Set();
  for (const line of String(section || "").split(/\r?\n/)) {
    const match = line.trim().match(/^- \*\*\[(WP-[^\]]+)\]\*\* - \[SUPERSEDED\]\s*$/);
    if (match) ids.add(match[1]);
  }
  return ids;
}

export function validateHistoricalSmoketestLineage({
  registryText,
  taskBoardText,
} = {}) {
  const errors = [];
  const registryRows = parseTraceabilityRegistryRows(registryText);
  const lineageRows = parseHistoricalSmoketestLineageRows(registryText);
  const baselineEntries = parseTaskBoardHistoricalBaselines(taskBoardText);
  const supersededIds = parseTaskBoardSupersededIds(taskBoardText);

  const registryByBaseWpId = new Map(registryRows.map((row) => [row.baseWpId, row]));
  const lineageByBaseWpId = new Map(lineageRows.map((row) => [row.baseWpId, row]));
  const baselineByHistoricalPacket = new Map();

  for (const entry of baselineEntries) {
    if (entry.malformed) {
      errors.push(
        `TASK_BOARD historical smoketest baseline entries must be \`- **[WP_ID]** - [${TASK_BOARD_BASELINE_TOKEN}] - base_wp_id: <BASE_WP_ID> - active_recovery: <WP_ID> - live_status: <STATUS>\` (${entry.raw})`,
      );
      continue;
    }
    baselineByHistoricalPacket.set(entry.historicalPacket, entry);
    if (entry.token !== TASK_BOARD_BASELINE_TOKEN) {
      errors.push(
        `TASK_BOARD historical baseline ${entry.historicalPacket} must use token [${TASK_BOARD_BASELINE_TOKEN}], not [${entry.token}]`,
      );
    }
  }

  for (const registryRow of registryRows) {
    if (!/historical failure\/live smoketest lineage is modeled below|blocked legacy history after|failed historical/i.test(registryRow.notes)) {
      continue;
    }
    if (!lineageByBaseWpId.has(registryRow.baseWpId)) {
      errors.push(
        `WP_TRACEABILITY_REGISTRY base WP ${registryRow.baseWpId} declares blocked historical lineage in Notes but is missing a row in ## Historical Failure + Live Smoketest Lineage`,
      );
    }
  }

  for (const lineageRow of lineageRows) {
    const registryRow = registryByBaseWpId.get(lineageRow.baseWpId);
    if (!registryRow) {
      errors.push(`Historical smoketest lineage row references unknown BASE_WP_ID ${lineageRow.baseWpId}`);
      continue;
    }

    if (!HISTORICAL_CLASSIFICATION_VALUES.has(lineageRow.historicalClassification)) {
      errors.push(
        `${lineageRow.baseWpId} historical classification must be one of ${[...HISTORICAL_CLASSIFICATION_VALUES].join(" | ")} (got ${lineageRow.historicalClassification || "<missing>"})`,
      );
    }

    if (!LIVE_SMOKETEST_STATUS_VALUES.has(lineageRow.liveSmoketestStatus)) {
      errors.push(
        `${lineageRow.baseWpId} live smoketest status must be one of ${[...LIVE_SMOKETEST_STATUS_VALUES].join(" | ")} (got ${lineageRow.liveSmoketestStatus || "<missing>"})`,
      );
    }

    if (!lineageRow.historicalFailedPacket.startsWith(`${lineageRow.baseWpId}-`)) {
      errors.push(
        `${lineageRow.baseWpId} historical failed packet must be a revision of the base WP (got ${lineageRow.historicalFailedPacket || "<missing>"})`,
      );
    }

    if (lineageRow.activeRecoveryPacket !== registryRow.activePacketId) {
      errors.push(
        `${lineageRow.baseWpId} active recovery packet (${lineageRow.activeRecoveryPacket}) must match the traceability registry active packet (${registryRow.activePacketId})`,
      );
    }

    if (lineageRow.historicalFailedPacket === lineageRow.activeRecoveryPacket) {
      errors.push(`${lineageRow.baseWpId} historical failed packet must differ from the active recovery packet`);
    }

    if (!/^AUDIT[-_]/.test(lineageRow.driverAudit)) {
      errors.push(`${lineageRow.baseWpId} driver audit must be a stable AUDIT_ID (got ${lineageRow.driverAudit || "<missing>"})`);
    }

    if (
      lineageRow.liveSmoketestStatus === "LIVE_SMOKETEST_BASELINE_RECOVERED"
      && !/^SMOKETEST-REVIEW-/.test(lineageRow.latestSmoketestReview)
    ) {
      errors.push(
        `${lineageRow.baseWpId} recovered live smoketest lineage must record LATEST_SMOKETEST_REVIEW as a stable SMOKETEST_REVIEW_ID`,
      );
    }

    if (
      lineageRow.liveSmoketestStatus === "LIVE_SMOKETEST_BASELINE_PENDING"
      && lineageRow.latestSmoketestReview !== "NONE"
      && !/^SMOKETEST-REVIEW-/.test(lineageRow.latestSmoketestReview)
    ) {
      errors.push(
        `${lineageRow.baseWpId} pending live smoketest lineage must use SMOKETEST_REVIEW_ID or NONE (got ${lineageRow.latestSmoketestReview})`,
      );
    }

    if (
      lineageRow.liveSmoketestStatus === "LIVE_SMOKETEST_BASELINE_RECOVERED"
      && !/^Done:/i.test(registryRow.taskBoardProjection)
    ) {
      errors.push(
        `${lineageRow.baseWpId} recovered live smoketest lineage must project to a Done task-board state (got ${registryRow.taskBoardProjection || "<missing>"})`,
      );
    }

    if (
      lineageRow.liveSmoketestStatus === "LIVE_SMOKETEST_BASELINE_PENDING"
      && /^Done:/i.test(registryRow.taskBoardProjection)
    ) {
      errors.push(
        `${lineageRow.baseWpId} pending live smoketest lineage must not project to Done while the recovery packet is still pending`,
      );
    }

    const baselineEntry = baselineByHistoricalPacket.get(lineageRow.historicalFailedPacket);
    if (!baselineEntry) {
      errors.push(
        `${lineageRow.baseWpId} historical failed packet ${lineageRow.historicalFailedPacket} is missing from TASK_BOARD historical smoketest baseline section`,
      );
    } else {
      if (baselineEntry.baseWpId !== lineageRow.baseWpId) {
        errors.push(
          `${lineageRow.historicalFailedPacket} task-board baseline base_wp_id (${baselineEntry.baseWpId}) must match the lineage row (${lineageRow.baseWpId})`,
        );
      }
      if (baselineEntry.activeRecoveryPacket !== lineageRow.activeRecoveryPacket) {
        errors.push(
          `${lineageRow.historicalFailedPacket} task-board baseline active_recovery (${baselineEntry.activeRecoveryPacket}) must match the lineage row (${lineageRow.activeRecoveryPacket})`,
        );
      }
      if (baselineEntry.liveSmoketestStatus !== lineageRow.liveSmoketestStatus) {
        errors.push(
          `${lineageRow.historicalFailedPacket} task-board baseline live_status (${baselineEntry.liveSmoketestStatus}) must match the lineage row (${lineageRow.liveSmoketestStatus})`,
        );
      }
    }

    if (!supersededIds.has(lineageRow.historicalFailedPacket)) {
      errors.push(
        `${lineageRow.historicalFailedPacket} must remain listed under TASK_BOARD ## Superseded (Archive) while it is also modeled as a historical smoketest baseline`,
      );
    }
  }

  for (const entry of baselineEntries) {
    if (entry.malformed) continue;
    const lineageRow = lineageRows.find((row) => row.historicalFailedPacket === entry.historicalPacket);
    if (!lineageRow) {
      errors.push(
        `TASK_BOARD historical smoketest baseline ${entry.historicalPacket} is not backed by a row in WP_TRACEABILITY_REGISTRY ## Historical Failure + Live Smoketest Lineage`,
      );
    }
  }

  return { errors };
}
