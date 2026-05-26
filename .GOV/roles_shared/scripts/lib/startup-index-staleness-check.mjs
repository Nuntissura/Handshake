#!/usr/bin/env node
/**
 * startup-index-staleness-check.mjs
 *
 * Startup-time visibility aid for the staged-index hazard described by the
 * KERNEL_BUILDER root-cause finding (4 stray `app/*` files were swept into a
 * `gov_kernel` commit because a previous session left them in the git index
 * and the new session never noticed).
 *
 * Behaviour:
 *   - Reads `git diff --cached --name-status` to enumerate pre-staged paths.
 *   - If any paths are staged, prints a "STAGED-INDEX WARNING" banner with
 *     every staged path, change kind (A/M/D/R/C/T/U), and the most recent
 *     commit that touched each path (`git log -1 --format=%h %s -- <path>`)
 *     so the operator/model can guess which prior session left it behind.
 *   - If the staging area is empty, prints a single `staged-index: clean`
 *     line so startup output still records the check ran.
 *   - Never exits non-zero. This is informational only because the operator
 *     may legitimately have work-in-progress staged. Startup MUST NOT block.
 *   - Skips with a single line when run outside a git work tree.
 *
 * Wired into role startup via .GOV/operator/scripts/orcstart.ps1.
 * Also exposed ad-hoc via `just gov-staged-index-survey`.
 *
 * Per [CX-205N], failures inside this informational check are best-effort
 * captured to gov_runtime/roles_shared/fail_capture.jsonl via fail-capture-lib;
 * the check never exits non-zero regardless.
 */

import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { existsSync } from "node:fs";

import { captureFailure, registerFailCaptureHook } from "./fail-capture-lib.mjs";

const SCRIPT_NAME = "startup-index-staleness-check.mjs";
const BANNER_RULE = "=".repeat(72);

registerFailCaptureHook(SCRIPT_NAME, { role: process.env.ORCSTART_ROLE || "" });

function resolveRepoRoot() {
  // .GOV/roles_shared/scripts/lib/this-file -> up 4 = repo root
  const fileRelativeRepoRoot = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    "..",
    "..",
    "..",
    "..",
  );
  try {
    const out = execFileSync(
      "git",
      ["-C", fileRelativeRepoRoot, "rev-parse", "--show-toplevel"],
      { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
    ).trim();
    if (out) return out;
  } catch {
    // fall through; non-git environment is handled below
  }
  return fileRelativeRepoRoot;
}

