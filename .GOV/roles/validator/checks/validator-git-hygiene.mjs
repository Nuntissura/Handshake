#!/usr/bin/env node
import { execSync } from "node:child_process";
import { readFileSync, statSync } from "node:fs";
import { REPO_ROOT, normalizePath } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { evaluateArtifactHygiene } from "../../../roles_shared/scripts/lib/artifact-hygiene-lib.mjs";
import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
registerFailCaptureHook("validator-git-hygiene.mjs", { role: "WP_VALIDATOR" });

const gitignorePath = ".gitignore";
const requiredPatterns = ["target/", "node_modules/", "*.pdb", "*.dSYM", ".DS_Store", "Thumbs.db"];
const artifactRegex =
  /(\/|^)(target\/|node_modules\/)|\.pdb$|\.dSYM$|\.DS_Store$|Thumbs\.db$/;

function fail(message, details = "") {
  failWithMemory("validator-git-hygiene.mjs", message, { role: "WP_VALIDATOR", details: details ? [details] : [] });
}

let gitignore;
try {
  gitignore = readFileSync(gitignorePath, "utf8");
} catch (err) {
  fail(`cannot read ${gitignorePath}: ${err.message}`);
}

const missing = requiredPatterns.filter((p) => !gitignore.includes(p));
if (missing.length > 0) {
  fail(`.gitignore missing patterns: ${missing.join(", ")}`);
}

let committedArtifacts = "";
try {
  const out = execSync("git ls-files", { stdio: "pipe", encoding: "utf8" });
  committedArtifacts = out
    .split("\n")
    .filter((line) => artifactRegex.test(line))
    .filter(Boolean)
    .join("\n");
} catch (err) {
  fail(`git ls-files failed: ${err.message}`);
}

if (committedArtifacts.trim().length > 0) {
  fail(`committed build artifacts detected:\n${committedArtifacts}`);
}

let largeUntracked = "";
try {
  const out = execSync("git ls-files --others --exclude-standard", {
    stdio: "pipe",
    encoding: "utf8",
  });
  const files = out.split("\n").filter(Boolean);
  const offenders = [];
  for (const f of files) {
    try {
      const st = statSync(f);
      if (st.size > 10 * 1024 * 1024) {
        offenders.push(`${f} (${(st.size / (1024 * 1024)).toFixed(1)}MB)`);
      }
    } catch {
      // ignore missing files
    }
  }
  largeUntracked = offenders.join("\n");
} catch (err) {
  fail(`git ls-files (untracked) failed: ${err.message}`);
}

if (largeUntracked.trim().length > 0) {
  fail(`untracked large files detected (>10MB):\n${largeUntracked}`);
}

const artifactEvaluation = evaluateArtifactHygiene({ repoRoot: REPO_ROOT });
if (artifactEvaluation.blockingIssues.length > 0) {
  fail(`artifact hygiene violations detected:\n${artifactEvaluation.blockingIssues.join("\n")}`);
}

console.log("validator-git-hygiene: PASS — .gitignore coverage and artifact checks clean.");
if (artifactEvaluation.reclaimableExternalDirs.length > 0) {
  console.log(
    `validator-git-hygiene: NOTE — reclaimable stale external artifact dirs: ${artifactEvaluation.reclaimableExternalDirs.map((entry) => normalizePath(entry.absPath)).join(", ")}`,
  );
}
