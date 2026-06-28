import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REQUIRED_SECTION = "## HBR Gate Obligations";
const REGISTRY_RELATIVE_PATH = ".GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json";

const ROLE_PROTOCOLS = [
  {
    role: "kernel_builder",
    path: ".GOV/roles/kernel_builder/KERNEL_BUILDER_PROTOCOL.md",
    pillars: ["INT", "SWARM", "VIS", "QUIET", "MAN"],
    focus: ["all active HBR rules in the registry", "INT pillar"],
  },
  {
    role: "integration_validator",
    path: ".GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md",
    pillars: ["INT", "SWARM", "VIS", "QUIET", "MAN"],
    focus: ["all active HBR rules in the registry", "validator-scan"],
  },
  {
    role: "wp_validator",
    path: ".GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md",
    pillars: ["INT", "SWARM", "VIS", "QUIET", "MAN"],
    focus: ["all active HBR rules in the registry", "per-MT verification path"],
  },
  {
    role: "coder",
    path: ".GOV/roles/coder/CODER_PROTOCOL.md",
    pillars: ["INT", "QUIET"],
    focus: ["product interconnectivity", "non-intrusive execution"],
  },
  {
    role: "orchestrator",
    path: ".GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md",
    pillars: ["SWARM", "MAN"],
    focus: ["parallel workflow safety", "ModelManual currency"],
  },
  {
    role: "classic_orchestrator",
    path: ".GOV/roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md",
    pillars: ["SWARM", "MAN"],
    focus: ["manual-relay only", "HBR gates apply equally"],
  },
  {
    role: "validator",
    path: ".GOV/roles/validator/VALIDATOR_PROTOCOL.md",
    pillars: ["INT", "SWARM", "VIS", "QUIET", "MAN"],
    focus: ["all active HBR rules in the registry", "PASS or merge-ready claim"],
  },
];

const EXCLUDED_PROTOCOLS = [
  ".GOV/roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md",
  ".GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md",
];

function repoRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

function readText(root, relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}

function hbrSection(text) {
  const start = text.indexOf(REQUIRED_SECTION);
  if (start === -1) return null;
  const rest = text.slice(start);
  const next = rest.slice(REQUIRED_SECTION.length).match(/\n## /);
  if (!next || next.index === undefined) return rest;
  return rest.slice(0, REQUIRED_SECTION.length + next.index);
}

function requireIncludes(failures, context, text, needle) {
  if (!text.includes(needle)) {
    failures.push(`${context} missing ${needle}`);
  }
}

function runCheck(root = repoRoot()) {
  const failures = [];

  const registry = JSON.parse(readText(root, REGISTRY_RELATIVE_PATH));
  if (registry.version !== "1.7.0") {
    failures.push(`${REGISTRY_RELATIVE_PATH} version expected 1.7.0, got ${registry.version || "<missing>"}`);
  }
  const activeRules = Array.isArray(registry.rules)
    ? registry.rules.filter((rule) => rule && rule.status === "ACTIVE")
    : [];
  if (activeRules.length < 23) {
    failures.push(`${REGISTRY_RELATIVE_PATH} expected at least 23 active HBR rules, got ${activeRules.length}`);
  }

  for (const protocol of ROLE_PROTOCOLS) {
    const text = readText(root, protocol.path);
    const section = hbrSection(text);
    if (!section) {
      failures.push(`${protocol.path} missing ${REQUIRED_SECTION}`);
      continue;
    }

    const duplicateCount = (text.match(/## HBR Gate Obligations/g) || []).length;
    if (duplicateCount !== 1) {
      failures.push(`${protocol.path} expected exactly one HBR Gate Obligations section, got ${duplicateCount}`);
    }

    requireIncludes(failures, protocol.path, section, "CX-131");
    requireIncludes(failures, protocol.path, section, "Master Spec §5.6");
    requireIncludes(failures, protocol.path, section, REGISTRY_RELATIVE_PATH);
    requireIncludes(failures, protocol.path, section, "packet.acceptance_matrix.hbr");
    requireIncludes(failures, protocol.path, section, "evidence_kind");
    requireIncludes(failures, protocol.path, section, "HandoffGate (MT-004) MUST PASS");
    requireIncludes(failures, protocol.path, section, "PENDING");
    requireIncludes(failures, protocol.path, section, "STEER");
    requireIncludes(failures, protocol.path, section, "BLOCKED");
    requireIncludes(failures, protocol.path, section, "CX-503B1");
    requireIncludes(failures, protocol.path, section, `Applicable pillars for this role: ${protocol.pillars.join(", ")}`);
    for (const focus of protocol.focus) {
      requireIncludes(failures, protocol.path, section, focus);
    }
  }

  for (const relativePath of EXCLUDED_PROTOCOLS) {
    if (!fs.existsSync(path.join(root, relativePath))) continue;
    const text = readText(root, relativePath);
    if (text.includes(REQUIRED_SECTION)) {
      failures.push(`${relativePath} must not gain ${REQUIRED_SECTION} in MT-038`);
    }
  }

  return failures;
}

const failures = runCheck();
if (failures.length > 0) {
  console.error(JSON.stringify({
    check: "role-protocol-hbr-linkage",
    verdict: "FAIL",
    failures,
  }, null, 2));
  process.exit(1);
}

const activeRuleCount = JSON.parse(readText(repoRoot(), REGISTRY_RELATIVE_PATH)).rules.filter((rule) => rule && rule.status === "ACTIVE").length;
console.log(`role-protocol-hbr-linkage ok (${ROLE_PROTOCOLS.length} protocols linked, ${activeRuleCount} active HBR rules)`);
