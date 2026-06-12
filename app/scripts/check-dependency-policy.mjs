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
//   3. forbidden-source-tripwire MT-228/229 no forbidden source defaults
//                                       (SQLite adapters, outside apps,
//                                       unmanaged daemons, dev servers)
//   4. docker-default-tripwire  MT-024  no docker-required patterns outside the
//                                       opt-in sandbox adapter exception
//   5. forbidden-manifest-scan  MT-025  no forbidden package declared in npm/
//                                       cargo manifests or the pnpm lockfile
//                                       (sqlite, redis-family, testcontainers, ...)
//   6. sqlite-cargo-inertness   MT-025  every sqlite-class union entry in the
//                                       cargo lockfiles proven INERT via
//                                       `cargo tree --all-features -i <crate>`
//   7. portable-artifacts       MT-026  artifact-boundary contract (cargo
//                                       target-dir, vite outDirs, tauri
//                                       frontendDist, gitignore, artifact root)
//   8. worker-bundling          MT-027  real builds; zero external worker
//                                       loads + zero CDN hosts in built output;
//                                       Monaco worker chunks bundled locally;
//                                       MT-026 dynamic no-stray-writes proof
//   9. third-party-notices-sync MT-029  committed THIRD_PARTY_NOTICES.json
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
import { readFileSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  auditPnpmLockSync,
  cargoTreeProvesAbsent,
  loadAllowlist,
  scanCargoLockUnionEntries,
  scanCdnReferences,
  scanDockerArtifacts,
  scanDockerDefault,
  scanFilesForPatterns,
  scanForbiddenManifestPackages,
  selfExemptPathSet,
  walkSourceFiles,
} from "./lib/dependency_policy_scans.mjs";
import { checkPortableArtifactBoundaries } from "./lib/portable_artifacts_check.mjs";

const appDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = join(appDir, "..");
const skipBuild = process.argv.includes("--skip-build");
const sourceTripwireFiles = collectFlagValues("--source-tripwire-files").map((path) =>
  resolve(path),
);

const REQUIRED_SOURCE_PATTERNS = [
  ["sqlite", "sqlx::Sqlite"],
  ["sqlite", "SqlitePool"],
  ["sqlite", "SqlitePoolOptions"],
  ["sqlite", "SqliteConnectOptions"],
  ["outside_app", "photoshop.exe"],
  ["outside_server_daemon", "ollama serve"],
  ["outside_server_daemon", "localhost:11434"],
  ["outside_server_daemon", "npm run dev"],
  ["outside_server_daemon", "localhost:5173"],
];

function progress(message) {
  process.stderr.write(`[dependency-policy] ${message}\n`);
}

