import { resultFor, transformOutsideFences } from "./text-utils.mjs";

const SMART_QUOTE_MAP = new Map([
  ["\u201c", "\""],
  ["\u201d", "\""],
  ["\u201e", "\""],
  ["\u201f", "\""],
  ["\u2018", "'"],
  ["\u2019", "'"],
  ["\u201a", "'"],
  ["\u201b", "'"],
]);

function normalizeString(value = "") {
  return Array.from(String(value || "")).map((char) => SMART_QUOTE_MAP.get(char) || char).join("");
}

function normalizeJsonValue(value) {
  if (Array.isArray(value)) return value.map((entry) => normalizeJsonValue(entry));
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([key, childValue]) => [key, normalizeJsonValue(childValue)]));
  }
  if (typeof value === "string") return normalizeString(value);
  return value;
}

export function normalizeSmartQuotes(input = "") {
  const text = String(input || "").trim();
  if (text.startsWith("{") || text.startsWith("[")) {
    try {
      const output = JSON.stringify(normalizeJsonValue(JSON.parse(text)));
      return resultFor("normalizeSmartQuotes", text, output, "converted smart quotes inside JSON string values");
    } catch {
      // Fall back to fence-aware text normalization.
    }
  }
  const output = transformOutsideFences(input, (line) =>
    normalizeString(line)
  );
  return resultFor("normalizeSmartQuotes", input, output, "converted smart quotes outside fenced code blocks");
}
