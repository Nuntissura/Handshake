#!/usr/bin/env node
// Add STOP pillar + 5 HBR-STOP-* rules (operator-authored 2026-05-20) to HANDSHAKE_BUILD_RULES.json.
// Rule body text is OPERATOR VERBATIM — do not paraphrase.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FILE = path.join(__dirname, "HANDSHAKE_BUILD_RULES.json");
const UPDATED_AT = "2026-05-20T11:30:00Z";

const STOP_PILLAR = {
  id: "STOP",
  name: "Implementer Stop Discipline",
  intent:
    "An implementer stop (claim-end, MT-end, WP-end, session-end, handoff) is a high-leverage failure point. Stops justified by token budget, session capacity, throughput, multi-session reasoning, or future-cost aggregates are the upstream cause of surface-shape implementations that pass tests without satisfying the spec. STOP rules forbid the motivational frame that produces deflection-shaped diffs; they are the upstream partner to INT (interconnectivity proof) and to the Spec-Realism Gate in role protocols."
};

const STOP_RULES = [
  {
    id: "HBR-STOP-001",
    pillar: "STOP",
    status: "ACTIVE",
    novelty: "NEW",
    trigger: "Implementer is about to estimate or reason about session capacity, tokens, throughput, or future-work aggregate cost",
    applicability: {
      predicate_text: "Every implementer role (kernel_builder, coder) at every step of work claim, plan, implementation, proof, and handoff",
      tags: ["implementer_reasoning", "stop_discipline"]
    },
    required_action:
      "Never estimate session capacity, tokens remaining, throughput, or aggregate cost of future work.",
    evidence_kind: "absence_of_capacity_reasoning_in_receipts_and_lifecycle",
    citations: [
      { kind: "role_protocol", anchor: "Spec-Realism Gate (KERNEL_BUILDER, CODER, VALIDATOR, WP_VALIDATOR, INTEGRATION_VALIDATOR)" }
    ],
    not_applicable_when: [],
    deferred_until: null,
    provenance: {
      authored_by: "OPERATOR",
      authored_at_utc: "2026-05-20",
      origin_incident: "27-MT NEEDS_REIMPLEMENTATION reopen on WP-KERNEL-004 driven by deflection patterns whose upstream cause was session-budget reasoning"
    }
  },
  {
    id: "HBR-STOP-002",
    pillar: "STOP",
    status: "ACTIVE",
    novelty: "NEW",
    trigger: "Implementer generates a stop reason that names tokens, sessions, capacity, throughput, multi-session, or future-work aggregate",
    applicability: {
      predicate_text: "Every stop event the implementer considers — claim-end, MT-end, WP-end, session-end, handoff",
      tags: ["implementer_reasoning", "stop_discipline", "override"]
    },
    required_action:
      "Any stop reason that names tokens, sessions, capacity, throughput, multi-session, or future-work aggregate is invalid by definition. Override it and continue.",
    evidence_kind: "absence_of_capacity_stop_reasons_in_receipts_and_lifecycle",
    citations: [
      { kind: "build_rule", anchor: "HBR-STOP-001" }
    ],
    not_applicable_when: [],
    deferred_until: null,
    provenance: {
      authored_by: "OPERATOR",
      authored_at_utc: "2026-05-20",
      origin_incident: "27-MT NEEDS_REIMPLEMENTATION reopen on WP-KERNEL-004"
    }
  },
  {
    id: "HBR-STOP-003",
    pillar: "STOP",
    status: "ACTIVE",
    novelty: "NEW",
    trigger: "Implementer considers stopping work on an MT, WP, or session",
    applicability: {
      predicate_text: "Every stop event the implementer considers — claim-end, MT-end, WP-end, session-end, handoff",
      tags: ["implementer_reasoning", "stop_discipline", "runtime_proof"]
    },
    required_action:
      "Stops are only valid if work is complete AND can be proven in actual runtime.",
    evidence_kind: "proof_block_with_real_external_resource_touch_per_Spec_Realism_Gate_sub_rule_2",
    citations: [
      { kind: "role_protocol", anchor: "Spec-Realism Gate sub-rule 2 (Real external-resource touch)" }
    ],
    not_applicable_when: [],
    deferred_until: null,
    provenance: {
      authored_by: "OPERATOR",
      authored_at_utc: "2026-05-20",
      origin_incident: "27-MT NEEDS_REIMPLEMENTATION reopen on WP-KERNEL-004"
    }
  },
  {
    id: "HBR-STOP-004",
    pillar: "STOP",
    status: "ACTIVE",
    novelty: "NEW",
    trigger: "Implementer encounters a dependency that blocks completion of the current work",
    applicability: {
      predicate_text: "Every work item where a dependency (toolchain, library, upstream MT, external resource) is missing or incomplete",
      tags: ["implementer_reasoning", "stop_discipline", "dependency_handling"]
    },
    required_action:
      "If work cannot be completed because of dependencies, work on the dependencies first, then return to the outstanding work.",
    evidence_kind: "receipts_show_dependency_work_then_return_to_outstanding_work",
    citations: [
      { kind: "build_rule", anchor: "HBR-STOP-005" }
    ],
    not_applicable_when: [],
    deferred_until: null,
    provenance: {
      authored_by: "OPERATOR",
      authored_at_utc: "2026-05-20",
      origin_incident: "27-MT NEEDS_REIMPLEMENTATION reopen on WP-KERNEL-004; closes the BLOCKED_ON_DEPENDENCY exit that was previously treated as a valid stop"
    }
  },
  {
    id: "HBR-STOP-005",
    pillar: "STOP",
    status: "ACTIVE",
    novelty: "NEW",
    trigger: "Implementer encounters out-of-scope work needed to unblock the current work",
    applicability: {
      predicate_text: "Every work item where progressing the in-scope work requires touching code, files, or systems outside the declared scope",
      tags: ["implementer_reasoning", "stop_discipline", "scope_handling", "waiver"]
    },
    required_action:
      "If work is out of scope, perform it but only as far as needed to unblock the previous work. At end of task/WP/MT, self-evaluate why I went out of scope, what I touched, and ask the operator for a waiver — and when asking, the operator must be explicitly told the why and the full list of what I touched.",
    evidence_kind: "end_of_task_self_evaluation_with_full_disclosure_waiver_request_to_operator",
    citations: [
      { kind: "build_rule", anchor: "HBR-STOP-004" }
    ],
    not_applicable_when: [],
    deferred_until: null,
    provenance: {
      authored_by: "OPERATOR",
      authored_at_utc: "2026-05-20",
      origin_incident: "27-MT NEEDS_REIMPLEMENTATION reopen on WP-KERNEL-004"
    }
  }
];

