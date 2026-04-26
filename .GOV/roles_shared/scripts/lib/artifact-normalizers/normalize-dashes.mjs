import { resultFor, transformOutsideFences } from "./text-utils.mjs";

const DASH_RE = /[\u2010\u2011\u2012\u2013\u2014\u2212]/g;

export function normalizeDashes(input = "") {
  const output = transformOutsideFences(input, (line) => line.replace(DASH_RE, "-"));
  return resultFor("normalizeDashes", input, output, "converted unicode dashes outside fenced code blocks");
}

