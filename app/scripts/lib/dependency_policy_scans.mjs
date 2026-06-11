// WP-KERNEL-009 / NativeDependencyAndPackaging — shared dependency-policy scanners.
//
// Single implementation of the tripwire scans consumed by BOTH:
//   - vitest tests under app/src/lib/dependency_policy/*.test.ts
//     (MT-018 bundled-library policy, MT-024 docker tripwire, MT-025 sqlite
//     tripwire, MT-019 lock audit), and
//   - the MT-032 validator hook app/scripts/check-dependency-policy.mjs.
//
// Data authority: app/src/lib/dependency_policy/runtime_dependency_allowlist.json
// (MT-017). Every scan derives its patterns from that document; nothing is
// hardcoded here except structural file-walking.
//
// IMPORTANT Cargo.lock nuance (MT-025): Cargo.lock records the feature-union
// of possible dependencies. sqlx ships an optional `sqlx-sqlite` behind its
// `sqlite` feature, so `libsqlite3-sys` appears in Cargo.lock even though the
// product never enables it. Raw lockfile greps therefore give FALSE positives.
// The authoritative active-graph proof is `cargo tree -e normal,build -i <crate>`,
// which resolves the real feature set. scanCargoLockUnionEntries() reports
// union entries as advisories that REQUIRE the cargo-tree proof, and
// cargoTreeProvesAbsent() runs that proof.

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import YAML from "yaml";

export const ALLOWLIST_RELATIVE_PATH =
  "app/src/lib/dependency_policy/runtime_dependency_allowlist.json";

/** Loads and minimally validates the allowlist document from the repo root. */
export function loadAllowlist(repoRoot) {
  const path = join(repoRoot, ...ALLOWLIST_RELATIVE_PATH.split("/"));
  const doc = JSON.parse(readFileSync(path, "utf8"));
  if (doc.schema !== "handshake.runtime_dependency_allowlist@1") {
    throw new Error(`allowlist schema mismatch at ${path}: ${doc.schema}`);
  }
  for (const key of [
    "allowed_external_runtime_inputs",
    "forbidden_runtime_dependency_classes",
    "bundled_libraries",
    "docker_opt_in_exceptions",
    "product_scan_roots",
    "product_manifests",
  ]) {
    if (!(key in doc)) throw new Error(`allowlist missing ${key}`);
  }
  return doc;
}

function forbiddenClass(allowlist, id) {
  const cls = allowlist.forbidden_runtime_dependency_classes.find((c) => c.id === id);
  if (!cls) throw new Error(`forbidden class ${id} missing from allowlist`);
  return cls;
}

const SOURCE_EXTENSIONS = new Set([
  ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".rs", ".css", ".html", ".json", ".toml",
]);

/** Recursively lists product source files under rootDir (skips node_modules/dist/target). */
export function walkSourceFiles(rootDir) {
  if (!existsSync(rootDir)) return [];
  const out = [];
  const skipDirs = new Set(["node_modules", "dist", "dist-harness", "target", ".git"]);
  const stack = [rootDir];
  while (stack.length > 0) {
    const dir = stack.pop();
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        if (!skipDirs.has(entry.name)) stack.push(full);
        continue;
      }
      const dot = entry.name.lastIndexOf(".");
      const ext = dot >= 0 ? entry.name.slice(dot).toLowerCase() : "";
      if (SOURCE_EXTENSIONS.has(ext)) out.push(full);
    }
  }
  return out;
}

function toRepoRel(repoRoot, fullPath) {
  return relative(repoRoot, fullPath).split(sep).join("/");
}

/**
 * Core text scan: returns violations for any file whose content contains one
 * of `patterns` (case-insensitive), excluding files whose repo-relative path
 * starts with one of `exceptPathPrefixes` or contains one of
 * `excludePathSubstrings`.
 */
export function scanFilesForPatterns({
  repoRoot,
  files,
  patterns,
  exceptPathPrefixes = [],
  excludePathSubstrings = [],
}) {
  const violations = [];
  const exceptionsApplied = [];
  const lowered = patterns.map((p) => p.toLowerCase());
  for (const file of files) {
    const rel = toRepoRel(repoRoot, file);
    if (excludePathSubstrings.some((s) => rel.includes(s))) continue;
    const exception = exceptPathPrefixes.find((p) => rel.startsWith(p));
    let content;
    try {
      content = readFileSync(file, "utf8").toLowerCase();
    } catch {
      continue;
    }
    const hits = lowered.filter((p) => content.includes(p));
    if (hits.length === 0) continue;
    if (exception) {
      exceptionsApplied.push({ path: rel, exception, patterns: hits });
      continue;
    }
    const lines = content.split(/\r?\n/);
    for (const pattern of hits) {
      const lineIndex = lines.findIndex((l) => l.includes(pattern));
      violations.push({
        path: rel,
        line: lineIndex + 1,
        pattern,
        snippet: lines[lineIndex]?.trim().slice(0, 160) ?? "",
      });
    }
  }
  return { violations, exceptionsApplied };
}

