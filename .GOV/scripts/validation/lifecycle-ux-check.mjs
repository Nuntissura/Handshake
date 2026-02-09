import fs from "node:fs";
import path from "node:path";

function fail(message) {
  console.error(message);
  process.exit(1);
}

function normalizeNewlines(content) {
  return content.replace(/\r\n/g, "\n");
}

function readUtf8(filePath) {
  return fs.readFileSync(filePath, "utf8");
}

function requireFileExists(filePath) {
  if (!fs.existsSync(filePath)) {
    fail(`Missing required governance surface file: ${filePath}`);
  }
}

// Canonical governance surfaces required by role/agentic protocols.
[
  "Handshake Codex v1.4.md",
  "justfile",
  path.join(".GOV", "roles", "orchestrator", "ORCHESTRATOR_PROTOCOL.md"),
  path.join(".GOV", "roles", "coder", "CODER_PROTOCOL.md"),
  path.join(".GOV", "roles", "validator", "VALIDATOR_PROTOCOL.md"),
  path.join(".GOV", "roles", "orchestrator", "agentic", "AGENTIC_PROTOCOL.md"),
  path.join(".GOV", "roles", "coder", "agentic", "AGENTIC_PROTOCOL.md"),
  path.join(".GOV", "roles", "validator", "agentic", "AGENTIC_PROTOCOL.md"),
  path.join(".GOV", "roles_shared", "BOUNDARY_RULES.md"),
  path.join(".GOV", "roles_shared", "EVIDENCE_LEDGER.md"),
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
    path.join(".GOV", "roles", "orchestrator", "ORCHESTRATOR_PROTOCOL.md"),
    path.join(".GOV", "roles", "coder", "CODER_PROTOCOL.md"),
    path.join(".GOV", "roles", "validator", "VALIDATOR_PROTOCOL.md"),
  ];

  const requiredStrings = [
    "## Gate Visibility Output [CX-GATE-UX-001]",
    "## Lifecycle Marker [CX-LIFE-001]",
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

// If docs/ protocol mirrors exist, require they are byte-for-byte mirrors of the canonical .GOV versions.
// (docs/ is a temporary compatibility bundle; mirror drift creates instruction divergence.)
{
  const mirrors = [
    {
      docs: path.join("docs", "ORCHESTRATOR_PROTOCOL.md"),
      gov: path.join(".GOV", "roles", "orchestrator", "ORCHESTRATOR_PROTOCOL.md"),
    },
    {
      docs: path.join("docs", "CODER_PROTOCOL.md"),
      gov: path.join(".GOV", "roles", "coder", "CODER_PROTOCOL.md"),
    },
    {
      docs: path.join("docs", "VALIDATOR_PROTOCOL.md"),
      gov: path.join(".GOV", "roles", "validator", "VALIDATOR_PROTOCOL.md"),
    },
  ];

  const drift = [];
  for (const { docs, gov } of mirrors) {
    if (!fs.existsSync(docs)) continue;
    requireFileExists(gov);

    const docsContent = normalizeNewlines(readUtf8(docs));
    const govContent = normalizeNewlines(readUtf8(gov));
    if (docsContent !== govContent) {
      drift.push({ docs, gov });
    }
  }

  if (drift.length > 0) {
    const fix = drift
      .map(({ docs, gov }) => `- Copy-Item -Force ${gov} ${docs}`)
      .join("\n");
    fail(
      [
        "docs/ protocol mirror drift detected (must match canonical .GOV role protocols):",
        ...drift.flatMap(({ docs, gov }) => [`- ${docs}`, `  != ${gov}`]),
        "",
        "Suggested fix (PowerShell):",
        fix,
      ].join("\n"),
    );
  }
}

console.log("lifecycle-ux-check ok");
