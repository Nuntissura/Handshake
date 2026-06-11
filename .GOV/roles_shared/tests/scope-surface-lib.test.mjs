import assert from "node:assert/strict";
import test from "node:test";
import {
  collectBudgetCountedFiles,
  classifyWpChangedPath,
  deriveWpScopeContract,
  matchesAnyScopeEntry,
  matchesScopeEntry,
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

test("classifyWpChangedPath accepts handshake_main-prefixed packet scope against product-root file paths", () => {
  const scopeContract = deriveWpScopeContract({
    wpId: "WP-TEST-SCOPE-v1",
    packetContent: packetFixture().replace(
      "  - src/backend/feature\n  - src/backend/shared.rs",
      "  - ../handshake_main/src/backend/feature\n  - ../handshake_main/src/backend/shared.rs",
    ),
  });

  assert.deepEqual(
    classifyWpChangedPath("src/backend/feature/a.rs", scopeContract),
    { path: "src/backend/feature/a.rs", kind: "IN_SCOPE", allowed: true },
  );
  assert.deepEqual(
    classifyWpChangedPath("src/backend/shared.rs", scopeContract),
    { path: "src/backend/shared.rs", kind: "IN_SCOPE", allowed: true },
  );
});

test("scopeDisciplineRequiresEnforcement gates only new packet versions", () => {
  assert.equal(scopeDisciplineRequiresEnforcement("2026-03-22"), false);
  assert.equal(scopeDisciplineRequiresEnforcement("2026-03-23"), true);
});

test("matchesScopeEntry strips trailing /** glob from scope entries", () => {
  assert.equal(
    matchesScopeEntry(
      "app/src/components/inference_lab/InferenceLab.tsx",
      "app/src/components/inference_lab/**",
    ),
    true,
  );
  assert.equal(
    matchesScopeEntry(
      "app/src/components/inference_lab/sub/Nested.tsx",
      "app/src/components/inference_lab/**",
    ),
    true,
  );
  assert.equal(
    matchesScopeEntry(
      "app/src/components/other/File.tsx",
      "app/src/components/inference_lab/**",
    ),
    false,
  );
});

test("matchesScopeEntry strips trailing /**/* and /* glob suffixes", () => {
  assert.equal(
    matchesScopeEntry(
      "src/backend/handshake_core/tests/lora.rs",
      "src/backend/handshake_core/tests/**/*",
    ),
    true,
  );
  assert.equal(
    matchesScopeEntry(
      "src/backend/handshake_core/tests/lora.rs",
      "src/backend/handshake_core/tests/*",
    ),
    true,
  );
});

test("matchesScopeEntry treats unstripped scope as a plain prefix", () => {
  assert.equal(
    matchesScopeEntry(
      "src/backend/feature/a.rs",
      "src/backend/feature",
    ),
    true,
  );
  assert.equal(
    matchesScopeEntry(
      "src/backend/feature_other/a.rs",
      "src/backend/feature",
    ),
    false,
  );
});

test("matchesAnyScopeEntry honours the /** glob across multiple entries", () => {
  const scope = [
    "app/src/components/inference_lab/**",
    "app/src/lib/ipc/lora.ts",
  ];
  assert.equal(
    matchesAnyScopeEntry(
      "app/src/components/inference_lab/LoraStackComposer.tsx",
      scope,
    ),
    true,
  );
  assert.equal(matchesAnyScopeEntry("app/src/lib/ipc/lora.ts", scope), true);
  assert.equal(matchesAnyScopeEntry("app/src/lib/ipc/other.ts", scope), false);
});

test("matchesScopeEntry refuses bare \"**\" / \"*\" wildcards to keep misconfig visible", () => {
  // Bare ** would otherwise match every file; we treat this as a packet
  // misconfiguration rather than silently accepting all changes.
  assert.equal(matchesScopeEntry("any/file.rs", "**"), false);
  assert.equal(matchesScopeEntry("any/file.rs", "*"), false);
});
