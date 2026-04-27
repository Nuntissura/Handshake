import { resultFor, transformOutsideFences } from "./text-utils.mjs";

const FIELD_LINE_RE = /^(\s*(?:-\s*)?(?:\*\*)?[A-Za-z][A-Za-z0-9_ ()/-]*(?:\*\*)?\s*:\s*.*?)[ \t]+$/u;

export function normalizeFieldValueWhitespace(input = "") {
  const output = transformOutsideFences(input, (line) => line.replace(FIELD_LINE_RE, "$1"));
  return resultFor("normalizeFieldValueWhitespace", input, output, "trimmed trailing whitespace on key-value lines");
}

