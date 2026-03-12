import fs from "node:fs";
import path from "node:path";
import {
  SESSION_PLUGIN_REQUESTS_FILE,
  SESSION_REGISTRY_FILE,
} from "../session-policy.mjs";
import {
  loadSessionLaunchRequests,
  loadSessionRegistry,
  validateLaunchRequestShape,
  validateRegistryShape,
} from "../session-registry-lib.mjs";

const repoRoot = process.cwd();
const registryPath = path.resolve(repoRoot, SESSION_REGISTRY_FILE);
const requestsPath = path.resolve(repoRoot, SESSION_PLUGIN_REQUESTS_FILE);

function fail(message, details = []) {
  console.error(`[SESSION_LAUNCH_RUNTIME_CHECK] ${message}`);
  for (const detail of details) console.error(`  - ${detail}`);
  process.exit(1);
}

if (!fs.existsSync(registryPath)) {
  fail("Missing role session registry", [SESSION_REGISTRY_FILE]);
}

if (!fs.existsSync(requestsPath)) {
  fail("Missing session launch requests file", [SESSION_PLUGIN_REQUESTS_FILE]);
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
  fail("Session launch request schema violations found", requestErrors);
}

console.log("session-launch-runtime-check ok");
