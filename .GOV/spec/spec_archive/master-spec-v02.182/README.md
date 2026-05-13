# Indexed Spec

STATUS: CURRENT_SPEC_ENTRYPOINT
OWNER: ORCHESTRATOR
SOURCE_SPEC: ../Handshake_Master_Spec_v02.182.md
RESET_BRIEF: ../../operator/docs_local/handshake-v2-kernel-reset-brief.md

## Purpose

This folder holds the indexed Handshake Master Spec entrypoint, the
machine-readable module resolver, and verbatim source modules.

`../SPEC_CURRENT.md` points to `indexed-spec-manifest.json` for hash/order
authority and `INDEX.json` for machine-readable module resolution. The v02.182 source
file remains the byte-exact source baseline and compatibility target until the
shared spec resolvers are migrated to the indexed manifest.

## Rules

- Do not edit `../Handshake_Master_Spec_v02.182.md` during inventory work.
- Preserve technical detail and product intent before rewriting anything.
- Treat each generated module as authoritative only through
  `indexed-spec-manifest.json`, `INDEX.json`, its hash, and its concordance row.
- Every source section, embedded block, addendum, machine-readable appendix, and
  source snapshot must stay mapped before rewritten modules replace verbatim
  modules.
- Do not invent content categories before the Operator defines which
  distinctions matter.
- Research-driven product growth remains part of the Master Spec workflow;
  this workspace changes the spec shape, not the rule that the spec grows with
  product work.

## Folders

- `INDEX.json`: machine-readable module resolver for tools and LLMs; it is not a document viewer or operator-facing projection.
- `indexed-spec-manifest.json`: machine authority for module order, hashes, and reconstruction.
- `spec-modules/`: indexed spec modules.
- `_transfer/`: archived transfer inventory, concordance, policy, and workspace notes.
