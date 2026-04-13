import fs from "node:fs";
import path from "node:path";
import { GOV_ROOT_REPO_REL, repoPathAbs } from "../scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";

registerFailCaptureHook("lifecycle-ux-check.mjs", { role: "SHARED" });

function fail(message) {
  failWithMemory("lifecycle-ux-check.mjs", message, { role: "SHARED" });
}

function readUtf8(filePath) {
  return fs.readFileSync(repoPathAbs(filePath), "utf8");
}

function requireFileExists(filePath) {
  if (!fs.existsSync(repoPathAbs(filePath))) {
    fail(`Missing required governance surface file: ${filePath}`);
  }
}

// Canonical governance surfaces required by role/agentic protocols.
[
  path.join(GOV_ROOT_REPO_REL, "codex", "Handshake_Codex_v1.4.md"),
  "justfile",
  path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "ORCHESTRATOR_PROTOCOL.md"),
  path.join(GOV_ROOT_REPO_REL, "roles", "coder", "CODER_PROTOCOL.md"),
  path.join(GOV_ROOT_REPO_REL, "roles", "validator", "VALIDATOR_PROTOCOL.md"),
  path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "agentic", "AGENTIC_PROTOCOL.md"),
  path.join(GOV_ROOT_REPO_REL, "roles", "coder", "agentic", "AGENTIC_PROTOCOL.md"),
  path.join(GOV_ROOT_REPO_REL, "roles", "validator", "agentic", "AGENTIC_PROTOCOL.md"),
  path.join(GOV_ROOT_REPO_REL, "roles_shared", "docs", "BOUNDARY_RULES.md"),
  path.join(GOV_ROOT_REPO_REL, "roles_shared", "docs", "EVIDENCE_LEDGER.md"),
].forEach(requireFileExists);

// Ensure the mechanical hard-gate helper prints operator-facing lifecycle + phase templates.
{
  const justfileContent = readUtf8("justfile");
  const match = justfileContent.match(
    /^hard-gate-wt-001:\s*\n([\s\S]*?)(?=^\S[^\\n]*?:\s*$|\Z)/m,
  );
  if (!match) {
    fail("justfile missing recipe: hard-gate-wt-001 [CX-WT-001]");
  }

  const recipeBody = match[1] || "";
  const requiredMarkers = [
    "LIFECYCLE [CX-LIFE-001]",
    "OPERATOR_ACTION:",
    "STATE:",
    "HARD_GATE_OUTPUT [CX-WT-001]",
    "PHASE_STATUS [CX-GATE-UX-001]",
  ];

  const missingMarkers = requiredMarkers.filter((m) => !recipeBody.includes(m));
  if (missingMarkers.length > 0) {
    fail(
      [
        "hard-gate-wt-001 template missing required operator-facing markers:",
        ...missingMarkers.map((m) => `- ${m}`),
      ].join("\n"),
    );
  }
}

// Ensure role protocols include lifecycle + gate UX requirements.
{
  const protocolFiles = [
    path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "ORCHESTRATOR_PROTOCOL.md"),
    path.join(GOV_ROOT_REPO_REL, "roles", "coder", "CODER_PROTOCOL.md"),
    path.join(GOV_ROOT_REPO_REL, "roles", "validator", "VALIDATOR_PROTOCOL.md"),
  ];

  const requiredStrings = [
    "## Gate Visibility Output [CX-GATE-UX-001]",
    "## Lifecycle Marker [CX-LIFE-001]",
    "OPERATOR_ACTION:",
  ];

  for (const filePath of protocolFiles) {
    const content = readUtf8(filePath);
    const missing = requiredStrings.filter((s) => !content.includes(s));
    if (missing.length > 0) {
      fail(
        [
          `Protocol missing required sections: ${filePath}`,
          ...missing.map((m) => `- ${m}`),
        ].join("\n"),
      );
    }
  }
}

console.log("lifecycle-ux-check ok");
