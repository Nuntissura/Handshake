#!/usr/bin/env node
// Phase 2: patch the 2 PENDING (not yet reopened/completed) MTs that are
// semantically downstream of the operator's 2026-05-20 cloud-lane clarification.
// MT-130 (cloud lane parity integration test) + MT-166 (session-spawn distillation).

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const CAPTURED_AT_UTC = "2026-05-20T10:45:00Z";

const TARGETS = [
  {
    id: "MT-130",
    clarification: {
      captured_at_utc: CAPTURED_AT_UTC,
      captured_by: "OPERATOR",
      topic: "cloud_lane_byok_dormant_cli_bridge_primary_parity_test_scope",
      summary:
        "This is the cross-lane parity integration test. Under operator's 2026-05-20 clarification: (a) BYOK adapters (MT-125/126/128) are architecturally required but operationally dormant for operator — test them against wiremock, not live vendors; (b) MT-127 CLI bridge is the load-bearing operator-production cloud transport — test it by spawning the actually-installed CLI binary (claude.exe / codex.exe / gemini.exe), not just cmd /c echo; (c) parity is uniformity of ModelRuntime trait surface + FR event shape + cancellation behavior + audit row shape — NOT uniformity of vendor-endpoint reachability.",
      prior_assumption_corrected:
        "Pre-clarification working assumption was that parity test would require live vendor endpoints to be meaningful. That was wrong. Parity is a trait-conformance + observability-shape property; wiremock proves it. Live vendor reachability is a deployment-config concern, not a parity concern.",
      implications: [
        "Parity test MUST run against: real local model file (LlamaCpp/Candle adapter loading a file from D:/Local Models) + wiremock for OpenAI BYOK + wiremock for Anthropic BYOK + wiremock for Google BYOK + real-spawned actual CLI binary for MT-127 OfficialCli bridge.",
        "Implementer MUST NOT require operator API keys at any point. If wiremock cannot be configured for a vendor's protocol, fix the wiremock setup; do not request keys.",
        "If actually-installed CLI binary for MT-127 is missing on the host (Get-Command claude / codex / gemini returns nothing), the parity test for that lane is BLOCKED_ON_DEPENDENCY with the named missing binary as blocker; do NOT fall back to cmd /c echo for the production parity assertion.",
        "Parity properties to assert uniformly: (i) ModelRuntime trait surface compiles + dispatches; (ii) FR-EVT-LLM-INFER-START / TOKEN / END shape identical across lanes; (iii) cancellation propagates and is observable; (iv) audit row shape correct (cloud_invocations for cloud lanes, kernel_process_lifecycle for local + CLI bridge subprocess); (v) capabilities struct returns correct per-lane truths (no false advertising on CLI bridge).",
        "BYOK-vendor reachability is explicitly NOT a parity property tested here. A separate smoke test outside this WP can probe live vendor reachability if and when an operator with credits configures their keys; that is a deployment concern, not a WP-KERNEL-004 concern."
      ]
    }
  },
  {
    id: "MT-166",
    clarification: {
      captured_at_utc: CAPTURED_AT_UTC,
      captured_by: "OPERATOR",
      topic: "distillation_secondary_flow_intra_session",
      summary:
        "This is a SECONDARY distillation flow (intra-session parent-child spawn-tree pairs as teacher-student data), parallel to the PRIMARY distillation flow built by MT-119/120/121/122/123/124. Under operator's 2026-05-20 clarification, the primary flow defaults to CLOUD-TEACHER -> LOCAL-STUDENT (frontier vendor model teaching a local student). This MT-166 flow is the local-larger-teacher modality the operator described as 'supported but not default': parent role (acting as larger-context teacher) -> specialized child role (student). Both flows coexist; this one does not replace cloud-teacher distillation.",
      prior_assumption_corrected:
        "There was no prior incorrect assumption attached to this MT specifically; the clarification is forward-context so the implementer understands which distillation modality this MT belongs to and how it relates to the cloud-teacher-primary flow.",
      implications: [
        "This flow is local-to-local by nature (parent and child are both Handshake-internal sessions); no cloud teacher transport needed for THIS flow. Cloud-teacher distillation is the responsibility of MT-119/120/122 corpus extraction + PEFT pipeline, not this MT.",
        "DISTILL_CORPUS opt-in semantics (MT-121) apply identically: spawn-tree pairs only enter the corpus when the session is flagged DISTILL_CORPUS=true at close. Default-deny per AC-DISTILL-OPT-IN.",
        "Output of this MT feeds the SAME PEFT pipeline (MT-122) as cloud-teacher corpus; the pipeline must accept both teacher-source modalities transparently. Teacher ModelId in the training run is the parent-role ModelId for this flow, the cloud-vendor ModelId for the cloud-teacher flow.",
        "License/provenance tagging (MT-120 pipeline reused) is simpler for this flow (no third-party vendor ToS to honor), but the spawn-tree provenance MUST still be captured: which parent role + which child role + which task spawned the pair. Operator-controlled internal data, but provenance is non-optional.",
        "Pass-outcome filtering (only successful parent-child pairs become candidates) is unique to this flow and does NOT apply to cloud-teacher distillation — keep the filter local to this MT, not in the shared MT-120 pipeline."
      ]
    }
  }
];

let patched = 0;
for (const { id, clarification } of TARGETS) {
  const filePath = path.join(__dirname, `${id}.json`);
  if (!fs.existsSync(filePath)) {
    console.error(`MISSING: ${filePath}`);
    continue;
  }
  const raw = fs.readFileSync(filePath, "utf8");
  const obj = JSON.parse(raw);
  if (!obj.lifecycle) {
    console.error(`NO lifecycle field: ${id}`);
    continue;
  }
  const currentStatus = obj.lifecycle.status;
  if (currentStatus === "COMPLETED") {
    console.error(`SKIP ${id}: status=COMPLETED — refusing to touch done MT`);
    continue;
  }
  obj.lifecycle.operator_clarification_20260520 = clarification;
  obj.updated_at_utc = CAPTURED_AT_UTC;
  fs.writeFileSync(filePath, JSON.stringify(obj, null, 2) + "\n", "utf8");
  console.log(`PATCHED: ${id} (status=${currentStatus}, topic=${clarification.topic})`);
  patched += 1;
}

console.log(`\nTotal patched: ${patched} / ${TARGETS.length}`);
