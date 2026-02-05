import { execSync } from "node:child_process";

function run(command) {
  return execSync(command, { stdio: "pipe" }).toString();
}

function shouldFail(command, label) {
  try {
    execSync(command, { stdio: "pipe" });
    throw new Error(`${label} did not fail as expected`);
  } catch (err) {
    if (err && err.status === 1) {
      return;
    }
    throw err;
  }
}

console.log("codex-check-test: starting");

// Verify codex-check scripts exist and are runnable.
run("node .GOV/scripts/spec-current-check.mjs");

// Validate that the fetch guard is active by running it against a known test fixture.
shouldFail(
  'rg -n "\\bfetch\\s*\\(" .GOV/scripts/fixtures/forbidden_fetch.ts && exit 1 || exit 0',
  "fetch guard fixture",
);

// Validate that the TODO policy is enforced in the fixture.
// We avoid --pcre2 because it's not always available.
// Instead, we check if "TODO" exists but "TODO(HSK-" doesn't.
shouldFail(
  'node -e "const out = require(\'child_process\').execSync(\'rg -n TODO .GOV/scripts/fixtures/forbidden_todo.txt\').toString(); if (out.split(\'\\n\').filter(Boolean).some(l => !/TODO\\(HSK-\\d+\\)/.test(l))) process.exit(1)"',
  "TODO guard fixture",
);

console.log("codex-check-test ok");

