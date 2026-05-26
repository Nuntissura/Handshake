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
      { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
    ).trim();
    if (out) return out;
  } catch {
    // fall through; non-git environment is handled below
  }
  return fileRelativeRepoRoot;
}

function runGit(repoRoot, args) {
  try {
    return execFileSync("git", ["-C", repoRoot, ...args], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
  } catch (error) {
    return null;
  }
}

function isInsideGitWorkTree(repoRoot) {
  if (!existsSync(repoRoot)) return false;
  const out = runGit(repoRoot, ["rev-parse", "--is-inside-work-tree"]);
  return typeof out === "string" && out.trim() === "true";
}

function parseNameStatus(text) {
  // `git diff --cached --name-status -z` would be safer but we follow the
  // task spec which calls for `--name-status`. Plain newline-tab parsing is
  // sufficient for the informational view; pathological filenames are rare
  // in this repo's governance lanes.
  const entries = [];
  if (!text) return entries;
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.replace(/\s+$/u, "");
    if (!line) continue;
    const cols = line.split("\t");
    if (cols.length < 2) continue;
    const status = cols[0];
    // For R (rename) and C (copy), git emits: R100\told\tnew  (or C75\told\tnew)
    const kind = status.charAt(0).toUpperCase();
    let displayPath = "";
    let extra = "";
    if ((kind === "R" || kind === "C") && cols.length >= 3) {
      displayPath = cols[2];
      extra = `from ${cols[1]}`;
    } else {
      displayPath = cols[cols.length - 1];
    }
    entries.push({
      status,
      kind,
      path: displayPath,
      extra,
    });
  }
  return entries;
}

function lastCommitTouching(repoRoot, repoRelPath) {
  // `--` separates revision from path; quoting is handled by execFileSync.
  const out = runGit(repoRoot, [
    "log",
    "-1",
    "--format=%h %s",
    "--",
    repoRelPath,
  ]);
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

  const nameStatus = runGit(absRepoRoot, ["diff", "--cached", "--name-status"]);
  if (nameStatus === null) {
    // Best-effort capture; do NOT exit non-zero (informational only).
    captureFailure(SCRIPT_NAME, "git diff --cached --name-status failed", {
      role: process.env.ORCSTART_ROLE || "",
    });
    emitNonGit("git diff --cached failed");
    return { ok: true, staged: 0, skipped: true };
  }

  const entries = parseNameStatus(nameStatus);
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
