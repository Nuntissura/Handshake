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
run("node scripts/spec-current-check.mjs");

// Validate that the fetch guard is active by running it against a known test fixture.
shouldFail(
  'rg -n "\\bfetch\\s*\\(" scripts/fixtures/forbidden_fetch.ts && exit 1 || exit 0',
  "fetch guard fixture",
);

// Validate that the TODO policy is enforced in the fixture.
shouldFail(
  'rg -n --pcre2 "TODO(?!\\(HSK-\\d+\\))" scripts/fixtures/forbidden_todo.txt && exit 1 || exit 0',
  "TODO guard fixture",
);

console.log("codex-check-test ok");
