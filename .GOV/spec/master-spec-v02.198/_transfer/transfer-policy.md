# Indexed Spec Policy

STATUS: CURRENT_INDEXED_SPEC_POLICY

## Transfer Constraints

- `SPEC_CURRENT.md` points to the indexed manifest as the current Master Spec
  entrypoint.
- `Handshake_Master_Spec_v02.182.md` remains the byte-exact source baseline and
  compatibility target until shared spec resolvers read the indexed manifest
  directly.
- Rewriting is allowed only when no technical detail or product intent is lost.
- The first transfer pass is neutral inventory and mapping, not restructuring.
- Module boundaries must be derived from the current TOC, section
  ranges, embedded/imported blocks, and the Operator reset brief.
- Kernel-first build order is a product strategy lens, not permission to cram
  the full Master Spec into new arbitrary buckets.
- Do not invent buckets before the Operator defines which distinctions matter.

## Neutral Map First

Each source range should first be recorded without category judgment:

- source file
- source line range
- source heading or block label
- current parent section
- observed notes
- open questions

## Minimum Concordance Row

Each row should eventually include:

- source file
- source line range
- source heading or block label
- source anchor when available
- proposed destination
- preservation status
- notes
