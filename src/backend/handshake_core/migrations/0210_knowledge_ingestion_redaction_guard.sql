-- WP-KERNEL-009 MT-091 hardening (#3): schema-enforced redaction-consistency
-- guard on knowledge_ingestion_spans.
--
-- Master Spec anchor: 2.3.13.11 KnowledgeSpan + WP constraint "Raw secret
-- bytes never reach a durable row." The 0163 table comment asserts
-- "content is the POST-REDACTION text (MT-091: raw secret bytes never land
-- here)", but 0163 enforced only row SHAPE (the `redaction_state IN
-- ('none','redacted')` CHECK and the content_hash format). Nothing at the
-- DB layer asserted the redaction INVARIANT, so a buggy or bypassing writer
-- could store `redaction_state='redacted'` with content that still carries
-- the raw secret bytes (no marker spliced in) and every CHECK would pass.
--
-- Gap closed (HARDENING-20260611, MEDIUM): code in
-- knowledge_ingestion::engine::redact_spans sets
-- `SpanRedaction::Redacted` ONLY after replacing each finding region with a
-- `[REDACTED:<kind>]` marker (secrets.rs::redact_text /
-- redact_span_with_whole_file_findings). The marker is therefore the
-- ground-truth proof that the raw bytes were excised. This migration lifts
-- that code-only invariant into a DB CHECK so EVERY writer (engine, raw SQL,
-- future code path, ops script) is held to it:
--
--   * redaction_state = 'redacted'  =>  content MUST contain a
--     '[REDACTED:' marker (the raw secret region was spliced out).
--
-- A span that claims redaction but stores raw secret bytes (no marker) is now
-- refused at INSERT/UPDATE with a check_violation, schema-enforcing the
-- "no raw secret bytes remain" invariant rather than trusting code alone.
--
-- Direction (deliberate, one-way): the CHECK is asserted ONLY on the
-- `redacted` state -- "if you declared this span redacted, the raw region
-- MUST have been replaced by a marker". The reverse ('none' must NOT contain
-- the marker) is intentionally NOT enforced: a legitimate non-secret source
-- file can literally contain the text `[REDACTED:...]` (e.g. documentation
-- about redaction), and a `none` span over it is correct -- there was no
-- secret to excise. Enforcing the reverse would reject that legitimate
-- content, so the guard targets exactly the leak (claimed-redacted but
-- raw-bytes-present) and nothing else.
--
-- Verifiability complement: content_hash is already SHA-256 of exactly the
-- stored (post-redaction, marker-bearing) content, so the stored bytes the
-- marker proves are also the bytes the hash commits to.
--
-- Migration range note: 0210-0219 is the WP-KERNEL-009 MT-091/094 DB-guard
-- hardening band (0200-0209 is the earlier MT-056/063 band; the original
-- ingestion chain 0160-0169 is frozen, so this guard ships additively rather
-- than as an edit to 0163).

ALTER TABLE knowledge_ingestion_spans
    ADD CONSTRAINT chk_knowledge_ingestion_spans_redaction_marker
    CHECK (
        redaction_state <> 'redacted'
        OR position('[REDACTED:' IN content) > 0
    );
