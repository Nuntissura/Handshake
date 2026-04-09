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
      nextOrchestratorAction: "",
      path: artifactRel,
      readyForDownstreamLaunch: false,
    };
  }

  const report = fs.readFileSync(artifactAbs, "utf8");
  const verdict = parseReadinessField(report, "VERDICT") || "<missing>";
  const nextOrchestratorAction = parseReadinessField(report, "NEXT_ORCHESTRATOR_ACTION");
  return {
    exists: true,
    verdict,
    nextOrchestratorAction,
    path: artifactRel,
    readyForDownstreamLaunch: verdict === "READY_FOR_ORCHESTRATOR_REVIEW",
  };
}

export function buildActivationManagerLaunchCommands(wpId, readiness = null) {
  const commands = [
    `just launch-activation-manager-session ${wpId}`,
  ];
  if (readiness?.exists) {
    commands.push(`# Current ACTIVATION_READINESS: ${readiness.verdict} (${readiness.path})`);
  }
  if (readiness?.nextOrchestratorAction) {
    commands.push(`# Readiness follow-up: ${readiness.nextOrchestratorAction}`);
  }
  commands.push(`just session-registry-status ${wpId}`);
  commands.push("# Activation Manager is mandatory for ORCHESTRATOR_MANAGED; do not launch coder or validators until truthful ACTIVATION_READINESS is in place.");
  return commands;
}

export function buildManualRelayCommands(wpId) {
  return [
    `just manual-relay-next ${wpId}`,
    `just session-registry-status ${wpId}`,
    "# manual-relay-dispatch will start the projected governed session when needed; keep the Operator in the relay loop.",
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
