import fs from "node:fs";
import path from "node:path";
import {
  ensureGovernanceRuntimeDir,
  repoRelativeGovernanceRuntimePath,
} from "./runtime-paths.mjs";

function nowStamp() {
  return new Date().toISOString().replace(/[:.]/g, "-");
}

function clipLine(line, maxChars = 220) {
  const normalized = String(line || "").replace(/\s+/g, " ").trim();
  if (!normalized) return "";
  return normalized.length > maxChars ? `${normalized.slice(0, maxChars - 3)}...` : normalized;
}

export function compactGateOutputSummary(output, { maxLines = 4, maxChars = 220 } = {}) {
  const lines = String(output || "")
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .filter((line) => line.trim().length > 0);
  if (lines.length === 0) return ["<no output>"];

  const preferred = lines.filter((line) =>
    /^(\[.+\]\s+(PASS|FAIL|CONTEXT_MISMATCH)|PASS:|FAIL:|WARN:|INFO:|Pre-work validation|Post-work checks|You may proceed|NOTE:|RESULT:|WHY:)/i.test(line.trim()),
  );
  const selected = (preferred.length > 0 ? preferred : lines).slice(-maxLines);
  return selected.map((line) => clipLine(line, maxChars));
}

export function writeGateOutputArtifact(gateName, wpId, sections = []) {
  const safeGate = String(gateName || "gate")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-");
  const safeWp = String(wpId || "WP-UNKNOWN").trim();
  const dirAbs = ensureGovernanceRuntimeDir("roles_shared", "GATE_OUTPUTS", safeGate, safeWp);
  const fileName = `${nowStamp()}.log`;
  const fileAbs = path.join(dirAbs, fileName);
  const content = sections
    .map((section) => {
      const title = String(section?.title || "OUTPUT").trim() || "OUTPUT";
      const body = String(section?.body || "").replace(/\r/g, "").trimEnd();
      return `## ${title}\n${body}\n`;
    })
    .join("\n");
  fs.writeFileSync(fileAbs, `${content.trimEnd()}\n`, "utf8");
  return repoRelativeGovernanceRuntimePath("roles_shared", "GATE_OUTPUTS", safeGate, safeWp, fileName);
}
