# Product Self-Hosting Backend Focus (2026-04-05)

## Purpose

This note records the current product-building focus so future Orchestrator and Validator sessions do not collapse Handshake into "repo governance in an IDE shell" or "Monaco plus terminal first."

The current priority is a backend-first self-hosting slice.

## Working Rules

- Handshake remains broader than the software-delivery kernel used to build this repo.
- Repo governance ports into Handshake as an additive software-delivery overlay.
- Repo governance MUST NOT overwrite Handshake-native governance, topology, worksurface model, project-profile extension model, or the broader multi-domain product scope.
- Dev Command Center, Monaco, and terminal UI work stay downstream of the missing backend control-plane contracts.

## Product Reality Check

The product code already contains meaningful backend runtime:

- `src/backend/handshake_core/src/terminal/`
- `src/backend/handshake_core/src/workflows.rs`
- `src/backend/handshake_core/src/llm/ollama.rs`
- `src/backend/handshake_core/src/locus/`
- `src/backend/handshake_core/src/flight_recorder/`
- `src/backend/handshake_core/src/role_mailbox.rs`

This means the next self-hosting blockers are not "terminal backend from zero" or "Ollama from zero."

The actual missing tranche is:

1. Product governance overlay registry for imported repo-governance artifacts.
2. Governed product-side runner for selected governance checks, rubrics, and scripts.
3. Product workflow mirror for software-delivery governance state inside Handshake runtime.
4. Session substrate completion:
   - spawn contract
   - workspace safety
   - crash recovery
   - model-session observability
5. Dev Command Center control-plane backend projections and APIs.
6. Only after that: Dev Command Center UI, typed viewers, Monaco shell, terminal shell, and broader self-hosting polish.

## Why The Large Governance Sync Must Be Split

The current repo governance surface is too large to treat as one packet:

- many protocols, rubrics, templates, checks, scripts, schemas, records, and sync rules
- repo-specific assumptions that must become software-delivery profile extensions instead of universal Handshake law
- Handshake-specific layers and worksurfaces that the repo does not model

Therefore the current split is:

1. `WP-1-Product-Governance-Artifact-Registry-v1`
2. `WP-1-Product-Governance-Check-Runner-v1`
3. `WP-1-Governance-Workflow-Mirror-v1`
4. session-substrate packets
5. `WP-1-Dev-Command-Center-Control-Plane-Backend-v1`
6. `WP-1-Dev-Command-Center-MVP-v1` and related UI packets

## Layer Reminder

When translating repo-governance behavior into Handshake:

- keep Handshake-native governance as the broader product authority
- treat software-delivery governance as one project-profile or overlay inside that authority
- prefer canonical backend artifacts and projections over UI-local state
- avoid frontloading UI shells before the control-plane contracts are implemented

## Activation Heuristic

When choosing the next packet in this area:

- choose the earliest missing backend contract that blocks multiple downstream packets
- avoid activating frontend-first packets until their backend prerequisites are done
- avoid "port the whole repo governance tree" packets; split by additive layer and runtime responsibility
