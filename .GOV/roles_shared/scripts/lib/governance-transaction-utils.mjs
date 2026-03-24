import fs from "node:fs";
import os from "node:os";
import path from "node:path";

function copyPath(sourcePath, targetPath, isDirectory) {
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  if (isDirectory) {
    fs.cpSync(sourcePath, targetPath, {
      recursive: true,
      force: true,
      preserveTimestamps: true,
    });
    return;
  }
  fs.copyFileSync(sourcePath, targetPath);
}

function removeIfExists(targetPath) {
  if (!fs.existsSync(targetPath)) return;
  fs.rmSync(targetPath, {
    recursive: true,
    force: true,
  });
}

export function createPathSnapshot(targetPaths, { label = "gov-transaction" } = {}) {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), `${label}-`));
  const snapshots = [];

  for (const rawTargetPath of targetPaths) {
    const normalizedTargetPath = String(rawTargetPath || "").trim();
    if (!normalizedTargetPath) continue;
    const targetPath = path.resolve(normalizedTargetPath);

    const exists = fs.existsSync(targetPath);
    const backupPath = path.join(tempRoot, String(snapshots.length));
    let isDirectory = false;

    if (exists) {
      const stats = fs.lstatSync(targetPath);
      isDirectory = stats.isDirectory();
      copyPath(targetPath, backupPath, isDirectory);
    }

    snapshots.push({
      targetPath,
      backupPath,
      exists,
      isDirectory,
    });
  }

  return {
    tempRoot,
    snapshots,
  };
}

export function restorePathSnapshot(snapshot) {
  const entries = Array.isArray(snapshot?.snapshots) ? [...snapshot.snapshots].reverse() : [];

  for (const entry of entries) {
    removeIfExists(entry.targetPath);
    if (!entry.exists) continue;
    copyPath(entry.backupPath, entry.targetPath, entry.isDirectory);
  }
}

export function cleanupPathSnapshot(snapshot) {
  if (!snapshot?.tempRoot) return;
  removeIfExists(snapshot.tempRoot);
}
