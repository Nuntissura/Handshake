// WP-KERNEL-009 / MT-024 — NoDockerDefaultProof.
//
// Tripwire over PRODUCT source (scan roots from the MT-017 allowlist:
// app/src, src/backend/handshake_core/src, app/src-tauri/src) rejecting
// Docker-default patterns: docker-compose requirements, testcontainers,
// `docker run` outside the documented opt-in exceptions.
//
// Opt-in exceptions (allowlist.docker_opt_in_exceptions): ONLY the
// HardIsolation sandbox adapter under
// src/backend/handshake_core/src/sandbox/docker/ — an explicitly selected
// isolation backend. The default sandbox adapter is WSL2 podman
// (sandbox/bootstrap.rs) and DockerAdapter::try_new is failure-tolerant, so
// every core product path works without Docker installed.
//
// Boundary note: repo-root docker-compose.test.yml is repo test
// infrastructure (justfile lanes), not product runtime; product scan roots
// exclude it by construction, and product manifests must not depend on
// testcontainers (asserted below).

import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterAll, describe, expect, it } from "vitest";
import {
  loadAllowlist,
  scanDockerDefault,
  scanFilesForPatterns,
  scanForbiddenManifestPackages,
} from "../../../scripts/lib/dependency_policy_scans.mjs";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "..");
const allowlist = loadAllowlist(repoRoot);

const tempDirs: string[] = [];
afterAll(() => {
  for (const dir of tempDirs) rmSync(dir, { recursive: true, force: true });
});

describe("MT-024 no docker default", () => {
  it("finds no docker-default patterns in product source outside opt-in exceptions", () => {
    const { violations, exceptionsApplied } = scanDockerDefault({ repoRoot, allowlist });
    expect(violations, JSON.stringify(violations, null, 2)).toHaveLength(0);
    // Every applied exception must be covered by a documented opt-in entry.
    const allowedPrefixes = allowlist.docker_opt_in_exceptions.map((e) => e.path_prefix);
    for (const applied of exceptionsApplied) {
      expect(
        allowedPrefixes.some((p) => applied.path.startsWith(p)),
        `exception applied outside documented opt-in list: ${applied.path}`,
      ).toBe(true);
    }
  });

  it("documents every opt-in exception with a reason naming the default-adapter proof", () => {
    expect(allowlist.docker_opt_in_exceptions.length).toBeGreaterThanOrEqual(1);
    for (const exception of allowlist.docker_opt_in_exceptions) {
      expect(exception.path_prefix.startsWith("src/backend/handshake_core/src/sandbox/")).toBe(true);
      expect(exception.reason.length).toBeGreaterThan(40);
      expect(exception.reason.toLowerCase()).toContain("opt-in");
    }
  });

  it("declares no testcontainers/docker tooling in product manifests", () => {
    const { violations } = scanForbiddenManifestPackages({ repoRoot, allowlist });
    const dockerViolations = violations.filter((v) => v.class === "docker_default");
    expect(dockerViolations, JSON.stringify(dockerViolations, null, 2)).toHaveLength(0);
  });

  it("scanner catches docker-default patterns in a negative fixture (tripwire is alive)", () => {
    const dir = mkdtempSync(join(tmpdir(), "hsk-docker-fixture-"));
    tempDirs.push(dir);
    const bad = join(dir, "runtime_boot.rs");
    writeFileSync(
      bad,
      `pub fn boot() { std::process::Command::new("docker").args(["run", "-d", "postgres:16"]).status().unwrap(); } // docker run required\n`,
      "utf8",
    );
    const cls = allowlist.forbidden_runtime_dependency_classes.find(
      (c) => c.id === "docker_default",
    )!;
    const { violations } = scanFilesForPatterns({
      repoRoot: dir,
      files: [bad],
      patterns: cls.source_scan_patterns,
    });
    expect(violations.length).toBeGreaterThanOrEqual(1);
    expect(violations[0].pattern).toBe("docker run");
  });

  it("honors opt-in path prefixes without hiding them (exceptions are reported)", () => {
    const dir = mkdtempSync(join(tmpdir(), "hsk-docker-exception-"));
    tempDirs.push(dir);
    const adapterDir = join(dir, "sandbox", "docker");
    mkdirSync(adapterDir, { recursive: true });
    const adapterFile = join(adapterDir, "adapter.rs");
    writeFileSync(adapterFile, `// spawns "docker run" for the opt-in HardIsolation tier\n`, "utf8");
    const cls = allowlist.forbidden_runtime_dependency_classes.find(
      (c) => c.id === "docker_default",
    )!;
    const { violations, exceptionsApplied } = scanFilesForPatterns({
      repoRoot: dir,
      files: [adapterFile],
      patterns: cls.source_scan_patterns,
      exceptPathPrefixes: ["sandbox/docker/"],
    });
    expect(violations).toHaveLength(0);
    expect(exceptionsApplied).toHaveLength(1);
    expect(exceptionsApplied[0].path).toBe("sandbox/docker/adapter.rs");
    expect(exceptionsApplied[0].patterns).toContain("docker run");
  });
});
