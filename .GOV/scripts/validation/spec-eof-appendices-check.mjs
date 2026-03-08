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

function parseVersion(version) {
  const match = String(version || "").match(/^v(\d+)\.(\d+)$/);
  if (!match) return null;
  return { major: Number(match[1]), minor: Number(match[2]) };
}

function isVersionAtLeast(version, minVersion) {
  const parsed = parseVersion(version);
  const minParsed = parseVersion(minVersion);
  if (!parsed || !minParsed) return false;
  if (parsed.major !== minParsed.major) return parsed.major > minParsed.major;
  return parsed.minor >= minParsed.minor;
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

function requireArray(value, fieldName, details = []) {
  if (!Array.isArray(value)) {
    fail("EOF appendix field must be an array.", [fieldName, ...details]);
  }
}

const resolved = resolveSpecCurrent();
const specVersion = resolveSpecVersionFromFilename(resolved.specFileName);
if (!specVersion) {
  fail("Could not resolve spec version from spec filename.", [
    `spec_file=${resolved.specFileName}`,
  ]);
}

const content = fs.readFileSync(resolved.specFilePath, "utf8");

const featureRegistrySchema = isVersionAtLeast(specVersion, "v02.142")
  ? "hs_feature_registry@2"
  : "hs_feature_registry@1";
const primitiveMatrixSchema = isVersionAtLeast(specVersion, "v02.143")
  ? "hs_primitive_tool_tech_matrix@2"
  : "hs_primitive_tool_tech_matrix@1";
const interactionMatrixSchema = isVersionAtLeast(specVersion, "v02.142")
  ? "hs_interaction_matrix@2"
  : "hs_interaction_matrix@1";

const required = [
  { id: "HS-APPX-FEATURE-REGISTRY", schema: featureRegistrySchema },
  { id: "HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX", schema: primitiveMatrixSchema },
  { id: "HS-APPX-UI-GUIDANCE", schema: "hs_ui_guidance@1" },
  { id: "HS-APPX-INTERACTION-MATRIX", schema: interactionMatrixSchema },
];

const parsedById = new Map();
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

  parsedById.set(r.id, parsed);
}

if (primitiveMatrixSchema === "hs_primitive_tool_tech_matrix@2") {
  const featureRegistry = parsedById.get("HS-APPX-FEATURE-REGISTRY");
  const primitiveMatrix = parsedById.get("HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX");
  const taskBoard = fs.readFileSync(".GOV/roles_shared/TASK_BOARD.md", "utf8");

  requireArray(featureRegistry?.features, "feature_registry.features");
  requireArray(primitiveMatrix?.primitives, "primitive_matrix.primitives");
  requireArray(primitiveMatrix?.tools, "primitive_matrix.tools");
  requireArray(primitiveMatrix?.technologies, "primitive_matrix.technologies");
  requireArray(primitiveMatrix?.feature_links, "primitive_matrix.feature_links");

  const featureIds = new Set(featureRegistry.features.map((row) => row.feature_id));
  const primitiveIds = new Set(primitiveMatrix.primitives.map((row) => row.primitive_id));
  const toolIds = new Set(primitiveMatrix.tools.map((row) => row.tool_id));
  const technologyIds = new Set(primitiveMatrix.technologies.map((row) => row.technology_id));
  const validCoverageStatuses = new Set(["SEEDED", "PARTIAL", "GAP"]);
  const seenFeatureLinks = new Set();

  for (const row of primitiveMatrix.feature_links) {
    const featureId = (row?.feature_id || "").trim();
    if (!featureId) {
      fail("Primitive matrix feature link is missing feature_id.");
    }
    if (!featureIds.has(featureId)) {
      fail("Primitive matrix references unknown feature_id.", [`feature_id=${featureId}`]);
    }
    if (seenFeatureLinks.has(featureId)) {
      fail("Primitive matrix has duplicate feature coverage rows.", [`feature_id=${featureId}`]);
    }
    seenFeatureLinks.add(featureId);

    requireArray(row.primitive_ids, `feature_links.${featureId}.primitive_ids`);
    requireArray(row.tool_ids, `feature_links.${featureId}.tool_ids`);
    requireArray(row.technology_ids, `feature_links.${featureId}.technology_ids`);
    requireArray(row.coverage_refs, `feature_links.${featureId}.coverage_refs`);
    requireArray(row.gap_stub_ids, `feature_links.${featureId}.gap_stub_ids`);

    const coverageStatus = (row.coverage_status || "").trim();
    if (!validCoverageStatuses.has(coverageStatus)) {
      fail("Primitive matrix feature link has invalid coverage_status.", [
        `feature_id=${featureId}`,
        `coverage_status=${coverageStatus || "<missing>"}`,
      ]);
    }
    if (row.coverage_refs.length === 0) {
      fail("Primitive matrix feature link must carry at least one coverage_ref.", [
        `feature_id=${featureId}`,
      ]);
    }
    if (coverageStatus !== "SEEDED" && row.gap_stub_ids.length === 0) {
      fail("PARTIAL/GAP feature link must point at at least one stub backlog item.", [
        `feature_id=${featureId}`,
        `coverage_status=${coverageStatus}`,
      ]);
    }

    for (const primitiveId of row.primitive_ids) {
      if (!primitiveIds.has(primitiveId)) {
        fail("Primitive matrix feature link references unknown primitive_id.", [
          `feature_id=${featureId}`,
          `primitive_id=${primitiveId}`,
        ]);
      }
    }
    for (const toolId of row.tool_ids) {
      if (!toolIds.has(toolId)) {
        fail("Primitive matrix feature link references unknown tool_id.", [
          `feature_id=${featureId}`,
          `tool_id=${toolId}`,
        ]);
      }
    }
    for (const technologyId of row.technology_ids) {
      if (!technologyIds.has(technologyId)) {
        fail("Primitive matrix feature link references unknown technology_id.", [
          `feature_id=${featureId}`,
          `technology_id=${technologyId}`,
        ]);
      }
    }
    for (const stubId of row.gap_stub_ids) {
      const stubNeedle = `**[${stubId}]** - [STUB]`;
      if (!taskBoard.includes(stubNeedle)) {
        fail("Primitive matrix feature link points at a stub that is not present in TASK_BOARD Stub Backlog.", [
          `feature_id=${featureId}`,
          `gap_stub_id=${stubId}`,
        ]);
      }
    }
  }

  for (const featureId of featureIds) {
    if (!seenFeatureLinks.has(featureId)) {
      fail("Feature registry entry is missing a primitive coverage row.", [`feature_id=${featureId}`]);
    }
  }
}

console.log(`spec-eof-appendices-check ok: ${resolved.specFileName}`);
