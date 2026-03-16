import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../..");

const ACP_BUILD_MANIFEST = [
  ".GOV/tools/handshake-acp-bridge/agent.mjs",
  ".GOV/roles/orchestrator/scripts/session-control-broker.mjs",
  ".GOV/roles_shared/scripts/session/session-control-lib.mjs",
  ".GOV/roles_shared/scripts/session/session-registry-lib.mjs",
  ".GOV/roles_shared/scripts/session/handshake-acp-client.mjs",
  ".GOV/roles_shared/scripts/session/session-policy.mjs",
  ".GOV/tools/vscode-session-bridge/extension.js",
];

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

export function computeAcpBuildId() {
  const hash = crypto.createHash("sha256");
  for (const relPath of ACP_BUILD_MANIFEST) {
    const absPath = path.resolve(REPO_ROOT, relPath);
    if (!fs.existsSync(absPath)) {
      throw new Error(`ACP build manifest entry is missing: ${normalizePath(relPath)}`);
    }
    hash.update(`${normalizePath(relPath)}\n`);
    hash.update(fs.readFileSync(absPath));
    hash.update("\n");
  }
  return `sha256:${hash.digest("hex").slice(0, 16)}`;
}

export const ACP_BUILD_ID = computeAcpBuildId();
export const ACP_BUILD_MANIFEST_FILES = [...ACP_BUILD_MANIFEST];
