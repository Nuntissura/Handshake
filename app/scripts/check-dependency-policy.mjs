// WP-KERNEL-009 / MT-032 — DependencyPolicyValidatorHook.
//
// Single command for the whole NativeDependencyAndPackaging policy lane:
//
//   pnpm run check:dependency-policy            (full: includes real builds)
//   pnpm run check:dependency-policy -- --skip-build   (reuse existing dists)
//
// Chains, in order, against the runtime dependency allowlist authority
// (app/src/lib/dependency_policy/runtime_dependency_allowlist.json, MT-017):
//
//   1. allowlist-gates          MT-017  schema/shape + operator gating invariants
//   2. cdn-source-tripwire      MT-018  no CDN hosts in product source roots
//   3. docker-default-tripwire  MT-024  no docker-required patterns outside the
//                                       opt-in sandbox adapter exception
//   4. forbidden-manifest-scan  MT-025  no forbidden package declared in npm/
//                                       cargo manifests or the pnpm lockfile
//                                       (sqlite, redis-family, testcontainers, ...)
//   5. sqlite-cargo-inertness   MT-025  every sqlite-class union entry in the
//                                       cargo lockfiles proven INERT via
//                                       `cargo tree --all-features -i <crate>`
//   6. portable-artifacts       MT-026  artifact-boundary contract (cargo
//                                       target-dir, vite outDirs, tauri
//                                       frontendDist, gitignore, artifact root)
//   7. worker-bundling          MT-027  real builds; zero external worker
//                                       loads + zero CDN hosts in built output;
//                                       Monaco worker chunks bundled locally;
//                                       MT-026 dynamic no-stray-writes proof
//   8. third-party-notices-sync MT-029  committed THIRD_PARTY_NOTICES.json
//                                       byte-identical to a fresh regeneration
//
// CARGO-SIDE CHECKS (not duplicated here — run in CI/validation via):
//   cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib dependency_policy::
// covering: MT-017 Rust/JSON allowlist parity, MT-025 direct-declaration scan +
// feature-aware activation closure over cargo metadata (sqlx-sqlite feature
// smuggling), MT-028 statically-linked tree-sitter grammar proofs. Checks 5
// and 8 above DO spawn cargo (`cargo tree` / `cargo metadata`) — graph
// resolution only, no compilation.
//
// Output contract: ONE machine-readable JSON summary on stdout (per-check
// pass/fail + detail), human progress on stderr, exit code 0 only when every
// check passes. Quiet/headless; no foreground windows.

import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  auditPnpmLockSync,
  cargoTreeProvesAbsent,
  loadAllowlist,
  scanCargoLockUnionEntries,
  scanCdnReferences,
  scanDockerDefault,
  scanForbiddenManifestPackages,
} from "./lib/dependency_policy_scans.mjs";
import { checkPortableArtifactBoundaries } from "./lib/portable_artifacts_check.mjs";

const appDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = join(appDir, "..");
const skipBuild = process.argv.includes("--skip-build");

function progress(message) {
  process.stderr.write(`[dependency-policy] ${message}\n`);
}

/** Runs a child node script that prints a single JSON object on stdout. */
function runJsonScript(scriptRelPath, args) {
  const result = spawnSync(process.execPath, [join(appDir, scriptRelPath), ...args], {
    cwd: appDir,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    stdio: ["ignore", "pipe", "inherit"],
  });
  try {
    return { exit: result.status ?? -1, json: JSON.parse(result.stdout) };
  } catch {
    return {
      exit: result.status ?? -1,
      json: { pass: false, error: `non-JSON output from ${scriptRelPath}: ${String(result.stdout).slice(0, 400)}` },
    };
  }
}

