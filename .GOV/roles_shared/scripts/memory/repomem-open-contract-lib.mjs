const GOVERNED_VALIDATOR_REPOMEM_ROLE_VALUES = new Set([
  "WP_VALIDATOR",
  "INTEGRATION_VALIDATOR",
]);

const WP_ID_RE = /^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/;

export function normalizeRepomemRole(value = "", fallback = "ORCHESTRATOR") {
  const normalized = String(value || "").trim().toUpperCase();
  if (normalized) return normalized;
  return String(fallback || "ORCHESTRATOR").trim().toUpperCase() || "ORCHESTRATOR";
}

export function validateRepomemOpenContract({
  providedRole = "",
  roleFlagProvided = false,
  wpId = "",
  environmentRole = "",
} = {}) {
  const normalizedEnvironmentRole = normalizeRepomemRole(environmentRole, "");
  const governedEnvironmentRole = GOVERNED_VALIDATOR_REPOMEM_ROLE_VALUES.has(normalizedEnvironmentRole)
    ? normalizedEnvironmentRole
    : "";
  const normalizedRole = normalizeRepomemRole(providedRole, governedEnvironmentRole || "ORCHESTRATOR");
  const governedRole = GOVERNED_VALIDATOR_REPOMEM_ROLE_VALUES.has(normalizedRole)
    ? normalizedRole
    : "";
  const normalizedWpId = String(wpId || "").trim();

  if (governedEnvironmentRole && !roleFlagProvided) {
    throw new Error(`Governed validator SESSION_OPEN requires explicit --role ${governedEnvironmentRole}.`);
  }
  if (governedEnvironmentRole && normalizedRole !== governedEnvironmentRole) {
    throw new Error(`Governed validator SESSION_OPEN role mismatch: expected --role ${governedEnvironmentRole}.`);
  }
  if (governedRole && !roleFlagProvided) {
    throw new Error(`Governed validator SESSION_OPEN requires explicit --role ${governedRole}.`);
  }
  if (governedRole && !normalizedWpId) {
    throw new Error(`Governed validator SESSION_OPEN requires --wp WP-{ID}.`);
  }
  if (governedRole && !WP_ID_RE.test(normalizedWpId)) {
    throw new Error(`Governed validator SESSION_OPEN requires a valid --wp WP-{ID}.`);
  }

  return {
    role: normalizedRole,
    wpId: normalizedWpId,
  };
}
