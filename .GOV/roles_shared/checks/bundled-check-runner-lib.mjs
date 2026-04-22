import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

function normalizeBundleScriptLabel(scriptName = "") {
  return String(scriptName || "").trim().replace(/\.mjs$/i, "");
}

export function formatBundledCheckFailure(scriptName, error) {
  const parts = [];
  if (Number.isInteger(error?.status)) {
    parts.push(`exit=${error.status}`);
  }
  if (error?.signal) {
    parts.push(`signal=${error.signal}`);
  }
  if (!parts.length && error?.code) {
    parts.push(`code=${error.code}`);
  }
  if (error?.killed) {
    parts.push("killed");
  }
  return `${normalizeBundleScriptLabel(scriptName)}${parts.length ? ` (${parts.join(", ")})` : ""}`;
}

export function runBundledChecks(importMetaUrl, scriptNames = []) {
  const checksDir = path.dirname(fileURLToPath(importMetaUrl));
  const failures = [];

  for (const scriptName of scriptNames) {
    try {
      execFileSync(process.execPath, [path.join(checksDir, scriptName)], {
        stdio: ["ignore", "inherit", "inherit"],
        env: process.env,
        cwd: process.cwd(),
      });
    } catch (error) {
      failures.push(formatBundledCheckFailure(scriptName, error));
    }
  }

  return failures;
}
