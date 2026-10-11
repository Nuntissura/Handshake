---
file_id: diagnostics-web-tokenizer
file_kind: diagnostics
lane: SMB-WEB
crate: hsk-studio-web
input: validation/R6-review-b.json (SMB-WEB 2 CONFIRMED + 1 PLAUSIBLE, candidate bd8533f); validation/R5-review-b.json (SMB-WEB FAIL_V1)
escalation: second FAIL of the same class (asset-gate bypass through incomplete HTML/CSS decoding) -> Opus remediation, new approach
updated_at: 2026-10-11T07:39:53Z
---

<topic id="root-cause" status="current" summary="cause, evidence">

## Root cause (<=120 chars)

The asset gate classifies its own partial reading of the bytes; every place that reading differs from a browser's hides a URL.

## Evidence (code at bd8533f, spec = WHATWG HTML 13.2 parsing, read 2026-10-11)

- R6 finding 1: `charref.rs:94-109` keeps a named reference without `;` verbatim. The WHATWG named character reference state matches the longest identifier of the 2231-entry table, including the 106 legacy names without `;` (`quot`, `QUOT`, `amp`, `lt`, `gt`, `not`, ...), and decodes it in text and in attribute values, except in an attribute when the next character is `=` or ASCII alphanumeric. `&quot` -> `"` opens/closes a CSS string a browser reads differently from the crate.
- R6 finding 2: `dom.rs:496-525` treats everything under `svg`/`math` as foreign (`foreign` counter). The tree-construction dispatcher routes start tags under HTML integration points (SVG `foreignObject desc title`, MathML `annotation-xml` with `encoding` text/html or application/xhtml+xml) and MathML text integration points (`mi mo mn ms mtext`, except `mglyph malignmark`) to the HTML rules, so `<style>` there is RAWTEXT (no character references). `assets.rs:446-452` decoded it.
- Same-class sites found by this diagnosis (not in the review, each a confirmed hiding payload against bd8533f, proven red by tests):
  - comments: the comment end bang state closes a comment at `--!>`; `dom.rs:384` searches only `-->`, so `<!-- --!><img src=//x><!-- -->` hides the image.
  - script data: the escaped and double-escaped states (`<!--<script>` ... `</script>` does not end the script); `dom.rs:247-264` ends at the first `</script`, so a later attribute value can swallow markup the browser parses.
  - CDATA: `<![CDATA[` is a CDATA section only when the adjusted current node is not in the HTML namespace; in HTML content it is a bogus comment that ends at the first `>`. `dom.rs:397-403` always ran to `]]>`.
  - `noscript`: with the scripting flag enabled (browsers) it is RAWTEXT; the crate parsed it as normal content.
  - breakout: an HTML start tag from the foreign-content breakout list (`b big blockquote body br center code dd div dl dt em embed h1-h6 head hr i img li listing menu meta nobr ol p pre ruby s small span strong strike sub sup table tt u ul var`, `font` with `color|face|size`) pops the foreign elements; the crate nested it and kept reading foreign content afterwards.
  - namespace of nested foreign elements: inside `svg` every non-breakout start tag (also `math`) becomes an SVG element (and vice versa), so `<svg><math><mi>` has no text integration point; a namespace-blind model gets this wrong in both directions.
  - `select` (old "in select" insertion mode ignores `style xmp iframe noembed noframes noscript title svg math`; the current spec, customizable select, parses them) and `frameset` (ignores every start tag except frame/frameset/noframes): engines read raw text differently there.
  - foreign `<style>`: the stylesheet is the child text content (Text + CDATA concatenated); `assets.rs:440-462` tokenized each child separately, so a comment node between two text runs could open a phantom CSS string.

</topic>

<topic id="why-r5-did-not-remove-the-class" status="current" summary="R5 fixed instances, not the model">

## Why R5 (5518980, 8682e6f) did not remove the class

R5 fixed the instances the reviewer named (CSS escapes via a CSS Syntax 3 tokenizer, which the R6 review verified as faithful; numeric references; base href; more URL attributes). It kept two partial models: a character-reference decoder that knows a subset of names and a deliberate "legacy names never produce URL characters" assumption (wrong for `&quot`), and an HTML parser whose tokenizer states and tree-construction context (namespace, integration points, script escapes, comment end, CDATA rule, scripting flag) were simplified. The V4-06 sweep regex matched URL-classification call sites, not the places where the parser decides how bytes are read, so the sweep could not find the remaining disagreements.

</topic>

<topic id="options" status="current" summary="crate vs own implementation, with evidence">

## Option (a): adopt a spec-compliant tokenizer crate

