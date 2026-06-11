// WP-KERNEL-009 / MT-026 — PortableBuildArtifacts.
//
// Pins where generated editor/index build artifacts live, so moving the
// project or worktree does not break builds (disk-agnostic policy):
//
//  - Rust: .cargo/config.toml routes ALL cargo output to the repo-sibling
//    "../Handshake_Artifacts/handshake-cargo-target" (outside the repo —
//    keeps the git mirror lean and survives repo moves as long as the
//    sibling moves with it; the path is RELATIVE, never absolute).
//  - Frontend app bundle: vite default outDir app/dist (gitignored). This is
//    intentionally INSIDE the repo: Tauri's frontendDist contract points at
//    ../dist relative to src-tauri, so the bundle must stay repo-relative.
//  - Dependency-policy harness: app/dist-harness (gitignored), same
//    repo-relative reasoning; consumed by MT-027/MT-030 checks.
//  - Playwright dependency-lane artifacts: repo-sibling Handshake_Artifacts
//    (HANDSHAKE_ARTIFACT_ROOT override supported for full portability).
//
// Boundary documented here ON PURPOSE: cargo artifacts are repo-EXTERNAL
// (large, machine-local), app bundles are repo-INTERNAL gitignored (packaged
// into the product by Tauri). A check failing here means someone moved an
// artifact root without updating the portability contract.
//
// Repeatable check: `pnpm run check:portable-artifacts` (package.json) runs
// this file; the same boundary logic (shared lib
// app/scripts/lib/portable_artifacts_check.mjs) also runs inside the MT-032
// validator hook `pnpm run check:dependency-policy`. The DYNAMIC counterpart
// — proving a real `pnpm build` writes nothing into app/src, src/, or the
// repo root — runs in app/scripts/check-worker-bundling.mjs (MT-027), which
// snapshots those trees around the actual build.

import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { checkPortableArtifactBoundaries } from "../../../scripts/lib/portable_artifacts_check.mjs";

const appDir = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const repoRoot = join(appDir, "..");

describe("MT-026 portable build artifacts", () => {
  it("passes the full shared boundary check (same logic as the MT-032 hook)", () => {
    const { violations, facts } = checkPortableArtifactBoundaries({ repoRoot });
    expect(violations, JSON.stringify({ violations, facts }, null, 2)).toHaveLength(0);
    // The shared check actually inspected every vite config in app/.
    expect(Object.keys(facts.vite_configs ?? {})).toContain("vite.config.ts");
    expect(Object.keys(facts.vite_configs ?? {})).toContain("vite.harness.config.ts");
  });

  it("routes cargo output to the relative repo-sibling Handshake_Artifacts dir", () => {
    const cargoConfigPath = join(repoRoot, ".cargo", "config.toml");
    expect(existsSync(cargoConfigPath), ".cargo/config.toml missing").toBe(true);
    const cargoConfig = readFileSync(cargoConfigPath, "utf8");
    const match = cargoConfig.match(/target-dir\s*=\s*"([^"]+)"/);
    expect(match, "no target-dir configured").not.toBeNull();
    const targetDir = match![1];
    expect(targetDir).toBe("../Handshake_Artifacts/handshake-cargo-target");
    // Portability contract: relative, no drive letters, no user-profile paths.
    expect(targetDir.startsWith("../")).toBe(true);
    expect(/^[A-Za-z]:/.test(targetDir)).toBe(false);
    expect(targetDir.toLowerCase()).not.toContain("users");
  });

  it("keeps the vite app bundle at the default repo-relative outDir (app/dist)", () => {
    const viteConfig = readFileSync(join(appDir, "vite.config.ts"), "utf8");
    // No outDir override → vite default "dist" (repo-relative, gitignored).
    expect(viteConfig.includes("outDir"), "main vite config must not override outDir").toBe(false);
    const gitignore = readFileSync(join(appDir, ".gitignore"), "utf8");
    expect(gitignore.split(/\r?\n/)).toContain("dist");
  });

  it("aligns Tauri frontendDist with the vite outDir boundary", () => {
    const tauriConf = JSON.parse(
      readFileSync(join(appDir, "src-tauri", "tauri.conf.json"), "utf8"),
    ) as { build?: { frontendDist?: string } };
    expect(tauriConf.build?.frontendDist).toBe("../dist");
  });

  it("keeps the dependency-policy harness output repo-relative and gitignored", () => {
    const harnessConfig = readFileSync(join(appDir, "vite.harness.config.ts"), "utf8");
    const match = harnessConfig.match(/outDir:\s*"([^"]+)"/);
    expect(match).not.toBeNull();
    expect(match![1]).toBe("dist-harness");
    const gitignore = readFileSync(join(appDir, ".gitignore"), "utf8");
    expect(gitignore.split(/\r?\n/)).toContain("dist-harness");
  });

  it("supports HANDSHAKE_ARTIFACT_ROOT override in the dependency playwright lane", () => {
    const config = readFileSync(join(appDir, "playwright.dependency.config.ts"), "utf8");
    expect(config).toContain("HANDSHAKE_ARTIFACT_ROOT");
    expect(config).toContain('"Handshake_Artifacts"');
  });
});
