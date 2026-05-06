#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import {
  GOV_ROOT_REPO_REL,
  REPO_ROOT,
  repoPathAbs,
  resolveWorkPacketPath,
  WORK_PACKET_STORAGE_ROOT_REPO_REL,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";

const wpId = String(process.argv[2] || "").trim();

if (!wpId || !wpId.startsWith("WP-")) {
  console.error("Usage: node .GOV/roles/orchestrator/scripts/orchestrator-checkpoint-activation.mjs WP-{ID}");
  process.exit(1);
}

function normalize(value) {
  return String(value || "").replace(/\\/g, "/");
}

function relPath(...segments) {
  return normalize(path.join(...segments));
}

function readTextIfExists(repoRelPath) {
  const absolutePath = repoPathAbs(repoRelPath);
  return fs.existsSync(absolutePath) ? fs.readFileSync(absolutePath, "utf8") : "";
}

function existingPath(repoRelPath) {
  return fs.existsSync(repoPathAbs(repoRelPath));
}

function git(args, options = {}) {
  return execFileSync("git", args, {
    cwd: REPO_ROOT,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    ...options,
  });
}

function gitLines(args) {
  return git(args).split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
}

function addIfExists(targets, repoRelPath) {
  const normalized = normalize(repoRelPath);
  if (normalized && existingPath(normalized)) targets.add(normalized);
}

function addSpecTarget(targets, refinementText) {
  const specTarget = refinementText.match(/^- SPEC_TARGET_RESOLVED:\s*.*->\s*([^\r\n]+)$/mi)?.[1]?.trim();
  if (!specTarget) return;
  addIfExists(targets, relPath(GOV_ROOT_REPO_REL, "spec", path.basename(specTarget)));
}

function addStubTargets(targets, refinementText) {
  const ids = new Set();
  for (const match of refinementText.matchAll(/\bWP-[A-Za-z0-9][A-Za-z0-9_-]*-v\d+\b/g)) {
    const candidate = match[0];
    if (candidate !== wpId) ids.add(candidate);
  }
  for (const stubWpId of ids) {
    addIfExists(targets, relPath(GOV_ROOT_REPO_REL, "task_packets", "stubs", `${stubWpId}.md`));
  }
}

function addWpAuditTargets(targets) {
  const auditRoots = [
    relPath(GOV_ROOT_REPO_REL, "Audits"),
  ];
  for (const root of auditRoots) {
    if (!existingPath(root)) continue;
    const candidates = gitLines(["ls-files", "-m", "-o", "--exclude-standard", "--", root]);
    for (const candidate of candidates) {
      if (!existingPath(candidate)) continue;
      const text = readTextIfExists(candidate);
      if (text.includes(wpId)) targets.add(normalize(candidate));
    }
  }
}

function stagedFiles() {
  return gitLines(["diff", "--cached", "--name-only"]);
}

function stageTargets(targets) {
  const targetList = [...targets].sort();
  if (!targetList.length) return;
  git(["add", "--", ...targetList], { stdio: ["ignore", "inherit", "inherit"] });
}

function main() {
  const stagedBefore = stagedFiles();
  if (stagedBefore.length) {
    console.error("[ORCHESTRATOR] Refusing activation checkpoint because files were already staged before scoped staging.");
    console.error("[ORCHESTRATOR] Unstage or commit these paths explicitly first:");
    for (const file of stagedBefore) console.error(`- ${file}`);
    process.exit(1);
  }

  const targets = new Set();
  const refinementPath = relPath(GOV_ROOT_REPO_REL, "refinements", `${wpId}.md`);
  const packetPath = normalize(resolveWorkPacketPath(wpId)?.packetPath || relPath(WORK_PACKET_STORAGE_ROOT_REPO_REL, wpId, "packet.md"));
  const packetDir = relPath(WORK_PACKET_STORAGE_ROOT_REPO_REL, wpId);

  addIfExists(targets, refinementPath);
  addIfExists(targets, packetPath);
  addIfExists(targets, packetDir);
  addIfExists(targets, relPath(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpId}.md`));
  addIfExists(targets, relPath(GOV_ROOT_REPO_REL, "roles_shared", "records", "TASK_BOARD.md"));
  addIfExists(targets, relPath(GOV_ROOT_REPO_REL, "roles_shared", "records", "WP_TRACEABILITY_REGISTRY.md"));
  addIfExists(targets, relPath(GOV_ROOT_REPO_REL, "roles_shared", "records", "BUILD_ORDER.md"));
  addIfExists(targets, relPath(GOV_ROOT_REPO_REL, "roles_shared", "records", "SIGNATURE_AUDIT.md"));
  addIfExists(targets, relPath(GOV_ROOT_REPO_REL, "spec", "SPEC_CURRENT.md"));
  addIfExists(targets, relPath(GOV_ROOT_REPO_REL, "reference", "PAST_WORK_INDEX.md"));

  const refinementText = readTextIfExists(refinementPath);
  addSpecTarget(targets, refinementText);
  addStubTargets(targets, refinementText);
  addWpAuditTargets(targets);

  stageTargets(targets);

  const staged = stagedFiles();
  if (!staged.length) {
    console.log("[ORCHESTRATOR] No activation checkpoint changes to commit.");
    return;
  }

  console.log("[ORCHESTRATOR] Activation checkpoint staged paths:");
  for (const file of staged) console.log(`- ${file}`);
  git(["commit", "-m", `gov: checkpoint packet+refinement+micro-tasks [${wpId}]`], { stdio: "inherit" });
}

main();
