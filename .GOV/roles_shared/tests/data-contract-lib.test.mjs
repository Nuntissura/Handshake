import assert from "node:assert/strict";
import test from "node:test";

import {
  deriveDataContractDecisionFromRefinement,
  deriveDataContractProfileFromRefinement,
  formatDataContractDecisionSection,
  formatDataContractMonitoringSection,
  packetUsesDataContractProfile,
  validateDataContractDecisionSection,
  validateDataContractSection,
} from "../scripts/lib/data-contract-lib.mjs";

test("packetUsesDataContractProfile enables the contract on 2026-04-01+ packets", () => {
  assert.equal(packetUsesDataContractProfile("2026-03-29"), false);
  assert.equal(packetUsesDataContractProfile("2026-04-01"), true);
});

test("deriveDataContractProfileFromRefinement detects SQL/LLM/Loom data posture signals", () => {
  const profile = deriveDataContractProfileFromRefinement({
    refinementData: {
      pillarsTouched: ["PILLAR: LLM-friendly data | STATUS: TOUCHED"],
    },
  });
  assert.equal(profile, "LLM_FIRST_DATA_V1");
});

test("deriveDataContractDecisionFromRefinement activates on concrete backend data scope", () => {
  const decision = deriveDataContractDecisionFromRefinement({
    inScopePaths: ["src/backend/handshake_core/src/locus/types.rs"],
  });
  assert.equal(decision.profile, "LLM_FIRST_DATA_V1");
  assert.equal(decision.decision, "ACTIVE_REQUIRED");
  assert.match(decision.evidence.join("\n"), /locus\/types\.rs/i);
});

test("validateDataContractSection accepts a fully declared active data contract", () => {
  const packet = [
    "- PACKET_FORMAT_VERSION: 2026-04-01",
    "- DATA_CONTRACT_PROFILE: LLM_FIRST_DATA_V1",
    "",
    formatDataContractMonitoringSection({
      profile: "LLM_FIRST_DATA_V1",
      inScopePaths: ["src/backend/demo.rs", "src/backend/model.rs"],
    }).trim(),
  ].join("\n");

  const validation = validateDataContractSection(packet, { packetPath: "packet.md" });
  assert.deepEqual(validation.errors, []);
});

test("validateDataContractSection rejects missing concrete surfaces for active data packets", () => {
  const packet = [
    "- PACKET_FORMAT_VERSION: 2026-04-01",
    "- DATA_CONTRACT_PROFILE: LLM_FIRST_DATA_V1",
    "",
    "## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)",
    "- DATA_CONTRACT_ACTIVE: YES",
    "- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY",
    "- LLM_READABILITY_POSTURE: REQUIRED",
    "- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE",
    "- PRIMARY_DATA_SURFACES:",
    "  - NONE",
    "- DATA_CONTRACT_RULES:",
    "  - Keep structure explicit.",
    "- VALIDATOR_DATA_PROOF_HINTS:",
    "  - Prove the emitted shape.",
  ].join("\n");

  const validation = validateDataContractSection(packet, { packetPath: "packet.md" });
  assert.match(validation.errors.join("\n"), /PRIMARY_DATA_SURFACES/);
});

test("validateDataContractDecisionSection rejects waived decision when scope is concretely data-bearing", () => {
  const packet = [
    "- PACKET_FORMAT_VERSION: 2026-04-01",
    "- DATA_CONTRACT_PROFILE: NONE",
    "",
    formatDataContractDecisionSection({
      decision: "WAIVED_NOT_DATA_BEARING",
      reason: "No governed data surface was believed to be in scope.",
      evidence: ["IN_SCOPE_PATHS reviewed: src/backend/handshake_core/src/locus/types.rs"],
    }).trim(),
  ].join("\n");

  const validation = validateDataContractDecisionSection(packet, {
    packetPath: "packet.md",
    inScopePaths: ["src/backend/handshake_core/src/locus/types.rs"],
  });
  assert.match(validation.errors.join("\n"), /conflicts with data-bearing IN_SCOPE_PATHS/i);
});