/**
 * MT-024 — Docker-default tripwire. Scans product source roots for
 * docker-required patterns; only paths under docker_opt_in_exceptions may
 * contain them.
 */
export function scanDockerDefault({ repoRoot, allowlist }) {
  const cls = forbiddenClass(allowlist, "docker_default");
  const files = allowlist.product_scan_roots.flatMap((root) =>
    walkSourceFiles(join(repoRoot, ...root.split("/"))),
  );
  return scanFilesForPatterns({
    repoRoot,
    files,
    patterns: cls.source_scan_patterns,
    exceptPathPrefixes: allowlist.docker_opt_in_exceptions.map((e) => e.path_prefix),
    // The policy data/scanner/test files legitimately contain the patterns.
    excludePathSubstrings: ["dependency_policy", "dependency-policy"],
  });
}

/**
 * MT-018 — CDN reference tripwire over product frontend source. All editor/UI
 * assets must be bundled; no runtime CDN hosts may appear in source.
 */
export function scanCdnReferences({ repoRoot, allowlist }) {
  const cls = forbiddenClass(allowlist, "cdn_runtime_asset");
  const files = allowlist.product_scan_roots.flatMap((root) =>
    walkSourceFiles(join(repoRoot, ...root.split("/"))),
  );
  return scanFilesForPatterns({
    repoRoot,
    files,
    patterns: cls.source_scan_patterns,
    excludePathSubstrings: ["dependency_policy", "dependency-policy"],
  });
}

/**
 * Direct dependency names declared in a package.json text (dependencies,
 * devDependencies, optionalDependencies). Pure text parser so negative
 * fixtures exercise the exact production code path.
 */
export function npmManifestDependencyNames(packageJsonText) {
  const pkg = JSON.parse(packageJsonText);
  return [
    ...Object.keys(pkg.dependencies ?? {}),
    ...Object.keys(pkg.devDependencies ?? {}),
    ...Object.keys(pkg.optionalDependencies ?? {}),
  ];
}

function parseManifestDeps(packageJsonPath) {
  const text = readFileSync(packageJsonPath, "utf8");
  return { pkg: JSON.parse(text), names: npmManifestDependencyNames(text) };
}

/** Extracts `[dependencies]`-style section dependency names from a Cargo.toml. */
export function cargoManifestDependencyNames(cargoTomlText) {
  const names = [];
  let inDepSection = false;
  for (const rawLine of cargoTomlText.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (line.startsWith("[")) {
      inDepSection = /^\[(target\.[^\]]*\.)?(dependencies|dev-dependencies|build-dependencies)\]/.test(
        line,
      );
      continue;
    }
    if (!inDepSection || line.length === 0 || line.startsWith("#")) continue;
    const eq = line.indexOf("=");
    if (eq <= 0) continue;
    let name = line.slice(0, eq).trim().replace(/^"|"$/g, "");
    // `alias = { package = "real-name", ... }` — the real crate is `package`.
    const pkgOverride = line.match(/package\s*=\s*"([^"]+)"/);
    if (pkgOverride) name = pkgOverride[1];
    if (/^[A-Za-z0-9_@./-]+$/.test(name)) names.push(name);
  }
  return names;
}

/** Package names appearing in a Cargo.lock (`name = "..."` entries). */
export function cargoLockPackageNames(cargoLockText) {
  const names = new Set();
  for (const match of cargoLockText.matchAll(/^name = "([^"]+)"$/gm)) {
    names.add(match[1]);
  }
  return [...names];
}

/** Package names appearing in a pnpm lockfile (importers + packages keys). */
export function pnpmLockPackageNames(pnpmLockText) {
  const lock = YAML.parse(pnpmLockText);
  const names = new Set();
  for (const importer of Object.values(lock.importers ?? {})) {
    for (const section of ["dependencies", "devDependencies", "optionalDependencies"]) {
      for (const name of Object.keys(importer?.[section] ?? {})) names.add(name);
    }
  }
  for (const key of Object.keys(lock.packages ?? {})) {
    // Keys look like "name@1.2.3" or "@scope/name@1.2.3(peer@x)".
    const versionAt = key.indexOf("@", key.startsWith("@") ? 1 : 0);
    const name = versionAt > 0 ? key.slice(0, versionAt) : key;
    if (name) names.add(name);
  }
  return [...names];
}

