// WP-KERNEL-009 / MT-019 — PackageLockAudit.
//
// Lockfile, version, license, and reproducibility audit for the dependencies
// WP-009 touches:
//  - pnpm lockfile exists and is in sync with app/package.json (every declared
//    dependency locked with the same specifier, and nothing locked that is not
//    declared) — the read-level counterpart of `pnpm install --frozen-lockfile`,
//  - both product Cargo.toml manifests have their direct dependencies pinned in
//    their Cargo.lock files (including tree-sitter grammar crates),
//  - a license inventory over the installed editor stack: every editor-stack
//    package carries a license allowed by its bundled-library rule (MT-017).
//
// Command-level reproducibility proof (run, not mocked):
//   cd app && pnpm install --frozen-lockfile   → exit 0
//   cargo metadata --format-version 1          → resolves (exercised by the
//     MT-029 notices generator and the MT-032 validator hook).

import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  auditEditorStackLicenses,
  auditPnpmLockSync,
  cargoLockPackageNames,
  cargoManifestDependencyNames,
  loadAllowlist,
} from "../../../scripts/lib/dependency_policy_scans.mjs";

const appDir = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const repoRoot = join(appDir, "..");
const allowlist = loadAllowlist(repoRoot);

describe("MT-019 package lock audit", () => {
  it("has every product lockfile present", () => {
    for (const rel of [
      ...allowlist.product_manifests.npm_lockfiles,
      ...allowlist.product_manifests.cargo_lockfiles,
    ]) {
      expect(existsSync(join(repoRoot, rel)), `${rel} missing`).toBe(true);
    }
  });

  it("keeps app/package.json and pnpm-lock.yaml in sync (frozen-lockfile equivalent)", () => {
    const { issues } = auditPnpmLockSync({ repoRoot, allowlist });
    expect(issues, JSON.stringify(issues, null, 2)).toHaveLength(0);
  });

  it("pins every direct cargo dependency of both product manifests in Cargo.lock", () => {
    const pairs: Array<[string, string]> = [
      [allowlist.product_manifests.cargo[0], allowlist.product_manifests.cargo_lockfiles[0]],
      [allowlist.product_manifests.cargo[1], allowlist.product_manifests.cargo_lockfiles[1]],
    ];
    for (const [manifestRel, lockRel] of pairs) {
      const manifest = readFileSync(join(repoRoot, manifestRel), "utf8");
      const lockNames = new Set(cargoLockPackageNames(readFileSync(join(repoRoot, lockRel), "utf8")));
      const directDeps = cargoManifestDependencyNames(manifest).filter(
        // Path-only workspace-internal deps (e.g. handshake_core) are still in
        // the lock; keep the filter total: everything must be locked.
        (name) => name.length > 0,
      );
      expect(directDeps.length).toBeGreaterThan(0);
      const missing = directDeps.filter((name) => !lockNames.has(name));
      expect(missing, `${manifestRel}: deps missing from ${lockRel}: ${missing.join(", ")}`).toHaveLength(0);
    }
  });

  it("pins the tree-sitter grammar crates in the backend lockfile", () => {
    const lockNames = new Set(
      cargoLockPackageNames(
        readFileSync(join(repoRoot, "src/backend/handshake_core/Cargo.lock"), "utf8"),
      ),
    );
    const rule = allowlist.bundled_libraries.find((r) => r.family === "tree-sitter");
    expect(rule).toBeDefined();
    for (const crate of rule!.package_patterns) {
      expect(lockNames.has(crate), `${crate} not pinned in Cargo.lock`).toBe(true);
    }
  });

  it("records an allowed license for every installed editor-stack package", () => {
    const { violations, inventory } = auditEditorStackLicenses({ repoRoot, allowlist, appDir });
    expect(violations, JSON.stringify(violations, null, 2)).toHaveLength(0);
    expect(inventory.length).toBeGreaterThanOrEqual(5);
    for (const entry of inventory) {
      expect(entry.license.length).toBeGreaterThan(0);
    }
    // The inventory is the validator-readable license evidence for MT-019.
    const families = new Set(inventory.map((e) => e.family));
    expect(families.has("tiptap")).toBe(true);
    expect(families.has("yjs")).toBe(true);
  });
});
