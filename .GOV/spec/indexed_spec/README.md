# Indexed Spec

STATUS: CURRENT_SPEC_ENTRYPOINT
OWNER: ORCHESTRATOR
SOURCE_SPEC: ../Handshake_Master_Spec_v02.182.md
RESET_BRIEF: ../../operator/docs_local/handshake-v2-kernel-reset-brief.md

## Purpose

This folder holds the indexed Handshake Master Spec entrypoint and verbatim
source modules.

`../SPEC_CURRENT.md` points to `indexed-spec-manifest.json`. The v02.182 source
file remains the byte-exact source baseline and compatibility target until the
shared spec resolvers are migrated to the indexed manifest.

## Rules

- Do not edit `../Handshake_Master_Spec_v02.182.md` during inventory work.
- Preserve technical detail and product intent before rewriting anything.
- Treat each generated module as authoritative only through
  `indexed-spec-manifest.json`, its hash, and its concordance row.
- Every source section, embedded block, addendum, machine-readable appendix, and
  source snapshot must stay mapped before rewritten modules replace verbatim
  modules.
- Do not invent content categories before the Operator defines which
  distinctions matter.
- Research-driven product growth remains part of the Master Spec workflow;
  this workspace changes the spec shape, not the rule that the spec grows with
  product work.

## Folders

- `inventory/`: factual reads from the current spec and reset brief.
- `concordance/`: source-to-module mapping drafts.
- `spec-modules/`: indexed spec modules.
- `workspace/`: temporary transfer notes and operator-reviewed migration plans.
