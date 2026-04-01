import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const JUSTFILE_PATH = path.resolve("justfile");

function recipeExists(text, recipeName) {
  const escaped = recipeName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`^${escaped}(?:\\s|:|$)`, "m").test(text);
}

test("justfile exposes the orchestrator ACP/session control surface", () => {
  const justfile = fs.readFileSync(JUSTFILE_PATH, "utf8");
  const requiredRecipes = [
    "ensure-wp-communications",
    "launch-coder-session",
    "launch-wp-validator-session",
    "launch-integration-validator-session",
    "orchestrator-steer-next",
    "start-coder-session",
    "start-wp-validator-session",
    "start-integration-validator-session",
    "steer-coder-session",
    "steer-wp-validator-session",
    "steer-integration-validator-session",
    "cancel-coder-session",
    "cancel-wp-validator-session",
    "cancel-integration-validator-session",
    "close-coder-session",
    "close-wp-validator-session",
    "close-integration-validator-session",
    "session-start",
    "session-send",
    "session-cancel",
    "session-close",
    "handshake-acp-broker-status",
    "handshake-acp-broker-stop",
    "wp-validator-query",
    "wp-review-request",
    "wp-validator-response",
    "wp-review-response",
    "operator-viewport",
    "operator-viewport-admin",
    "operator-monitor",
    "operator-admin",
  ];

  for (const recipeName of requiredRecipes) {
    assert.equal(recipeExists(justfile, recipeName), true, `Missing just recipe: ${recipeName}`);
  }
});
