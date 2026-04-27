// Resolve absolute paths to cloned external research harnesses without
// hardcoding drive letters or workstation-specific paths.
//
// Resolution order:
//   1. HANDSHAKE_HARNESSES_ROOT environment variable (process or persisted user env on Windows).
//   2. First `harnesses/` directory found by walking up from REPO_ROOT.
//   3. null when neither is available; callers decide how to handle a missing root.
//
// External research harnesses (pi-mono, hermes-agent, openclaw, openclaw-acpx, gastown, etc.)
// live OUTSIDE the Handshake repo by design. They are not tracked in git. This helper exists
// so governance scripts and fresh implementers can locate clones on disk without baking absolute
// paths into governance docs.

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

import { REPO_ROOT } from "./runtime-paths.mjs";

export const HARNESSES_ROOT_ENV_VAR = "HANDSHAKE_HARNESSES_ROOT";

function readPersistedUserEnv(name) {
  if (process.platform !== "win32") return "";
  try {
    return execFileSync(
      "powershell.exe",
      ["-NoLogo", "-NonInteractive", "-Command", `[Environment]::GetEnvironmentVariable('${name}','User')`],
      { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
    ).trim();
  } catch {
    return "";
  }
}

function findHarnessesRootByWalk(startDir) {
  let current = path.resolve(startDir);
  // Walk up at most 8 levels to bound the search.
  for (let depth = 0; depth < 8; depth += 1) {
    const candidate = path.join(current, "harnesses");
    if (fs.existsSync(candidate) && fs.statSync(candidate).isDirectory()) {
      return candidate;
    }
    const parent = path.dirname(current);
    if (parent === current) break;
    current = parent;
  }
  return null;
}

export function resolveHarnessesRoot() {
  const envValue = String(
    process.env[HARNESSES_ROOT_ENV_VAR]
      || readPersistedUserEnv(HARNESSES_ROOT_ENV_VAR)
      || "",
  ).trim();
  if (envValue) {
    const candidate = path.resolve(envValue);
    if (fs.existsSync(candidate) && fs.statSync(candidate).isDirectory()) return candidate;
  }
  return findHarnessesRootByWalk(REPO_ROOT);
}

export function resolveHarnessPath(harnessName, relPath = "") {
  if (!harnessName || typeof harnessName !== "string") {
    throw new Error("resolveHarnessPath: harnessName is required");
  }
  const root = resolveHarnessesRoot();
  if (!root) return null;
  const harnessDir = path.join(root, harnessName);
  return relPath ? path.join(harnessDir, relPath) : harnessDir;
}

export function harnessExists(harnessName) {
  const resolved = resolveHarnessPath(harnessName);
  return Boolean(resolved && fs.existsSync(resolved) && fs.statSync(resolved).isDirectory());
}
