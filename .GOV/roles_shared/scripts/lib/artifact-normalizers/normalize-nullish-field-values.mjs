import { resultFor, transformOutsideFences } from "./text-utils.mjs";

export function normalizeNullishFieldValues(input = "") {
  const output = transformOutsideFences(input, (line) =>
    line.replace(/^(\s*(?:-\s*)?(?:\*\*)?[A-Z][A-Z0-9_/-]*(?:\*\*)?\s*:\s*)(?:None|null|NULL)\s*$/u, "$1")
  );
  return resultFor("normalizeNullishFieldValues", input, output, "converted nullish omitted field values to empty values");
}

