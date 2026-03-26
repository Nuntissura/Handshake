#!/usr/bin/env node

import { GOV_ROOT_REPO_REL } from "../lib/runtime-paths.mjs";
import { OPERATOR_RULE_RESTATEMENT_INVALIDITY_CODE } from "../lib/wp-communications-lib.mjs";
import { appendWpReceipt } from "./wp-receipt-append.mjs";

function usage() {
  console.error(
    `Usage: node ${GOV_ROOT_REPO_REL}/roles_shared/scripts/wp/wp-operator-rule-restatement.mjs`
    + " WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> \"<SUMMARY>\""
    + " [SPEC_ANCHOR] [PACKET_ROW_REF]",
  );
  process.exit(1);
}

const [wpId, actorRole, actorSession, summary, specAnchor, packetRowRef] = process.argv.slice(2);
if (!wpId || !actorRole || !actorSession || !summary) {
  usage();
}

const { context, entry } = appendWpReceipt({
  wpId,
  actorRole,
  actorSession,
  receiptKind: "WORKFLOW_INVALIDITY",
  summary,
  stateAfter: "WORKFLOW_INVALID",
  targetRole: "ORCHESTRATOR",
  targetSession: null,
  requiresAck: false,
  specAnchor,
  packetRowRef,
  workflowInvalidityCode: OPERATOR_RULE_RESTATEMENT_INVALIDITY_CODE,
});

console.log(`[WP_INVALIDITY] appended ${entry.workflow_invalidity_code} for ${entry.wp_id}`);
console.log(`- ledger: ${context.receiptsFile}`);
console.log(`- timestamp_utc: ${entry.timestamp_utc}`);
