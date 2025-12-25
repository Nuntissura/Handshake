import { execSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

function run(command) {
  return execSync(command, { cwd: repoRoot, encoding: "utf8", stdio: "pipe" }).trim();
}

function safeRipgrep(pattern, target, options = "") {
  try {
    return run(`rg -n ${pattern} ${target} ${options}`.trim());
  } catch (err) {
    // rg exits 1 when no matches are found; treat that as empty output.
    if (err?.status === 1) return "";
    throw err;
  }
}

function fail(message, details = "") {
  console.error(message);
  if (details) {
    console.error(details);
  }
  process.exit(1);
}

// 1) Spec drift guard: SPEC_CURRENT must point to the latest master spec.
run("node scripts/spec-current-check.mjs");

// 2) Frontend fetch guard: only the shared API client may call fetch.
const fetchHits = safeRipgrep('"\\\\bfetch\\\\s*\\("', "app/src", "--iglob *.ts --iglob *.tsx");
if (fetchHits) {
  const violations = fetchHits
    .split("\n")
    .filter(Boolean)
    .filter((line) => !line.includes("app/src/lib/api.ts"));
  if (violations.length > 0) {
    fail("Forbidden fetch() usage outside API client:", violations.join("\n"));
  }
}

// 3) Backend println/eprintln guard: disallow direct stdout logging in production code.
const printlnHits = safeRipgrep('"\\b(?:e?println!)"', "src/backend/handshake_core/src");
if (printlnHits) {
  fail("Forbidden println!/eprintln! in backend source:", printlnHits);
}

// 4) TODO tagging guard: TODOs must be annotated with HSK issue IDs.
const todoHits = safeRipgrep("TODO", "src/backend/handshake_core/src app/src", "--iglob *.rs --iglob *.ts --iglob *.tsx");
if (todoHits) {
  const violations = todoHits
    .split("\n")
    .filter(Boolean)
    .filter((line) => !/TODO\(HSK-\d+\)/.test(line));
  if (violations.length > 0) {
    fail("Untracked TODOs found (require TODO(HSK-####)):", violations.join("\n"));
  }
}

console.log("codex-check ok");
