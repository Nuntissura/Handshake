# ARTIFACT_RETENTION_POLICY

**Status:** ACTIVE  
**Policy version:** `2026-04-05`

## Purpose

Keep `../Handshake Artifacts/` bounded without losing the governed proof needed for closeout, audit, and recovery.

## Canonical artifact roots

These top-level directories are durable and must be retained:

- `handshake-cargo-target/`
- `handshake-product/`
- `handshake-test/`
- `handshake-tool/`

## Auto-delete class

The governed cleanup path may remove only:

- repo-local `target/` directories
- stale noncanonical external artifact directories classified as `NONCANONICAL_EPHEMERAL_STALE`

## Preserve class

The governed cleanup path must preserve:

- all canonical top-level artifact roots
- recent noncanonical external artifact directories still classified as active/recent
- unknown noncanonical external artifact directories
- artifact retention manifests under `handshake-tool/artifact-retention/`

## Retention manifest

Every governed artifact cleanup or integration-validator closeout must write a JSON retention manifest under:

- `../Handshake Artifacts/handshake-tool/artifact-retention/`

The manifest records:

- cleanup scope and actor
- removed repo-local and external residue
- retained canonical directories
- retained noncanonical directories after cleanup
- cargo target-dir posture
- blocking issues, if any remain

## Operational rule

- `just artifact-hygiene-check` proves the artifact surface is acceptable.
- `just artifact-cleanup [--dry-run]` removes only reclaimable residue and writes a retention manifest.
- integration-validator closeout must write a retention manifest as part of terminal cleanup before promoting final truth.
