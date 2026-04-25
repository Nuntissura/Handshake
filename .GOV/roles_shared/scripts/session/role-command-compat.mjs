#!/usr/bin/env node
import { spawnSync } from "node:child_process";

function usage() {
  console.error("Usage: node .GOV/roles_shared/scripts/session/role-command-compat.mjs <validator-startup|validator-next|active-lane-brief> <ROLE> [WP_ID] [--json]");
  process.exit(2);
}

const command = String(process.argv[2] || "").trim();
const role = String(process.argv[3] || "").trim().toUpperCase();
const wpId = String(process.argv[4] || "").trim();
const extraArgs = process.argv.slice(5);

if (!["validator-startup", "validator-next", "active-lane-brief"].includes(command)) usage();
if (["validator-startup", "validator-next"].includes(command) && !["WP_VALIDATOR", "INTEGRATION_VALIDATOR", "VALIDATOR"].includes(role)) usage();
if (command === "active-lane-brief" && !["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"].includes(role)) usage();
if (["validator-next", "active-lane-brief"].includes(command) && !wpId) usage();

const listResult = spawnSync("just", ["--list"], {
  encoding: "utf8",
  stdio: ["ignore", "pipe", "pipe"],
  windowsHide: true,
});

if (listResult.status !== 0) {
  process.stderr.write(listResult.stderr || listResult.stdout || "failed to inspect just command surface\n");
  process.exit(listResult.status || 1);
}

const justList = `${listResult.stdout || ""}\n${listResult.stderr || ""}`;

if (command === "active-lane-brief" && !new RegExp(`^\\s*${command}\\s+role\\s+wp-id\\b`, "mi").test(justList)) {
  const runResult = spawnSync("node", [".GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs", role, wpId, ...extraArgs], {
    stdio: "inherit",
    windowsHide: true,
    env: {
      ...process.env,
    },
  });
  process.exit(runResult.status || 0);
}

const roleQualified = new RegExp(`^\\s*${command}\\s+role\\b`, "mi").test(justList);
const args = [command];

if (roleQualified) args.push(role);
if (["validator-next", "active-lane-brief"].includes(command)) args.push(wpId);
if (command === "active-lane-brief") args.push(...extraArgs);

const runResult = spawnSync("just", args, {
  stdio: "inherit",
  windowsHide: true,
  env: {
    ...process.env,
    HANDSHAKE_VALIDATOR_ROLE: role,
  },
});

process.exit(runResult.status || 0);
