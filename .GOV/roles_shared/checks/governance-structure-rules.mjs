import { GOV_ROOT_REPO_REL } from "../scripts/lib/runtime-paths.mjs";

export const govRootHotspots = {
  agents: {
    severity: "MEDIUM",
    reason: "root-level agent registry material is not part of the stable top-level governance layout",
    target: `${GOV_ROOT_REPO_REL}/roles_shared/docs/ or ${GOV_ROOT_REPO_REL}/reference/`,
  },
  Papers: {
    severity: "HIGH",
    reason: "papers are reference material, not active top-level authority",
    target: `${GOV_ROOT_REPO_REL}/reference/`,
  },
};

export const rolesSharedRootRules = {
  docs: new Map([
    ["AI_WORKFLOW_TEMPLATE.md", `${GOV_ROOT_REPO_REL}/templates/AI_WORKFLOW_TEMPLATE.md`],
    ["ARCHITECTURE.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/ARCHITECTURE.md`],
    ["BOUNDARY_RULES.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/BOUNDARY_RULES.md`],
    ["DEPRECATION_SUNSET_PLAN.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md`],
    ["EVIDENCE_LEDGER.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/EVIDENCE_LEDGER.md`],
    ["MIGRATION_GUIDE.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/MIGRATION_GUIDE.md`],
    ["OFFLINE_GIT_BACKUP_SETUP.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/OFFLINE_GIT_BACKUP_SETUP.md`],
    ["OWNERSHIP.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/OWNERSHIP.md`],
    ["PROJECT_INVARIANTS.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/PROJECT_INVARIANTS.md`],
    ["QUALITY_GATE.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/QUALITY_GATE.md`],
    ["REPO_RESILIENCE.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/REPO_RESILIENCE.md`],
    ["ROLE_SESSION_ORCHESTRATION.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`],
    ["ROLE_WORKFLOW_QUICKREF.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`],
    ["ROLE_WORKTREES.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/ROLE_WORKTREES.md`],
    ["RUNBOOK_DEBUG.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/RUNBOOK_DEBUG.md`],
    ["START_HERE.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/START_HERE.md`],
    ["TOOLING_GUARDRAILS.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/TOOLING_GUARDRAILS.md`],
    ["VALIDATOR_FILE_TOUCH_MAP.md", `${GOV_ROOT_REPO_REL}/roles_shared/docs/VALIDATOR_FILE_TOUCH_MAP.md`],
  ]),
  records: new Map([
    ["BUILD_ORDER.md", `${GOV_ROOT_REPO_REL}/roles_shared/records/BUILD_ORDER.md`],
    ["GIT_TOPOLOGY_REGISTRY.md", `${GOV_ROOT_REPO_REL}/roles_shared/records/GIT_TOPOLOGY_REGISTRY.md`],
    ["OSS_REGISTER.md", `${GOV_ROOT_REPO_REL}/roles_shared/records/OSS_REGISTER.md`],
    ["SIGNATURE_AUDIT.md", `${GOV_ROOT_REPO_REL}/roles_shared/records/SIGNATURE_AUDIT.md`],
    ["SPEC_DEBT_REGISTRY.md", `${GOV_ROOT_REPO_REL}/roles_shared/records/SPEC_DEBT_REGISTRY.md`],
    ["TASK_BOARD.md", `${GOV_ROOT_REPO_REL}/roles_shared/records/TASK_BOARD.md`],
    ["WP_TRACEABILITY_REGISTRY.md", `${GOV_ROOT_REPO_REL}/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`],
  ]),
  runtime: new Map([
    ["GIT_TOPOLOGY_REGISTRY.json", "../gov_runtime/roles_shared/GIT_TOPOLOGY_REGISTRY.json"],
    ["PRODUCT_GOVERNANCE_SNAPSHOT.json", `${GOV_ROOT_REPO_REL}/roles_shared/runtime/PRODUCT_GOVERNANCE_SNAPSHOT.json`],
    ["ROLE_SESSION_REGISTRY.json", "../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json"],
    ["SESSION_CONTROL_BROKER_STATE.json", "../gov_runtime/roles_shared/SESSION_CONTROL_BROKER_STATE.json"],
    ["SESSION_CONTROL_REQUESTS.jsonl", "../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl"],
    ["SESSION_CONTROL_RESULTS.jsonl", "../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl"],
    ["SESSION_LAUNCH_REQUESTS.jsonl", "../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl"],
    ["SESSION_CONTROL_OUTPUTS", "../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/"],
    ["validator_gates", "../gov_runtime/roles_shared/validator_gates/"],
    ["WP_COMMUNICATIONS", "../gov_runtime/roles_shared/WP_COMMUNICATIONS/"],
  ]),
  duplicateTemplates: new Map([
    ["REFINEMENT_TEMPLATE.md", `${GOV_ROOT_REPO_REL}/templates/REFINEMENT_TEMPLATE.md`],
    ["TASK_PACKET_TEMPLATE.md", `${GOV_ROOT_REPO_REL}/templates/TASK_PACKET_TEMPLATE.md`],
  ]),
};

export const roleRootRules = {
  orchestrator: {
    // ORCHESTRATOR_GATES.json moved to gov_runtime/ (not git-tracked) [CX-212D]
    runtime: new Map([]),
  },
  validator: {
    legacy: new Map([
      ["VALIDATOR_GATES.json", `${GOV_ROOT_REPO_REL}/reference/legacy/validator/VALIDATOR_GATES.json`],
    ]),
  },
  coder: {
    runtime: new Map(),
  },
};

export const docsRootHotspots = {
  "memory_dump.md": {
    severity: "MEDIUM",
    reason: "memory dump is not an active architecture surface",
    target: `${GOV_ROOT_REPO_REL}/reference/archaeology/ or a named subfolder under ${GOV_ROOT_REPO_REL}/docs/`,
  },
};
