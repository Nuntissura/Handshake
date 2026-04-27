/*
RGF-244 deterministic artifact-malformation absorber catalog, first cut:

1. line_endings: "a\r\nb\r\n" -> "a\nb\n" so CRLF packets evaluate like LF packets.
2. trailing_newline: "receipt" -> "receipt\n" and "receipt\n\n" -> "receipt\n" so JSONL appends stay bounded.
3. smart_quotes: "SUMMARY: \u201cPASS\u201d" -> "SUMMARY: \"PASS\"" outside fenced code.
4. unicode_dashes: "PASS \u2014 proof" -> "PASS - proof" outside fenced code.
5. json_string_array: {"refs":"[\"a\"]"} -> {"refs":["a"]} for receipt and tool-style array fields.
6. bullet_prefixed_fields: "- GOVERNANCE_VERDICT: PASS" -> "GOVERNANCE_VERDICT: PASS" for bare field specs.
7. heading_prefixed_fields: "#### Verdict: PASS" -> "Verdict: PASS" when the field spec is bare key-value.
8. field_value_whitespace: "RISK_TIER: HIGH   " -> "RISK_TIER: HIGH" outside fenced code.
9. windows_path_escapes: JSON path strings with doubled separators collapse by one level after JSON parse.
10. nullish_field_values: "MERGED_MAIN_COMMIT: None" -> "MERGED_MAIN_COMMIT: " when omission is allowed.
11. heading_level_drift: "### VALIDATION_REPORTS" -> "## VALIDATION_REPORTS" for unambiguous packet top-level headings.
12. rgf197_validator_report_shape: validator-report bullet fields and heading-prefixed fields graduate from inline parser tolerance to named absorbers.
13. superseding_report_noise: repeated malformed earlier reports are normalized before the existing latest-report logic chooses authority.
14. fenced_content_guard: quote and dash absorbers skip fenced code blocks so product snippets are not rewritten.
15. additive_only: absorbers never reject; validator/check logic remains the authority for unresolved malformed content.
*/

import fs from "node:fs";
import path from "node:path";
import { GOVERNANCE_RUNTIME_ROOT_ABS } from "../runtime-paths.mjs";
import { normalizeLineEndings } from "./normalize-line-endings.mjs";
import { normalizeTrailingNewline } from "./normalize-trailing-newline.mjs";
import { normalizeSmartQuotes } from "./normalize-smart-quotes.mjs";
import { normalizeDashes } from "./normalize-dashes.mjs";
import { normalizeJsonStringVsArray } from "./normalize-json-string-vs-array.mjs";
import { normalizeBulletPrefixedFields } from "./normalize-bullet-prefixed-fields.mjs";
import { normalizeHeadingPrefix } from "./normalize-heading-prefix.mjs";
import { normalizeFieldValueWhitespace } from "./normalize-field-value-whitespace.mjs";
import { normalizeWindowsPathEscapes } from "./normalize-windows-path-escapes.mjs";
import { normalizeNullishFieldValues } from "./normalize-nullish-field-values.mjs";
import { normalizeHeadingLevels } from "./normalize-heading-levels.mjs";

export {
  normalizeLineEndings,
  normalizeTrailingNewline,
  normalizeSmartQuotes,
  normalizeDashes,
  normalizeJsonStringVsArray,
  normalizeBulletPrefixedFields,
  normalizeHeadingPrefix,
  normalizeFieldValueWhitespace,
  normalizeWindowsPathEscapes,
  normalizeNullishFieldValues,
  normalizeHeadingLevels,
};

const TEXT_ARTIFACT_KINDS = new Set(["packet", "validator_report", "dossier", "receipt_text"]);
const JSON_ARTIFACT_KINDS = new Set(["receipt", "receipt_args", "json"]);

const ABSORBERS = [
  { name: "normalizeLineEndings", fn: normalizeLineEndings, appliesTo: "all" },
  { name: "normalizeSmartQuotes", fn: normalizeSmartQuotes, appliesTo: "all" },
  { name: "normalizeDashes", fn: normalizeDashes, appliesTo: TEXT_ARTIFACT_KINDS },
  { name: "normalizeJsonStringVsArray", fn: normalizeJsonStringVsArray, appliesTo: JSON_ARTIFACT_KINDS },
  { name: "normalizeBulletPrefixedFields", fn: normalizeBulletPrefixedFields, appliesTo: TEXT_ARTIFACT_KINDS },
  { name: "normalizeHeadingPrefix", fn: normalizeHeadingPrefix, appliesTo: TEXT_ARTIFACT_KINDS },
  { name: "normalizeFieldValueWhitespace", fn: normalizeFieldValueWhitespace, appliesTo: TEXT_ARTIFACT_KINDS },
  { name: "normalizeWindowsPathEscapes", fn: normalizeWindowsPathEscapes, appliesTo: JSON_ARTIFACT_KINDS },
  { name: "normalizeNullishFieldValues", fn: normalizeNullishFieldValues, appliesTo: TEXT_ARTIFACT_KINDS },
  { name: "normalizeHeadingLevels", fn: normalizeHeadingLevels, appliesTo: new Set(["packet", "validator_report"]) },
  { name: "normalizeTrailingNewline", fn: normalizeTrailingNewline, appliesTo: TEXT_ARTIFACT_KINDS },
];

function absorberApplies(entry, artifactKind) {
  if (entry.appliesTo === "all") return true;
  return entry.appliesTo.has(artifactKind);
}

export function absorberHitsLogPath({ runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS } = {}) {
  return path.join(path.resolve(runtimeRootAbs), "absorber_hits.jsonl");
}

export function appendAbsorberHit({
  artifactKind = "",
  wpId = "",
  applied = [],
  timestamp = new Date().toISOString(),
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  if (!Array.isArray(applied) || applied.length === 0) {
    return { appended: false, logPath: absorberHitsLogPath({ runtimeRootAbs }) };
  }
  const logPath = absorberHitsLogPath({ runtimeRootAbs });
  fs.mkdirSync(path.dirname(logPath), { recursive: true });
  const entry = {
    schema_id: "hsk.absorber_hit@1",
    schema_version: "absorber_hit_v1",
    timestamp,
    artifactKind: String(artifactKind || "").trim() || "unknown",
    wp_id: String(wpId || "").trim() || null,
    applied,
  };
  fs.appendFileSync(logPath, `${JSON.stringify(entry)}\n`, "utf8");
  return { appended: true, logPath, entry };
}

export function runAbsorber(input = "", {
  artifactKind = "text",
  wpId = "",
  logHits = true,
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  const normalizedArtifactKind = String(artifactKind || "text").trim() || "text";
  let output = String(input || "");
  const applied = [];

  for (const absorber of ABSORBERS) {
    if (!absorberApplies(absorber, normalizedArtifactKind)) continue;
    const result = absorber.fn(output);
    if (!result || typeof result.output !== "string") continue;
    if (result.applied) {
      output = result.output;
      applied.push({
        name: absorber.name,
        reason: result.reason || absorber.name,
      });
    }
  }

  let hit = { appended: false, logPath: absorberHitsLogPath({ runtimeRootAbs }) };
  if (logHits && applied.length > 0) {
    hit = appendAbsorberHit({
      artifactKind: normalizedArtifactKind,
      wpId,
      applied,
      runtimeRootAbs,
    });
  }

  return {
    output,
    applied,
    hit,
  };
}
