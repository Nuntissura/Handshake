import fs from "node:fs";
import path from "node:path";
import {
  SESSION_PLUGIN_REQUESTS_FILE,
  SESSION_REGISTRY_FILE,
} from "../scripts/session/session-policy.mjs";
import { REPO_ROOT } from "../scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import {
  loadSessionLaunchRequests,
  loadSessionRegistry,
  validateLaunchRequestShape,
  validateRegistryShape,
} from "../scripts/session/session-registry-lib.mjs";

const repoRoot = REPO_ROOT;
const registryPath = path.resolve(repoRoot, SESSION_REGISTRY_FILE);
const requestsPath = path.resolve(repoRoot, SESSION_PLUGIN_REQUESTS_FILE);

registerFailCaptureHook("session-launch-runtime-check.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("session-launch-runtime-check.mjs", message, { role: "SHARED", details });
}

if (!fs.existsSync(registryPath)) {
  fail("Missing role session registry", [SESSION_REGISTRY_FILE]);
}

if (!fs.existsSync(requestsPath)) {
  fail("Missing compatibility launch queue file", [SESSION_PLUGIN_REQUESTS_FILE]);
}

const { registry } = loadSessionRegistry(repoRoot);
const registryErrors = validateRegistryShape(registry);
if (registryErrors.length > 0) {
  fail("Role session registry schema violations found", registryErrors);
}

const { requests } = loadSessionLaunchRequests(repoRoot);
const requestErrors = [];
for (let index = 0; index < requests.length; index += 1) {
  const errors = validateLaunchRequestShape(requests[index]);
  for (const error of errors) requestErrors.push(`line ${index + 1}: ${error}`);
}

if (requestErrors.length > 0) {
  fail("Compatibility launch request schema violations found", requestErrors);
}

console.log("session-launch-runtime-check ok");
