import fs from "node:fs";
import path from "node:path";
import {
  REPO_ROOT,
  governanceRuntimeAbsPath,
} from "../../../../roles_shared/scripts/lib/runtime-paths.mjs";

function parseReadinessField(text = "", label = "") {
  const re = new RegExp(`^\\s*-\\s*${label}\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

export function normalizeWorkflowLane(value = "") {
  return String(value || "").trim().toUpperCase();
}

export function activationReadinessArtifactPath(wpId) {
  return governanceRuntimeAbsPath("roles", "activation_manager", "runtime", "activation_readiness", `${wpId}.md`);
}

export function readActivationReadinessState(wpId) {
  const artifactAbs = activationReadinessArtifactPath(wpId);
  const artifactRel = path.relative(REPO_ROOT, artifactAbs).replace(/\\/g, "/");
  if (!fs.existsSync(artifactAbs)) {
    return {
      exists: false,
      verdict: "<missing>",
      generatedAtUtc: "",
      stateSource: "",
      nextOrchestratorAction: "",
      path: artifactRel,
      readyForDownstreamLaunch: false,
      readyField: "",
      outstandingIssues: "",
      packetStatus: "",
    };
  }

  const report = fs.readFileSync(artifactAbs, "utf8");
  const verdict = parseReadinessField(report, "VERDICT") || "<missing>";
  const generatedAtUtc = parseReadinessField(report, "GENERATED_AT_UTC");
  const stateSource = parseReadinessField(report, "STATE_SOURCE");
  const readyField = parseReadinessField(report, "READY_FOR_DOWNSTREAM_LAUNCH");
  const packetStatus = parseReadinessField(report, "PACKET_STATUS");
  const outstandingIssues = parseReadinessField(report, "OUTSTANDING_ISSUES");
  const nextOrchestratorAction = parseReadinessField(report, "NEXT_ORCHESTRATOR_ACTION");
  const readyForDownstreamLaunch = /^YES$/i.test(readyField)
    || (readyField.length === 0 && verdict === "READY_FOR_ORCHESTRATOR_REVIEW");
  return {
    exists: true,
    verdict,
    generatedAtUtc,
    stateSource,
    nextOrchestratorAction,
    path: artifactRel,
    readyForDownstreamLaunch,
    readyField,
    outstandingIssues,
    packetStatus,
  };
}

export function activationReadinessRequiresActivationManager(wpId) {
  const readiness = readActivationReadinessState(wpId);
  return {
    readiness,
    requiresActivationManager: !readiness.readyForDownstreamLaunch,
  };
}

export function buildActivationManagerLaunchCommands(wpId, readiness = null) {
  const commands = [
    `just activation-manager readiness ${wpId} --write`,
    `just activation-manager next ${wpId}`,
  ];
  if (readiness?.exists) {
    commands.push(`# Current ACTIVATION_READINESS: ${readiness.verdict} (${readiness.path})`);
  }
  if (readiness?.generatedAtUtc) {
    commands.push(`# Readiness generated_at: ${readiness.generatedAtUtc}`);
  }
  if (readiness?.nextOrchestratorAction) {
    commands.push(`# Readiness follow-up: ${readiness.nextOrchestratorAction}`);
  }
  commands.push(`# If refreshed ACTIVATION_READINESS is still not READY_FOR_ORCHESTRATOR_REVIEW: just launch-activation-manager-session ${wpId}`);
  commands.push(`just session-registry-status ${wpId}`);
  commands.push("# Direct readiness refresh is the cheap recovery gate; do not wake Activation Manager, Coder, or validators until truthful ACTIVATION_READINESS is in place.");
  return commands;
}

export function buildManualRelayCommands(wpId) {
  return [
    `just manual-relay-next ${wpId}`,
    `just session-registry-status ${wpId}`,
    "# MANUAL_RELAY is owned by CLASSIC_ORCHESTRATOR; manual-relay-dispatch will start the projected governed session when needed while keeping the Operator in the relay loop.",
  ];
}

export function buildDownstreamGovernedLaunchCommands(wpId) {
  return [
    `just launch-coder-session ${wpId}`,
    `just launch-wp-validator-session ${wpId}`,
    `just session-registry-status ${wpId}`,
    `# Integration Validator stays downstream of WP validation PASS; launch later with: just launch-integration-validator-session ${wpId}`,
  ];
}
