import test from "node:test";
import assert from "node:assert/strict";
import {
  classifyWpChangedPath,
  deriveWpScopeContract,
  hasScopeOverlap,
  isGovernanceOnlyPath,
  matchesScopeEntry,
  normalizeRepoPath,
  parsePacketScopeList,
} from "../scripts/lib/scope-surface-lib.mjs";

test("parsePacketScopeList supports metadata bullets", () => {
  const packet = `
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/locus/types.rs
  - tests/scope_guard.rs
- OUT_OF_SCOPE:
  - app/src
`;

  assert.deepEqual(parsePacketScopeList(packet, "IN_SCOPE_PATHS", { stopLabels: ["OUT_OF_SCOPE"] }), [
    "src/backend/handshake_core/src/locus/types.rs",
    "tests/scope_guard.rs",
  ]);
  assert.deepEqual(parsePacketScopeList(packet, "OUT_OF_SCOPE"), ["app/src"]);
});

test("parsePacketScopeList supports heading style lists", () => {
  const packet = `
### IN_SCOPE_PATHS
- src/backend/handshake_core/src/storage
- app/src/routes

### OUT_OF_SCOPE
- tests
`;

  assert.deepEqual(parsePacketScopeList(packet, "IN_SCOPE_PATHS"), [
    "src/backend/handshake_core/src/storage",
    "app/src/routes",
  ]);
  assert.deepEqual(parsePacketScopeList(packet, "OUT_OF_SCOPE"), ["tests"]);
});

test("matchesScopeEntry handles file and directory scopes", () => {
  assert.equal(matchesScopeEntry("src/backend/handshake_core/src/storage/mod.rs", "src/backend/handshake_core/src/storage"), true);
  assert.equal(matchesScopeEntry("src/backend/handshake_core/src/storage/mod.rs", "src/backend/handshake_core/src/storage/"), true);
  assert.equal(matchesScopeEntry("src/backend/handshake_core/src/storage/mod.rs", "src/backend/handshake_core/src/locus"), false);
});

test("hasScopeOverlap catches conflicting in-scope/out-of-scope entries", () => {
  assert.deepEqual(
    hasScopeOverlap(["src/backend/handshake_core/src/storage"], ["src/backend/handshake_core/src/storage/mod.rs"]),
    {
      left: "src/backend/handshake_core/src/storage",
      right: "src/backend/handshake_core/src/storage/mod.rs",
    },
  );
});

test("classifyWpChangedPath separates in-scope, root governance, and junction drift", () => {
  const packet = `
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage
- OUT_OF_SCOPE:
  - app/src
`;
  const contract = deriveWpScopeContract({
    wpId: "WP-1-Scope-Hardening-v1",
    packetContent: packet,
  });

  assert.deepEqual(classifyWpChangedPath("src/backend/handshake_core/src/storage/mod.rs", contract), {
    path: "src/backend/handshake_core/src/storage/mod.rs",
    kind: "IN_SCOPE",
    allowed: true,
  });
  assert.deepEqual(classifyWpChangedPath("justfile", contract), {
    path: "justfile",
    kind: "ROOT_GOVERNANCE_OUT_OF_SCOPE",
    allowed: false,
  });
  assert.deepEqual(classifyWpChangedPath(".GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md", contract), {
    path: ".GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md",
    kind: "GOVERNANCE_JUNCTION_DRIFT",
    allowed: false,
  });
});

test("classifyWpChangedPath keeps general .GOV paths out of WP scope even if listed", () => {
  const packet = `
- IN_SCOPE_PATHS:
  - .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md
- OUT_OF_SCOPE:
  - NONE
`;
  const contract = deriveWpScopeContract({
    wpId: "WP-1-Scope-Hardening-v1",
    packetContent: packet,
  });

  assert.deepEqual(classifyWpChangedPath(".GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md", contract), {
    path: ".GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md",
    kind: "GOVERNANCE_JUNCTION_DRIFT",
    allowed: false,
  });
});

test("governance helper recognizes root governance-only surfaces", () => {
  assert.equal(isGovernanceOnlyPath("justfile"), true);
  assert.equal(isGovernanceOnlyPath("AGENTS.md"), true);
  assert.equal(isGovernanceOnlyPath(".github/workflows/ci.yml"), true);
  assert.equal(isGovernanceOnlyPath("src/backend/handshake_core/src/lib.rs"), false);
});

test("normalizeRepoPath strips local prefixes and slashes", () => {
  assert.equal(normalizeRepoPath("./src\\backend\\handshake_core\\src\\lib.rs"), "src/backend/handshake_core/src/lib.rs");
});