function collectFlagValues(flag) {
  const values = [];
  for (let idx = 0; idx < process.argv.length; idx += 1) {
    if (process.argv[idx] !== flag) continue;
    idx += 1;
    while (idx < process.argv.length && !process.argv[idx].startsWith("--")) {
      values.push(process.argv[idx]);
      idx += 1;
    }
    idx -= 1;
  }
  return values;
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

function sourceTripwireExceptionEntries(allowlist) {
  return allowlist.source_tripwire_exceptions?.entries ?? [];
}

function sourceTripwireExceptionKey(classId, path, pattern) {
  return `${classId}\0${path}\0${pattern.toLowerCase()}`;
}

function sourceTripwireExceptionSet(allowlist) {
  const keys = new Set();
  for (const entry of sourceTripwireExceptionEntries(allowlist)) {
    for (const pattern of entry.patterns ?? []) {
      keys.add(sourceTripwireExceptionKey(entry.class_id, entry.path, pattern));
    }
  }
  return keys;
}

function validateSourceTripwireAuthority(allowlist) {
  const problems = [];
  for (const [classId, pattern] of REQUIRED_SOURCE_PATTERNS) {
    const cls = allowlist.forbidden_runtime_dependency_classes.find((c) => c.id === classId);
    if (!cls) {
      problems.push(`source tripwire class ${classId} missing`);
      continue;
    }
    if (!cls.source_scan_patterns.includes(pattern)) {
      problems.push(`source tripwire pattern ${classId}:${pattern} missing`);
    }
  }

  for (const entry of sourceTripwireExceptionEntries(allowlist)) {
    if (!entry.class_id || !allowlist.forbidden_runtime_dependency_classes.some((c) => c.id === entry.class_id)) {
      problems.push(`source tripwire exception references unknown class: ${JSON.stringify(entry)}`);
    }
    if (!entry.path || entry.path.includes("\\")) {
      problems.push(`source tripwire exception path must be exact repo-relative POSIX form: ${JSON.stringify(entry)}`);
    }
    if (!Array.isArray(entry.patterns) || entry.patterns.length === 0) {
      problems.push(`source tripwire exception must be pattern-scoped: ${JSON.stringify(entry)}`);
    }
    if (!entry.reason || entry.reason.length < 24) {
      problems.push(`source tripwire exception must document its rationale: ${JSON.stringify(entry)}`);
    }
  }
  return problems;
}

function scanForbiddenSourceTripwires({ repoRoot, allowlist, files = null }) {
  const sourceFiles =
    files ??
    allowlist.product_scan_roots.flatMap((root) =>
      walkSourceFiles(join(repoRoot, ...root.split("/"))),
    );
  const exactExemptPaths = selfExemptPathSet(allowlist);
  const exceptionKeys = sourceTripwireExceptionSet(allowlist);
  const violations = [];
  const exceptionsApplied = [];
  const readErrors = [];

  for (const cls of allowlist.forbidden_runtime_dependency_classes) {
    if (!Array.isArray(cls.source_scan_patterns) || cls.source_scan_patterns.length === 0) {
      continue;
    }
    const scan = scanFilesForPatterns({
      repoRoot,
      files: sourceFiles,
      patterns: cls.source_scan_patterns,
      exceptPathPrefixes:
        cls.id === "docker_default"
          ? allowlist.docker_opt_in_exceptions.map((e) => e.path_prefix)
          : [],
      exactExemptPaths,
    });
    readErrors.push(...scan.readErrors);

    for (const violation of scan.violations) {
      const key = sourceTripwireExceptionKey(cls.id, violation.path, violation.pattern);
      if (exceptionKeys.has(key)) {
        exceptionsApplied.push({ class: cls.id, ...violation, exception: "source_tripwire_exceptions" });
      } else {
        violations.push({ class: cls.id, ...violation });
      }
    }
    for (const exception of scan.exceptionsApplied) {
      exceptionsApplied.push({ class: cls.id, ...exception, exception: "path_prefix" });
    }
  }

  return { violations, exceptionsApplied, readErrors };
}

function collectExplicitSourceFileReadErrors(files) {
  const errors = [];
  for (const file of files) {
    try {
      const stat = statSync(file);
      if (!stat.isFile()) {
        errors.push({ path: file, error: "not a regular file" });
        continue;
      }
      readFileSync(file, "utf8");
    } catch (error) {
      errors.push({
        path: file,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }
  return errors;
}

function dedupeFileReadErrors(errors) {
  const seen = new Set();
  const out = [];
  for (const error of errors) {
    const key = `${error.path}\0${error.error}`;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(error);
  }
  return out;
}

function allowlistGateCheck(allowlist) {
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
  problems.push(...validateSourceTripwireAuthority(allowlist));
  // MT-019 adjunct: manifest/lockfile stay in sync, so every scan below
  // operates on a lockfile that reflects the declared dependencies.
  const { issues } = auditPnpmLockSync({ repoRoot, allowlist });
  for (const issue of issues) problems.push(`lock sync: ${JSON.stringify(issue)}`);
  return { id: "allowlist-gates", mt: "MT-017", pass: problems.length === 0, problems };
}

function emitSummary({ checks, built, mode = "full" }) {
  const pass = checks.every((check) => check.pass);
  const summary = {
    schema: "handshake.dependency_policy_check@1",
    wp_id: "WP-KERNEL-009-Project-Knowledge-Index-Loom-Rich-Editor-v1",
    mt_id: "MT-032",
    pass,
    built,
    mode,
    ...(sourceTripwireFiles.length > 0
      ? { source_tripwire_files: sourceTripwireFiles }
      : {}),
    cargo_side_tests:
      "cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib dependency_policy:: (MT-017 parity, MT-025 activation closure, MT-028 grammar proofs)",
    checks,
  };
  process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
  process.exitCode = pass ? 0 : 1;
}

function main() {
  const checks = [];
  const allowlist = loadAllowlist(repoRoot); // throws on schema/shape drift

  // 1. MT-017 allowlist gates.
  progress("1/9 allowlist gates (MT-017)");
  checks.push(allowlistGateCheck(allowlist));

  if (sourceTripwireFiles.length > 0) {
    progress("2/2 forbidden source tripwire file probe (MT-228/229)");
    const fileReadErrors = collectExplicitSourceFileReadErrors(sourceTripwireFiles);
    const unreadableFiles = new Set(fileReadErrors.map((error) => error.path));
    const readableFiles = sourceTripwireFiles.filter((file) => !unreadableFiles.has(file));
    const { violations, exceptionsApplied, readErrors } = scanForbiddenSourceTripwires({
      repoRoot,
      allowlist,
      files: readableFiles,
    });
    const allReadErrors = dedupeFileReadErrors([...fileReadErrors, ...readErrors]);
    checks.push({
      id: "forbidden-source-tripwire",
      mt: "MT-228/MT-229",
      pass: violations.length === 0 && allReadErrors.length === 0,
      files: sourceTripwireFiles.length,
      violations,
      file_read_errors: allReadErrors,
      exceptions_applied: exceptionsApplied.length,
    });
    emitSummary({ checks, built: false, mode: "source-tripwire-files" });
    return;
  }

  // 2. MT-018 CDN tripwire over product source.
  progress("2/9 CDN source tripwire (MT-018)");
  {
    const { violations } = scanCdnReferences({ repoRoot, allowlist });
    checks.push({ id: "cdn-source-tripwire", mt: "MT-018", pass: violations.length === 0, violations });
  }

  // 3. MT-228/229 forbidden source defaults from the allowlist authority.
  progress("3/9 forbidden source tripwire (MT-228/229)");
  {
    const { violations, exceptionsApplied, readErrors } = scanForbiddenSourceTripwires({
      repoRoot,
      allowlist,
    });
    checks.push({
      id: "forbidden-source-tripwire",
      mt: "MT-228/MT-229",
      pass: violations.length === 0 && readErrors.length === 0,
      violations,
      file_read_errors: readErrors,
      exceptions_applied: exceptionsApplied.length,
    });
  }

  // 3. MT-024 docker-default tripwire (code-source patterns + H2 artifact files).
  progress("4/9 docker-default tripwire (MT-024)");
  {
    const { violations, exceptionsApplied } = scanDockerDefault({ repoRoot, allowlist });
    // H2: docker-orchestration ARTIFACT files (docker-compose*.yml, Dockerfile,
    // Containerfile, *.dockerfile, docker-invoking .sh) that the code-source
    // walker's extension filter would miss.
    const artifactScan = scanDockerArtifacts({ repoRoot, allowlist });
    const allViolations = [...violations, ...artifactScan.violations];
    checks.push({
      id: "docker-default-tripwire",
      mt: "MT-024",
      pass: allViolations.length === 0,
      violations: allViolations,
      exceptions_applied: exceptionsApplied.length,
      docker_artifact_scan: {
        violations: artifactScan.violations,
        exceptions_applied: artifactScan.exceptionsApplied.length,
      },
    });
  }

  // 4. MT-025 forbidden manifest/lockfile packages (all classes).
  progress("5/9 forbidden manifest scan (MT-025)");
  {
    const { violations } = scanForbiddenManifestPackages({ repoRoot, allowlist });
    checks.push({ id: "forbidden-manifest-scan", mt: "MT-025", pass: violations.length === 0, violations });
  }

  // 5. MT-025 sqlite union-entry inertness via cargo tree (graph-only, no build).
  progress("6/9 sqlite cargo inertness (MT-025, spawns cargo tree)");
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
  progress("7/9 portable artifacts (MT-026)");
  {
    const { violations, facts } = checkPortableArtifactBoundaries({ repoRoot });
    checks.push({ id: "portable-artifacts", mt: "MT-026", pass: violations.length === 0, violations, facts });
  }

  // 7. MT-027 worker bundling on REAL built output (+ MT-026 dynamic proof).
  progress(`8/9 worker bundling (MT-027${skipBuild ? ", reusing existing dists" : ", running real builds"})`);
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
        // H4 split-host evasion + H1 single-occurrence cap, surfaced per tree.
        split_host_cdn_hits: tree.split_host_cdn_hits?.length ?? 0,
        esm_sh_occurrence_count:
          tree.occurrence_caps?.find((c) => c.pattern === "esm.sh")?.count ?? 0,
        occurrence_violations: tree.occurrence_violations?.length ?? 0,
      })),
    });
  }

  // 8. MT-029 third-party notices sync (spawns cargo metadata, graph-only).
  progress("9/9 third-party notices sync (MT-029)");
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

  emitSummary({ checks, built: !skipBuild });
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
