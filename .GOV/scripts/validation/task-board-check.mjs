import fs from "node:fs";
import path from "node:path";

const TASK_BOARD_PATH = ".GOV/roles_shared/TASK_BOARD.md";

function fail(message, details = []) {
  console.error(`[TASK_BOARD_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function readTaskBoard() {
  if (!fs.existsSync(TASK_BOARD_PATH)) {
    fail("Missing task board", [`Expected: ${TASK_BOARD_PATH}`]);
  }
  return fs.readFileSync(TASK_BOARD_PATH, "utf8");
}

function sectionKeyFromHeading(headingLine) {
  const heading = headingLine.replace(/^##\s+/, "").trim();
  if (heading === "In Progress") return "IN_PROGRESS";
  if (heading === "Done") return "DONE";
  if (heading.startsWith("Superseded")) return "SUPERSEDED";
  return null;
}

function checkLines(lines) {
  const doneRe = /^-\s+\*\*\[(WP-[^\]]+)\]\*\*\s+-\s+\[(VALIDATED|FAIL|OUTDATED_ONLY)\]\s*$/;
  const supersededRe = /^-\s+\*\*\[(WP-[^\]]+)\]\*\*\s+-\s+\[SUPERSEDED\]\s*$/;
  const inProgressRe = /^-\s+\*\*\[(WP-[^\]]+)\]\*\*\s+-\s+\[IN_PROGRESS\]\s*$/;

  let active = null;
  const violations = [];
  const doneEntries = [];

  for (let index = 0; index < lines.length; index += 1) {
    const lineNumber = index + 1;
    const line = lines[index];

    if (line.startsWith("## ")) {
      active = sectionKeyFromHeading(line);
      continue;
    }

    if (!active) continue;
    if (!line.trim().startsWith("-")) continue;

    if (active === "DONE" && !doneRe.test(line)) {
      violations.push(
        `${TASK_BOARD_PATH}:${lineNumber}: Done entries must be \`- **[WP_ID]** - [VALIDATED|FAIL|OUTDATED_ONLY]\`: ${line.trim()}`
      );
      continue;
    }
    if (active === "DONE") {
      const m = line.match(doneRe);
      if (m) doneEntries.push({ wpId: m[1], status: m[2], lineNumber });
    }

    if (active === "SUPERSEDED" && !supersededRe.test(line)) {
      violations.push(
        `${TASK_BOARD_PATH}:${lineNumber}: Superseded entries must be \`- **[WP_ID]** - [SUPERSEDED]\`: ${line.trim()}`
      );
      continue;
    }

    if (active === "IN_PROGRESS" && !inProgressRe.test(line)) {
      violations.push(
        `${TASK_BOARD_PATH}:${lineNumber}: In Progress entries must be \`- **[WP_ID]** - [IN_PROGRESS]\`: ${line.trim()}`
      );
    }
  }

  if (violations.length > 0) {
    fail("Task board format violations found", violations);
  }

  // Semantic guard: if a WP is marked Done on the task board and the packet is in the
  // modern format (PACKET_FORMAT_VERSION present), it must include a Validator verdict line.
  // This prevents "status sync" commits from marking VALIDATED without the canonical packet report.
  const packetDir = path.join("docs", "task_packets");
  const semanticViolations = [];
  for (const entry of doneEntries) {
    const packetPath = path.join(packetDir, `${entry.wpId}.md`);
    if (!fs.existsSync(packetPath)) {
      semanticViolations.push(
        `${TASK_BOARD_PATH}:${entry.lineNumber}: Done WP has no task packet file: ${packetPath.replace(/\\/g, "/")}`
      );
      continue;
    }
    const packetText = fs.readFileSync(packetPath, "utf8");
    const isModernPacket = /^\s*-\s*PACKET_FORMAT_VERSION\s*:/mi.test(packetText);
    if (!isModernPacket) continue;
    const hasVerdict = /^\s*Verdict\s*:\s*(PASS|FAIL|OUTDATED_ONLY)\b/mi.test(packetText);
    if (!hasVerdict) {
      semanticViolations.push(
        `${TASK_BOARD_PATH}:${entry.lineNumber}: ${entry.wpId} is marked [${entry.status}] but task packet is missing a Validator verdict line (expected under ## VALIDATION_REPORTS).`
      );
    }
  }
  if (semanticViolations.length > 0) {
    fail("Task board semantic violations found", semanticViolations);
  }
}

const content = readTaskBoard();
checkLines(content.split(/\r?\n/));
console.log("task-board-check ok");

