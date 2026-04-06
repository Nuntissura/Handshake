#!/usr/bin/env node
/**
 * PreToolUse hook: blocks Write/Edit on product code during validator sessions.
 *
 * Claude Code hook convention:
 *   - CLAUDE_TOOL_NAME  — tool being invoked
 *   - CLAUDE_TOOL_INPUT — JSON string of tool parameters
 *   - exit 0 → allow
 *   - exit 2 → block with feedback (stdout printed to model)
 *
 * RGF-105 — Mechanical Tool-Call Guards for Validator Sessions
 */

const toolName = process.env.CLAUDE_TOOL_NAME || "";

if (toolName !== "Write" && toolName !== "Edit") {
  process.exit(0);
}

let filePath = "";
try {
  const input = JSON.parse(process.env.CLAUDE_TOOL_INPUT || "{}");
  filePath = String(input.file_path || "").replace(/\\/g, "/");
} catch {
  // Malformed input — allow rather than block (fail-open for parse errors)
  process.exit(0);
}

if (!filePath) {
  process.exit(0);
}

const PRODUCT_CODE_PATTERNS = ["/src/", "/app/", "/tests/"];

if (PRODUCT_CODE_PATTERNS.some((p) => filePath.includes(p))) {
  console.log(
    "[VALIDATOR_WRITE_GUARD] BLOCKED: Validators must not edit product code. Send fix instructions to the coder instead."
  );
  process.exit(2);
}

// .GOV paths and everything else: allow
process.exit(0);