| Candidate | Evidence | Fit |
|---|---|---|
| html5gum 0.8.4 (MIT, 2026-06-22, untitaker/html5gum) | docs.rs: "fully implements 13.2.5", "passes html5lib's tokenizer test suite", "No unsafe Rust", optional deps only (jetscii, html5ever, afl). Emitter: `emit_current_tag(&mut self) -> Option<State>` ("used to switch tokenizer state for tree building"), `adjusted_current_node_present_but_not_in_html_namespace` ("By default ... false and thus all CDATA sections are tokenized as bogus comments"); spans: DefaultEmitter `StartTag.span` + attribute VALUE spans only, attribute names are `BTreeMap` keys (order and duplicates lost); other spans need a custom emitter via `move_position`. | Exact tokenizer incl. full character-reference table. Tree construction (namespace, integration points, breakout, select, noscript) stays the caller's job, so finding 2 is NOT fixed by adoption. Needs a rewrite of the span parser around a custom emitter. |
| html5ever 0.40.1 (MIT OR Apache-2.0, servo) | local registry source: `TokenSink::process_token(&self, token, line_number: u64)` (line numbers only, no byte offsets); closure tendril 0.5.1 (157 `unsafe` sites), web_atoms 0.3.0 (build script, phf/string_cache), log, memchr. | Full tree builder (fixes both), but cannot give byte spans (STU-WEB-003 text-is-authority) and adds an unsafe, build-script closure. Possible later as a test-only differential oracle. |
| lol_html 3.0.1 (BSD-3-Clause, 2026-07-29) | docs.rs: 11 required deps (selectors 0.37, cssparser 0.36, encoding_rs, hashbrown, ...); `Element::get_attribute`: "The value may have HTML/XML entities"; `source_location()` only for start tags. | Streaming rewriter; attribute values not decoded; heavy closure. |
| html5tokenizer 0.5.2 (MIT, spans) | docs.rs: last release 2023-09-28, "not yet fuzz tested", "does not yet implement tree construction". | Stale; tree construction missing. |

## Option (b): complete own implementation of the exact WHATWG pieces the gate depends on

- Character references: the full named table (2231 entries, generated as data from web_atoms 0.3.0 `entities.rs`, MIT OR Apache-2.0, which is the WHATWG `entities.json`; 12 entries spot-checked against html.spec.whatwg.org/multipage/named-characters.html: nGt; fjlig; acE; quot QUOT; amp CounterClockwiseContourIntegral; ThickSpace; nvlt; bne; not notin; all equal), longest-prefix match, attribute rule (`=`/alphanumeric), numeric states with the C1 (0x80-0x9F) windows-1252 table, one decoder with a Text/Attribute context used everywhere.
- Tokenizer states the scanner already has, made exact: comment end (`--!>`), script data escaped/double-escaped, CDATA vs bogus comment by namespace.
- Tree-construction subset, explicit: per-element namespace (HTML/SVG/MathML), the dispatcher (HTML rules at the root, in HTML elements, at HTML integration points, at MathML text integration points except mglyph/malignmark, and for `svg` under annotation-xml), foreign start tags inherit the adjusted current node's namespace, breakout start tags refused typed (the crate never re-nests), `noscript` accepted only when its scripting-enabled RAWTEXT reading ends at the same close tag, `select` refuses the elements engines read differently, `frameset` refused.
- Invariant: inside the accepted subset the crate's reading equals the browser's; everything else is a typed refusal (the module's existing rule: "never guessed, never silently re-nested").

## Decision: (b), no new dependency

Reason: finding 2 and five of the eight same-class sites are tree-construction decisions; no tokenizer crate makes them (html5gum and html5tokenizer leave them to the caller by design), and the only crate that makes them (html5ever) has no byte spans. The exact pieces needed are a bounded table (data) plus ~300 lines of state logic inside the existing span parser, which keeps every span, quote style and patch guarantee. No line in `builders/dep-requests.jsonl` (no dependency added). Follow-up offered to the lead: html5ever as a dev-only differential oracle (needs a steward lock sync; not done here).

</topic>

<topic id="fix-and-proof" status="current" summary="what changes, how it is proven">

## Fix

- `src/entity_table.rs` (generated data), `src/charref.rs` (exact decoder, Text/Attribute contexts), `src/entity.rs` (`decode_entities` = exact Text context), `src/dom.rs` (namespace + dispatcher + script/comment/CDATA states + noscript/select/frameset/breakout refusals), `src/assets.rs` (attribute context decode, child-text-content style assembly, foreign-only decode, SMIL href animation scanning), `src/source_dom_patch.rs` (unsafe raw-text check uses the same close finder), `src/error.rs` (typed refusals), DESCRIPTOR.
- `Denied(UndecodableReference)` is no longer produced: with the complete table an unknown `&name;` is literal text for a browser too (ambiguous ampersand state), so it is classified like any other value.

## Proof

- Adversarial tests per V4-08, each red on bd8533f (red run on an export of bd8533f plus the new tests, outside the shared tree, V4-11/12), green on the fix commit; results recorded in `builders/lanes/hsk-studio-web.json` notes `remediation_R6`.

## Result (2026-10-11T07:39:53Z)

- Commits `ca0d27e` (fix) and `08ad0ad` (V4-07: growth bound checked over the whole table), pushed to `codex/handshake-studio-assets`.
- Red: `Handshake_Artifacts/WP-KERNEL-STUDIO/web-r6-redproof/` (git archive of bd8533f + the 9 new tests, own owner dir): 9 failed, 15 passed. On bd8533f both R6 payloads reported nothing, `url(&quot//e.invalid/w&quot)` was offered to the port, and the comment, script-escape, CDATA, noscript, select, frameset and svg-breakout payloads parsed with the URL hidden.
- Green at 08ad0ad: 1 unit + 24 integration tests pass; check and clippy (`--tests --examples`) clean for the crate.
- The V4-06 sweep (regex in the lane file) lists 57 sites, each fixed or not-applicable.
- Two earlier expectations now follow the spec: `&amp` without `;` decodes in text, and `&notarealname;` is literal (port decides) instead of `Denied(UndecodableReference)`.

</topic>
