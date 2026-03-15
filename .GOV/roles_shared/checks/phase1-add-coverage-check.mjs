import fs from "node:fs";
import path from "node:path";

const SPEC_CURRENT_PATH = ".GOV/roles_shared/SPEC_CURRENT.md";
const STUB_DIR = ".GOV/task_packets/stubs";
const PHASE_HEADING = "### 7.6.3 Phase 1";
const NEXT_PHASE_HEADING = "### 7.6.4 Phase 2";
const COVERAGE_PREFIX = "- ROADMAP_ADD_COVERAGE:";

function fail(message, details = []) {
  console.error(`[PHASE1_ADD_COVERAGE_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function readText(filePath) {
  if (!fs.existsSync(filePath)) {
    fail("Missing required file", [filePath]);
  }
  return fs.readFileSync(filePath, "utf8");
}

function parseCurrentSpecTarget() {
  const specCurrent = readText(SPEC_CURRENT_PATH);
  const match = specCurrent.match(/Handshake_Master_Spec_(v\d+(?:\.\d+)*)\.md/);
  if (!match) {
    fail("Unable to parse SPEC_CURRENT target", [
      `${SPEC_CURRENT_PATH}: expected Handshake_Master_Spec_vXX.XXX.md`,
    ]);
  }
  return {
    versionTag: match[1],
    fileName: `Handshake_Master_Spec_${match[1]}.md`,
  };
}

function findPhaseRange(lines) {
  let start = -1;
  let end = -1;
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (start === -1 && line.startsWith(PHASE_HEADING)) {
      start = index + 1;
      continue;
    }
    if (start !== -1 && line.startsWith(NEXT_PHASE_HEADING)) {
      end = index + 1;
      break;
    }
  }
  if (start === -1 || end === -1 || end <= start) {
    fail("Unable to locate Phase 1 roadmap block in current spec", [
      `Expected headings: "${PHASE_HEADING}" and "${NEXT_PHASE_HEADING}"`,
    ]);
  }
  return { start, end };
}

function collectPhase1CurrentVersionAddLines(specLines, phaseStart, phaseEnd, versionTag) {
  const token = `[ADD ${versionTag}]`;
  const result = [];
  for (let lineNumber = phaseStart; lineNumber < phaseEnd; lineNumber += 1) {
    const line = specLines[lineNumber - 1] ?? "";
    if (line.includes(token)) {
      result.push({ lineNumber, lineText: line.trim() });
    }
  }
  return result;
}

function parseLineSet(raw) {
  const result = new Set();
  const tokens = raw
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean);
  for (const token of tokens) {
    const rangeMatch = token.match(/^(\d+)\s*-\s*(\d+)$/);
    if (rangeMatch) {
      const start = Number(rangeMatch[1]);
      const end = Number(rangeMatch[2]);
      if (!Number.isInteger(start) || !Number.isInteger(end) || start <= 0 || end <= 0 || end < start) {
        throw new Error(`Invalid line range: "${token}"`);
      }
      for (let line = start; line <= end; line += 1) result.add(line);
      continue;
    }
    const single = Number(token);
    if (!Number.isInteger(single) || single <= 0) {
      throw new Error(`Invalid line number: "${token}"`);
    }
    result.add(single);
  }
  return result;
}

function collectCoverageFromStubs(versionTag) {
  if (!fs.existsSync(STUB_DIR)) {
    fail("Missing stub directory", [STUB_DIR]);
  }

  const coverage = new Map();
  const parseErrors = [];
  const stubFiles = fs.readdirSync(STUB_DIR).filter((name) => name.endsWith(".md"));

  for (const fileName of stubFiles) {
    const fullPath = path.join(STUB_DIR, fileName);
    const lines = fs.readFileSync(fullPath, "utf8").split(/\r?\n/);
    for (let index = 0; index < lines.length; index += 1) {
      const line = lines[index].trim();
      if (!line.startsWith(COVERAGE_PREFIX)) continue;

      const payload = line.slice(COVERAGE_PREFIX.length).trim();
      const match = payload.match(/^SPEC=(v\d+(?:\.\d+)*);\s*PHASE=(7\.6\.3);\s*LINES=(.+)$/);
      if (!match) {
        parseErrors.push(
          `${fullPath.replace(/\\/g, "/")}:${index + 1}: invalid ROADMAP_ADD_COVERAGE format. Expected: SPEC=vXX.XXX; PHASE=7.6.3; LINES=123,124-126`
        );
        continue;
      }

      const spec = match[1];
      const linesRaw = match[3];
      if (spec !== versionTag) continue;

      try {
        const lineSet = parseLineSet(linesRaw);
        for (const coveredLine of lineSet) {
          const refs = coverage.get(coveredLine) ?? [];
          refs.push(fullPath.replace(/\\/g, "/"));
          coverage.set(coveredLine, refs);
        }
      } catch (error) {
        parseErrors.push(
          `${fullPath.replace(/\\/g, "/")}:${index + 1}: ${(error && error.message) || "invalid line set"}`
        );
      }
    }
  }

  if (parseErrors.length > 0) {
    fail("ROADMAP_ADD_COVERAGE metadata parse errors", parseErrors);
  }

  return coverage;
}

const currentSpec = parseCurrentSpecTarget();
const specContent = readText(currentSpec.fileName);
const specLines = specContent.split(/\r?\n/);
const phaseRange = findPhaseRange(specLines);
const phaseAddLines = collectPhase1CurrentVersionAddLines(
  specLines,
  phaseRange.start,
  phaseRange.end,
  currentSpec.versionTag
);

if (phaseAddLines.length === 0) {
  console.log(`phase1-add-coverage-check ok (no Phase 1 [ADD ${currentSpec.versionTag}] lines found)`);
  process.exit(0);
}

const coveredByStubs = collectCoverageFromStubs(currentSpec.versionTag);
const expectedLineNumbers = new Set(phaseAddLines.map((item) => item.lineNumber));

const missing = [];
for (const item of phaseAddLines) {
  if (!coveredByStubs.has(item.lineNumber)) {
    missing.push(
      `${currentSpec.fileName}:${item.lineNumber} not covered by any stub ROADMAP_ADD_COVERAGE (text: ${item.lineText})`
    );
  }
}

const extra = [];
for (const coveredLine of coveredByStubs.keys()) {
  if (!expectedLineNumbers.has(coveredLine)) {
    const refs = coveredByStubs.get(coveredLine) ?? [];
    extra.push(
      `Stub ROADMAP_ADD_COVERAGE references ${currentSpec.fileName}:${coveredLine}, but that line is not a Phase 1 [ADD ${currentSpec.versionTag}] item (${refs.join(", ")})`
    );
  }
}

if (missing.length > 0 || extra.length > 0) {
  fail("Phase 1 current-version ADD coverage mismatches detected", [...missing, ...extra]);
}

console.log(
  `phase1-add-coverage-check ok (${phaseAddLines.length} Phase 1 [ADD ${currentSpec.versionTag}] lines covered)`
);
