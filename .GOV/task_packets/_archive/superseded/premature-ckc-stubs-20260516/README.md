---
file_id: premature-ckc-stubs-20260516-readme
file_kind: archive_note
updated_at: 2026-05-16
---

<topic id="archive-rationale" status="active" version="1" summary="Premature CKC rebuild stubs archived after operator correction." updated_at="2026-05-16">

# Premature CKC Stubs Archive

These files were created too early and are retained here only as correction evidence:

- `WP-1-Atelier-Lens-CKC-Greenroom-v1.md`
- `WP-1-Atelier-Lens-CKC-Greenroom-v1.contract.json`
- `WP-1-Atelier-Lens-CKC-Kernel-v1.md`
- `WP-1-Atelier-Lens-CKC-Kernel-v1.contract.json`
- `WP-1-Atelier-Lens-CKC-Vertical-Slice-v1.md`
- `WP-1-Atelier-Lens-CKC-Vertical-Slice-v1.contract.json`

They are not active backlog truth. The corrected sequence is:

1. Use greenroom as today's reference and research activity, not as its own work packet.
2. Consolidate all Atelier/Lens-adjacent stubs into a single active consolidation work packet stub.
3. Complete CKC greenroom review and CKC research.
4. Create CKC rebuild work packet stubs only after the consolidation, greenroom review, and research basis are complete.

Handshake storage remains PostgreSQL-only. SQLite is forbidden in runtime, tests, fixtures, mocks, examples, fallbacks, local caches, compatibility adapters, temporary harnesses, imports, exports, and any other Handshake product path.

</topic>
