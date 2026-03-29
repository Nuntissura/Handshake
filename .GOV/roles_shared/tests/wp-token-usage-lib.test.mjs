import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import assert from "node:assert/strict";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const tokenLibHref = pathToFileURL(path.resolve(__dirname, "../scripts/session/wp-token-usage-lib.mjs")).href;

async function withTempRepo(fn) {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "wp-token-usage-"));
  try {
    return await fn(repoRoot);
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  }
}

async function loadTokenLib(governanceRuntimeRoot) {
  return import(`${tokenLibHref}?runtime_root=${encodeURIComponent(governanceRuntimeRoot)}&t=${Date.now()}`);
}

async function withRuntimeRoot(governanceRuntimeRoot, fn) {
  const previous = process.env.HANDSHAKE_GOV_RUNTIME_ROOT;
  process.env.HANDSHAKE_GOV_RUNTIME_ROOT = governanceRuntimeRoot;
  try {
    return await fn();
  } finally {
    if (previous == null) delete process.env.HANDSHAKE_GOV_RUNTIME_ROOT;
    else process.env.HANDSHAKE_GOV_RUNTIME_ROOT = previous;
  }
}

test("parseUsageFromOutputJsonl aggregates all turn.completed usage entries", async () => withTempRepo(async (repoRoot) => {
  const { parseUsageFromOutputJsonl } = await loadTokenLib(path.join(repoRoot, "gov_runtime"));
  const outputFile = path.join(repoRoot, "gov_runtime", "roles_shared", "SESSION_CONTROL_OUTPUTS", "CODER_WP-TEST", "cmd-1.jsonl");
  fs.mkdirSync(path.dirname(outputFile), { recursive: true });
  fs.writeFileSync(outputFile, [
    JSON.stringify({ timestamp: "2026-03-29T10:00:00Z", type: "thread.started", thread_id: "thread-1" }),
    JSON.stringify({ timestamp: "2026-03-29T10:00:01Z", type: "turn.completed", usage: { input_tokens: 100, cached_input_tokens: 25, output_tokens: 10 } }),
    JSON.stringify({ timestamp: "2026-03-29T10:00:02Z", type: "turn.completed", usage: { input_tokens: 50, cached_input_tokens: 5, output_tokens: 8 } }),
  ].join("\n"), "utf8");

  const usage = parseUsageFromOutputJsonl(outputFile);

  assert.equal(usage.threadId, "thread-1");
  assert.equal(usage.turnCount, 2);
  assert.deepEqual(usage.usageTotals, {
    input_tokens: 150,
    cached_input_tokens: 30,
    output_tokens: 18,
  });
}));

test("syncWpTokenUsageLedger rolls up commands by WP and role", async () => withTempRepo(async (repoRoot) => {
  const { syncWpTokenUsageLedger } = await loadTokenLib(path.join(repoRoot, "gov_runtime"));
  const coderOutput = path.join(repoRoot, "gov_runtime", "roles_shared", "SESSION_CONTROL_OUTPUTS", "CODER_WP-TEST-v1", "cmd-coder.jsonl");
  const validatorOutput = path.join(repoRoot, "gov_runtime", "roles_shared", "SESSION_CONTROL_OUTPUTS", "WP_VALIDATOR_WP-TEST-v1", "cmd-wpval.jsonl");
  fs.mkdirSync(path.dirname(coderOutput), { recursive: true });
  fs.writeFileSync(coderOutput, `${JSON.stringify({ timestamp: "2026-03-29T10:00:01Z", type: "turn.completed", usage: { input_tokens: 100, cached_input_tokens: 40, output_tokens: 12 } })}\n`, "utf8");

  const first = await withRuntimeRoot(path.join(repoRoot, "gov_runtime"), () => syncWpTokenUsageLedger(repoRoot, {
    command_id: "cmd-coder",
    command_kind: "START_SESSION",
    session_key: "CODER:WP-TEST-v1",
    wp_id: "WP-TEST-v1",
    role: "CODER",
    status: "COMPLETED",
    processed_at: "2026-03-29T10:00:05Z",
    output_jsonl_file: path.relative(repoRoot, coderOutput),
  }, {
    session: {
      requested_model: "gpt-5.4",
      reasoning_config_value: "xhigh",
      session_thread_id: "thread-coder",
    },
  }));
  const firstCommandCount = first.ledger.summary.command_count;
  fs.mkdirSync(path.dirname(validatorOutput), { recursive: true });
  fs.writeFileSync(validatorOutput, `${JSON.stringify({ timestamp: "2026-03-29T10:05:01Z", type: "turn.completed", usage: { input_tokens: 80, cached_input_tokens: 10, output_tokens: 9 } })}\n`, "utf8");
  const second = await withRuntimeRoot(path.join(repoRoot, "gov_runtime"), () => syncWpTokenUsageLedger(repoRoot, {
    command_id: "cmd-wpval",
    command_kind: "SEND_PROMPT",
    session_key: "WP_VALIDATOR:WP-TEST-v1",
    wp_id: "WP-TEST-v1",
    role: "WP_VALIDATOR",
    status: "COMPLETED",
    processed_at: "2026-03-29T10:05:05Z",
    output_jsonl_file: path.relative(repoRoot, validatorOutput),
  }, {
    session: {
      requested_model: "gpt-5.4",
      reasoning_config_value: "xhigh",
      session_thread_id: "thread-wpval",
    },
  }));

  assert.equal(firstCommandCount, 1);
  assert.equal(second.ledger.summary.command_count, 2);
  assert.deepEqual(second.ledger.summary.usage_totals, {
    input_tokens: 180,
    cached_input_tokens: 50,
    output_tokens: 21,
  });
  assert.equal(second.ledger.summary_source, "RAW_OUTPUT_SCAN");
  assert.equal(second.ledger.ledger_health.status, "MATCH");
  assert.equal(second.ledger.role_totals.CODER.usage_totals.input_tokens, 100);
  assert.equal(second.ledger.role_totals.WP_VALIDATOR.usage_totals.input_tokens, 80);
}));

