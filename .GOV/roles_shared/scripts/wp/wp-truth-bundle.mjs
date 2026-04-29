#!/usr/bin/env node

import {
  buildWpTruthBundle,
  formatWpTruthBundleCompact,
} from "../lib/wp-truth-bundle-lib.mjs";
import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";
import { REPO_ROOT } from "../lib/runtime-paths.mjs";

registerFailCaptureHook("wp-truth-bundle.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("wp-truth-bundle.mjs", message, { role: "SHARED", details });
}

const args = process.argv.slice(2);
const wpId = String(args.find((arg) => !String(arg || "").startsWith("--")) || "").trim();
const jsonMode = args.includes("--json");
const noWrite = args.includes("--no-write");

if (!wpId || !/^WP-/.test(wpId)) {
  fail("Usage: just wp-truth-bundle WP-{ID} [--json] [--no-write]");
}

const result = buildWpTruthBundle({
  repoRoot: REPO_ROOT,
  wpId,
  writeDetail: !noWrite,
});
if (!result.ok) {
  fail(result.error || "Failed to build WP truth bundle", [wpId]);
}

if (jsonMode) {
  console.log(JSON.stringify(result.bundle, null, 2));
} else {
  process.stdout.write(formatWpTruthBundleCompact(result.bundle));
}

