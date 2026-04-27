import { GOV_ROOT_REPO_REL, repoPathAbs } from "../scripts/lib/runtime-paths.mjs";

export const PHASE_VALUES = ["STARTUP", "HANDOFF", "VERDICT", "CLOSEOUT"];
const PHASE_VALUE_SET = new Set(PHASE_VALUES);
const STARTUP_ROLE_VALUES = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);
const HANDOFF_ROLE_VALUES = new Set(["CODER", "WP_VALIDATOR"]);
const VERDICT_ROLE_VALUES = new Set(["WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);
const STARTUP_INTERNAL_FLAG_SET = new Set(["--committed-handoff-preflight"]);

function normalizePhase(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function formatCommandToken(value) {
  const normalized = String(value || "").trim();
  if (!normalized) return "";
  if (!/[\s"]/u.test(normalized)) return normalized;
  return `"${normalized.replace(/(["\\])/g, "\\$1")}"`;
}

function requireWpId(value) {
  const normalized = String(value || "").trim();
  if (!normalized) throw new Error("WP_ID is required");
  if (!normalized.startsWith("WP-")) throw new Error("WP_ID must start with WP-");
  return normalized;
}

function scriptPath(relativePath) {
  return repoPathAbs(`${GOV_ROOT_REPO_REL}/${relativePath}`);
}

function step(label, relativePath, args = []) {
  return {
    label,
    scriptPath: relativePath ? scriptPath(relativePath) : "",
    args,
  };
}

export function buildPhaseCheckCommand({
  phase = "",
  wpId = "",
  role = "",
  session = "",
  args = [],
} = {}) {
  const parts = [
    "just",
    "phase-check",
    normalizePhase(phase),
    String(wpId || "").trim(),
  ];
  const normalizedRole = normalizeRole(role);
  const normalizedSession = String(session || "").trim();
  if (normalizedRole) parts.push(normalizedRole);
  if (normalizedSession) parts.push(normalizedSession);
  const normalizedArgs = Array.isArray(args) ? args.map((value) => String(value || "").trim()).filter(Boolean) : [];
  parts.push(...normalizedArgs.map((value) => formatCommandToken(value)));
  return parts.filter(Boolean).join(" ");
}

export function buildPhaseCheckPlan({
  phase = "",
  wpId = "",
  role = "",
  session = "",
  args = [],
} = {}) {
  const normalizedPhase = normalizePhase(phase);
  const normalizedWpId = requireWpId(wpId);
  const normalizedRole = normalizeRole(role);
  const normalizedSession = String(session || "").trim();
  const normalizedArgs = Array.isArray(args) ? args.map((value) => String(value || "")).filter((value) => value !== "") : [];
  const startupInternalFlags = new Set(normalizedArgs.filter((value) => STARTUP_INTERNAL_FLAG_SET.has(String(value || "").trim())));

  if (!PHASE_VALUE_SET.has(normalizedPhase)) {
    throw new Error(`PHASE must be one of ${PHASE_VALUES.join(", ")}`);
  }

  if (normalizedPhase === "STARTUP") {
    if (!STARTUP_ROLE_VALUES.has(normalizedRole)) {
      throw new Error("STARTUP phase requires role CODER, WP_VALIDATOR, or INTEGRATION_VALIDATOR");
    }
    if (normalizedRole === "CODER") {
      const plan = [
        step("ensure-wp-communications", "roles_shared/scripts/wp/ensure-wp-communications.mjs", [normalizedWpId]),
        step("active-lane-brief", "roles_shared/scripts/session/active-lane-brief-lib.mjs", [normalizedRole, normalizedWpId, "--json"]),
        step("wp-communication-health-check", "roles_shared/checks/wp-communication-health-check.mjs", [
          normalizedWpId,
          "STARTUP",
          normalizedRole,
          normalizedSession,
        ]),
        step("gate-check", "", [normalizedWpId]),
      ];
      if (!startupInternalFlags.has("--committed-handoff-preflight")) {
        plan.push(step("pre-work-check", "roles/coder/checks/pre-work-check.mjs", [normalizedWpId]));
      }
      return plan;
    }
    return [
      step("ensure-wp-communications", "roles_shared/scripts/wp/ensure-wp-communications.mjs", [normalizedWpId]),
      step("active-lane-brief", "roles_shared/scripts/session/active-lane-brief-lib.mjs", [normalizedRole, normalizedWpId, "--json"]),
      step("wp-communication-health-check", "roles_shared/checks/wp-communication-health-check.mjs", [
        normalizedWpId,
        "STARTUP",
        normalizedRole,
        normalizedSession,
      ]),
    ];
  }

  if (normalizedPhase === "HANDOFF") {
    const handoffRole = normalizedRole || "WP_VALIDATOR";
    if (!HANDOFF_ROLE_VALUES.has(handoffRole)) {
      throw new Error("HANDOFF phase role must be CODER or WP_VALIDATOR");
    }
    if (handoffRole === "CODER") {
      return [
        step("gate-check", "", [normalizedWpId]),
        step("post-work-check", "roles/coder/checks/post-work-check.mjs", [normalizedWpId, ...normalizedArgs]),
        step("role-mailbox-export-check", "roles_shared/checks/role_mailbox_export_check.mjs"),
        step("wp-communication-health-check", "roles_shared/checks/wp-communication-health-check.mjs", [normalizedWpId, "KICKOFF"]),
      ];
    }
    return [
      step("active-lane-brief", "roles_shared/scripts/session/active-lane-brief-lib.mjs", [handoffRole, normalizedWpId, "--json"]),
      step("validator-packet-complete", "roles/validator/scripts/lib/validator-governance-lib.mjs", [normalizedWpId]),
      step("validator-handoff-check", "roles/validator/scripts/lib/validator-governance-lib.mjs", [normalizedWpId, ...normalizedArgs]),
      step("wp-communication-health-check", "roles_shared/checks/wp-communication-health-check.mjs", [normalizedWpId, "HANDOFF"]),
    ];
  }

  if (normalizedPhase === "VERDICT") {
    const verdictRole = normalizedRole || "INTEGRATION_VALIDATOR";
    if (!VERDICT_ROLE_VALUES.has(verdictRole)) {
      throw new Error("VERDICT phase role must be WP_VALIDATOR or INTEGRATION_VALIDATOR");
    }
    return [
      step("active-lane-brief", "roles_shared/scripts/session/active-lane-brief-lib.mjs", [verdictRole, normalizedWpId, "--json"]),
      step("validator-packet-complete", "roles/validator/scripts/lib/validator-governance-lib.mjs", [normalizedWpId]),
      step("wp-communication-health-check", "roles_shared/checks/wp-communication-health-check.mjs", [normalizedWpId, "VERDICT"]),
    ];
  }

  return [
    ...buildPhaseCheckPlan({
      phase: "VERDICT",
      wpId: normalizedWpId,
      role: normalizedRole || "INTEGRATION_VALIDATOR",
    }),
    step("integration-validator-context-brief", "roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs", [normalizedWpId]),
    step("integration-validator-closeout-check", "roles/validator/scripts/lib/integration-validator-closeout-lib.mjs", [normalizedWpId, ...normalizedArgs]),
    step("launch-memory-manager", "roles/memory_manager/scripts/launch-memory-manager.mjs", ["--force"]),
    // RGF-254: governance-support check — surfaces when the ACP-launched
    // intelligent review has not run within the cadence window so accumulated
    // one-off captures get reviewed instead of dead-lettering.
    step("intelligent-review-cadence-check", "roles/memory_manager/checks/intelligent-review-cadence-check.mjs", []),
  ];
}
