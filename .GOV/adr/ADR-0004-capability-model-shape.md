# ADR-0004: Capability Model Shape - Axes + Full IDs + Profiles

- **Status:** Accepted
- **Date:** 2026-01-23
- **Context:** The Master Spec defines a capability and consent model that must be enforced deterministically across AI jobs and workflows. The system must avoid drift by centralizing capability identifiers, profile definitions, and job-to-profile mappings.

## Decision
- Model capabilities as:
  - **Axes** for coarse grants (e.g., `fs.read`, `fs.write`, `proc.exec`, `net.http`, `device`, `secrets.use`).
  - **Full IDs** for specific privileges (e.g., `terminal.exec`, `export.debug_bundle`, `export.governance_pack`, `fr.read`, `diagnostics.read`, `jobs.read`).
- Maintain **named profiles** (e.g., Analyst, Coder, Operator) that whitelist a set of allowed capabilities.
- Maintain a **job kind -> profile mapping** and **job kind -> required capability IDs** as a Single Source of Truth.
- Enforce an explicit policy boundary:
  - Unknown capability IDs MUST produce `HSK-4001: UnknownCapability`.
  - Known but not granted capabilities are denied without silently widening access.

## Alternatives Considered
- **Stringly-typed capabilities without a registry/SSoT:** Rejected; leads to drift and weak auditability.
- **Implicit allow-listing by convention:** Rejected; not deterministic and hard to validate.

## Consequences
- **Pros:** Deterministic enforcement; easier audits; clear place to update when adding a new capability or job kind.
- **Cons:** Requires maintenance when new job kinds/capabilities are introduced; demands discipline to avoid bypasses.

## Follow-ups
- Keep the canonical capability identifiers and profiles aligned with the Master Spec and update this ADR when the model changes.
