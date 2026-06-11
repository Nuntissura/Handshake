// WP-KERNEL-009 / MT-025 — NoSQLiteTransitiveRegression.
//
// SQLite is forbidden in any form in PRODUCT code (PostgreSQL+EventLedger is
// canonical). Scope: product manifests/lockfiles from the MT-017 allowlist
// (app/, src/backend/, app/src-tauri/). node:sqlite used by REPO GOVERNANCE
// scripts is out of product scope by construction (scan roots/manifests never
// include .GOV or repo scripts).
//
// Three layers, weakest to strongest:
//  1. manifest scan      — no sqlite package may be DECLARED (npm + cargo);
//  2. pnpm lockfile scan — no sqlite package may appear anywhere in the
//     resolved npm graph (hard violation);
//  3. cargo active-graph proof — Cargo.lock legitimately contains INERT
//     feature-union entries (sqlx ships an optional sqlx-sqlite; the union
//     lockfile lists libsqlite3-sys although no feature ever enables it), so
//     raw lock greps are false positives. The authoritative proof is
//     `cargo tree --all-features -e normal,build -i <crate>` showing the
//     crate is NOT in the feature-resolved graph. This test RUNS cargo for
//     every sqlite-class union advisory found in both product lockfiles.

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  cargoLockPackageNames,
  cargoManifestDependencyNames,
  cargoTreeProvesAbsent,
  loadAllowlist,
  npmManifestDependencyNames,
  pnpmLockPackageNames,
  scanCargoLockUnionEntries,
  scanFilesForPatterns,
  scanForbiddenManifestPackages,
  walkSourceFiles,
} from "../../../scripts/lib/dependency_policy_scans.mjs";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "..");
const allowlist = loadAllowlist(repoRoot);

describe("MT-025 no sqlite transitive regression", () => {
  it("declares no sqlite package in any product manifest (npm + cargo + pnpm lockfile)", () => {
    const { violations } = scanForbiddenManifestPackages({ repoRoot, allowlist });
    const sqliteViolations = violations.filter((v) => v.class === "sqlite");
    expect(sqliteViolations, JSON.stringify(sqliteViolations, null, 2)).toHaveLength(0);
  });

  it("proves every sqlite union entry in the cargo lockfiles is INERT via cargo tree", () => {
    const { advisories } = scanCargoLockUnionEntries({ repoRoot, allowlist });
    const sqliteAdvisories = advisories.filter((a) => a.class === "sqlite");
    // The union entries exist today (sqlx optional dep); the proof is that
    // none of them is reachable in the feature-resolved graph.
    expect(sqliteAdvisories.length).toBeGreaterThanOrEqual(1);
    const checked = new Set<string>();
    for (const advisory of sqliteAdvisories) {
      const manifestDir = join(repoRoot, dirname(advisory.lockfile));
      const key = `${manifestDir}::${advisory.package}`;
      if (checked.has(key)) continue;
      checked.add(key);
      expect(
        cargoTreeProvesAbsent({ manifestDir, crateName: advisory.package }),
        `${advisory.package} is ACTIVE in the dependency graph of ${advisory.lockfile} — sqlite regression`,
      ).toBe(true);
    }
    expect(checked.size).toBeGreaterThanOrEqual(2);
  }, 180_000);

  it("scanner catches sqlite packages in negative fixtures (tripwire is alive)", () => {
    const sqliteClass = allowlist.forbidden_runtime_dependency_classes.find(
      (c) => c.id === "sqlite",
    )!;

    // npm manifest fixture — runs through the SAME parser the production
    // manifest scan uses (npmManifestDependencyNames inside
    // scanForbiddenManifestPackages), covering all three dependency sections.
    const npmNames = npmManifestDependencyNames(
      JSON.stringify({
        dependencies: { "better-sqlite3": "^11.0.0" },
        devDependencies: { "sql.js": "^1.10.0" },
        optionalDependencies: { "@sqlite.org/sqlite-wasm": "^3.46.0" },
      }),
    );
    const npmHits = npmNames.filter((n) => sqliteClass.npm_package_names.includes(n));
    expect(npmHits.sort()).toEqual(["@sqlite.org/sqlite-wasm", "better-sqlite3", "sql.js"]);

    // cargo manifest fixture (direct declaration must be caught).
    const cargoNames = cargoManifestDependencyNames(
      ['[dependencies]', 'rusqlite = "0.32"', 'serde = "1"'].join("\n"),
    );
    expect(cargoNames).toContain("rusqlite");
    expect(
      cargoNames.some((n) =>
        sqliteClass.cargo_crate_name_substrings.some((s) => n.includes(s)),
      ),
    ).toBe(true);

    // cargo lock fixture.
    const lockNames = cargoLockPackageNames(
      ['[[package]]', 'name = "libsqlite3-sys"', 'version = "0.30.1"'].join("\n"),
    );
    expect(lockNames).toContain("libsqlite3-sys");

    // pnpm lock fixture.
    const pnpmNames = pnpmLockPackageNames(
      [
        "lockfileVersion: '9.0'",
        "importers:",
        "  .:",
        "    dependencies:",
        "      sql.js:",
        "        specifier: ^1.10.0",
        "        version: 1.10.0",
        "packages:",
        "  sql.js@1.10.0:",
        "    resolution: {integrity: sha512-x}",
      ].join("\n"),
    );
    expect(pnpmNames).toContain("sql.js");
  });

  it("finds no node:sqlite usage in product source scan roots", () => {
    const sqliteClass = allowlist.forbidden_runtime_dependency_classes.find(
      (c) => c.id === "sqlite",
    )!;
    expect(sqliteClass.source_scan_patterns).toContain("node:sqlite");
    const files = allowlist.product_scan_roots.flatMap((root) =>
      walkSourceFiles(join(repoRoot, ...root.split("/"))),
    );
    const { violations } = scanFilesForPatterns({
      repoRoot,
      files,
      patterns: sqliteClass.source_scan_patterns,
      // Policy data/scanner/test files legitimately contain the pattern.
      excludePathSubstrings: ["dependency_policy", "dependency-policy"],
    });
    expect(violations, JSON.stringify(violations, null, 2)).toHaveLength(0);
  });
});
