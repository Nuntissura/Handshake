# 05) CI, Hooks, and Determinism Config (Kernel)

This kernel assumes governance is not â€œdocumented onlyâ€; it is **enforced** by:
- local hooks (pre-commit)
- CI workflows
- determinism configs (EOL, formatting, toolchain pinning)

Objective:
- A small-context agent should be able to run the same checks locally as CI runs remotely (â€œCI parityâ€).

## 1. CI parity (normative)

Kernel rule:
- CI MUST execute the same governance gates that developers are expected to run locally (or a strict superset).

Minimum recommended CI checks:
- Governance/doc presence checks (required navigation + pointer files).
- Spec pointer correctness (`.GOV/roles_shared/SPEC_CURRENT.md` resolves).
- Task Board formatting check (machine-parseable state).
- Codex checks (forbidden patterns; repo invariants).
- Gate tooling checks (phase gate; pre-work/post-work where applicable).
- Supply-chain checks (licenses/vulns) if the project includes them as hard requirements.

Failure modes if CI parity is missing:
- Developers â€œpass locallyâ€ but fail CI due to hidden requirements.
- Small-context handoffs break because the command surface isnâ€™t authoritative.

## 2. Pre-commit hooks (recommended)

Purpose:
- Catch high-frequency governance violations before they hit CI.

Kernel recommendation:
- A pre-commit hook SHOULD run:
  - fast doc/gov checks (Codex checks, task board check)
  - format checks if fast and deterministic
  - it SHOULD NOT run long builds/tests unless the project explicitly requires it (to avoid disabling hooks).

Hard rule:
- Hooks MUST NOT mutate tracked files automatically unless that behavior is explicitly codified (auto-formatters are allowed if the repo policy is to apply them).

## 3. Determinism configuration surface (kernel-required categories)

Projects MUST define, in-repo, the determinism surface that makes gates reliable.

Common required categories:

### 3.1 End-of-line policy
Purpose:
- Prevent line-ending drift across OSes, which breaks hash-based gates and window-based diff checks.

Contract:
- Define an explicit EOL policy (example: `eol=lf` via `.gitattributes`).
- Gate tooling MUST treat this policy as authoritative and handle CRLF/LF comparisons deterministically.

### 3.2 Ignore policy (`.gitignore`)
Purpose:
- Prevent transient artifacts from polluting diffs and confusing manifest gates.

Contract:
- Tool outputs that are not part of audit artifacts must be ignored (target dirs, caches, node_modules, build outputs).

### 3.3 Toolchain pinning (language/runtime-specific)
Purpose:
- Make CI reproducible and prevent â€œworks on my machineâ€ drift.

Examples (implementation-defined):
- Rust: toolchain version, cargo target dir policy, lint/deny policies.
- Node: pinned package manager and lockfiles.
- Python: pinned interpreter + lockfile.

## 4. Governance-command allowlists (optional hardening)

Some environments restrict what commands an agent may run.

Kernel recommendation:
- Keep an allowlist config that enumerates â€œapproved commandsâ€ for automation agents.

Failure mode if missing:
- Agents run dangerous commands by accident or in the wrong repo, causing loss of work or secret leakage.

## 5. Drift hazards and required mitigations

### 5.1 Version reference drift
Hazard:
- CI/hooks/docs mention an old Codex/spec/protocol version while the repo root has newer versions.

Mitigation (kernel recommendation):
- Add a CI check that asserts referenced governance file names exist and match the latest version pointer(s).
- Prefer pointers (`.GOV/roles_shared/SPEC_CURRENT.md`) over hardcoding version strings in many places.

### 5.2 Template drift
Hazard:
- Agents generate packets/refinements from memory and omit required sections, breaking gates.

Mitigation:
- Keep canonical templates under `.GOV/templates/`.
- Add checks that assert templates contain mandatory fields (manifest block, required headings).


