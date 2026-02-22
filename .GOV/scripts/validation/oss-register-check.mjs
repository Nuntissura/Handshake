import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

function resolveRepoRoot() {
  try {
    const out = execFileSync("git", ["rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return out;
  } catch {
    // Ignore; fall back to relative-to-file resolution.
  }

  // This file lives at: /.GOV/scripts/validation/oss-register-check.mjs
  // Up 3 => repo root.
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

const repoRoot = path.resolve(resolveRepoRoot());
process.chdir(repoRoot);

const OSS_REGISTER_PATH = ".GOV/roles_shared/OSS_REGISTER.md";
const CARGO_LOCK_PATH = "src/backend/handshake_core/Cargo.lock";
const PACKAGE_JSON_PATH = "app/package.json";

const HEADER_PATTERN =
  "| component_id | name | upstream_ref | license | integration_mode_default | capabilities_required | pinning_policy | compliance_notes | test_fixture | used_by_modules |";

const VALID_MODES = new Set(["embedded_lib", "external_process", "external_service"]);

function fail(code, message, details = []) {
  console.error(`[OSS_REGISTER_CHECK] ${code}: ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function readTextFile(code, filePath) {
  try {
    return fs.readFileSync(filePath, "utf8");
  } catch (err) {
    fail(code, `Cannot read ${filePath}`, [String(err?.message ?? err)]);
  }
}

function isCopyleftLicense(license) {
  const licenseUpper = String(license).toUpperCase();
  return licenseUpper.includes("AGPL") || (licenseUpper.includes("GPL") && !licenseUpper.includes("LGPL"));
}

function parseOssRegister() {
  if (!fs.existsSync(OSS_REGISTER_PATH)) {
    fail("HSK-OSS-000", "Missing OSS register", [`Expected: ${OSS_REGISTER_PATH}`]);
  }

  const content = readTextFile("HSK-OSS-000", OSS_REGISTER_PATH);
  const lines = content.split(/\r?\n/);
  const entries = [];
  let inTable = false;
  let headerFound = false;

  for (let index = 0; index < lines.length; index += 1) {
    const lineNum = index + 1;
    const line = lines[index];
    const trimmed = line.trim();

    if (trimmed === HEADER_PATTERN) {
      inTable = true;
      headerFound = true;
      continue;
    }

    if (trimmed.startsWith("|") && trimmed.includes("---")) {
      continue;
    }

    if (inTable && (trimmed === "" || trimmed.startsWith("#"))) {
      inTable = false;
      continue;
    }

    if (inTable && trimmed.startsWith("|") && trimmed.endsWith("|")) {
      const cols = trimmed
        .replace(/^\|/, "")
        .replace(/\|$/, "")
        .split("|")
        .map((c) => c.trim());

      if (cols.length !== 10) {
        fail("HSK-OSS-006", `Row format error at line ${lineNum}: expected 10 columns, found ${cols.length}`, [
          `Row: '${trimmed}'`,
        ]);
      }

      const componentId = cols[0];
      const name = cols[1];
      const license = cols[3];
      const integrationModeDefault = cols[4];

      if (!VALID_MODES.has(integrationModeDefault)) {
        fail(
          "HSK-OSS-002",
          `Invalid integration_mode_default '${integrationModeDefault}' for name '${name}' at line ${lineNum}`,
          ["Must be one of: embedded_lib, external_process, external_service"]
        );
      }

      entries.push({
        componentId,
        name,
        license,
        integrationModeDefault,
      });
    }
  }

  if (!headerFound) {
    fail("HSK-OSS-001", "OSS_REGISTER.md missing required table header", [
      `Expected exact match: '${HEADER_PATTERN}'`,
    ]);
  }

  if (entries.length === 0) {
    fail("HSK-OSS-001", "OSS_REGISTER.md has header but no valid data rows");
  }

  if (entries.length < 100) {
    fail("HSK-OSS-001", `OSS register looks unexpectedly small (${entries.length} entries)`, [
      "Expected at least 100 entries (Cargo.lock has ~400 packages).",
    ]);
  }

  return entries;
}

function parseCargoLockPackages() {
  const content = readTextFile("HSK-OSS-000", CARGO_LOCK_PATH);
  const out = new Set();
  for (const line of content.split(/\r?\n/)) {
    if (!line.startsWith('name = "')) continue;
    const m = line.match(/^name = "([^"]+)"\s*$/);
    if (m?.[1]) out.add(m[1]);
  }
  return out;
}

function parsePackageJsonDeps() {
  const content = readTextFile("HSK-OSS-000", PACKAGE_JSON_PATH);
  let json;
  try {
    json = JSON.parse(content);
  } catch (err) {
    fail("HSK-OSS-000", `Invalid JSON in ${PACKAGE_JSON_PATH}`, [String(err?.message ?? err)]);
  }

  const deps = new Set();
  for (const key of ["dependencies", "devDependencies"]) {
    const obj = json?.[key];
    if (obj && typeof obj === "object" && !Array.isArray(obj)) {
      for (const depName of Object.keys(obj)) deps.add(depName);
    }
  }
  return deps;
}

const register = parseOssRegister();
const registered = new Set(register.map((e) => e.name.toLowerCase()));

const cargoDeps = parseCargoLockPackages();
const missingCargo = [...cargoDeps].filter((dep) => !registered.has(dep.toLowerCase()));
if (missingCargo.length > 0) {
  fail("HSK-OSS-003", "Cargo.lock packages not in OSS_REGISTER.md", [JSON.stringify(missingCargo)]);
}

const npmDeps = parsePackageJsonDeps();
const missingNpm = [...npmDeps].filter((dep) => !registered.has(dep.toLowerCase()));
if (missingNpm.length > 0) {
  fail("HSK-OSS-004", "package.json deps not in OSS_REGISTER.md", [JSON.stringify(missingNpm)]);
}

const copyleftViolations = register
  .filter((e) => isCopyleftLicense(e.license))
  .filter((e) => e.integrationModeDefault !== "external_process")
  .map(
    (e) =>
      `${e.componentId} (name: ${e.name}, license: ${e.license}) has integration_mode_default '${e.integrationModeDefault}' - must be 'external_process'`
  );

if (copyleftViolations.length > 0) {
  fail("HSK-OSS-005", "Copyleft isolation violations", copyleftViolations);
}
