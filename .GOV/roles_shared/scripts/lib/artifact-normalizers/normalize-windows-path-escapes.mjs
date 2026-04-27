import { resultFor } from "./text-utils.mjs";

function normalizePathString(value = "") {
  const text = String(value || "");
  if (!/(?:^[A-Za-z]:\\\\|\\\\)/.test(text)) return text;
  return text.replace(/\\\\+/g, "\\");
}

function normalizeValue(value) {
  if (Array.isArray(value)) return value.map((entry) => normalizeValue(entry));
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([key, childValue]) => [key, normalizeValue(childValue)]));
  }
  if (typeof value === "string") return normalizePathString(value);
  return value;
}

export function normalizeWindowsPathEscapes(input = "") {
  const text = String(input || "").trim();
  if (!text) return { output: String(input || ""), applied: false, reason: "" };
  try {
    const parsed = JSON.parse(text);
    const output = JSON.stringify(normalizeValue(parsed));
    return resultFor("normalizeWindowsPathEscapes", text, output, "collapsed doubled Windows path backslashes in JSON strings");
  } catch {
    return { output: String(input || ""), applied: false, reason: "" };
  }
}

