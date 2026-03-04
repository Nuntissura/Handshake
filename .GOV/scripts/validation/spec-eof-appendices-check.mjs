import fs from "node:fs";

import { resolveSpecCurrent } from "./refinement-check.mjs";

function fail(msg, details = []) {
  console.error(msg);
  for (const d of details) console.error(`- ${d}`);
  process.exit(1);
}

function escapeRegExp(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function resolveSpecVersionFromFilename(specFileName) {
  const m = specFileName.match(/_v([0-9.]+)\.md$/);
  if (!m) return null;
  return `v${m[1]}`;
}

function extractAppendixJson({ content, appendixId }) {
  const beginRe = new RegExp(
    `<!--\\s*HS_APPENDIX:BEGIN\\s+id=${escapeRegExp(appendixId)}\\s+schema=([^\\s]+)\\s*-->`,
    "m",
  );
  const beginMatch = content.match(beginRe);
  if (!beginMatch) {
    fail("Missing required EOF appendix BEGIN marker.", [`id=${appendixId}`]);
  }

  const schemaFromMarker = (beginMatch[1] || "").trim();

  const beginIdx = content.indexOf(beginMatch[0]);
  const afterBeginIdx = beginIdx + beginMatch[0].length;

  const endRe = new RegExp(
    `<!--\\s*HS_APPENDIX:END\\s+id=${escapeRegExp(appendixId)}\\s*-->`,
    "m",
  );
  const endMatch = content.slice(afterBeginIdx).match(endRe);
  if (!endMatch) {
    fail("Missing required EOF appendix END marker.", [`id=${appendixId}`]);
  }

  const blockBody = content.slice(afterBeginIdx, afterBeginIdx + endMatch.index);

  const jsonRe = /```json\s*\r?\n([\s\S]*?)\r?\n```/m;
  const jsonMatch = blockBody.match(jsonRe);
  if (!jsonMatch) {
    fail("Missing ```json fenced block for EOF appendix.", [`id=${appendixId}`]);
  }

  const jsonText = (jsonMatch[1] || "").trim();
  if (!jsonText) {
    fail("Empty JSON body for EOF appendix.", [`id=${appendixId}`]);
  }

  let parsed;
  try {
    parsed = JSON.parse(jsonText);
  } catch (e) {
    fail("Invalid JSON in EOF appendix block.", [
      `id=${appendixId}`,
      String(e?.message || e),
    ]);
  }

  return { schemaFromMarker, parsed };
}

const resolved = resolveSpecCurrent();
const specVersion = resolveSpecVersionFromFilename(resolved.specFileName);
if (!specVersion) {
  fail("Could not resolve spec version from spec filename.", [
    `spec_file=${resolved.specFileName}`,
  ]);
}

const content = fs.readFileSync(resolved.specFilePath, "utf8");

const required = [
  { id: "HS-APPX-FEATURE-REGISTRY", schema: "hs_feature_registry@1" },
  { id: "HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX", schema: "hs_primitive_tool_tech_matrix@1" },
  { id: "HS-APPX-UI-GUIDANCE", schema: "hs_ui_guidance@1" },
  { id: "HS-APPX-INTERACTION-MATRIX", schema: "hs_interaction_matrix@1" },
];

for (const r of required) {
  const { schemaFromMarker, parsed } = extractAppendixJson({
    content,
    appendixId: r.id,
  });

  if (schemaFromMarker !== r.schema) {
    fail("EOF appendix schema mismatch (marker).", [
      `id=${r.id}`,
      `expected_schema=${r.schema}`,
      `got_schema=${schemaFromMarker}`,
    ]);
  }

  const schemaFromJson = (parsed?.schema || "").trim();
  if (schemaFromJson !== r.schema) {
    fail("EOF appendix schema mismatch (json).", [
      `id=${r.id}`,
      `expected_schema=${r.schema}`,
      `got_schema=${schemaFromJson || "<missing>"}`,
    ]);
  }

  const specVersionFromJson = (parsed?.spec_version || "").trim();
  if (specVersionFromJson !== specVersion) {
    fail("EOF appendix spec_version mismatch.", [
      `id=${r.id}`,
      `expected_spec_version=${specVersion}`,
      `got_spec_version=${specVersionFromJson || "<missing>"}`,
    ]);
  }
}

console.log(`spec-eof-appendices-check ok: ${resolved.specFileName}`);

