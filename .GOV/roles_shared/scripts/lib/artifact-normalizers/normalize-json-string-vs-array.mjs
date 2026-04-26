import { resultFor } from "./text-utils.mjs";

const ARRAY_FIELD_NAMES = new Set([
  "refs",
  "file_targets",
  "proof_commands",
  "changed_files",
  "commands",
  "checks",
  "details",
  "evidence",
  "findings",
  "output_lines",
  "in_scope_paths",
  "out_of_scope_paths",
]);

function parseJsonArrayString(value) {
  const trimmed = String(value || "").trim();
  if (!trimmed.startsWith("[") || !trimmed.endsWith("]")) return null;
  try {
    const parsed = JSON.parse(trimmed);
    return Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function normalizeValue(value, key = "") {
  if (Array.isArray(value)) {
    return value.map((entry) => normalizeValue(entry));
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([childKey, childValue]) => [
      childKey,
      normalizeValue(childValue, childKey),
    ]));
  }
  if (typeof value === "string" && ARRAY_FIELD_NAMES.has(String(key || "").trim())) {
    const parsed = parseJsonArrayString(value);
    if (parsed) return parsed;
  }
  return value;
}

export function normalizeJsonStringVsArray(input = "") {
  const text = String(input || "").trim();
  if (!text) return { output: String(input || ""), applied: false, reason: "" };
  try {
    const parsed = JSON.parse(text);
    const normalized = normalizeValue(parsed);
    const output = JSON.stringify(normalized);
    return resultFor("normalizeJsonStringVsArray", text, output, "parsed JSON-encoded arrays back to arrays");
  } catch {
    return { output: String(input || ""), applied: false, reason: "" };
  }
}

