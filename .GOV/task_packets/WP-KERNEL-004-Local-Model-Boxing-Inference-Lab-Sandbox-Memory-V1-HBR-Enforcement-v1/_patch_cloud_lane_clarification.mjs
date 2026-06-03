#!/usr/bin/env node
// One-shot patch: inject operator's 2026-05-20 cloud-lane architecture clarification
// into the lifecycle of every distillation + cloud-lane MT in the 27 reopened.
// Operator clarification: distillation defaults to cloud-teacher -> local-student;
// BYOK is architecturally required but operationally dormant; CLI bridge is the
// load-bearing operator-production cloud transport.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const CAPTURED_AT_UTC = "2026-05-20T10:30:00Z";

const DISTILLATION_MTS = ["MT-119", "MT-120", "MT-121", "MT-122", "MT-123", "MT-124"];
const CLOUD_LANE_MTS = ["MT-125", "MT-126", "MT-127", "MT-128", "MT-129"];

const DISTILLATION_CLARIFICATION = {
  captured_at_utc: CAPTURED_AT_UTC,
  captured_by: "OPERATOR",
  topic: "distillation_cloud_teacher_default",
  summary:
    "Distillation pipeline defaults to CLOUD-TEACHER -> LOCAL-STUDENT. Larger-local-teacher -> smaller-local-student is SUPPORTED but NOT the default. Teacher source is therefore typically a frontier cloud model (Claude / GPT / Gemini class); local-larger-teacher (e.g. Llama-70B -> Llama-7B) is the alternate non-default path.",
  prior_assumption_corrected:
    "Pre-clarification working assumption was that distillation was local-teacher-only (Llama-70B teaching Llama-7B). That was wrong. Cloud-teacher is the default.",
  implications: [
    "Teacher ModelId in the PEFT pipeline (MT-122) MUST accept both cloud and local sources; cloud is the default path the operator actually uses.",
    "Cloud-teacher invocation routes through MT-127 CLI bridge for the operator's production setup (subscription plans, no BYOK keys). See cloud-lane clarification on MT-127.",
    "License tagging (MT-120) MUST capture cloud-teacher provenance per vendor (OpenAI ToS for GPT teacher, Anthropic Acceptable Use for Claude teacher, Google ToS for Gemini teacher). Local-larger-teacher provenance is simpler but still must be tagged.",
    "Corpus extractor (MT-119) MUST support cloud-teacher invocation flow + capture, not only local-model session replay.",
    "DISTILL_CORPUS opt-in semantics (MT-121) apply uniformly across teacher source; consent gating happens once per session regardless of teacher being cloud or local.",
    "Wiremock is acceptable for MT-122 PEFT pipeline unit / integration tests (proves PEFT invocation + LoRA artifact shape). The FEATURE itself is operationally inert until a real teacher (CLI bridge or BYOK) is reachable at runtime."
  ]
};

const CLOUD_LANE_CLARIFICATION = {
  captured_at_utc: CAPTURED_AT_UTC,
  captured_by: "OPERATOR",
  topic: "cloud_lane_byok_dormant_cli_bridge_primary",
  summary:
    "Operator's production cloud-model transport is MT-127 CLI bridge (Claude Code / Codex CLI / gemini-cli) routed through operator's subscription plans. BYOK adapters (MT-125 OpenAI, MT-126 Anthropic, MT-128 Google) are ARCHITECTURALLY REQUIRED but OPERATIONALLY DORMANT for operator: build honestly, test against wiremock, ship implemented-but-unused. Operator has no BYOK accounts with credit for any of the three vendors.",
  prior_assumption_corrected:
    "Pre-clarification working assumption was that BYOK MTs needed real vendor API keys to prove correctness. That was wrong on two counts: (1) the operator has no credits and does not use BYOK, (2) the Spec-Realism Gate's 'real external resource touch' is satisfied by wiremock (real TCP port + real reqwest HTTP call), not specifically by a paid vendor endpoint.",
  implications: [
    "Implementer MUST NOT request operator API keys at any point. Asking for keys was the prior session's deflection-shaped failure mode and is explicitly out of scope for closing these MTs.",
    "Wiremock is the live test surface for BYOK MTs. It binds a real TCP port answering the documented vendor protocol shape (OpenAI Chat Completions / Responses, Anthropic Messages, Google Gemini generateContent). That satisfies Spec-Realism Gate sub-rule 2.",
    "MT-127 CLI bridge is LOAD-BEARING for operator production: it is the cloud-teacher transport for distillation (see distillation clarification on MT-119/120/122) and the general cloud invocation transport. Its proof SHOULD spawn the actually-installed CLI binary on the host (claude.exe / codex.exe / gemini.exe), not just a cmd /c echo placeholder. Use `Get-Command claude` / `Get-Command codex` / `Get-Command gemini` to resolve the binary; if not installed, document the install path the operator should take.",
    "MT-128 Operator Secrets Vault must be fully implemented (encrypt-at-rest via OS keychain, per-call retrieval, never-log invariants, consent gate). The vault is the storage substrate for the dormant BYOK lanes; functionality must be complete even though the operator never populates it.",
    "MT-129 Cloud lane UI MUST surface both lane types: CLI bridge as primary (with binary-present status), BYOK as available-but-empty (with 'add key to activate' affordance). The empty BYOK state is the correct ship state for this operator, not a defect to flag.",
    "BYOK MTs (125, 126, 128 for vault) close on wiremock proof + honest ModelRuntime trait impl. No LiveClientUnavailable scaffolding. No deferred-to-follow-on. Real reqwest -> real socket -> wiremock answering protocol shape -> real audit row to Postgres -> real cancellation observable wiremock-side."
  ]
};

const TARGETS = [
  ...DISTILLATION_MTS.map((id) => ({ id, clarification: DISTILLATION_CLARIFICATION })),
  ...CLOUD_LANE_MTS.map((id) => ({ id, clarification: CLOUD_LANE_CLARIFICATION }))
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
  obj.lifecycle.operator_clarification_20260520 = clarification;
  obj.updated_at_utc = CAPTURED_AT_UTC;
  fs.writeFileSync(filePath, JSON.stringify(obj, null, 2) + "\n", "utf8");
  console.log(`PATCHED: ${id} (topic=${clarification.topic})`);
  patched += 1;
}

console.log(`\nTotal patched: ${patched} / ${TARGETS.length}`);