/**
 * Scans product manifests (npm + cargo, from allowlist.product_manifests) for
 * forbidden package names across every forbidden class. Hard violations:
 * a forbidden package declared as a direct dependency, or a forbidden npm
 * package anywhere in the pnpm lockfile.
 */
export function scanForbiddenManifestPackages({ repoRoot, allowlist }) {
  const violations = [];
  const classes = allowlist.forbidden_runtime_dependency_classes;

  for (const manifestRel of allowlist.product_manifests.npm) {
    const manifestPath = join(repoRoot, ...manifestRel.split("/"));
    const { names } = parseManifestDeps(manifestPath);
    for (const cls of classes) {
      for (const name of names) {
        if (cls.npm_package_names.includes(name)) {
          violations.push({ class: cls.id, ecosystem: "npm", manifest: manifestRel, package: name });
        }
      }
    }
  }

  for (const lockRel of allowlist.product_manifests.npm_lockfiles) {
    const lockPath = join(repoRoot, ...lockRel.split("/"));
    const names = pnpmLockPackageNames(readFileSync(lockPath, "utf8"));
    for (const cls of classes) {
      for (const name of names) {
        if (
          cls.npm_package_names.includes(name) ||
          (cls.id === "sqlite" && name.toLowerCase().includes("sqlite"))
        ) {
          violations.push({ class: cls.id, ecosystem: "npm", manifest: lockRel, package: name });
        }
      }
    }
  }

  for (const manifestRel of allowlist.product_manifests.cargo) {
    const manifestPath = join(repoRoot, ...manifestRel.split("/"));
    const names = cargoManifestDependencyNames(readFileSync(manifestPath, "utf8"));
    for (const cls of classes) {
      for (const name of names) {
        if (cls.cargo_crate_name_substrings.some((s) => name.toLowerCase().includes(s))) {
          violations.push({ class: cls.id, ecosystem: "cargo", manifest: manifestRel, package: name });
        }
      }
    }
  }

  return { violations };
}

/**
 * MT-025 — Cargo.lock union-entry advisories. Returns crates in the lockfiles
 * matching forbidden cargo substrings. These are NOT hard violations (see file
 * header); each advisory must be proven inert via cargoTreeProvesAbsent().
 */
export function scanCargoLockUnionEntries({ repoRoot, allowlist }) {
  const advisories = [];
  const classes = allowlist.forbidden_runtime_dependency_classes;
  for (const lockRel of allowlist.product_manifests.cargo_lockfiles) {
    const lockPath = join(repoRoot, ...lockRel.split("/"));
    const names = cargoLockPackageNames(readFileSync(lockPath, "utf8"));
    for (const cls of classes) {
      for (const name of names) {
        if (cls.cargo_crate_name_substrings.some((s) => name.toLowerCase().includes(s))) {
          advisories.push({ class: cls.id, lockfile: lockRel, package: name });
        }
      }
    }
  }
  return { advisories };
}

/**
 * Authoritative active-graph proof: returns true when `cargo tree -i crate`
 * (with --all-features) proves the crate is NOT in the feature-resolved
 * dependency graph of the manifest at manifestDir.
 */