test("readWpTokenUsageLedger detects raw-output drift beyond the tracked command ledger", async () => withTempRepo(async (repoRoot) => {
  const { syncWpTokenUsageLedger, readWpTokenUsageLedger } = await loadTokenLib(path.join(repoRoot, "gov_runtime"));
  const coderOutput = path.join(repoRoot, "gov_runtime", "roles_shared", "SESSION_CONTROL_OUTPUTS", "CODER_WP-TEST-v1", "cmd-coder.jsonl");
  const validatorOutput = path.join(repoRoot, "gov_runtime", "roles_shared", "SESSION_CONTROL_OUTPUTS", "WP_VALIDATOR_WP-TEST-v1", "cmd-wpval.jsonl");
  fs.mkdirSync(path.dirname(coderOutput), { recursive: true });
  fs.mkdirSync(path.dirname(validatorOutput), { recursive: true });
  fs.writeFileSync(coderOutput, `${JSON.stringify({ timestamp: "2026-03-29T10:00:01Z", type: "turn.completed", usage: { input_tokens: 100, cached_input_tokens: 40, output_tokens: 12 } })}\n`, "utf8");
  fs.writeFileSync(validatorOutput, `${JSON.stringify({ timestamp: "2026-03-29T10:05:01Z", type: "turn.completed", usage: { input_tokens: 80, cached_input_tokens: 10, output_tokens: 9 } })}\n`, "utf8");

  await withRuntimeRoot(path.join(repoRoot, "gov_runtime"), () => syncWpTokenUsageLedger(repoRoot, {
    command_id: "cmd-coder",
    command_kind: "START_SESSION",
    session_key: "CODER:WP-TEST-v1",
    wp_id: "WP-TEST-v1",
    role: "CODER",
    status: "COMPLETED",
    processed_at: "2026-03-29T10:00:05Z",
    output_jsonl_file: path.relative(repoRoot, coderOutput),
  }, {
    session: {
      requested_model: "gpt-5.4",
      reasoning_config_value: "xhigh",
      session_thread_id: "thread-coder",
    },
  }));

  const { ledger } = await withRuntimeRoot(path.join(repoRoot, "gov_runtime"), () => readWpTokenUsageLedger(repoRoot, "WP-TEST-v1"));

  assert.equal(ledger.summary_source, "RAW_OUTPUT_SCAN");
  assert.equal(ledger.summary.command_count, 2);
  assert.deepEqual(ledger.summary.usage_totals, {
    input_tokens: 180,
    cached_input_tokens: 50,
    output_tokens: 21,
  });
  assert.equal(ledger.tracked_summary.command_count, 1);
  assert.equal(ledger.raw_scan.summary.command_count, 2);
  assert.equal(ledger.ledger_health.status, "DRIFT");
  assert.equal(ledger.ledger_health.missing_tracked_command_count, 1);
  assert.deepEqual(ledger.ledger_health.missing_tracked_command_ids_sample, ["cmd-wpval"]);
}));
