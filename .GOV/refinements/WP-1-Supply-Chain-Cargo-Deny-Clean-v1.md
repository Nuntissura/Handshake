## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Supply-Chain-Cargo-Deny-Clean-v1
- CREATED_AT: 2026-02-08T20:51:28.191Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- SPEC_TARGET_SHA1: d16eb1eb5045e858112b2ce477f27aa0200621b0
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja080220262221
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Supply-Chain-Cargo-Deny-Clean-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Current repo hygiene command surface is not aligned with the spec's supply-chain closure gate:
  - Running `cargo deny check advisories licenses bans sources` from the repo root fails because there is no root `Cargo.toml`.
  - Running `cargo deny check advisories licenses bans sources` from `src/backend/handshake_core` currently FAILS due to (a) active RustSec advisories and (b) a license classification failure.
- Observed failures (2026-02-08):
  - Advisories:
    - RUSTSEC-2024-0363: `sqlx 0.8.0` (solution: upgrade to >= 0.8.1; e.g. `cargo update -p sqlx`)
    - RUSTSEC-2026-0009: `time 0.3.44` (solution: upgrade to >= 0.3.47; e.g. `cargo update -p time`)
  - Licenses:
    - `ring 0.17.9` flagged as unlicensed (license expression not retrieved with required confidence). Likely requires a `deny.toml` clarification/override for `ring` (or other policy fix) so `cargo deny` can resolve a valid license expression deterministically.
- CI parity risk: `just validate` and/or CI workflows that call `cargo deny` must invoke it with the correct manifest context (e.g., run within the backend crate dir or pass manifest path) and must reach 0 violations to satisfy the closure gate.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE

### RED_TEAM_ADVISORY (security failure modes)
- Disabling `cargo deny` checks, or adding broad ignore/allow rules to force green, can silently ship known vulnerable dependencies and/or unknown license posture. This violates the Phase Closure Gate's "zero violations" requirement and undermines supply-chain assurances.
- Preferred remediation order: (1) upgrade dependencies to patched versions, (2) tighten/clarify policy for ambiguous license detection (e.g., `ring`), (3) only then consider narrowly scoped ignore entries with explicit justification (if the spec allows it and the risk is accepted).

### PRIMITIVES (traits/structs/enums)
- NONE

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The Master Spec explicitly requires `cargo deny` checks as part of hygiene and as a Phase Closure Gate with "0 violations", and provides a `deny.toml` template/policy shape.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec already defines (a) the required command to run, (b) the closure gate semantics ("0 violations"), and (c) the expected `deny.toml` structure. The work is remediation/implementation alignment, not missing spec text.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 6.11 Hygiene Gate [CX-631] (HYGIENE_COMMANDS includes cargo deny)
- CONTEXT_START_LINE: 29014
- CONTEXT_END_LINE: 29018
- CONTEXT_TOKEN: [CX-631] HYGIENE_COMMANDS
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 6.11 Hygiene Gate (commands + scope)
  
  [CX-630] HYGIENE_SCOPE: Changes SHOULD stay scoped to the task; avoid drive-by refactors or unrelated cleanups.
  [CX-631] HYGIENE_COMMANDS: For repo-changing work, assistants SHOULD run (or explicitly note not run): `just docs-check`; `just codex-check`; `pnpm -C {{FRONTEND_ROOT_DIR}} run lint`; `pnpm -C {{FRONTEND_ROOT_DIR}} test`; `pnpm -C {{FRONTEND_ROOT_DIR}} run depcruise`; `cargo fmt`; `cargo clippy --all-targets --all-features`; `cargo test --manifest-path {{BACKEND_CARGO_TOML}}`; `cargo deny check advisories licenses bans sources`.
  [CX-632] HYGIENE_TODOS: When touching code near TODOs, assistants SHOULD either resolve them or leave a dated note explaining why they remain.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 6.5 Phase Closure Gate (Explicit Requirements) [CX-609B] (supply chain clean)
- CONTEXT_START_LINE: 32399
- CONTEXT_END_LINE: 32403
- CONTEXT_TOKEN: Supply chain audit clean
- EXCERPT_ASCII_ESCAPED:
  ```text
  - [ ] **Supply chain audit clean** (zero violations)
    ```bash
    cargo deny check    # Should return 0 violations
    npm audit           # Should return 0 critical/high vulnerabilities
    ```
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 7.5.4.9.2 Template Index (HARD) (deny.toml template shape)
- CONTEXT_START_LINE: 29398
- CONTEXT_END_LINE: 29435
- CONTEXT_TOKEN: ###### Template File: `deny.toml`
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### Template File: `deny.toml`
  Intent: Supply-chain policy config for cargo-deny (license/advisory/bans/sources).
  ````toml
  [advisories]
  db-urls = ["https://github.com/RustSec/advisory-db"]
  ignore = [
      "RUSTSEC-2025-0119", # number_prefix
      "RUSTSEC-2024-0436", # paste
  ]
  yanked = "deny"
  
  [licenses]
  allow = [
    "Apache-2.0",
    "MIT",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "CC0-1.0",
    "Unlicense",
    "MPL-2.0",
    "Unicode-DFS-2016",
    "Unicode-3.0",
    "CDLA-Permissive-2.0",
  ]
  confidence-threshold = 0.8
  
  [bans]
  multiple-versions = "warn"
  wildcards = "deny"
  
  [sources]
  unknown-registry = "deny"
  unknown-git = "deny"
  allow-registry = ["https://github.com/rust-lang/crates.io-index"]
  
  ````
  ```
