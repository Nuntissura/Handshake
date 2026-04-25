#!/usr/bin/env node
import path from "node:path";
import { spawnSync } from "node:child_process";

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

function resolveGovernanceRoot() {
  const fromEnv = String(process.env.HANDSHAKE_GOV_ROOT || "").trim();
  if (fromEnv) return fromEnv;
  return ".GOV";
}

const subcommand = String(process.argv[2] || "").trim();
if (!subcommand) {
  console.error("Usage: node .GOV/roles_shared/scripts/session/repomem-compat.mjs <subcommand> [content] [flags...]");
  process.exit(2);
}

const govRoot = resolveGovernanceRoot();
const scriptPath = normalizePath(path.join(govRoot, "roles_shared", "scripts", "memory", "repomem.mjs"));
const args = [scriptPath, ...process.argv.slice(2)];
const result = spawnSync(process.execPath, args, {
  stdio: "inherit",
  windowsHide: true,
  env: process.env,
});

process.exit(result.status || 0);