function runGit(repoRoot, args) {
  try {
    const stdout = execFileSync("git", ["-C", repoRoot, ...args], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    return { ok: true, stdout, stderr: "" };
  } catch (error) {
    const stderr = typeof error?.stderr === "string"
      ? error.stderr
      : (error?.stderr ? String(error.stderr) : "");
    return { ok: false, stdout: "", stderr };
  }
}

function gitStdoutOrNull(result) {
  return result && result.ok ? result.stdout : null;
}

function isInsideGitWorkTree(repoRoot) {
  if (!existsSync(repoRoot)) return false;
  const out = gitStdoutOrNull(runGit(repoRoot, ["rev-parse", "--is-inside-work-tree"]));
  return typeof out === "string" && out.trim() === "true";
}

function parseNameStatusNul(text) {
  // Parse `git diff --cached --name-status -z` output.
  //
  // In `-z` mode every field is NUL-delimited (no embedded TABs), and the
  // two record shapes are:
  //
  //   Normal (A/M/D/T/U):   "<STATUS>\0<PATH>\0"
  //   Rename/Copy (R/C):    "<STATUS>\0<OLDPATH>\0<NEWPATH>\0"
  //
  // We peek the leading character of each status record to decide whether to
  // consume 1 trailing path record (normal) or 2 trailing path records
  // (rename/copy). This is robust against filenames containing TAB or NEWLINE,
  // which the non-`-z` tab-delimited form mis-splits.
  const entries = [];
  if (!text) return entries;
  const records = text.split("\0");
  // A well-formed `-z` stream ends with a NUL, producing a trailing empty
  // record; tolerate streams that don't.
  let i = 0;
  while (i < records.length) {
    const status = records[i];
    if (status === undefined || status === "") {
      i += 1;
      continue;
    }
    const kind = status.charAt(0).toUpperCase();
    if (kind === "R" || kind === "C") {
      // Status, then oldpath, then newpath, each as its own NUL-delimited record.
      const oldPath = records[i + 1] ?? "";
      const newPath = records[i + 2] ?? "";
      entries.push({
        status,
        kind,
        path: newPath,
        extra: oldPath ? `from ${oldPath}` : "",
      });
      i += 3;
      continue;
    }
    // Normal: status record, then one path record.
    const pathPart = records[i + 1] ?? "";
    entries.push({
      status,
      kind,
      path: pathPart,
      extra: "",
    });
    i += 2;
  }
  return entries;
}

function lastCommitTouching(repoRoot, repoRelPath) {
  // `--` separates revision from path; quoting is handled by execFileSync.
  const out = gitStdoutOrNull(runGit(repoRoot, [
    "log",
    "-1",
    "--format=%h %s",
    "--",
    repoRelPath,
  ]));
  if (!out) return "";
  return out.split(/\r?\n/u)[0].trim();
}

function formatRow({ kind, statusRaw, repoRelPath, extra, lastCommit }) {
  const kindCol = kind.padEnd(2, " ");
  const statusCol = String(statusRaw || kind).padEnd(4, " ");
  const pathCol = repoRelPath;
  const commitCol = lastCommit || "(no prior commit)";
  const extraCol = extra ? `  (${extra})` : "";
  return `  ${kindCol} ${statusCol} ${pathCol}${extraCol}  | last_touched: ${commitCol}`;
}

function emitClean() {
  console.log("staged-index: clean");
}

function emitNonGit(reason) {
  console.log(`staged-index: skipped (${reason})`);
}

function emitWarning(repoRoot, entries) {
  console.log(BANNER_RULE);
  console.log("STAGED-INDEX WARNING [startup-index-staleness-check]");
  console.log(BANNER_RULE);
  console.log(`repo_root: ${repoRoot}`);
  console.log(`staged_paths_count: ${entries.length}`);
  console.log("informational_only: yes (startup continues)");
  console.log("");
  console.log("Pre-staged files were found in the git index at session start.");
  console.log("A previous session may have left them; they will be swept into the");
  console.log("next `git commit` unless reset. Inspect each path and decide:");
  console.log("  - keep         -> leave staged for an intentional commit");
  console.log("  - unstage      -> git restore --staged <path>");
  console.log("  - drop content -> git restore --staged <path> && git checkout -- <path>");
  console.log("");
  console.log("STAGED_PATHS:");
  for (const entry of entries) {
    const lastCommit = lastCommitTouching(repoRoot, entry.path);
    console.log(formatRow({
      kind: entry.kind,
      statusRaw: entry.status,
      repoRelPath: entry.path,
      extra: entry.extra,
      lastCommit,
    }));
  }
  console.log("");
  console.log("KIND_LEGEND: A=added M=modified D=deleted R=renamed C=copied T=typechange U=unmerged");
  console.log(BANNER_RULE);
}

export function runStartupIndexStalenessCheck({ repoRoot = resolveRepoRoot() } = {}) {
  const absRepoRoot = path.resolve(repoRoot);

  if (!isInsideGitWorkTree(absRepoRoot)) {
    emitNonGit("not inside a git work tree");
    return { ok: true, staged: 0, skipped: true };
  }

  const nameStatusResult = runGit(absRepoRoot, ["diff", "--cached", "--name-status", "-z"]);
  if (!nameStatusResult.ok) {
    // Best-effort capture; do NOT exit non-zero (informational only).
    const stderrRaw = (nameStatusResult.stderr || "").trim();
    const stderrPreview = stderrRaw
      ? stderrRaw.replace(/\s+/gu, " ").slice(0, 200)
      : "";
    captureFailure(SCRIPT_NAME, "git diff --cached --name-status -z failed", {
      role: process.env.ORCSTART_ROLE || "",
      details: stderrPreview ? [`stderr: ${stderrPreview}`] : [],
    });
    const suffix = stderrPreview ? `: ${stderrPreview}` : "";
    emitNonGit(`git diff --cached failed${suffix}`);
    return { ok: true, staged: 0, skipped: true };
  }

  const entries = parseNameStatusNul(nameStatusResult.stdout);
  if (entries.length === 0) {
    emitClean();
    return { ok: true, staged: 0, skipped: false };
  }

  emitWarning(absRepoRoot, entries);
  return { ok: true, staged: entries.length, skipped: false };
}

const invokedDirectly = (() => {
  try {
    return fileURLToPath(import.meta.url) === path.resolve(process.argv[1] || "");
  } catch {
    return false;
  }
})();

if (invokedDirectly) {
  runStartupIndexStalenessCheck();
  process.exit(0);
}