function main() {
  const checks = [];
  const allowlist = loadAllowlist(repoRoot); // throws on schema/shape drift

  // 1. MT-017 allowlist gates.
  progress("1/8 allowlist gates (MT-017)");
  {
    const problems = [];
    for (const input of allowlist.allowed_external_runtime_inputs) {
      if (input.operator_gated !== true) problems.push(`${input.kind} not operator_gated`);
      if (input.default_enabled !== false) problems.push(`${input.kind} not default-off`);
    }
    for (const id of ["outside_app", "outside_server_daemon", "docker_default", "sqlite", "cdn_runtime_asset"]) {
      if (!allowlist.forbidden_runtime_dependency_classes.some((c) => c.id === id)) {
        problems.push(`forbidden class ${id} missing`);
      }
    }
    // MT-019 adjunct: manifest/lockfile stay in sync, so every scan below
    // operates on a lockfile that reflects the declared dependencies.
    const { issues } = auditPnpmLockSync({ repoRoot, allowlist });
    for (const issue of issues) problems.push(`lock sync: ${JSON.stringify(issue)}`);
    checks.push({ id: "allowlist-gates", mt: "MT-017", pass: problems.length === 0, problems });
  }

  // 2. MT-018 CDN tripwire over product source.
  progress("2/8 CDN source tripwire (MT-018)");
  {
    const { violations } = scanCdnReferences({ repoRoot, allowlist });
    checks.push({ id: "cdn-source-tripwire", mt: "MT-018", pass: violations.length === 0, violations });
  }

  // 3. MT-024 docker-default tripwire.
  progress("3/8 docker-default tripwire (MT-024)");
  {
    const { violations, exceptionsApplied } = scanDockerDefault({ repoRoot, allowlist });
    checks.push({
      id: "docker-default-tripwire",
      mt: "MT-024",
      pass: violations.length === 0,
      violations,
      exceptions_applied: exceptionsApplied.length,
    });
  }

  // 4. MT-025 forbidden manifest/lockfile packages (all classes).
  progress("4/8 forbidden manifest scan (MT-025)");
  {
    const { violations } = scanForbiddenManifestPackages({ repoRoot, allowlist });
    checks.push({ id: "forbidden-manifest-scan", mt: "MT-025", pass: violations.length === 0, violations });
  }

  // 5. MT-025 sqlite union-entry inertness via cargo tree (graph-only, no build).
  progress("5/8 sqlite cargo inertness (MT-025, spawns cargo tree)");
  {
    const { advisories } = scanCargoLockUnionEntries({ repoRoot, allowlist });
    const sqliteAdvisories = advisories.filter((a) => a.class === "sqlite");
    const proofs = [];
    let pass = true;
    const seen = new Set();
    for (const advisory of sqliteAdvisories) {
      const manifestDir = join(repoRoot, ...advisory.lockfile.split("/").slice(0, -1));
      const key = `${advisory.lockfile}::${advisory.package}`;
      if (seen.has(key)) continue;
      seen.add(key);
      let inert = false;
      let error = null;
      try {
        inert = cargoTreeProvesAbsent({ manifestDir, crateName: advisory.package });
      } catch (err) {
        error = err instanceof Error ? err.message : String(err);
      }
      if (!inert) pass = false;
      proofs.push({ lockfile: advisory.lockfile, package: advisory.package, inert, ...(error ? { error } : {}) });
    }
    checks.push({ id: "sqlite-cargo-inertness", mt: "MT-025", pass, advisories: sqliteAdvisories.length, proofs });
  }

  // 6. MT-026 portable artifact boundary.
  progress("6/8 portable artifacts (MT-026)");
  {
    const { violations, facts } = checkPortableArtifactBoundaries({ repoRoot });
    checks.push({ id: "portable-artifacts", mt: "MT-026", pass: violations.length === 0, violations, facts });
  }

  // 7. MT-027 worker bundling on REAL built output (+ MT-026 dynamic proof).
  progress(`7/8 worker bundling (MT-027${skipBuild ? ", reusing existing dists" : ", running real builds"})`);
  {
    const { exit, json } = runJsonScript(
      "scripts/check-worker-bundling.mjs",
      skipBuild ? ["--skip-build"] : [],
    );
    checks.push({
      id: "worker-bundling",
      mt: "MT-027",
      pass: exit === 0 && json.pass === true,
      built: json.built ?? false,
      stray_writes: json.stray_writes ?? [],
      failures: json.failures ?? (json.error ? [json.error] : []),
      trees: (json.trees ?? []).map((tree) => ({
        dist: tree.dist,
        bundles_monaco: tree.bundles_monaco,
        worker_chunks: tree.worker_chunks?.length ?? 0,
        external_worker_refs: tree.external_worker_refs?.length ?? 0,
        cdn_hits: tree.cdn_hits?.length ?? 0,
        cdn_exceptions_applied: tree.cdn_exceptions_applied?.length ?? 0,
      })),
    });
  }

  // 8. MT-029 third-party notices sync (spawns cargo metadata, graph-only).
  progress("8/8 third-party notices sync (MT-029)");
  {
    const { exit, json } = runJsonScript("scripts/generate-third-party-notices.mjs", ["--check"]);
    checks.push({
      id: "third-party-notices-sync",
      mt: "MT-029",
      pass: exit === 0 && json.pass === true,
      notice_count: json.notice_count ?? null,
      ...(json.error ? { error: json.error } : {}),
    });
  }

  const pass = checks.every((check) => check.pass);
  const summary = {
    schema: "handshake.dependency_policy_check@1",
    wp_id: "WP-KERNEL-009-Project-Knowledge-Index-Loom-Rich-Editor-v1",
    mt_id: "MT-032",
    pass,
    built: !skipBuild,
    cargo_side_tests:
      "cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib dependency_policy:: (MT-017 parity, MT-025 activation closure, MT-028 grammar proofs)",
    checks,
  };
  process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
  process.exitCode = pass ? 0 : 1;
}

try {
  main();
} catch (error) {
  process.stdout.write(
    `${JSON.stringify(
      {
        schema: "handshake.dependency_policy_check@1",
        mt_id: "MT-032",
        pass: false,
        error: error instanceof Error ? error.message : String(error),
      },
      null,
      2,
    )}\n`,
  );
  process.exitCode = 1;
}