export function cargoTreeProvesAbsent({ manifestDir, crateName }) {
  let stdout;
  try {
    stdout = execFileSync(
      "cargo",
      ["tree", "--all-features", "-e", "normal,build", "-i", crateName],
      { cwd: manifestDir, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
    );
  } catch (error) {
    const text = `${error.stdout ?? ""}${error.stderr ?? ""}`;
    // cargo errors with "package ID specification ... did not match any packages"
    // when the crate is not even in Cargo.lock — that also proves absence.
    if (text.includes("did not match any packages")) return true;
    throw new Error(`cargo tree -i ${crateName} failed in ${manifestDir}: ${text.slice(0, 400)}`);
  }
  const meaningful = stdout
    .split(/\r?\n/)
    .filter((l) => l.trim().length > 0 && !l.startsWith("warning:"));
  return meaningful.length === 0;
}

/**
 * MT-027 — finds worker-load sites in built JS whose URL argument is a
 * LITERAL http(s) origin: `new Worker("https://...")`,
 * `new Worker(new URL("https://...", ...))`, `importScripts("https://...")`.
 * Bundled-local loads (relative URLs resolved against import.meta.url /
 * self.location) are the only allowed form. Doc-comment URLs near a worker
 * call (e.g. the MDN links TypeScript ships next to its importScripts
 * mentions) are NOT loads and are not flagged; dynamic non-literal loads
 * cannot be resolved statically and are covered by the MT-030 runtime proof
 * (network cut, zero external requests).
 */
export function externalWorkerLoads(content, path) {
  const violations = [];
  const sitePattern =
    /(?:new\s+(?:Shared)?Worker|importScripts)\s*\(\s*(?:new\s+URL\s*\(\s*)?["'`](https?:\/\/[^"'`]+)/g;
  for (const match of content.matchAll(sitePattern)) {
    violations.push({
      path,
      site: match[0].slice(0, 80),
      url: match[1].slice(0, 160),
    });
  }
  return violations;
}

/**
 * MT-027 — applies the allowlist's built_output_scan_exceptions to occurrences
 * of a forbidden CDN pattern in built-output text: a hit is exempt only when
 * the exception's required_context_marker appears within max_marker_distance
 * characters (self-verifying scope — the exception cannot widen to unrelated
 * occurrences of the same pattern). Returns repo-relative paths via `relPath`.
 */
export function partitionCdnHits({ content, relPath, pattern, allowlist }) {
  const exceptions = allowlist.built_output_scan_exceptions ?? [];
  const violations = [];
  const exempted = [];
  let index = 0;
  const lowered = content.toLowerCase();
  while ((index = lowered.indexOf(pattern, index)) !== -1) {
    const applicable = exceptions.find((exc) => {
      if (exc.pattern !== pattern) return false;
      const from = Math.max(0, index - exc.max_marker_distance);
      const to = Math.min(content.length, index + exc.max_marker_distance);
      return content.slice(from, to).includes(exc.required_context_marker);
    });
    if (applicable) {
      exempted.push({
        path: relPath,
        pattern,
        dependency: applicable.dependency,
        marker: applicable.required_context_marker,
      });
    } else {
      violations.push({ path: relPath, pattern, offset: index });
    }
    index += pattern.length;
  }
  return { violations, exempted };
}

/**
 * MT-019 — pnpm lockfile/manifest sync audit. Every declared dependency must
 * have a matching importer entry with the same specifier, and vice versa.
 */
export function auditPnpmLockSync({ repoRoot, allowlist }) {
  const issues = [];
  const manifestRel = allowlist.product_manifests.npm[0];
  const lockRel = allowlist.product_manifests.npm_lockfiles[0];
  const { pkg } = parseManifestDeps(join(repoRoot, ...manifestRel.split("/")));
  const lock = YAML.parse(readFileSync(join(repoRoot, ...lockRel.split("/")), "utf8"));
  const importer = lock.importers?.["."];
  if (!importer) return { issues: [{ kind: "missing_importer", detail: lockRel }] };

  for (const section of ["dependencies", "devDependencies"]) {
    const declared = pkg[section] ?? {};
    const locked = importer[section] ?? {};
    for (const [name, specifier] of Object.entries(declared)) {
      if (!(name in locked)) {
        issues.push({ kind: "missing_in_lock", section, package: name });
      } else if (locked[name].specifier !== specifier) {
        issues.push({
          kind: "specifier_mismatch",
          section,
          package: name,
          manifest: specifier,
          lock: locked[name].specifier,
        });
      }
    }
    for (const name of Object.keys(locked)) {
      if (!(name in declared)) {
        issues.push({ kind: "missing_in_manifest", section, package: name });
      }
    }
  }
  return { issues };
}

/**
 * MT-018 — editor-stack resolution audit. Every editor-stack dependency
 * declared in app/package.json must resolve from the npm registry (integrity
 * present; no link:/file:/git resolutions; tarball host, when present, must be
 * registry.npmjs.org).
 */
export function auditEditorStackResolution({ repoRoot, allowlist }) {
  const violations = [];
  const audited = [];
  const manifestRel = allowlist.product_manifests.npm[0];
  const lockRel = allowlist.product_manifests.npm_lockfiles[0];
  const { pkg } = parseManifestDeps(join(repoRoot, ...manifestRel.split("/")));
  const lock = YAML.parse(readFileSync(join(repoRoot, ...lockRel.split("/")), "utf8"));
  const importer = lock.importers?.["."] ?? {};

  const npmRules = allowlist.bundled_libraries.filter((r) => r.ecosystem === "npm");
  const matchesRule = (name) =>
    npmRules.some((rule) =>
      rule.package_patterns.some((p) =>
        p.endsWith("*") ? name.startsWith(p.slice(0, -1)) : name === p,
      ),
    );

  const declared = { ...(pkg.dependencies ?? {}), ...(pkg.devDependencies ?? {}) };
  for (const name of Object.keys(declared)) {
    if (!matchesRule(name)) continue;
    const entry = importer.dependencies?.[name] ?? importer.devDependencies?.[name];
    if (!entry) {
      violations.push({ package: name, problem: "missing importer entry in lockfile" });
      continue;
    }
    const version = entry.version;
    if (/^(link:|file:|git\+|https?:)/.test(version)) {
      violations.push({ package: name, problem: `non-registry resolution: ${version}` });
      continue;
    }
    // pnpm lockfileVersion 9: importer versions carry peer-dependency suffixes
    // ("3.13.0(@tiptap/pm@3.13.0)") but the `packages` section is keyed by the
    // bare "name@version"; peer-suffixed keys live under `snapshots`.
    const bareVersion = version.includes("(") ? version.slice(0, version.indexOf("(")) : version;
    const packagesKey = `${name}@${bareVersion}`;
    const lockPkg = lock.packages?.[packagesKey];
    if (!lockPkg) {
      violations.push({ package: name, problem: `no packages entry for ${packagesKey}` });
      continue;
    }
    const resolution = lockPkg.resolution ?? {};
    if (!resolution.integrity) {
      violations.push({ package: name, problem: `no integrity hash for ${packagesKey}` });
      continue;
    }
    if (resolution.tarball && !/^https:\/\/registry\.npmjs\.org\//.test(resolution.tarball)) {
      violations.push({
        package: name,
        problem: `tarball outside npm registry: ${resolution.tarball}`,
      });
      continue;
    }
    audited.push({ package: name, version: bareVersion });
  }
  return { violations, audited };
}

/**
 * MT-019 — license inventory over installed editor-stack packages. Reads each
 * installed package.json under app/node_modules and checks its license against
 * the bundled-library rule for its family.
 */
export function auditEditorStackLicenses({ repoRoot, allowlist, appDir }) {
  const violations = [];
  const inventory = [];
  const { audited, violations: resolutionViolations } = auditEditorStackResolution({
    repoRoot,
    allowlist,
  });
  violations.push(...resolutionViolations);
  for (const { package: name, version } of audited) {
    const pkgJsonPath = join(appDir, "node_modules", ...name.split("/"), "package.json");
    if (!existsSync(pkgJsonPath)) {
      violations.push({ package: name, problem: "not installed (node_modules missing entry)" });
      continue;
    }
    const installed = JSON.parse(readFileSync(pkgJsonPath, "utf8"));
    const license =
      typeof installed.license === "string"
        ? installed.license
        : installed.license?.type ?? "UNKNOWN";
    const rule = allowlist.bundled_libraries.find(
      (r) =>
        r.ecosystem === "npm" &&
        r.package_patterns.some((p) =>
          p.endsWith("*") ? name.startsWith(p.slice(0, -1)) : name === p,
        ),
    );
    if (!rule) {
      violations.push({ package: name, problem: "no bundled-library rule (allowlist drift)" });
      continue;
    }
    if (!rule.allowed_licenses.includes(license)) {
      violations.push({
        package: name,
        problem: `license ${license} not in allowed set [${rule.allowed_licenses.join(", ")}]`,
      });
      continue;
    }
    inventory.push({ package: name, version, license, family: rule.family });
  }
  return { violations, inventory };
}

/** Convenience: repo root resolved from this file (app/scripts/lib → repo root). */
export function repoRootFromScriptsLib(importMetaUrl) {
  const here = new URL(".", importMetaUrl).pathname;
  // On Windows the pathname starts with a leading slash (/D:/...). Normalize.
  const normalized = decodeURIComponent(
    here.startsWith("/") && here[2] === ":" ? here.slice(1) : here,
  );
  return join(normalized, "..", "..", "..");
}

/** True when a path exists (re-exported for consumers that avoid fs imports). */
export function pathExists(p) {
  return existsSync(p);
}

/** Reads file mtime in ms, or 0 when missing. */
export function mtimeMs(p) {
  try {
    return statSync(p).mtimeMs;
  } catch {
    return 0;
  }
}
