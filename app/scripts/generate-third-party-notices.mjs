// WP-KERNEL-009 / MT-029 — LicenseAndNoticeReceipts generator.
//
// Generates app/src-tauri/resources/THIRD_PARTY_NOTICES.json — the
// machine-readable license/notice receipt for every bundled third-party
// library family declared in the runtime dependency allowlist (MT-017):
//
//   npm  (walked from app/pnpm-lock.yaml, INCLUDING transitives):
//     monaco-editor, @tiptap/*, prosemirror-* + its MIT helper packages,
//     yjs family (yjs, lib0, y-protocols, y-prosemirror, isomorphic.js),
//     @xterm/*, @excalidraw/*
//   cargo (from `cargo metadata` feature-resolved graph, no compile):
//     tree-sitter* crates (statically linked grammars, MT-028)
//
// Each notice: { name, version, license, registry, ecosystem, family }.
// License text source: the INSTALLED package.json (pnpm virtual store) for
// npm; cargo metadata's license field for crates. Every license must be in
// the family's allowed_licenses set (license-clean gate, deny.toml-aligned).
//
// The generated file is COMMITTED and ships inside the product bundle
// (tauri.conf.json bundle.resources includes "resources/"). Determinism: no
// timestamps, stable sort by ecosystem/name/version — so the sync check is a
// byte-for-byte comparison.
//
// Usage:
//   node scripts/generate-third-party-notices.mjs            # (re)write file
//   node scripts/generate-third-party-notices.mjs --check    # regenerate +
//       diff against the committed file; exit 1 on drift (MT-032 hook + the
//       vitest third_party_notices.test.ts call this)
//   node scripts/generate-third-party-notices.mjs --stdout   # print JSON
//
// Failure modes (all hard errors, never silently skipped):
//   - a bundled package in the lockfile is not installed (cannot read its
//     real license) → run `pnpm install`;
//   - a license outside the family's allowed set → policy violation;
//   - cargo metadata fails → cargo/toolchain problem.

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { loadAllowlist, pnpmLockPackageNames } from "./lib/dependency_policy_scans.mjs";
import YAML from "yaml";

const appDir = join(fileURLToPath(new URL(".", import.meta.url)), "..");
const repoRoot = join(appDir, "..");
const NOTICES_RELATIVE_PATH = "app/src-tauri/resources/THIRD_PARTY_NOTICES.json";
const noticesPath = join(repoRoot, ...NOTICES_RELATIVE_PATH.split("/"));

const NPM_REGISTRY = "https://registry.npmjs.org";
const CRATES_REGISTRY = "https://crates.io";

function matchesPattern(name, pattern) {
  return pattern.endsWith("*") ? name.startsWith(pattern.slice(0, -1)) : name === pattern;
}

function npmFamilyFor(allowlist, name) {
  const rule = allowlist.bundled_libraries.find(
    (r) => r.ecosystem === "npm" && r.package_patterns.some((p) => matchesPattern(name, p)),
  );
  return rule ?? null;
}

/** All bare `name@version` pairs in the pnpm lockfile packages section. */
function lockfilePackagesWithVersions(lockText) {
  const lock = YAML.parse(lockText);
  const out = [];
  for (const key of Object.keys(lock.packages ?? {})) {
    const versionAt = key.indexOf("@", key.startsWith("@") ? 1 : 0);
    if (versionAt <= 0) continue;
    out.push({ name: key.slice(0, versionAt), version: key.slice(versionAt + 1) });
  }
  return out;
}

let pnpmStoreEntries = null;

/**
 * Resolves the INSTALLED package dir for name@version: direct node_modules
 * path first, then a walk of the pnpm virtual store. The walk matches on the
 * installed package.json's own name+version instead of store directory names,
 * because pnpm v10 TRUNCATES long peer-suffixed store dirs
 * (e.g. ".pnpm/@tiptap+extension-blockquot_<hash>") — name-prefix matching
 * silently misses those.
 */
