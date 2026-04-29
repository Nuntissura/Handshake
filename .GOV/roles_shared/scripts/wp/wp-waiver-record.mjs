#!/usr/bin/env node

import path from "node:path";
import { fileURLToPath } from "node:url";
import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";
import {
  displayWaiverLedgerPath,
  recordBaselineCompileWaiver,
} from "../lib/baseline-waiver-ledger-lib.mjs";
import { REPO_ROOT } from "../lib/runtime-paths.mjs";

registerFailCaptureHook("wp-waiver-record.mjs", { role: "SHARED" });

function fail(message, details = []) {
  failWithMemory("wp-waiver-record.mjs", message, { role: "SHARED", details });
}

function parseArgs(argv = process.argv.slice(2)) {
  const wpId = String(argv.shift() || "").trim();
  if (!wpId || !/^WP-/.test(wpId)) {
    fail("Usage: node .GOV/roles_shared/scripts/wp/wp-waiver-record.mjs WP-{ID} --blocker-command <cmd> --allowed-edit-paths <paths> --operator-authority-ref <ref> [--failing-files <paths>] [--proof-command <cmd>] [--expiry-condition <text>] [--allowed-edit-kind <kind>]");
  }
  const options = {
    wpId,
    blockerCommand: "",
    failingFiles: "",
    allowedEditPaths: "",
    allowedEditKind: "",
    expiryCondition: "",
    operatorAuthorityRef: "",
    proofCommand: "",
    finalOutcome: "",
    status: "ACTIVE",
  };
  while (argv.length > 0) {
    const token = String(argv.shift() || "").trim();
    const value = () => String(argv.shift() || "").trim();
    if (token === "--blocker-command") options.blockerCommand = value();
    else if (token === "--failing-files") options.failingFiles = value();
    else if (token === "--allowed-edit-paths") options.allowedEditPaths = value();
    else if (token === "--allowed-edit-kind") options.allowedEditKind = value();
    else if (token === "--expiry-condition") options.expiryCondition = value();
    else if (token === "--operator-authority-ref") options.operatorAuthorityRef = value();
    else if (token === "--proof-command") options.proofCommand = value();
    else if (token === "--final-outcome") options.finalOutcome = value();
    else if (token === "--status") options.status = value();
    else fail(`Unknown argument: ${token}`);
  }
  return options;
}

export function recordWaiverFromCliOptions(options = {}) {
  return recordBaselineCompileWaiver({
    wpId: options.wpId,
    waiver: {
      blocker_command: options.blockerCommand,
      failing_files: options.failingFiles,
      allowed_edit_paths: options.allowedEditPaths,
      allowed_edit_kind: options.allowedEditKind,
      expiry_condition: options.expiryCondition,
      operator_authority_ref: options.operatorAuthorityRef,
      proof_command: options.proofCommand,
      final_outcome: options.finalOutcome,
      status: options.status,
    },
  });
}

function main() {
  const options = parseArgs();
  const result = recordWaiverFromCliOptions(options);
  console.log(`[WP_WAIVER_RECORD] recorded ${result.entry.waiver_id}`);
  console.log(`[WP_WAIVER_RECORD] ledger=${displayWaiverLedgerPath(REPO_ROOT, options.wpId)}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
