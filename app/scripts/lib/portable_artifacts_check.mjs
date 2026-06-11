// WP-KERNEL-009 / MT-026 — portable build-artifact boundary check.
//
// Single implementation consumed by BOTH:
//   - the vitest app/src/lib/dependency_policy/portable_build_artifacts.test.ts
//     (wired as `pnpm run check:portable-artifacts`), and
//   - the MT-032 validator hook app/scripts/check-dependency-policy.mjs.
//
// THE ARTIFACT BOUNDARY (disk-agnostic portability contract):
//   - Cargo output is repo-EXTERNAL: .cargo/config.toml routes target-dir to
//     the RELATIVE repo-sibling "../Handshake_Artifacts/handshake-cargo-target"
//     (large, machine-local, survives repo moves when the sibling moves too;
//     never an absolute path, drive letter, or user-profile path).
//   - The frontend app bundle is repo-INTERNAL gitignored: vite default outDir
//     app/dist. Intentionally inside the repo because Tauri's frontendDist
//     contract points at ../dist relative to src-tauri and packages it into
//     the product.
//   - The dependency-policy harness bundle: app/dist-harness (gitignored),
//     same repo-relative reasoning; consumed by MT-027/MT-030 proofs.
//   - Playwright dependency-lane results: repo-sibling Handshake_Artifacts,
//     overridable via HANDSHAKE_ARTIFACT_ROOT for full portability.
//   - NO build output may land in app/src, src/, or the repo root. The static
//     config boundary is enforced here; the DYNAMIC proof (filesystem snapshot
//     around a real build) runs in app/scripts/check-worker-bundling.mjs
//     (MT-027), which performs the real `pnpm build`.
//
// A violation here means someone moved an artifact root without updating the
// portability contract — treat it as a portability regression (project-quality
// defect), not a flaky check.

import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const EXPECTED_CARGO_TARGET_DIR = "../Handshake_Artifacts/handshake-cargo-target";

function readText(path) {
  return readFileSync(path, "utf8");
}

/**
 * Runs every static portable-artifact boundary check. Returns
 * `{ violations, facts }`; an empty `violations` array means the boundary
 * holds. `facts` carries the observed values for validator-readable output.
 */
export function checkPortableArtifactBoundaries({ repoRoot }) {
  const violations = [];
  const facts = {};
  const appDir = join(repoRoot, "app");

  // 1. Cargo target-dir: relative repo-sibling, never absolute/user-local.
  const cargoConfigPath = join(repoRoot, ".cargo", "config.toml");
  if (!existsSync(cargoConfigPath)) {
    violations.push({ check: "cargo_target_dir", problem: ".cargo/config.toml missing" });
  } else {
    const match = readText(cargoConfigPath).match(/target-dir\s*=\s*"([^"]+)"/);
    const targetDir = match?.[1] ?? null;
    facts.cargo_target_dir = targetDir;
    if (targetDir !== EXPECTED_CARGO_TARGET_DIR) {
      violations.push({
        check: "cargo_target_dir",
        problem: `target-dir is ${targetDir ?? "<unset>"}; expected ${EXPECTED_CARGO_TARGET_DIR}`,
      });
    }
    if (targetDir && (/^[A-Za-z]:/.test(targetDir) || targetDir.toLowerCase().includes("users"))) {
      violations.push({
        check: "cargo_target_dir",
        problem: `target-dir must be a relative non-user path, got ${targetDir}`,
      });
    }
  }

  // 2. Every vite config in app/: outDir either default (unset → app/dist for
  //    the main config) or a plain app-relative directory. Never "../" (repo
  //    root / siblings), never absolute, never under src/.
  const viteConfigs = readdirSync(appDir).filter((name) => /^vite.*\.config\.ts$/.test(name));
  facts.vite_configs = {};
  if (viteConfigs.length === 0) {
    violations.push({ check: "vite_out_dirs", problem: "no vite configs found under app/" });
  }
  for (const name of viteConfigs) {
    const text = readText(join(appDir, name));
    const outDirMatch = text.match(/outDir:\s*"([^"]+)"/);
    const outDir = outDirMatch?.[1] ?? "(default: dist)";
    facts.vite_configs[name] = outDir;
    if (!outDirMatch) continue;
    const value = outDirMatch[1];
    if (
      value.startsWith("..") ||
      value.startsWith("/") ||
      /^[A-Za-z]:/.test(value) ||
      value === "src" ||
      value.startsWith("src/")
    ) {
      violations.push({
        check: "vite_out_dirs",
        problem: `${name} writes outside the app artifact boundary: outDir "${value}"`,
      });
    }
  }

  // 3. Main vite config must keep the DEFAULT outDir (app/dist) — Tauri's
  //    frontendDist contract depends on it.
  const mainViteConfig = readText(join(appDir, "vite.config.ts"));
  if (mainViteConfig.includes("outDir")) {
    violations.push({
      check: "vite_main_out_dir",
      problem: "app/vite.config.ts overrides outDir; the Tauri frontendDist contract requires the default app/dist",
    });
  }

  // 4. Harness bundle confined to app/dist-harness.
  const harnessConfigPath = join(appDir, "vite.harness.config.ts");
  if (!existsSync(harnessConfigPath)) {
    violations.push({ check: "harness_out_dir", problem: "app/vite.harness.config.ts missing" });
  } else {
    const harnessOut = readText(harnessConfigPath).match(/outDir:\s*"([^"]+)"/)?.[1];
    facts.harness_out_dir = harnessOut ?? null;
    if (harnessOut !== "dist-harness") {
      violations.push({
        check: "harness_out_dir",
        problem: `harness outDir is ${harnessOut ?? "<unset>"}; expected dist-harness`,
      });
    }
  }

  // 5. Tauri packages exactly the vite bundle (../dist relative to src-tauri).
  const tauriConfPath = join(appDir, "src-tauri", "tauri.conf.json");
  const frontendDist = JSON.parse(readText(tauriConfPath))?.build?.frontendDist ?? null;
  facts.tauri_frontend_dist = frontendDist;
  if (frontendDist !== "../dist") {
    violations.push({
      check: "tauri_frontend_dist",
      problem: `tauri.conf.json build.frontendDist is ${frontendDist ?? "<unset>"}; expected ../dist`,
    });
  }

  // 6. Generated bundles stay out of git (the repo mirror must stay lean).
  const gitignoreLines = readText(join(appDir, ".gitignore")).split(/\r?\n/);
  for (const required of ["dist", "dist-harness"]) {
    if (!gitignoreLines.includes(required)) {
      violations.push({
        check: "gitignore",
        problem: `app/.gitignore must ignore "${required}"`,
      });
    }
  }

  // 7. Playwright dependency lane writes results to the repo-sibling artifact
  //    root with an env override for portability.
  const playwrightConfig = readText(join(appDir, "playwright.dependency.config.ts"));
  if (
    !playwrightConfig.includes("HANDSHAKE_ARTIFACT_ROOT") ||
    !playwrightConfig.includes('"Handshake_Artifacts"')
  ) {
    violations.push({
      check: "playwright_artifact_root",
      problem:
        "playwright.dependency.config.ts must resolve outputDir from HANDSHAKE_ARTIFACT_ROOT with the Handshake_Artifacts sibling default",
    });
  }

  return { violations, facts };
}
