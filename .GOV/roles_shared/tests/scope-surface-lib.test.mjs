import assert from "node:assert/strict";
import test from "node:test";
import {
  collectBudgetCountedFiles,
  deriveWpScopeContract,
  parsePacketScopeDiscipline,
  scopeDisciplineRequiresEnforcement,
} from "../scripts/lib/scope-surface-lib.mjs";

function packetFixture(scopeBlock = "") {
  return `# Task Packet: WP-TEST-SCOPE-v1

## METADATA
- WP_ID: WP-TEST-SCOPE-v1
- PACKET_FORMAT_VERSION: 2026-03-23

## SCOPE
- IN_SCOPE_PATHS:
  - src/backend/feature
  - src/backend/shared.rs
- OUT_OF_SCOPE:
  - tests/
- TOUCHED_FILE_BUDGET: 3
- BROAD_TOOL_ALLOWLIST: FORMATTER, SEARCH_REPLACE

${scopeBlock}`.trim();
}

test("parsePacketScopeDiscipline parses file budget and broad tool allowlist", () => {
  const parsed = parsePacketScopeDiscipline(packetFixture());

  assert.equal(parsed.touchedFileBudget, 3);
  assert.equal(parsed.touchedFileBudgetValid, true);
  assert.deepEqual(parsed.broadToolAllowlist, ["FORMATTER", "SEARCH_REPLACE"]);
  assert.equal(parsed.broadToolAllowlistValid, true);
});

test("parsePacketScopeDiscipline rejects NONE mixed with other allowlist tokens", () => {
  const parsed = parsePacketScopeDiscipline(packetFixture().replace("FORMATTER, SEARCH_REPLACE", "NONE, FORMATTER"));

  assert.equal(parsed.broadToolAllowlistValid, false);
  assert.deepEqual(parsed.invalidBroadToolTokens, ["NONE_WITH_OTHERS"]);
});

test("collectBudgetCountedFiles counts only in-scope implementation files", () => {
  const scopeContract = deriveWpScopeContract({
    wpId: "WP-TEST-SCOPE-v1",
    packetContent: packetFixture(),
  });

  const counted = collectBudgetCountedFiles([
    "src/backend/feature/a.rs",
    "src/backend/feature/b.rs",
    "src/backend/shared.rs",
    "tests/feature_tests.rs",
    ".GOV/task_packets/WP-TEST-SCOPE-v1/packet.md",
    "justfile",
  ], scopeContract);

  assert.deepEqual(counted, [
    "src/backend/feature/a.rs",
    "src/backend/feature/b.rs",
    "src/backend/shared.rs",
  ]);
});

test("scopeDisciplineRequiresEnforcement gates only new packet versions", () => {
  assert.equal(scopeDisciplineRequiresEnforcement("2026-03-22"), false);
  assert.equal(scopeDisciplineRequiresEnforcement("2026-03-23"), true);
});