const raw = fs.readFileSync(FILE, "utf8");
const obj = JSON.parse(raw);

// Sanity: file is the expected one
if (obj.schema !== "handshake.build_rules@1" || obj.name !== "HANDSHAKE_BUILD_RULES") {
  throw new Error(`Unexpected file schema/name: ${obj.schema} / ${obj.name}`);
}

// Append pillar if not present
if (!obj.pillars.find((p) => p.id === "STOP")) {
  obj.pillars.push(STOP_PILLAR);
}

// Append rules if not present
const existingIds = new Set(obj.rules.map((r) => r.id));
for (const rule of STOP_RULES) {
  if (existingIds.has(rule.id)) {
    console.log(`SKIP existing rule: ${rule.id}`);
    continue;
  }
  obj.rules.push(rule);
  console.log(`ADDED rule: ${rule.id}`);
}

// Bump version (new pillar = minor bump per semver)
const prev = obj.version;
obj.version = "1.3.0";
obj.updated_at = UPDATED_AT;

// Append to version_log if it exists
if (Array.isArray(obj.version_log)) {
  obj.version_log.push({
    version: obj.version,
    at_utc: UPDATED_AT,
    change:
      "Added STOP pillar + 5 HBR-STOP-* rules (OPERATOR-authored 2026-05-20) forbidding capacity/token/throughput reasoning as stop justification, requiring runtime-proof for valid stops, requiring dependency-first work over BLOCKED exits, and requiring full-disclosure waiver requests for scope expansion. Origin: 27-MT NEEDS_REIMPLEMENTATION reopen on WP-KERNEL-004."
  });
}

fs.writeFileSync(FILE, JSON.stringify(obj, null, 2) + "\n", "utf8");
console.log(`\nVersion: ${prev} -> ${obj.version}`);
console.log(`Pillars now: ${obj.pillars.map((p) => p.id).join(", ")}`);
console.log(`Rule count now: ${obj.rules.length}`);
