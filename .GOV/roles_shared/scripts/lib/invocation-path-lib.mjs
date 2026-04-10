import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

function defaultRealpathSync(targetPath) {
  return fs.realpathSync.native(targetPath);
}

export function canonicalInvocationPath(rawPath, {
  realpathSync = defaultRealpathSync,
} = {}) {
  const resolvedPath = path.resolve(String(rawPath || "").trim());
  try {
    return realpathSync(resolvedPath);
  } catch {
    return resolvedPath;
  }
}

export function isInvokedAsMain(importMetaUrl, argv1, options = {}) {
  const scriptArg = String(argv1 || "").trim();
  if (!scriptArg) return false;
  const invokedPath = canonicalInvocationPath(scriptArg, options);
  const modulePath = canonicalInvocationPath(fileURLToPath(importMetaUrl), options);
  return invokedPath === modulePath;
}