function installedPackageDir(name, version) {
  const direct = join(appDir, "node_modules", ...name.split("/"));
  if (existsSync(join(direct, "package.json"))) {
    const parsed = JSON.parse(readFileSync(join(direct, "package.json"), "utf8"));
    if (parsed.version === version) return direct;
  }
  const storeDir = join(appDir, "node_modules", ".pnpm");
  pnpmStoreEntries ??= readdirSync(storeDir);
  for (const entry of pnpmStoreEntries) {
    const candidate = join(storeDir, entry, "node_modules", ...name.split("/"));
    const manifestPath = join(candidate, "package.json");
    if (!existsSync(manifestPath)) continue;
    try {
      const parsed = JSON.parse(readFileSync(manifestPath, "utf8"));
      if (parsed.name === name && parsed.version === version) return candidate;
    } catch {
      // Unreadable store entry — keep walking; the final error names the package.
    }
  }
  throw new Error(
    `${name}@${version} appears in the pnpm lockfile but is not installed — run pnpm install before generating notices`,
  );
}

/**
 * License of an installed package: the package.json `license` field, falling
 * back to detecting the SPDX id from the shipped LICENSE file header (some
 * packages — e.g. @excalidraw/mermaid-to-excalidraw — ship a LICENSE file but
 * omit the manifest field). Returns { license, license_evidence }.
 */
function installedLicense(name, version) {
  const dir = installedPackageDir(name, version);
  const manifest = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
  const fromField =
    typeof manifest.license === "string" ? manifest.license : manifest.license?.type;
  if (fromField) return { license: fromField, license_evidence: "package.json" };
  for (const candidate of ["LICENSE", "LICENSE.md", "LICENSE.txt", "license"]) {
    const licensePath = join(dir, candidate);
    if (!existsSync(licensePath)) continue;
    const header = readFileSync(licensePath, "utf8").slice(0, 200);
    if (/\bMIT License\b/i.test(header)) {
      return { license: "MIT", license_evidence: `${candidate} file header` };
    }
  }
  throw new Error(
    `${name}@${version}: no license field in installed package.json and no recognizable LICENSE file`,
  );
}

function npmNotices(allowlist) {
  const lockRel = allowlist.product_manifests.npm_lockfiles[0];
  const lockText = readFileSync(join(repoRoot, ...lockRel.split("/")), "utf8");
  // Sanity: the lockfile parser sees packages at all (guards silent format drift).
  if (pnpmLockPackageNames(lockText).length < 10) {
    throw new Error(`pnpm lockfile ${lockRel} parsed to <10 packages — format drift?`);
  }
  const notices = [];
  for (const { name, version } of lockfilePackagesWithVersions(lockText)) {
    const rule = npmFamilyFor(allowlist, name);
    if (!rule) continue;
    const { license, license_evidence } = installedLicense(name, version);
    if (!rule.allowed_licenses.includes(license)) {
      throw new Error(
        `${name}@${version}: license ${license} outside allowed set [${rule.allowed_licenses.join(", ")}] for family ${rule.family}`,
      );
    }
    notices.push({
      name,
      version,
      license,
      license_evidence,
      registry: NPM_REGISTRY,
      ecosystem: "npm",
      family: rule.family,
    });
  }
  return notices;
}

