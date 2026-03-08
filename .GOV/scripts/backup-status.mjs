#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import {
  resolveBackupRoot,
  resolveNasBackupRoot,
} from "./git-topology-lib.mjs";

const SNAPSHOT_NAME_RE = /^\d{8}-\d{6}Z-[A-Za-z0-9._-]+$/;

function listSnapshotDirs(root) {
  if (!root || !fs.existsSync(root)) return [];
  return fs.readdirSync(root, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && SNAPSHOT_NAME_RE.test(entry.name))
    .map((entry) => entry.name)
    .sort();
}

function hasGuide(root) {
  if (!root) return false;
  return fs.existsSync(path.join(root, "OFFLINE_GIT_BACKUP_SETUP.md"));
}

function buildStatus(root, label) {
  const configured = Boolean(root);
  if (!configured) {
    return {
      label,
      configured: false,
      reachable: false,
      latest: "NOT_CONFIGURED",
      manifest: "N/A",
      guide: "N/A",
    };
  }

  const reachable = fs.existsSync(root);
  const snapshots = reachable ? listSnapshotDirs(root) : [];
  const latest = snapshots.length > 0 ? snapshots[snapshots.length - 1] : "NONE";
  const manifestOk = latest !== "NONE"
    && fs.existsSync(path.join(root, latest, "manifests", "repo_resilience_manifest.json"));

  return {
    label,
    configured: true,
    reachable,
    latest,
    manifest: latest === "NONE" ? "NONE" : (manifestOk ? "OK" : "MISSING"),
    guide: hasGuide(root) ? "OK" : "MISSING",
  };
}

const localRoot = resolveBackupRoot("");
const nasRoot = resolveNasBackupRoot("");
const localStatus = buildStatus(localRoot, "LOCAL");
const nasStatus = buildStatus(nasRoot, "NAS");

console.log("BACKUP_STATUS");
console.log(`- LOCAL_ROOT: ${localStatus.configured ? "CONFIGURED" : "NOT_CONFIGURED"}`);
console.log(`- LOCAL_SNAPSHOT_ROOT_EXISTS: ${localStatus.reachable ? "YES" : "NO"}`);
console.log(`- LOCAL_LATEST_SNAPSHOT: ${localStatus.latest}`);
console.log(`- LOCAL_LATEST_MANIFEST: ${localStatus.manifest}`);
console.log(`- LOCAL_SETUP_GUIDE: ${localStatus.guide}`);
console.log(`- NAS_ROOT: ${nasStatus.configured ? "CONFIGURED" : "NOT_CONFIGURED"}`);
console.log(`- NAS_SNAPSHOT_ROOT_EXISTS: ${nasStatus.reachable ? "YES" : "NO"}`);
console.log(`- NAS_LATEST_SNAPSHOT: ${nasStatus.latest}`);
console.log(`- NAS_LATEST_MANIFEST: ${nasStatus.manifest}`);
console.log(`- NAS_SETUP_GUIDE: ${nasStatus.guide}`);
