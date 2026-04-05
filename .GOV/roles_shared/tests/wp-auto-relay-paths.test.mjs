import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  orchestratorSteerScriptPath as receiptRelayPath,
  REVIEW_NOTIFICATION_APPEND_OPTIONS,
} from "../scripts/wp/wp-receipt-append.mjs";
import { orchestratorSteerScriptPath as notificationRelayPath } from "../scripts/wp/wp-notification-append.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../../..");
const orchestratorSteerPath = path.resolve(
  repoRoot,
  ".GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs",
);

test("receipt and notification auto-relay resolve the live orchestrator steer helper", () => {
  assert.equal(path.normalize(receiptRelayPath()), path.normalize(orchestratorSteerPath));
  assert.equal(path.normalize(notificationRelayPath()), path.normalize(orchestratorSteerPath));
  assert.equal(fs.existsSync(orchestratorSteerPath), true);
});

test("review-derived notifications suppress their own auto-relay because the parent receipt already relays", () => {
  assert.equal(REVIEW_NOTIFICATION_APPEND_OPTIONS.assumeTransactionLock, true);
  assert.equal(REVIEW_NOTIFICATION_APPEND_OPTIONS.autoRelay, false);
});
