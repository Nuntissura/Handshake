#!/usr/bin/env node
/**
 * DAL audit: checks the SurrealDB boundary, SurrealQL safety, typed storage
 * isolation, SurrealKit rollout hygiene, and removal of legacy relational
 * runtime backends.
 * Exits non-zero on violations or missing required sections.
 */
import { execSync } from "node:child_process";
import { printValidatorContextMismatchAndExit, requireValidatorProductTargets } from "../scripts/lib/validator-product-targets-lib.mjs";
import { REPO_ROOT } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";

const backendSrc = "src/backend/handshake_core/src";
const migrationsDir = "src/backend/handshake_core/migrations";
let repoRoot = REPO_ROOT;

function runRg(pattern, paths, extraArgs = "") {
  const cmd = `rg --hidden --no-heading --line-number "${pattern}" ${paths.join(" ")} ${extraArgs}`;
  try {
    const out = execSync(cmd, { stdio: "pipe", encoding: "utf8", cwd: repoRoot });
    return out.trim();
  } catch (err) {
    if (err.status === 1) return "";
    throw err;
  }
}

let failures = [];
const targetContext = requireValidatorProductTargets("validator-dal-audit", [backendSrc, migrationsDir], {
  extraDetails: ["This audit inspects product SurrealDB storage code and SurrealKit rollout/schema surfaces only."],
});
repoRoot = targetContext.repoRoot || REPO_ROOT;
const existingTargetSet = new Set(targetContext.existingTargets);
if (!existingTargetSet.has(backendSrc)) {
  printValidatorContextMismatchAndExit("validator-dal-audit", targetContext, [
    `required_backend_source=${backendSrc}`,
  ]);
}
if (!existingTargetSet.has(migrationsDir)) {
  failures.push(`CX-DBP-VAL-013 (SurrealKit rollout hygiene): rollout/schema dir missing: ${migrationsDir}`);
}
const storageTargets = [backendSrc, migrationsDir].filter((target) => existingTargetSet.has(target));

// CX-DBP-VAL-010: No direct DB access outside storage/
{
  const outPool = runRg("state\\.pool", [backendSrc], '--glob "!**/storage/**"');
  const outSurreal = runRg("surrealdb::|Surreal<", [backendSrc], '--glob "!**/storage/**"');
  const hits = [outPool, outSurreal].filter(Boolean).join("\n");
  if (hits) {
    failures.push(`CX-DBP-VAL-010 (DB boundary) violations:\n${hits}`);
  }
}

// CX-DBP-VAL-011: SurrealQL safety and authenticated record-user permissions
{
  const interpolated = runRg("query\\(format!|query\\(&format!", [backendSrc]);
  const permissions = runRg("PERMISSIONS|DEFINE ACCESS|AUTHENTICATE", storageTargets);
  if (interpolated) {
    failures.push(`CX-DBP-VAL-011 (interpolated SurrealQL) violations:\n${interpolated}`);
  }
  if (!permissions) {
    failures.push("CX-DBP-VAL-011 (SurrealQL safety): no authenticated record-user permissions found.");
  }
}

// CX-DBP-VAL-012: Typed storage boundary (concrete SurrealDB client leakage)
{
  const out = runRg("surrealdb::|Surreal<", [backendSrc], '--glob "!**/storage/**"');
  if (out) {
    failures.push(`CX-DBP-VAL-012 (typed storage boundary) violations:\n${out}`);
  }
}

// CX-DBP-VAL-013: SurrealKit rollout hygiene
{
  const rollout = runRg("SurrealKit|app-cutover|rollout", storageTargets);
  const stages = runRg("start|app-cutover|complete|rollback", storageTargets);
  if (!rollout || !stages) {
    failures.push("CX-DBP-VAL-013 (SurrealKit rollout hygiene): rollout implementation or required stages missing.");
  }
}

// CX-DBP-VAL-014: SurrealDB-only authority
{
  const surreal = runRg("surrealdb|SurrealDB|Surreal<", storageTargets);
  const legacy = runRg("postgres|Postgres|PgPool|PgConnection|sqlx|SQLite|SqlitePool|rusqlite", storageTargets);
  if (!surreal) {
    failures.push("CX-DBP-VAL-014 (SurrealDB authority): no SurrealDB implementation/tests found.");
  }
  if (legacy) {
    failures.push(`CX-DBP-VAL-014 (forbidden legacy relational runtime):\n${legacy}`);
  }
}

if (failures.length > 0) {
  console.error("validator-dal-audit: FAIL");
  failures.forEach((f) => {
    console.error("----");
    console.error(f);
  });
  process.exit(1);
}

console.log("validator-dal-audit: PASS (DAL checks clean).");
