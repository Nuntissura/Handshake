#!/usr/bin/env node
/**
 * Error/trace determinism audit:
 * - Flags stringly errors in production paths
 * - Flags unseeded randomness/time sources in production paths
 *
 * Exits non-zero on findings.
 */
import fs from "node:fs";
import path from "node:path";

const targets = ["src/backend/handshake_core/src"];
const waiverMarker = "WAIVER [CX-573E]";

const stringErrorPatterns = [
  'Err\\(\\s*"', // Err("msg")
  "Err\\(\\s*String::from",
  "Err\\(\\s*format!",
  'map_err\\(\\|.*\\|\\s*"', // map_err(|_| "msg")
  "map_err\\(\\|.*\\|\\s*String::from",
  "map_err\\(\\|.*\\|\\s*format!",
  "anyhow!\\(",
  "bail!\\(",
];

const nondeterminismPatterns = [
  "rand::",
  "thread_rng",
  "rand\\(",
  "Instant::now\\(",
  "SystemTime::now\\(",
];

function toPosixPath(filePath) {
  return filePath.replace(/\\/g, "/");
}

function shouldExclude(relativePosixPath) {
  if (relativePosixPath.includes("/tests/")) return true;

  const parts = relativePosixPath.split("/");
  const filename = parts.at(-1) ?? "";
  if (filename.includes("test")) return true;
  for (const part of parts.slice(0, -1)) {
    if (part.includes("test")) return true;
  }

  return false;
}

function collectTargetFiles() {
  const files = [];

  for (const target of targets) {
    const targetAbs = path.resolve(process.cwd(), target);
    if (!fs.existsSync(targetAbs)) continue;

    const stack = [{ absDir: targetAbs, relDir: target }];
    while (stack.length > 0) {
      const next = stack.pop();
      if (!next) break;

      let entries;
      try {
        entries = fs.readdirSync(next.absDir, { withFileTypes: true });
      } catch (err) {
        console.error(
          `validator-error-codes: failed to read directory ${next.absDir}: ${err.message}`
        );
        process.exit(1);
      }

      entries.sort((a, b) => a.name.localeCompare(b.name));

      for (const entry of entries) {
        const absPath = path.join(next.absDir, entry.name);
        const relPath = path.join(next.relDir, entry.name);
        const relPosix = toPosixPath(relPath);

        if (entry.isDirectory()) {
          if (shouldExclude(`${relPosix}/`)) continue;
          stack.push({ absDir: absPath, relDir: relPath });
          continue;
        }

        if (!entry.isFile()) continue;
        if (shouldExclude(relPosix)) continue;
        if (!relPosix.endsWith(".rs")) continue;

        files.push({ absPath, relPosix });
      }
    }
  }

  files.sort((a, b) => a.relPosix.localeCompare(b.relPosix));
  return files;
}

function normalizeNewlines(text) {
  return text.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
}

function buildLineStarts(lines) {
  const starts = new Array(lines.length);
  let offset = 0;
  for (let i = 0; i < lines.length; i += 1) {
    starts[i] = offset;
    offset += lines[i].length + 1;
  }
  return starts;
}

function findLineIndex(lineStarts, offset) {
  let low = 0;
  let high = lineStarts.length - 1;
  while (low <= high) {
    const mid = Math.floor((low + high) / 2);
    if (lineStarts[mid] <= offset) {
      low = mid + 1;
    } else {
      high = mid - 1;
    }
  }
  return Math.max(0, low - 1);
}

function hasAdjacentWaiver(lines, lineIndex) {
  const cur = lines[lineIndex] ?? "";
  const prev = lineIndex > 0 ? lines[lineIndex - 1] ?? "" : "";
  return cur.includes(waiverMarker) || prev.includes(waiverMarker);
}

function scanPatternAcrossFiles(files, pattern, label) {
  const regex = new RegExp(pattern, "g");
  const hits = [];

  for (const file of files) {
    let text;
    try {
      text = fs.readFileSync(file.absPath, "utf8");
    } catch (err) {
      console.error(
        `validator-error-codes: ${label} scan failed: cannot read ${file.relPosix}: ${err.message}`
      );
      process.exit(1);
    }

    const normalized = normalizeNewlines(text);
    const lines = normalized.split("\n");
    const lineStarts = buildLineStarts(lines);

    regex.lastIndex = 0;
    const matchedLineNumbers = new Set();

    while (true) {
      const match = regex.exec(normalized);
      if (!match) break;

      const lineIndex = findLineIndex(lineStarts, match.index);
      const lineNumber = lineIndex + 1;

      if (
        label === "determinism" &&
        (pattern === "Instant::now\\(" || pattern === "SystemTime::now\\(") &&
        hasAdjacentWaiver(lines, lineIndex)
      ) {
        continue;
      }

      if (matchedLineNumbers.has(lineNumber)) continue;
      matchedLineNumbers.add(lineNumber);
      hits.push(`${file.relPosix}:${lineNumber}:${lines[lineIndex] ?? ""}`);
    }
  }

  return hits.join("\n");
}

const findings = [];
const files = collectTargetFiles();

for (const pat of stringErrorPatterns) {
  const out = scanPatternAcrossFiles(files, pat, "string-error");
  if (out) {
    findings.push(`STRING_ERROR pattern "${pat}":\n${out}`);
  }
}

for (const pat of nondeterminismPatterns) {
  const out = scanPatternAcrossFiles(files, pat, "determinism");
  if (out) {
    findings.push(`NONDETERMINISM pattern "${pat}":\n${out}`);
  }
}

if (findings.length > 0) {
  console.error("validator-error-codes: FAIL/WARN findings detected");
  findings.forEach((f) => {
    console.error("----");
    console.error(f);
  });
  process.exit(1);
}

console.log(
  "validator-error-codes: PASS - no stringly errors or nondeterminism patterns detected."
);
