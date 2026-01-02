import fs from "node:fs";

const TASK_BOARD_PATH = "docs/TASK_BOARD.md";

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
}

const content = readTaskBoard();
checkLines(content.split(/\r?\n/));
console.log("task-board-check ok");
