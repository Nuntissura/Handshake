import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

function resolveRepoRoot() {
  // This file lives at: /.GOV/scripts/validation/build-order-check.mjs
  // Up 4 => repo root.
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

const repoRoot = resolveRepoRoot();
process.chdir(repoRoot);

try {
  execFileSync("node", [".GOV/scripts/build-order-sync.mjs", "--check"], {
    stdio: "inherit",
  });
} catch {
  process.exit(1);
}

console.log("build-order-check ok");

