import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  docsRootHotspots,
  govRootHotspots,
  roleRootRules,
  rolesSharedRootRules,
} from "./governance-structure-rules.mjs";

function resolveRepoRoot() {
  try {
    const out = execFileSync("git", ["rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return out;
  } catch {
    // Ignore and fall back.
  }
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

function toPosix(value) {
  return value.split(path.sep).join("/");
}

function pushFinding(findings, scope, relPath, severity, reason, target) {
  findings.push({ scope, relPath, severity, reason, target });
}

function auditGovRoot(repoRoot, findings) {
  const govRoot = path.join(repoRoot, ".GOV");
  for (const [name, meta] of Object.entries(govRootHotspots)) {
    const full = path.join(govRoot, name);
    if (fs.existsSync(full)) {
      pushFinding(findings, ".GOV root", toPosix(path.relative(repoRoot, full)), meta.severity, meta.reason, meta.target);
    }
  }
}

function auditDocsRoot(repoRoot, findings) {
  const docsRoot = path.join(repoRoot, ".GOV", "docs");
  for (const [name, meta] of Object.entries(docsRootHotspots)) {
    const full = path.join(docsRoot, name);
    if (fs.existsSync(full)) {
      pushFinding(findings, ".GOV/docs", toPosix(path.relative(repoRoot, full)), meta.severity, meta.reason, meta.target);
    }
  }
}

function auditRolesSharedRoot(repoRoot, findings) {
  const sharedRoot = path.join(repoRoot, ".GOV", "roles_shared");
  const buckets = [
    ["docs", rolesSharedRootRules.docs],
    ["records", rolesSharedRootRules.records],
    ["runtime", rolesSharedRootRules.runtime],
    ["duplicate template", rolesSharedRootRules.duplicateTemplates],
  ];
  for (const [bucket, mapping] of buckets) {
    for (const [name, target] of mapping.entries()) {
      const full = path.join(sharedRoot, name);
      if (fs.existsSync(full)) {
        pushFinding(
          findings,
          ".GOV/roles_shared root",
          toPosix(path.relative(repoRoot, full)),
          bucket === "runtime" ? "HIGH" : "MEDIUM",
          `${bucket} material is still loose at roles_shared root`,
          target,
        );
      }
    }
  }
}

function auditRoleRoots(repoRoot, findings) {
  const rolesRoot = path.join(repoRoot, ".GOV", "roles");
  for (const [roleName, groups] of Object.entries(roleRootRules)) {
    const roleRoot = path.join(rolesRoot, roleName);
    for (const [bucket, mapping] of Object.entries(groups)) {
      for (const [name, target] of mapping.entries()) {
        const full = path.join(roleRoot, name);
        if (fs.existsSync(full)) {
          pushFinding(
            findings,
            `.GOV/roles/${roleName} root`,
            toPosix(path.relative(repoRoot, full)),
            bucket === "legacy" ? "MEDIUM" : "HIGH",
            `${bucket} surface still lives at role root`,
            target,
          );
        }
      }
    }
  }
}

function printFindings(findings) {
  if (findings.length === 0) {
    console.log("governance-structure-check: clean");
    return;
  }
  console.log(`governance-structure-check: ${findings.length} hotspot(s)`);
  for (const finding of findings) {
    console.log(`- [${finding.severity}] ${finding.scope}: ${finding.relPath}`);
    console.log(`  reason: ${finding.reason}`);
    console.log(`  target: ${finding.target}`);
  }
}

const repoRoot = path.resolve(resolveRepoRoot());
process.chdir(repoRoot);

const strict = process.argv.includes("--strict");
const findings = [];

auditGovRoot(repoRoot, findings);
auditDocsRoot(repoRoot, findings);
auditRolesSharedRoot(repoRoot, findings);
auditRoleRoots(repoRoot, findings);

printFindings(findings);

if (strict && findings.length > 0) {
  process.exit(1);
}
