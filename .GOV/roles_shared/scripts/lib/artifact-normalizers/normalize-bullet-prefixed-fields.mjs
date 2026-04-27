import { resultFor, transformOutsideFences } from "./text-utils.mjs";

export function normalizeBulletPrefixedFields(input = "") {
  const output = transformOutsideFences(input, (line) =>
    line.replace(/^(\s*)-\s+([A-Z][A-Z0-9_ ()/-]*\s*:\s*.*)$/u, "$1$2")
  );
  return resultFor("normalizeBulletPrefixedFields", input, output, "removed bullet prefix from bare key-value fields");
}

