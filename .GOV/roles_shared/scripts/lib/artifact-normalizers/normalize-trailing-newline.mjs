import { resultFor } from "./text-utils.mjs";

export function normalizeTrailingNewline(input = "") {
  const output = `${String(input || "").replace(/[\r\n]+$/g, "")}\n`;
  return resultFor("normalizeTrailingNewline", input, output, "ensured exactly one trailing newline");
}

