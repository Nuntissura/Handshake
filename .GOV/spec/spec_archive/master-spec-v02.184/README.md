# Indexed Spec

STATUS: CURRENT_VERSIONED_SPEC_BUNDLE
OWNER: ORCHESTRATOR
SOURCE_SPEC: ../Handshake_Master_Spec_v02.182.md
RESET_BRIEF: ../../operator/docs_local/handshake-v2-kernel-reset-brief.md

## Purpose

This folder holds the active versioned indexed Handshake Master Spec bundle, the
machine-readable module resolver, module metadata, and the manifest-declared
machine-readable changelog.

`../SPEC_CURRENT.md` points to this bundle's `indexed-spec-manifest.json` for hash/order
authority and `INDEX.json` for machine-readable module resolution. The v02.182
monolith remains the source baseline/provenance file; this v02.184 bundle is the
current authority.

## Rules

- Do not edit `../Handshake_Master_Spec_v02.182.md` during versioned bundle work.
- Preserve technical detail and product intent before rewriting anything.
- Treat each module as authoritative only through
  `indexed-spec-manifest.json`, `INDEX.json`, its uniform `spec_version`, its hash,
  and its manifest entry.
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
- `indexed-spec-manifest.json`: machine authority for bundle version, module order, hashes, changelog path, archive root, and reconstruction.
- `spec-modules/`: indexed spec modules with machine-readable frontmatter.
- `spec-changelog.jsonl`: machine-readable Master Spec changelog for this bundle.
- `_transfer/`: archived transfer inventory, concordance, policy, and workspace notes.
