# 06) Versioning and Drift Control (Kernel)

This kernel assumes:
- specs evolve over time
- tooling and docs must remain synchronized
- small-context models will otherwise “remember the wrong version”

The system therefore treats drift as a first-class failure mode.

## 1. Versioned specs + a single pointer

Kernel rules:
- Master Spec files MUST be versioned (`..._vNN.NNN.md`) and immutable once superseded (append-only history).
- `docs/SPEC_CURRENT.md` MUST be the single authoritative pointer to the current Master Spec.
- All enforcement scripts and protocols SHOULD resolve the spec via `docs/SPEC_CURRENT.md` rather than hardcoding filenames.

Failure modes prevented:
- “Coding against old spec” when multiple versions exist.
- Validators reviewing against a different spec than coders used.

## 2. One-time approvals and auditability

Kernel recommendation:
- Use one-time approval tokens (signatures) as evidence that:
  - a refinement was reviewed
  - a scope contract was accepted
  - a spec enrichment was intentionally approved

Hard rule:
- Approvals must be recorded in append-only audit logs with deterministic formatting so tools can confirm their existence.

## 3. Compatibility shims (allowed, but must be explicit)

Projects evolve directory layouts and filenames. Shims are allowed to avoid breaking tooling, but they must be explicit.

Kernel rule:
- If a legacy path exists (example: `docs/TASK_PACKET_TEMPLATE.md`), it MUST be labeled as a shim that points to the canonical template (example: `docs/templates/TASK_PACKET_TEMPLATE.md`).

Failure mode prevented:
- Agents copy an obsolete template and generate non-conforming packets.

## 4. Drift detection checklist (kernel-recommended)

Add a “drift guard” check in CI that detects:
- Spec pointer drift:
  - `docs/SPEC_CURRENT.md` points to a non-existent file
  - `docs/SPEC_CURRENT.md` does not point to the latest spec by parsed version policy
- Governance reference drift:
  - docs/CI/hooks reference a Codex filename that does not exist
  - scripts reference protocol files that moved/renamed
- Template drift:
  - required headings/fields removed from canonical templates
- Roadmap determinism drift (if used):
  - Coverage Matrix missing/duplicated rows
  - invalid phase tokens
  - mismatch between matrix titles and actual heading titles

## 5. Drift handling policy (what to do when drift is found)

Kernel approach:
1. Treat drift as a governance failure, not as “cleanup”.
2. Create an explicit remediation artifact:
   - update pointers (preferred) rather than renaming many files
   - add compatibility shims if necessary
3. Record the decision in an audit log or changelog section so future models do not re-litigate it.

## 6. Why this matters for small-context models

Small models fail by:
- losing the active spec version
- hallucinating missing requirements
- using the wrong template or missing gates

This kernel prevents that by:
- forcing all “truth” into a small set of stable artifacts
- making drift detectable by scripts, not by memory

