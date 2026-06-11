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
  scanDockerArtifacts,
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

// H2 — docker-orchestration ARTIFACT files (.yml/.yaml/Dockerfile/Containerfile/
// .sh) that the code-source extension filter would let slip past MT-024.
describe("MT-024 H2 docker-artifact walker", () => {
  /** Builds a fake repo with a product scan root, plus an allowlist that points
   * product_scan_roots at it (and re-uses the real docker_artifact_scan config). */
  function fakeRepo() {
    const dir = mkdtempSync(join(tmpdir(), "hsk-docker-artifact-"));
    tempDirs.push(dir);
    const scanRoot = "app/src";
    mkdirSync(join(dir, "app", "src"), { recursive: true });
    const repoAllowlist = {
      ...allowlist,
      product_scan_roots: [scanRoot],
      // Keep the real opt-in exception semantics, but anchor a fixture prefix
      // under the fake scan root so the exception path can be exercised.
      docker_opt_in_exceptions: [
        { path_prefix: "app/src/sandbox/docker/", reason: "opt-in HardIsolation fixture" },
      ],
    };
    return { dir, repoAllowlist };
  }

  it("the real product scan roots contain NO docker-orchestration artifacts today", () => {
    const { violations } = scanDockerArtifacts({ repoRoot, allowlist });
    expect(violations, JSON.stringify(violations, null, 2)).toHaveLength(0);
  });

  it("FAILS on a docker-compose.dev.yml dropped into a product scan root (evasion case)", () => {
    const { dir, repoAllowlist } = fakeRepo();
    writeFileSync(
      join(dir, "app", "src", "docker-compose.dev.yml"),
      "services:\n  db:\n    image: postgres:16\n",
      "utf8",
    );
    const { violations } = scanDockerArtifacts({ repoRoot: dir, allowlist: repoAllowlist });
    expect(violations).toHaveLength(1);
    expect(violations[0].path).toBe("app/src/docker-compose.dev.yml");
  });

  it("FAILS on a bare Dockerfile and a *.dockerfile and a Containerfile", () => {
    const { dir, repoAllowlist } = fakeRepo();
    writeFileSync(join(dir, "app", "src", "Dockerfile"), "FROM postgres:16\n", "utf8");
    writeFileSync(join(dir, "app", "src", "build.dockerfile"), "FROM node:22\n", "utf8");
    writeFileSync(join(dir, "app", "src", "Containerfile"), "FROM alpine\n", "utf8");
    const { violations } = scanDockerArtifacts({ repoRoot: dir, allowlist: repoAllowlist });
    const paths = violations.map((v) => v.path).sort();
    expect(paths).toEqual([
      "app/src/Containerfile",
      "app/src/Dockerfile",
      "app/src/build.dockerfile",
    ]);
  });

  it("FAILS on a .sh that shells out to docker, but not on an unrelated .sh", () => {
    const { dir, repoAllowlist } = fakeRepo();
    writeFileSync(join(dir, "app", "src", "boot.sh"), "#!/bin/sh\ndocker run -d postgres:16\n", "utf8");
    writeFileSync(join(dir, "app", "src", "fmt.sh"), "#!/bin/sh\necho formatting\n", "utf8");
    const { violations } = scanDockerArtifacts({ repoRoot: dir, allowlist: repoAllowlist });
    expect(violations).toHaveLength(1);
    expect(violations[0].path).toBe("app/src/boot.sh");
    expect(violations[0].reason).toContain("docker run");
  });

  it("PASSES (no violation) once the artifact is removed", () => {
    const { dir, repoAllowlist } = fakeRepo();
    // no artifact written
    const { violations } = scanDockerArtifacts({ repoRoot: dir, allowlist: repoAllowlist });
    expect(violations).toHaveLength(0);
  });

  it("allows a docker artifact UNDER the documented opt-in sandbox exception prefix", () => {
    const { dir, repoAllowlist } = fakeRepo();
    mkdirSync(join(dir, "app", "src", "sandbox", "docker"), { recursive: true });
    writeFileSync(
      join(dir, "app", "src", "sandbox", "docker", "docker-compose.hardiso.yml"),
      "services:\n  iso:\n    image: alpine\n",
      "utf8",
    );
    const { violations, exceptionsApplied } = scanDockerArtifacts({
      repoRoot: dir,
      allowlist: repoAllowlist,
    });
    expect(violations).toHaveLength(0);
    expect(exceptionsApplied).toHaveLength(1);
    expect(exceptionsApplied[0].path).toBe("app/src/sandbox/docker/docker-compose.hardiso.yml");
  });
});
