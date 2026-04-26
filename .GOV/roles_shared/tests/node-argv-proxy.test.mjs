import assert from "node:assert/strict";
import test from "node:test";

import {
  buildForwardedArgv,
  splitRawFlags,
} from "../scripts/lib/node-argv-proxy.mjs";

test("splitRawFlags tokenizes variadic just flags into literal argv tokens", () => {
  assert.deepEqual(
    splitRawFlags('--decisions Decision text with parentheses (repro) --wp WP-TEST-v1'),
    ["--decisions", "Decision", "text", "with", "parentheses", "(repro)", "--wp", "WP-TEST-v1"],
  );
});

test("splitRawFlags preserves quoted values as single forwarded argv tokens", () => {
  assert.deepEqual(
    splitRawFlags('--sync-mode FAIL --context "Final lane FAIL current main candidate does not compile" --sync-debug'),
    [
      "--sync-mode",
      "FAIL",
      "--context",
      "Final lane FAIL current main candidate does not compile",
      "--sync-debug",
    ],
  );
  assert.deepEqual(
    splitRawFlags("--flag '' --label 'quoted single token'"),
    ["--flag", "", "--label", "quoted single token"],
  );
});

test("buildForwardedArgv appends tokenized raw flags after base args", () => {
  const result = buildForwardedArgv([
    ".GOV/roles_shared/scripts/memory/repomem.mjs",
    "close",
    "summary text",
    "--raw-flags",
    '--decisions Decision text with parentheses (repro)',
  ]);

  assert.equal(result.targetScript, ".GOV/roles_shared/scripts/memory/repomem.mjs");
  assert.deepEqual(result.forwardedArgs, [
    "close",
    "summary text",
    "--decisions",
    "Decision",
    "text",
    "with",
    "parentheses",
    "(repro)",
  ]);
});

test("buildForwardedArgv preserves quoted closeout context payloads", () => {
  const result = buildForwardedArgv([
    ".GOV/roles_shared/checks/phase-check.mjs",
    "CLOSEOUT",
    "WP-TEST-v1",
    "--raw-flags",
    '--sync-mode FAIL --context "Final lane FAIL current main candidate does not compile and handoff proof is not reproducible"',
  ]);

  assert.equal(result.targetScript, ".GOV/roles_shared/checks/phase-check.mjs");
  assert.deepEqual(result.forwardedArgs, [
    "CLOSEOUT",
    "WP-TEST-v1",
    "--sync-mode",
    "FAIL",
    "--context",
    "Final lane FAIL current main candidate does not compile and handoff proof is not reproducible",
  ]);
});

test("buildForwardedArgv treats a trailing raw-flags marker as empty flags", () => {
  const result = buildForwardedArgv([
    ".GOV/roles_shared/scripts/memory/memory-refresh.mjs",
    "--raw-flags",
  ]);

  assert.equal(result.targetScript, ".GOV/roles_shared/scripts/memory/memory-refresh.mjs");
  assert.deepEqual(result.forwardedArgs, []);
});
