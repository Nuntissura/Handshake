#!/usr/bin/env node
/**
 * DAL audit: checks DB boundary, SQL portability, trait boundary, migration hygiene, dual-backend hints.
 * Exits non-zero on violations or missing required sections.
 */
import { execSync } from "node:child_process";
import { readdirSync } from "node:fs";

const root = process.cwd();
const backendSrc = "src/backend/handshake_core/src";
const migrationsDir = "src/backend/handshake_core/migrations";

function runRg(pattern, paths, extraArgs = "") {
  const cmd = `rg --hidden --no-heading --line-number "${pattern}" ${paths.join(" ")} ${extraArgs}`;
  try {
    const out = execSync(cmd, { stdio: "pipe", encoding: "utf8" });
    return out.trim();
  } catch (err) {
    if (err.status === 1) return "";
    throw err;
  }
}

let failures = [];

// CX-DBP-VAL-010: No direct DB access outside storage/
{
  const outPool = runRg("state\\.pool", [backendSrc], '--glob "!**/storage/**"');
  const outSqlx = runRg("sqlx::query", [backendSrc], '--glob "!**/storage/**"');
  const hits = [outPool, outSqlx].filter(Boolean).join("\n");
  if (hits) {
    failures.push(`CX-DBP-VAL-010 (DB boundary) violations:\n${hits}`);
  }
}

// CX-DBP-VAL-011: SQL portability (SQLite-only patterns)
{
  const patterns = ["\\?1", "strftime\\(", "CREATE TRIGGER"];
  const hits = patterns
    .map((p) => runRg(p, [backendSrc, migrationsDir]))
    .filter(Boolean)
    .join("\n");
  if (hits) {
    failures.push(`CX-DBP-VAL-011 (SQL portability) violations:\n${hits}`);
  }
}

// CX-DBP-VAL-012: Trait boundary (concrete pool leakage)
{
  const out = runRg("SqlitePool", [backendSrc], '--glob "!**/storage/**"');
  if (out) {
    failures.push(`CX-DBP-VAL-012 (trait boundary) violations:\n${out}`);
  }
}

// CX-DBP-VAL-013: Migration hygiene (basic check: consecutive numbering)
try {
  const allFiles = readdirSync(migrationsDir);

  // Only treat `000X_name.sql` as versioned ups; ignore `*.down.sql` in numbering checks.
  const upFiles = allFiles.filter(
    (f) => /^\d{4}_.+\.sql$/.test(f) && !f.endsWith(".down.sql"),
  );

  const nums = upFiles.map((f) => parseInt(f.slice(0, 4), 10)).sort((a, b) => a - b);
  for (let i = 1; i < nums.length; i++) {
    if (nums[i] !== nums[i - 1] + 1) {
      failures.push(
        `CX-DBP-VAL-013 (migration hygiene): numbering gap between ${nums[i - 1]} and ${nums[i]}`,
      );
      break;
    }
  }

  // Phase 1 requirement (spec v02.106 CX-DBP-022): every up migration must have a matching down file.
  const fileSet = new Set(allFiles);
  const missingDown = upFiles
    .map((up) => up.replace(/\.sql$/, ".down.sql"))
    .filter((down) => !fileSet.has(down));
  if (missingDown.length > 0) {
    failures.push(
      `CX-DBP-VAL-013 (migration hygiene): missing down migrations for:\n${missingDown.join("\n")}`,
    );
  }
} catch (err) {
  failures.push(`CX-DBP-VAL-013 (migration hygiene): failed to read migrations dir: ${err.message}`);
}

// CX-DBP-VAL-014: Dual-backend readiness (presence of postgres/parameterization hints)
{
  const out = runRg("postgres|Postgres|PgPool|PgConnection", [backendSrc, migrationsDir]);
  if (!out) {
    failures.push("CX-DBP-VAL-014 (dual-backend readiness): no PostgreSQL hints/tests found; add or document gap.");
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