function cargoNotices(allowlist) {
  const cargoRule = allowlist.bundled_libraries.find((r) => r.ecosystem === "cargo");
  if (!cargoRule) throw new Error("no cargo bundled-library rule in the allowlist");
  const manifestRel = allowlist.product_manifests.cargo[0];
  const manifestPath = join(repoRoot, ...manifestRel.split("/"));
  const raw = execFileSync(
    "cargo",
    ["metadata", "--format-version", "1", "--manifest-path", manifestPath],
    { encoding: "utf8", maxBuffer: 256 * 1024 * 1024, stdio: ["ignore", "pipe", "pipe"] },
  );
  const metadata = JSON.parse(raw);
  const byId = new Map(metadata.packages.map((p) => [p.id, p]));
  const resolvedIds = new Set((metadata.resolve?.nodes ?? []).map((n) => n.id));

  const notices = [];
  for (const id of resolvedIds) {
    const pkg = byId.get(id);
    if (!pkg) continue;
    // tree-sitter family: the declared patterns plus any tree-sitter-* support
    // crate the grammars pull in (e.g. tree-sitter-language) — all part of the
    // statically linked parser stack (MT-028).
    const inFamily =
      cargoRule.package_patterns.some((p) => matchesPattern(pkg.name, p)) ||
      pkg.name.startsWith("tree-sitter");
    if (!inFamily) continue;
    if (!pkg.license) throw new Error(`${pkg.name}@${pkg.version}: no license in cargo metadata`);
    // Cargo SPDX expressions like "MIT OR Apache-2.0" are acceptable when any
    // alternative is in the allowed set (licensee's choice).
    const alternatives = pkg.license.split(/\s+OR\s+/i).map((s) => s.trim());
    if (!alternatives.some((alt) => cargoRule.allowed_licenses.includes(alt))) {
      throw new Error(
        `${pkg.name}@${pkg.version}: license ${pkg.license} outside allowed set [${cargoRule.allowed_licenses.join(", ")}]`,
      );
    }
    if (!pkg.source || !pkg.source.includes("crates.io")) {
      throw new Error(`${pkg.name}@${pkg.version}: not sourced from crates.io (${pkg.source})`);
    }
    notices.push({
      name: pkg.name,
      version: pkg.version,
      license: pkg.license,
      license_evidence: "cargo metadata",
      registry: CRATES_REGISTRY,
      ecosystem: "cargo",
      family: cargoRule.family,
    });
  }
  return notices;
}

export function generateNotices() {
  const allowlist = loadAllowlist(repoRoot);
  const notices = [...npmNotices(allowlist), ...cargoNotices(allowlist)].sort(
    (a, b) =>
      a.ecosystem.localeCompare(b.ecosystem) ||
      a.name.localeCompare(b.name) ||
      a.version.localeCompare(b.version),
  );
  return {
    schema: "handshake.third_party_notices@1",
    wp_id: "WP-KERNEL-009-Project-Knowledge-Index-Loom-Rich-Editor-v1",
    mt_id: "MT-029",
    generated_by: "app/scripts/generate-third-party-notices.mjs",
    description:
      "License/notice receipts for the bundled third-party library families declared in the runtime dependency allowlist (npm families walked from app/pnpm-lock.yaml including transitives; tree-sitter crates from the cargo metadata feature-resolved graph). Regenerate with `pnpm run generate:third-party-notices`; `pnpm run check:third-party-notices` fails on drift.",
    notice_count: notices.length,
    notices,
  };
}

function main() {
  const args = process.argv.slice(2);
  const document = generateNotices();
  const rendered = `${JSON.stringify(document, null, 2)}\n`;

  if (args.includes("--stdout")) {
    process.stdout.write(rendered);
    return;
  }
  if (args.includes("--check")) {
    const committed = existsSync(noticesPath) ? readFileSync(noticesPath, "utf8") : null;
    const pass = committed !== null && committed.replace(/\r\n/g, "\n") === rendered;
    process.stdout.write(
      `${JSON.stringify({
        check: "third-party-notices",
        mt: "MT-029",
        pass,
        notice_count: document.notice_count,
        path: NOTICES_RELATIVE_PATH,
        ...(pass
          ? {}
          : {
              error:
                committed === null
                  ? "THIRD_PARTY_NOTICES.json missing — run pnpm run generate:third-party-notices"
                  : "THIRD_PARTY_NOTICES.json drifted from lockfile/metadata — regenerate and commit",
            }),
      })}\n`,
    );
    process.exitCode = pass ? 0 : 1;
    return;
  }
  writeFileSync(noticesPath, rendered);
  process.stdout.write(
    `${JSON.stringify({ written: NOTICES_RELATIVE_PATH, notice_count: document.notice_count })}\n`,
  );
}

main();
