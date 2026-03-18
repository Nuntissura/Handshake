#!/usr/bin/env node

/**
 * Captures `just gov-check` output and, on failure, writes the error details
 * as a notification + thread message to the relevant WP's coder.
 *
 * Usage:
 *   node .GOV/roles/orchestrator/scripts/gov-check-feedback.mjs [WP-{ID}] [--session=<orchestrator-session>]
 *
 * If WP-{ID} is provided, the failure notification targets that specific WP.
 * Otherwise, the script attempts to extract WP references from the error output.
 */

import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { appendWpThreadEntry } from "../../../roles_shared/scripts/wp/wp-thread-append.mjs";
import { appendWpNotification } from "../../../roles_shared/scripts/wp/wp-notification-append.mjs";

function extractWpIds(text) {
  const matches = text.match(/WP-[A-Za-z0-9_-]+/g);
  if (!matches) return [];
  return [...new Set(matches)];
}

function truncateOutput(text, maxLines = 30) {
  const lines = String(text || "").split(/\r?\n/);
  if (lines.length <= maxLines) return text;
  return [...lines.slice(0, maxLines), `... (${lines.length - maxLines} more lines truncated)`].join("\n");
}

function runCli() {
  const args = process.argv.slice(2);
  const wpIdArg = args.find((arg) => arg.startsWith("WP-")) || "";
  const sessionArg = args.find((arg) => arg.startsWith("--session="))?.slice("--session=".length) || "orchestrator";

  let stdout = "";
  let stderr = "";
  let exitCode = 0;

  try {
    stdout = execFileSync("just", ["gov-check"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 120000,
    });
    console.log("[GOV_CHECK_FEEDBACK] gov-check passed — no feedback needed.");
    console.log(stdout);
    return;
  } catch (error) {
    exitCode = error.status || 1;
    stdout = String(error.stdout || "").trim();
    stderr = String(error.stderr || "").trim();
  }

  const fullOutput = [stdout, stderr].filter(Boolean).join("\n");
  console.error(`[GOV_CHECK_FEEDBACK] gov-check FAILED (exit ${exitCode})`);
  console.error(fullOutput);

  const detectedWpIds = wpIdArg ? [wpIdArg] : extractWpIds(fullOutput);
  if (detectedWpIds.length === 0) {
    console.error("[GOV_CHECK_FEEDBACK] no WP references found in failure output — cannot route feedback.");
    process.exit(exitCode);
  }

  const feedbackMessage = [
    "GOV_CHECK FAILURE — action required.",
    "",
    "The governance checks failed. Review the following output and fix the violations before continuing.",
    "",
    "```",
    truncateOutput(fullOutput),
    "```",
    "",
    "Run `just gov-check` to verify your fix. Do not proceed with handoff until gov-check passes clean.",
  ].join("\n");

  for (const wpId of detectedWpIds) {
    try {
      appendWpThreadEntry({
        wpId,
        actorRole: "ORCHESTRATOR",
        actorSession: sessionArg,
        message: feedbackMessage,
        target: "@coder",
        targetRole: "CODER",
        recordReceipt: true,
      });
      console.log(`[GOV_CHECK_FEEDBACK] sent failure feedback to ${wpId} CODER via thread + notification`);
    } catch (error) {
      console.error(`[GOV_CHECK_FEEDBACK] could not send feedback to ${wpId}: ${error.message}`);
    }
  }

  process.exit(exitCode);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runCli();
}
