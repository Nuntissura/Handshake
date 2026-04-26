import { resultFor } from "./text-utils.mjs";

export function normalizeLineEndings(input = "") {
  const output = String(input || "").replace(/\r\n?/g, "\n");
  return resultFor("normalizeLineEndings", input, output, "converted CRLF/CR line endings to LF");
}

