#!/usr/bin/env node
/**
 * RGF-122: Extract semantic and procedural memories from smoketest reviews.
 *
 * Usage:
 *   node memory-extract-from-smoketests.mjs              (all smoketest files)
 *   node memory-extract-from-smoketests.mjs <file.md>    (specific file)
 *
 * Parses structured sections:
 *   SMOKE-FIND-* → semantic memory (failure patterns, governance lessons)
 *   SMOKE-CONTROL-* → semantic memory (positive controls, validated patterns)
 */

import fs from "node:fs";
import path from "node:path";
import { GOV_ROOT_ABS } from "../lib/runtime-paths.mjs";
import {
  openGovernanceMemoryDb,
  closeDb,
  addMemory,
} from "./governance-memory-lib.mjs";

const SMOKETEST_DIR = path.join(GOV_ROOT_ABS, "Audits", "smoketest");

function parseField(lines, startIdx, fieldName) {
  for (let i = startIdx; i < lines.length; i++) {
    const match = lines[i].match(new RegExp(`^-\\s*${fieldName}:\\s*(.+)$`));
    if (match) return match[1].trim();
  }
  return "";
}

function parseMultilineBlock(lines, startIdx, blockName) {
  const result = [];
  let found = false;
  for (let i = startIdx; i < lines.length; i++) {
    if (lines[i].match(new RegExp(`^-\\s*${blockName}:`))) {
      found = true;
      const sameLine = lines[i].replace(new RegExp(`^-\\s*${blockName}:\\s*`), "").trim();
      if (sameLine) result.push(sameLine);
      continue;
    }
    if (found) {
      if (/^\s{2,}-\s/.test(lines[i]) || /^\s{2,}\S/.test(lines[i])) {
        result.push(lines[i].trim().replace(/^-\s*/, ""));
      } else {
        break;
      }
    }
  }
  return result.join(" | ");
}

function extractFindings(content, sourceFile) {
  const lines = content.split(/\r?\n/);
  const memories = [];

  for (let i = 0; i < lines.length; i++) {
    const findingMatch = lines[i].match(/FINDING_ID:\s*(SMOKE-FIND-[\w-]+)/);
    if (findingMatch) {
      const findingId = findingMatch[1];
      const category = parseField(lines, i, "CATEGORY");
      const roleOwner = parseField(lines, i, "ROLE_OWNER");
      const severity = parseField(lines, i, "SEVERITY");
      const surface = parseField(lines, i, "SURFACE");
      const failureClass = parseField(lines, i, "FAILURE_CLASS");
      const whatWentWrong = parseMultilineBlock(lines, i, "What went wrong");
      const mechanicalFix = parseMultilineBlock(lines, i, "Mechanical fix direction");
      const impact = parseMultilineBlock(lines, i, "Impact");

      const summary = `[${severity}] ${category}/${failureClass}: ${whatWentWrong || surface}`.slice(0, 200);
      const contentBlock = [
        `Finding: ${findingId}`,
        `Category: ${category}`,
        `Role: ${roleOwner}`,
        `Severity: ${severity}`,
        `Surface: ${surface}`,
        `Failure class: ${failureClass}`,
        whatWentWrong ? `What went wrong: ${whatWentWrong}` : "",
        impact ? `Impact: ${impact}` : "",
        mechanicalFix ? `Fix direction: ${mechanicalFix}` : "",
      ].filter(Boolean).join("\n");

      memories.push({
        memoryType: mechanicalFix ? "procedural" : "semantic",
        topic: `${findingId}: ${category}`,
        summary,
        fileScope: surface,
        importance: severity === "HIGH" ? 0.9 : severity === "MEDIUM" ? 0.7 : 0.5,
        content: contentBlock,
        sourceArtifact: path.basename(sourceFile),
        sourceRole: "ORCHESTRATOR",
        metadata: { finding_id: findingId, severity, category, failure_class: failureClass },
      });
    }

    const controlMatch = lines[i].match(/CONTROL_ID:\s*(SMOKE-CONTROL-[\w-]+)/);
    if (controlMatch) {
      const controlId = controlMatch[1];
      const controlType = parseField(lines, i, "CONTROL_TYPE");
      const surface = parseField(lines, i, "SURFACE");
      const whatWentWell = parseMultilineBlock(lines, i, "What went well");
      const whyMattered = parseMultilineBlock(lines, i, "Why it mattered");

      memories.push({
        memoryType: "semantic",
        topic: `${controlId}: ${controlType}`,
        summary: `[POSITIVE] ${whatWentWell || surface}`.slice(0, 200),
        fileScope: surface,
        importance: 0.6,
        content: [
          `Control: ${controlId}`,
          `Type: ${controlType}`,
          `Surface: ${surface}`,
          whatWentWell ? `What went well: ${whatWentWell}` : "",
          whyMattered ? `Why it mattered: ${whyMattered}` : "",
        ].filter(Boolean).join("\n"),
        sourceArtifact: path.basename(sourceFile),
        sourceRole: "ORCHESTRATOR",
        metadata: { control_id: controlId, control_type: controlType },
      });
    }
  }

  return memories;
}

function processFile(db, filePath) {
  if (!fs.existsSync(filePath)) return 0;
  const content = fs.readFileSync(filePath, "utf8");
  const memories = extractFindings(content, filePath);
  let added = 0;
  for (const mem of memories) {
    const dupe = db.prepare(
      "SELECT id FROM memory_index WHERE topic = ? AND memory_type = ?"
    ).get(mem.topic, mem.memoryType);
    if (dupe) continue;
    addMemory(db, mem);
    added++;
  }
  return added;
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

const arg = process.argv[2] || "";
const { db } = openGovernanceMemoryDb();

try {
  if (arg && arg !== "--all") {
    const filePath = path.isAbsolute(arg) ? arg : path.resolve(arg);
    const added = processFile(db, filePath);
    console.log(`[memory-smoketest] ${path.basename(filePath)}: extracted ${added} memories`);
  } else {
    if (!fs.existsSync(SMOKETEST_DIR)) {
      console.log("[memory-smoketest] No smoketest directory found");
      process.exit(0);
    }
    const files = fs.readdirSync(SMOKETEST_DIR).filter(f => f.endsWith(".md")).sort();
    let total = 0;
    for (const file of files) {
      const added = processFile(db, path.join(SMOKETEST_DIR, file));
      if (added > 0) console.log(`[memory-smoketest] ${file}: extracted ${added} memories`);
      total += added;
    }
    console.log(`[memory-smoketest] Total: ${total} memories from ${files.length} reviews`);
  }
} finally {
  closeDb(db);
}
