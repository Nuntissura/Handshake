---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
module_id: "14-28"
section_id: "14.28"
title: "14.28 Web Authoring, Code Intelligence & Site Publishing"
status: "STAGED_DRAFT_NOT_IN_ACTIVE_MANIFEST"
new_domain: true
new_domain_note: "Section 14 has no web domain in v02.205. The operator added the web-authoring source application to Studio scope on 2026-09-04 and noted it pairs with the design-system source as a design-to-web pipeline. This module creates the domain."
source_baseline_version: "v02.205"
source_baseline_path: ".GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md"
provenance_sidecar: "14-28-web-authoring.provenance.json"
body_sha256: "ASSIGNED_AT_BUNDLE_ASSEMBLY"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---

## 14.28 Web Authoring, Code Intelligence & Site Publishing

This sub-section is the normative Studio feature set for web authoring: the web document-type set, the tag and attribute vocabulary with its enumerated values, code intelligence (hinting, colouring, validation, linting), the CSS property surface, responsive breakpoints and media queries, templates and snippets, the Insert object and behaviour catalogue, site definition, and publishing transports. It is a NEW domain; section 14 in v02.205 has no web catalog. Web documents are `StudioDocument` instances in **web mode** ([STU-WEB-003]), so they share the selection, history/undo (14.19), colour (14.8), collaboration (14.17), command (14.14), export (14.13) and model-steerability (14.16) surfaces with every other Studio domain.

**14.28.2 ([STU-WEB-030] through [STU-WEB-044]) is a STUDIO-WIDE contract, not a web-only one.** It is authored here because its entire evidence basis is the web source application's sixty property inspectors and their binding declarations. See [STU-WEB-030] for the anchor-placement caveat.

---

### 14.28.0 Scope, posture and dedup

**[STU-WEB-001] Domain scope.** Studio's web-authoring domain covers: authoring and editing web source documents; a live rendered view of them; code intelligence over their languages; a CSS authoring surface; a responsive breakpoint surface; a template and snippet system; a site definition binding a local root to remote and testing targets; and publishing to those targets. It is the design-to-web half of the pipeline whose design half is 14.10.

**[STU-WEB-002] Out of scope.** Studio does NOT ship a web application server, a database server, or a server-side runtime. The eight server models of [STU-WEB-124] are recognised for editing, code intelligence, publishing and round-trip fidelity ONLY. Studio MUST NOT execute server-side code, MUST NOT open a database connection on the operator's behalf, and MUST NOT require any server to be reachable to author, validate or publish a document.

**[STU-WEB-003] Web mode of the unified document.** A web document is a `StudioDocument` whose `mode` is `web`. Its authority payload is the ordered source text plus a parsed element tree; the tree is a projection over the text, and the TEXT is authority. Every structural edit MUST be expressible as a text edit, so a document that Studio cannot fully model still round-trips byte-for-byte. A web document MUST NOT be a separate document type with its own siloed model ([STU-DOC-001]).

**[STU-WEB-004] Text-is-authority corollary.** Because text is authority, Studio MUST preserve, on every save: original line endings, original indentation character and width, original attribute quoting style, original tag-name case, and any construct Studio did not parse. Reformatting happens only when an explicit format command is invoked ([STU-WEB-028]).

**[STU-WEB-005] Dedup.** Per [STU-SECTION-003], no source product name is a Studio surface, command, panel, view, template, snippet, behaviour, server-model or transport name. The source application's names appear only as provenance in the sidecar.

**[STU-WEB-006] Local-first.** Every capability in this sub-section works with no account, no sign-in and no network, except publishing ([STU-WEB-127]), which requires only the operator's own declared remote target. No vendor cloud is a runtime dependency ([STU-OVR-002]).

**[STU-WEB-007] Storage.** Web-document authority records, site definitions, publish receipts, snippet libraries, template records and lint results are SurrealDB `SCHEMAFULL` tables under [STU-SDB-002] through [STU-SDB-008]. Document source bytes, published-file snapshots and framework bundles are content-addressed artifacts referenced from those records ([STU-ASSET-008]). No SQLite, libSQL, Turso or PostgreSQL is introduced anywhere, including caches, fixtures and the code-intelligence index ([STU-OVR-003]).

**[STU-WEB-008] No new router.** Opening a web document, a site, or a published-file diff from another Studio surface or from another Handshake module is one more navigation target on the existing navigation layers; this domain adds no routing machinery.

**[STU-WEB-009] Capability basis.** The registry rows tagged `web` number 458. That figure sizes the domain; it is NOT the clause count and NOT a coverage target. The normative surface is what this sub-section states.

**[STU-WEB-010] Generative-feature posture.** The web source application ships no generative or model-inference feature on any captured surface. Studio's web domain therefore inherits no generative baseline; any model assistance in this domain is a Studio model-lane capability under [STU-ARC-005], never a ported vendor feature.

---

### 14.28.1 Document types, tag libraries and the attribute vocabulary

**[STU-WEB-011] Document-type record (normative shape).** A web document type is a record carrying: `type_id` (stable string), `title`, `internal_kind` (see [STU-WEB-012]), `windows_extensions` (ordered array), `mac_extensions` (ordered array), `mime_type`, `writes_byte_order_mark` (boolean), `dtd_context` (see [STU-WEB-014]), `server_model_id` (nullable), and `starter_document_id` (nullable). Extension arrays are ORDERED: the first entry is the default extension for a new document of that type.

**[STU-WEB-012] Internal-kind enumeration (normative, closed).** Studio MUST accept exactly six members and no others: `HTML`, `TEXT`, `XML`, `XSLT`, `DYNAMIC`, and `TEMPLATE` for the template-document family of [STU-WEB-090]. `DYNAMIC` means the document embeds server-side code and carries a `server_model_id`.

**[STU-WEB-013] Document-type set (normative, thirty-nine rows).** Studio MUST ship exactly these document types. Extensions are Windows extensions; the Mac list is identical unless stated.

| # | type_id | Title | Kind | Extensions | MIME | BOM | DTD context | Server model |
|---|---|---|---|---|---|---|---|---|
| 1 | `html` | HTML | HTML | html, htm, shtml, shtm, stm, tpl, lasso, xhtml | text/html | false | html | — |
| 2 | `css` | CSS | TEXT | css | text/css | false | — | — |
| 3 | `less` | LESS | TEXT | less | text/css | false | — | — |
| 4 | `scss` | SCSS | TEXT | scss | text/css | false | — | — |
| 5 | `sass` | Sass | TEXT | sass | text/css | false | — | — |
| 6 | `javascript` | JavaScript | TEXT | js | text/javascript | false | — | — |
| 7 | `json` | JSON | TEXT | json | application/json | false | — | — |
| 8 | `xml` | XML | XML | xml, xsd, rss, rdf, dtd, vtm, vtml, csn, config, mxi | text/xml | false | xml | — |
| 9 | `svg` | SVG | XML | svg | image/svg+xml | false | svg | — |
| 10 | `text` | Text | TEXT | txt | text/plain | false | — | — |
| 11 | `wml` | WML | XML | wml | text/xml | false | wml | — |
| 12 | `tld` | TLD | XML | tld | text/xml | false | jsp_tag_library | — |
| 13 | `edml` | EDML | XML | edml, edm | text/xml | false | none | — |
| 14 | `vbscript` | VBScript | TEXT | vbs | text/vbscript | false | — | — |
| 15 | `csharp` | C# | TEXT | cs | text/cs | false | — | — |
| 16 | `vb` | VB | TEXT | vb | text/vb | false | — | — |
| 17 | `java` | Java | TEXT | java | text/java | false | — | — |
| 18 | `actionscript` | ActionScript | TEXT | as | text/as | false | — | — |
| 19 | `actionscript_comm` | ActionScript communications | TEXT | asc | text/asc | false | — | — |
| 20 | `actionscript_remote` | ActionScript remote | TEXT | asr | text/asr | false | — | — |
| 21 | `xslt_page` | XSLT (entire page) | XSLT | xsl, xslt | text/xsl | false | xslt | `xslt` |
| 22 | `xslt_fragment` | XSLT (fragment) | XSLT | xsl, xslt | text/xsl | false | none | `xslt` |
| 23 | `template_html` | HTML template | TEMPLATE | dwt | text/html | false | html | — |
| 24 | `library_item` | Library item | HTML | lbi | text/html | false | none | — |
| 25 | `php` | PHP | DYNAMIC | php, php3, php4, php5, phtml | text/html | false | html | `php_mysql` |
| 26 | `asp_js` | ASP JavaScript | DYNAMIC | asp | text/html | false | html | `asp_js` |
| 27 | `asp_vb` | ASP VBScript | DYNAMIC | asp | text/html | false | html | `asp_vbs` |
| 28 | `coldfusion` | ColdFusion | DYNAMIC | cfm, cfml | text/html | false | html | `coldfusion` |
| 29 | `coldfusion_component` | ColdFusion component | DYNAMIC | cfc | text/html | false | none | `coldfusion` |
| 30 | `jsp` | JSP | DYNAMIC | jsp, jst | text/html | false | html | `jsp` |
| 31 | `aspnet_vb` | ASP.NET VB | DYNAMIC | aspx, ascx, asmx | text/html | **true** | html | `aspnet_vb` |
| 32 | `aspnet_csharp` | ASP.NET C# | DYNAMIC | aspx, ascx, asmx | text/html | **true** | html | `aspnet_csharp` |
| 33 | `template_asp_vb` | ASP VBScript template | TEMPLATE | dwt.asp | text/html | false | html | — |
| 34 | `template_asp_js` | ASP JavaScript template | TEMPLATE | dwt.asp | text/html | false | html | — |
| 35 | `template_coldfusion` | ColdFusion template | TEMPLATE | dwt.cfm | text/html | false | html | — |
| 36 | `template_jsp` | JSP template | TEMPLATE | dwt.jsp | text/html | false | html | — |
| 37 | `template_aspnet_csharp` | ASP.NET C# template | TEMPLATE | dwt.aspx | text/html | **true** | html | — |
| 38 | `template_aspnet_vb` | ASP.NET VB template | TEMPLATE | dwt.aspx | text/html | **true** | html | — |
| 39 | `template_php` | PHP template | TEMPLATE | dwt.php | text/html | false | html | — |

Four rows write a byte-order mark by default and thirty-five do not; the flag is per document type and MUST be operator-overridable per document with the override stored on the document record.

**[STU-WEB-014] DTD-context enumeration (normative, closed).** `dtd_context` MUST take exactly one of seven members and no others: `html`, `xml`, `svg`, `wml`, `xslt`, `jsp_tag_library`, `none`. `dtd_context` selects which tag libraries ([STU-WEB-017]) and which validator rule set ([STU-WEB-052]) apply.

**[STU-WEB-015] Doctype declaration set (normative, ten rows).** Studio MUST offer exactly these ten doctype declarations when creating or converting an HTML-family document: `html_401_transitional`, `html_401_strict`, `html_401_frameset`, `html_5`, `xhtml_1_transitional`, `xhtml_1_strict`, `xhtml_11`, `xhtml_1_frameset`, `xhtml_mobile_1`, `xslt_1`. Each carries its literal declaration string, which MUST be emitted verbatim. `html_5` is the default for a new HTML document.

**[STU-WEB-016] Extension-to-MIME map (normative, thirteen rows).** For non-source assets referenced from a web document, Studio MUST resolve the served MIME type from this table: `bmp` → image/bmp; `gif` → image/gif; `ico` → image/x-icon; `jpeg` and `jpg` → image/jpeg; `pdf` → application/pdf; `png` → image/png; `rss` → application/rss+xml; `svg` → image/svg+xml; `swf` → application/x-shockwave-flash; `tif` and `tiff` → image/tiff; `ttf` → font/ttf. Unlisted extensions resolve to `application/octet-stream` and MUST be reported in the publish receipt so the operator can correct the server configuration.

**[STU-WEB-017] Tag library record (normative shape).** A tag library carries `library_id`, `display_name`, `applies_to_document_types` (array of `type_id` from [STU-WEB-013]), `namespace_prefix` (nullable), `declared_tag_count`, and `tag_chooser_id` (nullable). A tag definition resolves against exactly the libraries whose `applies_to_document_types` contains the current document's `type_id`.

**[STU-WEB-018] Tag library set (normative, ten rows).**

| library_id | Prefix | Declared tags | Applies to |
|---|---|---|---|
| `html` | — | 121 | html, asp_js, asp_vb, aspnet_csharp, aspnet_vb, coldfusion, jsp, php, library_item, xslt_page, xslt_fragment, and all seven template types |
| `cfml` | — | 111 | coldfusion, coldfusion_component, template_coldfusion |
| `aspnet` | `<asp:` | 69 | aspnet_csharp, aspnet_vb, template_aspnet_csharp, template_aspnet_vb |
| `jsp` | `<jsp:` | 15 | jsp, template_jsp |
| `jrun` | `<jrun:` | 22 | jsp, template_jsp |
| `asp` | — | 2 | asp_js, asp_vb, template_asp_js, template_asp_vb |
| `php` | — | 8 | php, template_php |
| `templates` | — | 2 | html, all seven template types, xslt_page, xslt_fragment |
| `xslt` | `<xsl:` | 35 | xslt_page, xslt_fragment |
| `svg` | `<http://www.w3.org/2000/svg:` | 81 | svg, html, template_html, php |

Total declared tag definitions across all libraries: 470, over 466 index-declared references. The four-definition surplus is definitions reachable by file but not indexed; Studio MUST load them and MUST record the index/definition delta in a startup diagnostic rather than silently dropping either side.

**[STU-WEB-019] Tag definition record (normative shape).** `{tag_name, library_id, has_end_tag, tag_type, formatting_rules, property_inspector_id, attributes[]}` where `has_end_tag` ∈ {`yes`, `no`, `xml`} (`xml` = self-closing form required in XML-family documents) and `tag_type` ∈ {`empty`, `nonempty`, null}.

**[STU-WEB-020] Formatting-rule record (normative, closed set of ten keys).** Every tag definition carries a formatting-rule record drawn from exactly these keys, each with the stated domain:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*
| Key | Domain | Meaning |
|---|---|---|
| `nlbeforetag` | 0 \| 1 | newlines emitted before the opening tag |
| `nlbeforetag_min` | 0 | lower clamp when the formatter compresses |
| `nlbeforetag_max` | 2 | upper clamp when the formatter expands |
| `nlaftertag` | 0 \| 1 | newlines after the closing tag |
| `nlaftertag_min` | 0 | lower clamp |
| `nlaftertag_max` | 2 | upper clamp |
| `nlbeforecontents` | 0 \| 1 | newlines before the tag's contents |
| `nlaftercontents` | 0 \| 1 | newlines after the tag's contents |
| `indentcontents` | yes \| no | indent the contents one level |
| `formatcontents` | yes \| no | when `no`, the contents are verbatim and the formatter MUST NOT touch them |
| `preserveattrcase` | yes | when present, attribute-name case is preserved verbatim |

`formatcontents: no` is what protects `<pre>`, `<script>`, `<style>` and `<textarea>` content; a formatter that reflows inside those elements is a defect. `preserveattrcase` is what protects the SVG and namespaced attribute vocabularies.

**[STU-WEB-021] Attribute definition record (normative shape).** `{name, value_type, case_sensitive, allowed_values[]}`. `allowed_values` is present only when `value_type` is the enumerated type and is an ordered array of `{value, caption?}`.

**[STU-WEB-022] Attribute value-type enumeration (normative, closed, twenty members).** Studio MUST normalise the source vocabulary to lower case and MUST support exactly these twenty value types. The counts are the shipped distribution across the 9,741 attribute definitions and are stated so an implementer sizes the picker correctly, not as a target:

| value_type | Count | Editor affordance |
|---|---|---|
| `text` | 3,619 | free text |
| `unspecified` | 2,898 | free text with no validation |
| `enumerated` | 2,147 | closed picker over `allowed_values` |
| `color` | 409 | colour well bound to an explicit `StudioColorProfile` ([STU-DOC-003]) |
| `cssstyle` | 197 | inline style editor |
| `style` | 148 | style-attribute editor |
| `cssid` | 109 | id picker over the document's declared ids |
| `flag` | 61 | valueless boolean attribute |
| `relativepath` | 54 | site-relative path picker ([STU-WEB-118]) |
| `font` | 32 | font-stack picker |
| `cssclass` | 17 | class picker over the document's reachable stylesheets |
| `xpath` | 14 | XPath expression editor |
| `filepath` | 12 | absolute-path picker |
| `directory` | 7 | folder picker |
| `filename` | 5 | filename editor |
| `width:src` | 4 | numeric width auto-derived from the referenced `src` asset |
| `height:src` | 4 | numeric height auto-derived from `src` |
| `xpath_context` | 2 | XPath context editor |
| `width:data` | 1 | numeric width auto-derived from the referenced `data` asset |
| `height:data` | 1 | numeric height auto-derived from `data` |

The four `*:src` and `*:data` types are DERIVED: their value is read from the referenced asset, which for Studio means the asset is resolved through CKC ([STU-ASSET-005]) and its intrinsic dimensions are read from the CKC record, not by opening the file a second time.

**[STU-WEB-023] Enumerated attribute values.** The shipped enumerated vocabulary is 14,733 values across the 2,147 enumerated attribute definitions. Studio MUST store each attribute's `allowed_values` as an ORDERED array and MUST offer them in declared order, not alphabetically, because declared order encodes the vendor's preferred-first convention. An attribute value outside its `allowed_values` is a validation warning, never a silent rewrite: the operator's text stands and the diagnostic names the attribute, the offending value and the allowed set.

**[STU-WEB-024] Cross-tag attribute groups.** Beyond per-tag attributes, Studio MUST support cross-tag attribute groups: 3 group files declaring 3 groups and 82 attribute definitions, applied across 272 tag applications. A cross-tag group is a named set of attributes injected into every tag that declares the group, so a global attribute set is defined once. Cross-tag attributes MUST merge with, not replace, a tag's own attributes; a name collision resolves in favour of the tag's own definition and MUST emit a load-time diagnostic.

**[STU-WEB-025] Tag-name and attribute-name case.** Tag matching is case-insensitive for HTML-family document types and case-sensitive for XML-family types (`xml`, `svg`, `xslt_page`, `xslt_fragment`, `wml`, `tld`, `edml`). Attribute-name case follows the definition's `case_sensitive` flag, overridden by `preserveattrcase` on the tag.

**[STU-WEB-026] Property-inspector coverage.** 265 of the 470 tag definitions declare a property-inspector surface. Studio MUST resolve that surface through the contextual binding contract of 14.28.2, and a tag with no declared inspector MUST fall back to the generic attribute-table inspector rather than showing nothing.

**[STU-WEB-027] Third-party tag registration.** Studio MUST accept operator- or plugin-registered tag definitions that extend the library set without editing a shipped library. A registered library declares the same record as [STU-WEB-017] and participates in resolution, code hinting, colouring and validation on equal terms. Registration is a plugin capability under [STU-AUT-018] and is consent-gated.

**[STU-WEB-028] Source formatting command.** Studio MUST expose `web.format_source` operating over a document or a selection, driven entirely by [STU-WEB-020] plus an operator-configurable indent character, indent width, line-length target and attribute-wrap policy. It is a `StudioCommand`: dry-runnable (returns the diff), receipted and undoable as one operation. It MUST be idempotent — formatting already-formatted text produces byte-identical output.

**[STU-WEB-029] Code-comment commands.** Studio MUST expose `web.apply_comment` and `web.remove_comment` resolving the comment syntax from the language at the cursor, including inside an embedded block (a script block inside HTML uses the script language's comment syntax, not HTML's). Embedded-block boundaries come from the colouring scheme's declared block delimiters ([STU-WEB-047]).

---

### 14.28.2 Contextual property-panel binding (Studio-wide contract)

**[STU-WEB-030] Anchor-placement caveat and scope.** This block is a STUDIO-WIDE shell contract. It governs how ANY Studio panel — raster, vector, layout, typography, colour, effects, design-system, prototyping, whiteboard, video, and web — is selected for the current selection. It is anchored under `[STU-WEB-NNN]` because that is the prefix allocated to this module and because its entire evidence basis is this domain's sixty property inspectors and their binding declarations. When section 14.21 (operator unification surface) is next revised, these fifteen clauses MUST be re-anchored under an operator-allocated shell prefix and this block replaced by a cross-reference. Until then, `[STU-WEB-030]` through `[STU-WEB-044]` are the citable anchors for the contextual panel mechanism across all of Studio.

**[STU-WEB-031] The rejected alternative.** Studio MUST NOT adopt a whole-application persona, mode or workspace toggle that swaps the entire tool and panel set as a unit. The operator rejected that model. Studio's panel surface changes because the SELECTION changed, not because the operator switched personas. Named layout presets ([STU-UNI-002]) may change panel PLACEMENT and tool prominence; they MUST NOT change which panel is correct for a given selection.

**[STU-WEB-032] A panel declares its own binding.** Every contextual panel is registered with a declarative binding record. The shell resolves panels by evaluating those records against the current selection. A panel MUST NOT be selected by a hard-coded switch statement over document mode or layer kind anywhere in the shell.

**[STU-WEB-033] Panel binding record (normative shape).** A contextual panel declares exactly:

| Field | Type | Required | Semantics |
|---|---|---|---|
| `panel_id` | stable string | yes | unique across the registry |
| `binds_to` | selector expression ([STU-WEB-034]) | yes | what selection this panel claims |
| `priority` | integer ([STU-WEB-036]) | yes | lower wins |
| `selection_scope` | token ([STU-WEB-035]) | yes | how the selection must relate to the matched node |
| `document_types` | array of `type_id`, or `*` | yes | which document types this panel is eligible in |
| `discriminator` | `{field, value}` or null | no | secondary match on a field of the matched node ([STU-WEB-037]) |
| `context_requirement` | key/value map or null | no | additional runtime context that must hold ([STU-WEB-038]) |
| `render_flags` | set of tokens ([STU-WEB-039]) | no | chrome hints |
| `author_id_prefix` | stable string | yes | the AccessKit / Argus id namespace for every control this panel renders ([STU-WEB-043]) |

**[STU-WEB-034] Selector expression grammar (normative, closed).** `binds_to` is exactly one of:

| Form | Matches |
|---|---|
| a literal node-kind or tag name, e.g. `video` | that kind or tag |
| a pipe alternation, e.g. `input\|div` | any of the listed kinds or tags |
| `*` | any selection |
| `*LOCKED*` | a region the document model marks as locked or generated ([STU-WEB-094]) |
| `*COMMENT*` | a comment node |
| a namespaced name, e.g. `template:editable` or `xsl:for-each` | that namespaced element |

Alternation members are matched with the case rule of [STU-WEB-025]. No other form is legal; regular expressions and script predicates are NOT part of the grammar, because binding resolution must be statically analysable ([STU-WEB-042]).

**[STU-WEB-035] Selection-scope enumeration (normative, closed).** Exactly three members:

| Token | The panel claims the selection when |
|---|---|
| `exact` | the selected node itself matches `binds_to` |
| `within` | the selection is inside a node matching `binds_to`, at any depth |
| `inside_locked` | the selection is inside a node matching `binds_to` AND that node is a locked or generated region |

`exact` beats `within` at equal priority; `inside_locked` beats both at equal priority ([STU-WEB-040]).

**[STU-WEB-036] Priority band enumeration (normative).** `priority` is an integer; LOWER wins. Studio MUST reserve exactly these five bands and MUST NOT introduce a sixth without amending this clause:

| Band | Reserved for | Example |
|---|---|---|
| 1 | generated, locked or server-owned constructs whose editing surface must pre-empt the underlying element | a server behaviour's parameter inspector over its host element |
| 5 | the built-in inspector for a specific element or node kind | the video-element inspector |
| 6 | a discriminated refinement of a band-5 panel | the viewport-meta inspector, which refines the generic meta inspector |
| 10 | template-construct inspectors | the editable-region inspector |
| 50 | component and widget inspectors, including plugin-supplied ones | a plugin's carousel inspector |

The shipped evidence uses exactly these five values across sixty inspectors; the band meanings above are Studio's normative reading of them.

**[STU-WEB-037] Discriminator.** A `discriminator` narrows a match on a field of the matched node — for example `binds_to: meta` with `discriminator: {field: "name", value: "viewport"}`. When two panels bind the same `binds_to` and one carries a discriminator, the discriminated panel MUST declare a lower-winning priority (typically band 6 over band 5) so the refinement pre-empts the generic panel. A discriminated panel that does NOT outrank its generic sibling is a registry defect and MUST be reported at registration, not at selection time.

**[STU-WEB-038] Context requirement.** `context_requirement` gates a panel on runtime context beyond the node — for example a server-model requirement (`{server_model: "coldfusion"}`) so a server-specific inspector never appears in a document whose server model differs. Context keys are drawn from a closed set: `server_model`, `document_mode`, `capability` (a consent-granted capability name), and `feature_flag`. An unmet context requirement removes the panel from the candidate set entirely; it MUST NOT render disabled.

**[STU-WEB-039] Render-flag enumeration (normative, closed).** Exactly five members: `horizontal_rule` (draw a separator above this panel's group), `vertical_rule` (draw a separator between its two control columns), `hide_class_control` (suppress the shell's generic class/style control because this panel supplies its own), `disabled_in_live_view` (the panel's controls are read-only while the live rendered view is active), and `full_width` (the panel occupies the full inspector width rather than the two-column default). Render flags are presentation only and MUST NOT affect resolution.

**[STU-WEB-040] Resolution algorithm (normative, deterministic).** Given a selection, the shell MUST:

1. Build the candidate set: every registered panel whose `document_types` admits the current document type, whose `context_requirement` is satisfied, and whose `binds_to` matches under its `selection_scope`.
2. Sort candidates by `priority` ascending; break ties by `selection_scope` in the order `inside_locked` < `exact` < `within`; break remaining ties by presence of a `discriminator` (discriminated first); break remaining ties by `panel_id` lexicographic ascending.
3. The first candidate is the PRIMARY panel. Remaining candidates whose priority band differs from the primary's are SECONDARY panels and are rendered, in sorted order, below the primary.
4. Candidates in the SAME band as the primary that are not the primary are SUPPRESSED and MUST be listed in the resolution receipt ([STU-WEB-041]).

Step 2's final tiebreak on `panel_id` exists so resolution is total and deterministic: two panels can never be equally ranked, and the outcome can never depend on registration order, map iteration order or timing.

**[STU-WEB-041] Resolution receipt.** Every resolution MUST produce a machine-readable receipt: the selection identity, the candidate set with each candidate's priority, scope, discriminator and match reason, the chosen primary, the ordered secondaries, and the suppressed set with the reason for each suppression. The receipt MUST be readable by a model through the command surface, because "why is this panel showing" is otherwise unanswerable without a screenshot. The receipt is diagnostic output, not authority.

**[STU-WEB-042] Registry static analysis.** Because [STU-WEB-034] admits no script predicates, the whole registry is statically analysable. Studio MUST run a registration-time check that reports: two panels with identical `(binds_to, selection_scope, priority, discriminator, document_types)` (an unresolvable-by-design tie, resolved only by the `panel_id` tiebreak and therefore a design defect); a discriminated panel that does not outrank its generic sibling ([STU-WEB-037]); a panel whose `binds_to` names a tag or node kind that exists in no registered library or primitive set; and a panel whose `author_id_prefix` collides with another's. These are registry defects reported at load, never at selection.

**[STU-WEB-043] Model-visibility obligation.** Every contextual panel MUST expose, through the command surface: its binding record; its resolved state for the current selection; the ordered list of its controls with each control's stable `author_id` (built from `author_id_prefix`), kind, current value, value domain and enabled state; and a typed setter per control. A model MUST be able to read what panel is showing, why, and what it can change, without a screenshot ([STU-CON-007], 14.16).

**[STU-WEB-044] Panel lifecycle hooks (normative, closed set of four).** A panel implementation declares exactly four hooks, and the shell MUST NOT call anything else on it: `can_claim(selection) -> bool` (a cheap refinement AFTER declarative matching, never a replacement for it — a panel that claims nothing declaratively and everything in `can_claim` defeats [STU-WEB-042] and is a defect); `initialize_ui()` (build controls once); `inspect(selection)` (populate control values from the selection); and `apply(control_id, value)` (emit the `StudioCommand` that performs the edit). `apply` MUST NOT write authority directly; it emits a command that carries the receipt, undo entry and promotion path of [STU-AUT-002]. Hook coverage in the shipped evidence: 60 of 60 inspectors implement `inspect`, 60 implement `can_claim`, 33 implement `initialize_ui`, and 8 additionally declare a help target; Studio makes `inspect` and `apply` mandatory and the other two optional.

---

### 14.28.3 Code intelligence

**[STU-WEB-045] Colouring-scheme record (normative shape).** A syntax-colouring scheme carries `scheme_id`, `display_name`, `applies_to_document_types` (may be empty for schemes reachable only as an embedded block), `priority` (integer), `ignore_case` (boolean), `ignore_tags` (boolean), `keyword_lists[]`, `delimiters[]`, `embedded_block_delimiters[]`, `tag_specs[]`, and `sample_text`.

**[STU-WEB-046] Colouring-scheme priority.** Priority selects which scheme owns a text region when more than one applies; the shipped bands are 1 (plain text fallback), 5 (whole-document wrapper schemes such as templates and library items), 10 (comment schemes), 20 (embedded script and expression languages), and 50 (top-level document languages). Studio MUST resolve overlapping schemes by priority and MUST record the winning scheme per region so a model can ask what language a byte range is in.

**[STU-WEB-047] Embedded-block delimiters.** A scheme declares the delimiter pairs that open and close an embedded region owned by a different scheme. This is the mechanism by which a script block inside HTML is coloured, hinted, validated and commented as script rather than as markup. Delimiter resolution MUST be exact and MUST NOT be regex-guessed, so a language boundary is never ambiguous.

**[STU-WEB-048] Shipped colouring surface (sizing contract).** Studio MUST ship colouring for at minimum the languages the document-type set of [STU-WEB-013] declares. The shipped source baseline is 33 schemes carrying 46 keyword lists and 6,894 keywords, over 271 syntax token classes, with 15 theme definitions each supplying a default palette. Studio's own scheme count need not match, but the loader, the tokeniser and the theme surface MUST be specified and tested at that scale, and the largest single scheme in the baseline carries 3,522 keywords across six lists — that is the per-scheme scale the keyword matcher must handle without degrading typing latency.

**[STU-WEB-049] Token-class and theme contract.** A theme maps each of the declared syntax token classes to a colour. Token classes are the stable identifiers; theme files bind colours to them. Adding a language MUST NOT require touching every theme: an unmapped token class MUST fall back to a declared default class, and the fallback MUST be reported once per session, not silently.

**[STU-WEB-050] Code-hint record (normative shape).** A code hint entry carries `pattern` (the literal text that opens the hint, e.g. `background-clip:`), `additional_dismiss_chars` (string of characters that close the hint), `allow_whitespace_prefix` (boolean), `display_restriction` (a token naming the language context in which the hint may appear, e.g. `css`), `allow_multiple_values` (boolean), and `items[]` where each item is `{label, value, icon_id}`.

**[STU-WEB-051] Code-hint sizing and grouping.** Hints are organised into menu groups; the shipped baseline is 29 hint files, 22 menu groups and 4,811 hint items, plus 23 description files supplying per-item documentation, 10 built-in-code files and 2 content-management-system files. A hint item MUST be able to carry a description shown alongside the completion; a hint group with no descriptions is legal but MUST be flagged in the UserManual coverage report.

**[STU-WEB-052] Validator rule sets (normative, ten rows).** Studio MUST ship a markup validator driven by declarative rule sets, one per DTD context and version. The shipped baseline:

| Rule set | Tags | Attribute rules | Valid-value rules | Invalid rules | Context rules | Requirement sets |
|---|---|---|---|---|---|---|
| `validator` (union) | 327 | 4,124 | 4,537 | 121 | 325 | 85 / 86 |
| `html_all` | 106 | 1,768 | 2,593 | 65 | 106 | 14 / 14 |
| `xhtml10_transitional` | 89 | 1,603 | 2,774 | — | 88 | 13 / 13 |
| `xhtml10_frameset` | 91 | 1,623 | 2,696 | — | 90 | 13 / 13 |
| `xhtml10_strict` | 79 | 1,377 | 1,822 | — | 78 | 10 / 10 |
| `cfml` | 111 | 1,061 | 30 | 55 | 96 | 62 / 67 |
| `wml` | 31 | 96 | 30 | — | 31 | — |
| `smil` | 19 | 244 | 57 | 1 | 19 | 9 / 9 |
| `value_map` | — | 433 value keys | — | — | — | — |
| `versions` | 20 version keys | — | — | — | — | — |

**[STU-WEB-053] Validator record shapes (normative).** A validator rule set declares, per tag: a version-applicability record; an attribute list where each attribute names its permitted value key; tag options; a tag-context rule (which parents or ancestors the tag is legal in); explicit invalid combinations; and attribute-requirement sets (groups of attributes of which at least one, exactly one, or all must be present).

**[STU-WEB-054] Value-map record (normative shape).** The value map is the shared vocabulary of attribute value SHAPES referenced by every rule set: `{key_id, note, quote_rule, is_regex, type}` where `quote_rule` ∈ {`May Be Double`, `May Be Quoted`, `Must Be Double`, `Must Not Be Quoted`} and `type` is either a literal permitted value or, when `is_regex` is true, a regular expression the value must match. The shipped map is 433 keys. Studio MUST evaluate regex keys with a linear-time engine and MUST NOT admit a pattern whose evaluation is not bounded, because the value map is applied per attribute per keystroke in a live validator.

**[STU-WEB-055] Validator output contract.** Validation produces typed diagnostics, never a rewrite. Each diagnostic carries `severity` (`error` \| `warning` \| `info`), `rule_set_id`, `rule_id`, `byte_range`, `element_path`, a human message, and a machine `code`. Diagnostics MUST be readable through the command surface so a model can validate a document headlessly.

**[STU-WEB-056] Linter rule sets (normative, three languages with shipped defaults).** Studio MUST ship a configurable linter per language with these exact shipped default configurations. Every rule is operator-overridable per site and per document.

- **CSS — 35 rules.** Enabled by default: `duplicate-properties`, `floats`, `vendor-prefix`. Disabled by default (32): `important`, `adjoining-classes`, `known-properties`, `box-sizing`, `box-model`, `overqualified-elements`, `display-property-grouping`, `bulletproof-font-face`, `compatible-vendor-prefixes`, `regex-selectors`, `errors`, `duplicate-background-images`, `empty-rules`, `selector-max-approaching`, `gradients`, `fallback-colors`, `font-sizes`, `font-faces`, `star-property-hack`, `outline-none`, `import`, `ids`, `underscore-property-hack`, `rules-count`, `qualified-headings`, `selector-max`, `shorthand`, `text-indent`, `unique-headings`, `universal-selector`, `unqualified-attributes`, `zero-units`.
- **Markup — 23 rules.** Enabled by default (8): `tagname-lowercase`, `attr-value-double-quotes`, `attr-no-duplication`, `doctype-first`, `tag-pair`, `spec-char-escape`, `id-unique`, `src-not-empty`, `alt-require`. Disabled by default (15): `attr-lowercase`, `attr-value-not-empty`, `tag-self-close`, `title-require`, `head-script-disabled`, `doctype-html5`, `id-class-value`, `style-disabled`, `inline-style-disabled`, `inline-script-disabled`, `space-tab-mixed-disabled`, `id-class-ad-disabled`, `href-abs-or-rel`, `attr-unsafe-chars`.
- **Script — 69 options.** Numeric defaults: `maxerr` = 50, `indent` = 4. Enforcing options true by default: `bitwise`, `curly`, `eqeqeq`, `forin`, `freeze`, `noarg`, `nonbsp`, `undef`, `unused`, `strict`. Environment options true by default: `browser`, `devel`, `worker`, plus the two library environments the baseline enables. Every remaining option is false by default, and the five limit options (`maxparams`, `maxdepth`, `maxstatements`, `maxcomplexity`, `maxlen`) are unset rather than zero — unset means "no limit", and zero means "limit of zero", which are different.

**[STU-WEB-057] Script-language level configurations.** Studio MUST ship at least four script-language level configurations selectable per site: language levels 3, 5, 6 and the current level, each declaring `ecma_version`, `source_type` (`module`), and JSX support. The shipped baseline enables the browser and one library environment in every level and sets `constructor-super` to error while defaulting every other rule off; Studio MUST preserve "default off, opt in per site" as the posture so an imported site does not drown in diagnostics.

**[STU-WEB-058] Linter output contract.** Linter diagnostics use the same shape as [STU-WEB-055] with `rule_set_id` naming the language and `rule_id` naming the rule. Linting MUST be runnable over a selection, a document, or an entire site as a batch job on the batch runner ([STU-AUT-012]) under the headless/quiet law (14.20).

**[STU-WEB-059] Tag-highlight and code-navigation surface.** Studio MUST provide: balance-braces, select-parent-tag, collapse-full-tag, collapse-selection, expand-all, indent, outdent, word-wrap toggle, and a code-navigator listing the CSS rules affecting the current selection. Each is a `StudioCommand` with a stable id, so all of them are model-invokable.

---

### 14.28.4 The CSS property surface

**[STU-WEB-060] CSS authoring surface structure.** Studio's CSS authoring surface is a four-level structure: SOURCE (which stylesheet), MEDIA (which `@media` query, [STU-WEB-076]), SELECTOR (which rule), PROPERTIES (which declarations). Every level is addressable by the command surface; a model MUST be able to create a rule in a named media query in a named stylesheet in one call.

**[STU-WEB-061] Property category set (normative, five rows).** The property surface is grouped into exactly five categories:

| category_id | Label | Property slots |
|---|---|---|
| `layout` | Layout | 22 |
| `text` | Text | 22 |
| `border` | Border | 4 top-level (expanding to the per-side family) |
| `background` | Background | 9 |
| `more` | More | unbounded — free-form property/value entry for anything outside the curated set |

The `more` category is mandatory: the curated set is a fast path, never a ceiling. Any property the CSS parser accepts is authorable there.

**[STU-WEB-062] Curated property set (normative, 82 entries).** Studio MUST ship the curated property surface as exactly 82 property entries carrying 924 ordered option entries. Each entry is `{property, display_name, control_type, default_value, supports_negative_values, option_count, options[]}`. `options` is ORDERED and MAY contain group separators; separators are rendered as dividers and are not selectable values.

**[STU-WEB-063] Control-type enumeration (normative, closed, fourteen members).** Exactly: `Menu` (closed picker), `MenuAndHotText` (unit picker plus scrubbable number), `DoubleMenuAndHotText` (two such pairs, e.g. `background-position`), `HotTextValue` (scrubbable number with a keyword alternative), `RangeHotText` (scrubbable number over a bounded range), `MenuAndUrl` (picker plus asset URL), `FontMenu` (font-stack picker), `ColorWellWithTextEdit` (colour well plus text entry), `GradientWellWithTextEdit` (gradient well plus text entry), `GroupedPictureButton` (segmented icon buttons), `GroupedBoxControls` (four-side box editor), `GroupedBorderControls` (per-side border editor), `CompositeControlLayout` (one property composed of several sub-controls), `MultiControlLayout` (a repeating multi-value property such as `box-shadow`).

**[STU-WEB-064] Length-unit vocabulary (normative, closed, fifteen members).** Every length-bearing CSS control offers exactly these units, in this order: `px`, `pt`, `pc`, `in`, `cm`, `mm`, `%`, `em`, `rem`, `ex`, `ch`, `vw`, `vh`, `vmin`, `vmax`. A control that omits units (`text-indent`, `line-height`, `letter-spacing`, `word-spacing`, `vertical-align`, `border-width` and the shadow offsets) offers the first eleven only; the four viewport units are offered on the sizing, spacing, radius and grid controls. Resolution units are a separate closed set of three: `dpi`, `dpcm`, `dppx`.

**[STU-WEB-065] Layout-category property contracts (normative).**

| Property | control_type | default | negative | Options / notes |
|---|---|---|---|---|
| `width`, `height` | MenuAndHotText | `auto` | no | `auto` plus the fifteen units |
| `min-width`, `min-height` | MenuAndHotText | `0px` | no | fifteen units, no keyword |
| `max-width`, `max-height` | MenuAndHotText | `none` | no | `none` plus fifteen units |
| `display` | Menu | `inline` | — | `inherit`, `none`, `block`, `list-item`, `inline`, `inline-block`, `inline-table`, `table`, `table-caption`, `table-cell`, `table-column`, `table-column-group`, `table-footer-group`, `table-header-group`, `table-row`, `table-row-group`, `run-in`, `compact`, `marker` (19) |
| `box-sizing` | Menu | `content-box` | no | `content-box`, `border-box`, `inherit` |
| `visibility` | Menu | `visible` | — | `inherit`, `visible`, `hidden`, `collapse` |
| `float` | GroupedPictureButton | unset | — | `left`, `right`, `none` |
| `clear` | GroupedPictureButton | unset | — | `left`, `right`, `both`, `none` |
| `overflow-x`, `overflow-y` | Menu | `visible` | — | `visible`, `hidden`, `scroll`, `auto`, `no-content`, `no-display` |
| `position` | Menu | `static` | — | `static`, `absolute`, `fixed`, `relative` |
| `z-index` | HotTextValue | `auto` | — | `auto` or an integer |
| `opacity` | RangeHotText | `1` | — | see [STU-WEB-066] |
| `margin` | GroupedBoxControls | unset | yes | `auto` plus fifteen units, per side |
| `padding` | GroupedBoxControls | unset | no | `auto` plus fifteen units, per side |
| `top`, `right`, `bottom`, `left` | GroupedBoxControls | unset | yes | `auto` plus fifteen units, per side |

**[STU-WEB-066] `opacity` parameter contract.**

*Derivation: parameter table taken whole; yields 1 microtask whose acceptance criteria are its seven bound fields, each stored separately with unknown preserved.*
| Field | Value |
|---|---|
| hard_min | 0 |
| hard_max | 1 |
| soft_min | 0 |
| soft_max | 1 |
| default | 1 |
| unit | dimensionless ratio |
| precision | 2 decimal places |
| step / coarse_step / fine_step | 0.01 / 0.1 / 0.001 |

**[STU-WEB-067] Generic length-control parameter contract.** Every `MenuAndHotText`, `DoubleMenuAndHotText`, `GroupedBoxControls` and `GroupedBorderControls` numeric field follows this contract unless a per-property clause overrides it:

*Derivation: parameter table taken whole; yields 1 microtask whose acceptance criteria are its seven bound fields, each stored separately with unknown preserved.*
| Field | Value |
|---|---|
| hard_min | 0 when `supports_negative_values` is false, otherwise NOT DECLARED IN SOURCE (Studio declares -100000, labelled `studio_declared`) |
| hard_max | NOT DECLARED IN SOURCE (Studio declares 100000, labelled `studio_declared`) |
| soft_min | 0, or -1000 when negatives are permitted |
| soft_max | 1000 |
| default | the property's declared default from [STU-WEB-065] / [STU-WEB-068] / [STU-WEB-069] |
| unit | the selected member of [STU-WEB-064]; the unit is a SEPARATE stored field, never fused into the numeric value |
| precision | 3 decimal places |
| step / coarse_step / fine_step | 1 / 10 / 0.1 |

The CSS source does not declare numeric bounds on any of these properties; every bound above that is not 0 is Studio-declared and MUST be labelled as such. `supports_negative_values` IS declared per property and MUST be honoured: it is false on `width`, `height`, `min-*`, `max-*`, `box-sizing`, `padding`, `font-size`, `border-width` and every per-side border width, `border-radius`, `border-spacing`, and the blur components of `box-shadow` and `text-shadow`.

**[STU-WEB-068] Text-category property contracts (normative).**

| Property | control_type | default | Options |
|---|---|---|---|
| `color` | ColorWellWithTextEdit | undefined | colour, bound to an explicit `StudioColorProfile` |
| `font-family` | FontMenu | `default font` | `inherit` plus the resolved font stacks |
| `font-style` | Menu | `normal` | `normal`, `italic`, `oblique` |
| `font-variant` | Menu | `normal` | `normal`, `small-caps` |
| `font-weight` | Menu | `normal` | `normal`, `bold`, `bolder`, `lighter`, `100`…`900` (13) |
| `font-size` | MenuAndHotText | `medium` | eleven units plus `xx-small`, `x-small`, `small`, `medium`, `large`, `x-large`, `xx-large`, `smaller`, `larger` |
| `line-height` | MenuAndHotText | `normal` | eleven units plus `normal` |
| `letter-spacing`, `word-spacing` | MenuAndHotText | `normal` | `normal` plus eleven units |
| `text-indent` | MenuAndHotText | `0px` | `px`, `pt`, `pc`, `in`, `cm`, `mm`, `%` |
| `white-space` | Menu | `normal` | `normal`, `nowrap`, `pre`, `pre-line`, `pre-wrap` |
| `vertical-align` | MenuAndHotText | `baseline` | eleven units plus `baseline`, `sub`, `super`, `top`, `text-top`, `middle`, `bottom`, `text-bottom` |
| `text-align` | GroupedPictureButton | unset | `left`, `center`, `right`, `justify` |
| `text-decoration` | GroupedPictureButton | unset | `none`, `underline`, `overline`, `line-through` |
| `text-transform` | GroupedPictureButton | unset | `none`, `capitalize`, `uppercase`, `lowercase` |
| `list-style-type` | Menu | undefined | `none`, `armenian`, `circle`, `cjk-decimal`, `decimal`, `decimal-leading-zero`, `disc`, `georgian`, `hebrew`, `hiragana`, `hiragana-iroha`, `katakana`, `katakana-iroha`, `lower-alpha`, `lower-greek`, `lower-latin`, `lower-roman`, `square`, `upper-alpha`, `upper-latin`, `upper-roman` (21) |
| `list-style-position` | GroupedPictureButton | unset | `inside`, `outside` |
| `list-style-image` | MenuAndUrl | `none` | `url`, `none` |
| `text-shadow` | MultiControlLayout | unset | see [STU-WEB-070] |

**[STU-WEB-069] Border and background property contracts (normative).**

- `border` is a `GroupedBorderControls` composite over four sides, each side carrying width / style / colour. Per-side width: `MenuAndHotText`, default `medium`, options `thin`, `medium`, `thick` plus eleven units, negatives forbidden. Per-side style: `Menu`, default `none`, options `none`, `dotted`, `dashed`, `solid`, `double`, `groove`, `ridge`, `inset`, `outset`, `hidden` (10). Per-side colour: `ColorWellWithTextEdit`.
- `border-radius` is `GroupedBoxControls` over four corners; fifteen units; negatives forbidden; no shorthand default.
- `border-collapse` is `GroupedPictureButton` with `collapse`, `separate`.
- `border-spacing` is `DoubleMenuAndHotText`, default `0px`, fifteen units, negatives forbidden.
- `background-color` is `ColorWellWithTextEdit`.
- `background-image` is a `MultiControlLayout` of two alternative editors: a `MenuAndUrl` (default `url`) and a `GradientWellWithTextEdit` (default `none`); both accept `url` or `none`.
- `background-position` is `DoubleMenuAndHotText`, default `0% 0%`; horizontal options `left`, `right`, `center` plus eleven units; vertical options `top`, `bottom`, `center` plus eleven units.
- `background-size` is `DoubleMenuAndHotText`, default `auto auto`; options `auto`, `cover`, `contain` plus eleven units.
- `background-clip` is `Menu`, default `border-box`, options `padding-box`, `border-box`, `content-box`.
- `background-origin` is `Menu`, default `padding-box`, same three options.
- `background-repeat` is `GroupedPictureButton`, default `repeat`, options `repeat`, `repeat-x`, `repeat-y`, `no-repeat`.
- `background-attachment` is `Menu`, default `scroll`, options `scroll`, `fixed`.

**[STU-WEB-070] Shadow property contracts.** `box-shadow` is a `MultiControlLayout` of six sub-controls: `h-shadow` (default `0px`), `v-shadow` (default `0px`), `blur` (default `0px`, negatives forbidden), `spread` (default `0px`), `color` (`ColorWellWithTextEdit`), and `inset` (`GroupedPictureButton`, single option `inset`, default off). `text-shadow` is the same minus `spread` and `inset`. Each numeric sub-control offers the eleven non-viewport units and follows [STU-WEB-067]. Multiple shadow layers are supported; the layer array is ordered and order is authority.

**[STU-WEB-071] Recognised property vocabulary.** Beyond the curated 82, the CSS engine MUST recognise the shipped vocabulary of 369 property names for hinting, colouring and validation, and MUST accept any syntactically valid declaration through the `more` category ([STU-WEB-061]). Recognition is for assistance; it MUST NOT gate authoring.

**[STU-WEB-072] Vendor-prefix emission rules.** Studio MUST support declarative vendor-prefix rules: per property, per value, a set of `{target_vendor, emitted_property, emitted_value}` triples, plus a delete/disable sentinel that removes the emitted prefixes when the base declaration is removed. The shipped baseline declares one such property with four value rules; the mechanism, not the count, is normative. Prefix emission MUST be reversible: removing the base declaration removes its emitted prefixes in the same edit.

**[STU-WEB-073] Transition authoring vocabulary (normative, four closed lists).**

- **Animatable properties (49):** `background-color`, `background-image`, `background-position`, `border-bottom-color`, `border-bottom-width`, `border-color`, `border-left-color`, `border-left-width`, `border-right-color`, `border-right-width`, `border-spacing`, `border-top-color`, `border-top-width`, `border-width`, `bottom`, `color`, `crop`, `font-size`, `font-weight`, `height`, `left`, `letter-spacing`, `line-height`, `margin-bottom`, `margin-left`, `margin-right`, `margin-top`, `max-height`, `max-width`, `min-height`, `min-width`, `opacity`, `outline-color`, `outline-offset`, `outline-width`, `padding-bottom`, `padding-left`, `padding-right`, `padding-top`, `right`, `text-indent`, `text-shadow`, `top`, `transform`, `vertical-align`, `visibility`, `width`, `word-spacing`, `z-index`.
- **Pseudo-classes (8):** `active`, `checked`, `disabled`, `enabled`, `focus`, `hover`, `indeterminate`, `target`.
- **Timing functions (6):** `linear`, `ease`, `ease-in`, `ease-out`, `ease-in-out`, `cubic-bezier(x1,y1,x2,y2)`.
- **Vendor prefixes for transitions (2):** `-webkit-`, `-o-`.

**[STU-WEB-074] Gradient editor contract.** The gradient editor's saved-swatch strip holds at most 5 entries. That is a shipped numeric constant, not a Studio judgement; Studio MAY raise it but MUST state the chosen value in the UserManual and MUST NOT silently differ. The gradient stop editor itself is the Studio gradient primitive (`StudioGradient`, 14.8), shared with every other domain — there is no web-only gradient model.

**[STU-WEB-075] SVG export defaults for web output.** When Studio writes SVG into a web document, the defaults are: `trim_to_art_bounds` = true, `responsive` = true, `decimal_precision` = 1. `decimal_precision` contract: hard_min 0; hard_max NOT DECLARED IN SOURCE (Studio declares 10); soft_min 0; soft_max 6; default 1; unit = decimal places; precision integer.

---

### 14.28.5 Responsive authoring: media queries and breakpoints

**[STU-WEB-076] Media-query record (normative shape).** A media query MUST be stored as `{media_type?, conditions[], rules[]}` where each condition MUST be `{feature, operator, value, unit?}`. `media_type` is drawn from [STU-WEB-077]; `feature` from [STU-WEB-078].

**[STU-WEB-077] Media-type enumeration (normative, closed, eight members).** `screen`, `print`, `handheld`, `aural`, `braille`, `projection`, `tty`, `tv`.

**[STU-WEB-078] Media-feature set (normative, closed, twenty-three members with their control kinds and value domains).**

| Feature | Control kind | Value domain |
|---|---|---|
| `media` | popup | the eight media types of [STU-WEB-077] |
| `orientation` | popup | `landscape`, `portrait` |
| `min-width`, `max-width`, `width` | number + unit | the fifteen units of [STU-WEB-064] |
| `min-height`, `max-height`, `height` | number + unit | the fifteen units |
| `min-device-width`, `max-device-width`, `device-width` | number + unit | the fifteen units |
| `min-device-height`, `max-device-height`, `device-height` | number + unit | the fifteen units |
| `min-resolution`, `max-resolution`, `resolution` | number + unit | `dpi`, `dpcm`, `dppx` |
| `min-aspect-ratio`, `max-aspect-ratio`, `aspect-ratio` | ratio | two positive integers |
| `min-device-aspect-ratio`, `max-device-aspect-ratio`, `device-aspect-ratio` | ratio | two positive integers |

**[STU-WEB-079] Visual breakpoint ruler contract (normative numeric constants).** Studio MUST provide a visual breakpoint ruler over the live view. Its constants are shipped values, not judgements:

| Constant | Value | Meaning |
|---|---|---|
| `min_width_media_query` | 7 | smallest authorable `min-width` breakpoint, in px |
| `max_width_media_query` | 5000 | largest authorable `max-width` breakpoint, in px |
| `min_range_value` | 5 | smallest authorable span between the two edges of a min-and-max range query, in px |
| `min_content_width` | 80 | smallest rendered content width the ruler will let the operator drag to, in px |
| `min_handle_width` | 20 | smallest draggable handle width, in px |
| `upper_limit` | 9999.999 | largest numeric value accepted in a breakpoint field |
| `max_decimal_digits` | 3 | decimal places accepted in a breakpoint field |
| `max_digits` | 8 | total digits accepted in a breakpoint field |
| `ruler_height` | 16 | ruler band height, in px |
| `add_button_width` | 17 | width of the add-breakpoint control, in px |
| `add_guide_width` | 300 | width of the add-breakpoint guide overlay, in px |
| `pixel_adjust` | 2 | pixel adjustment applied when converting a drag position to a breakpoint value |
| `border_correction` | 4 | pixel correction for the ruler's own border |

**[STU-WEB-080] Breakpoint parameter contract.**

*Derivation: parameter table taken whole; yields 1 microtask whose acceptance criteria are its seven bound fields, each stored separately with unknown preserved.*
| Field | Value |
|---|---|
| hard_min | 7 |
| hard_max | 5000 |
| soft_min | 320 |
| soft_max | 1920 |
| default | none (a new breakpoint takes the current viewport width) |
| unit | px |
| precision | 3 decimal places, at most 8 total digits |
| step / coarse_step / fine_step | 1 / 10 / 0.001 |

**[STU-WEB-081] Breakpoint-kind enumeration (normative, closed, three members).** `min_width`, `max_width`, `min_and_max_width`. A `min_and_max_width` breakpoint carries two values whose difference MUST be at least `min_range_value` ([STU-WEB-079]); a smaller span is a validation error, not a clamp.

**[STU-WEB-082] Responsive-framework binding record (normative shape).** Studio MUST support declaratively described CSS grid frameworks so a document authored against one is editable visually. A framework descriptor carries: `framework_name`, `version_supported_from`, `version_detect_pattern`, `breakpoint_count`, `grid_class_prefixes` (ordered), `media_query_detect_pattern`, `container_class`, `fluid_container_class`, `row_class`, `column_class_patterns` (ordered, one per prefix), `offset_class_patterns` (ordered), `hide_class_patterns` (ordered), `default_breakpoint_min_widths` (ordered), and `total_columns`.

**[STU-WEB-083] Shipped framework descriptors (normative, three major versions).**

| Version family | Breakpoints | Class prefixes | Default breakpoint min-widths (px) | Total columns |
|---|---|---|---|---|
| v3 family | 3 | `xs`, `sm`, `md`, `lg` | 768, 992, 1200 | 12 |
| v4 family | 4 | `xs`, `sm`, `md`, `lg`, `xl` | 576, 768, 992, 1200 | 12 |
| v5 family | 5 | `xs`, `sm`, `md`, `lg`, `xl`, `xxl` | 576, 768, 992, 1200, 1400 | 12 |

The v3 family declares column classes of the form `col-{prefix}-{n}`, offsets `col-{prefix}-offset-{n}`, and hide classes `hidden-{prefix}`. The v4 and v5 families declare `col-{n}` and `col-{prefix}-{n}`, offsets `offset-{n}` and `offset-{prefix}-{n}`, and display-utility hide pairs of the form `d-none d-{prefix}-block`. Each family ships a bundle path, a stylesheet path, a script path, and a helper-script path; the v4 and v5 families additionally ship a positioning-helper script.

**[STU-WEB-084] Grid parameter contracts (shipped bounds).**

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| number of columns | 1 | 24 | 6 | 12 | 12 | count | integer |
| gutter width | 1 | 120 | 8 | 40 | 30 | px | integer |

These four bounds are DECLARED in the source and MUST NOT be relabelled Studio-declared.

**[STU-WEB-085] Framework version pinning.** A site records the exact framework version it targets (the shipped baseline pins three: `3.4.1`, `4.4.1`, `5.3.8`) plus its helper-library version (baseline `3.7.1`) and, for v4 and v5, its positioning-helper version (baseline `2.11.8` for v5). Version identity is authority: a visual edit MUST emit classes valid for the pinned version, and switching versions is an explicit migration command with a receipt naming every rewritten class.

**[STU-WEB-086] Preprocessor support.** Studio MUST author, compile and source-map the three preprocessor document types of [STU-WEB-013] (`less`, `scss`, `sass`). A site declares its preprocessor source root, output root, output style and source-map policy. Compilation runs on the batch runner ([STU-AUT-012]) as a headless bounded job, and a compile failure MUST produce a typed diagnostic ([STU-WEB-055]) rather than a partial stylesheet write. The shipped preprocessor baseline is 107 framework files across three bundled mixin libraries; Studio MUST be able to resolve an import graph of at least that size.

**[STU-WEB-087] Live-view responsive preview.** The live rendered view MUST be resizable to an arbitrary width, MUST show which media queries are active at that width, and MUST let the operator jump to any declared breakpoint. Width and active-query state MUST be readable and settable through the command surface so a model can verify a responsive layout at a named width without operating a window ([STU-CON-007]).

**[STU-WEB-088] Design-to-web breakpoint binding.** Where a document is generated from a design-system source ([STU-WEB-135]), each design breakpoint MUST map to exactly one media query and the mapping MUST be recorded on the document. An unmapped design breakpoint is a generation-receipt warning naming the breakpoint.

**[STU-WEB-089] Viewport meta contract.** An HTML document MUST be able to declare a viewport meta element through a dedicated inspector bound per [STU-WEB-037] (`binds_to: meta`, discriminator `{name: viewport}`, priority band 6). A responsive document with no viewport declaration is a lint warning, not an error.

---

### 14.28.6 Templates, library items and snippets

**[STU-WEB-090] Template document contract.** A template document is a `StudioDocument` of a template document type ([STU-WEB-013] rows 23, 33–39). It declares regions that instance documents may edit; everything else is locked in the instance. Studio MUST support nested templates: a template may itself be based on another template, and region locking composes.

**[STU-WEB-091] Region-kind enumeration (normative, closed, five members).**

| Kind | Behaviour in an instance |
|---|---|
| `editable` | content is freely editable |
| `optional` | the whole region may be shown or hidden per instance |
| `editable_optional` | both: shown/hidden and, when shown, editable |
| `repeating` | the region may be duplicated, reordered and deleted per instance |
| `repeating_table` | a repeating region whose repeat unit is a table row |

**[STU-WEB-092] Template expression contract.** A template may carry expressions evaluated when an instance is generated. Two forms exist and MUST be distinguished on the document: an evaluated expression, and a pass-through expression that is emitted verbatim into the instance for later server-side evaluation. Both carry a single embedded code string. Confusing the two silently corrupts server-side pages, so the distinction is authority, not presentation.

**[STU-WEB-093] Template update propagation.** Editing a template updates every instance that references it, in one transaction per instance with a receipt naming each changed byte range. An instance whose locked region has drifted (because the file was edited outside Studio) MUST be reported and MUST NOT be silently overwritten; the operator or model chooses reapply-template or detach.

**[STU-WEB-094] Locked-region model.** An instance's non-editable content is a LOCKED region. Locked regions are the `*LOCKED*` selector target of [STU-WEB-034] and the `inside_locked` selection scope of [STU-WEB-035]. Studio MUST refuse a text edit inside a locked region with a typed error naming the region and the owning template, and MUST offer detach as the remedy.

**[STU-WEB-095] Template-safe behaviours.** A behaviour or object insertion that would write into the document head or into a locked region is refused inside a template instance unless the behaviour declares itself template-safe ([STU-PRO-137]). This is a hard gate, not a warning.

**[STU-WEB-096] Library item contract.** A library item is a reusable markup fragment stored as its own document (`library_item` type, `.lbi`) and referenced by instances. Editing the item updates every reference. Library items and templates are different mechanisms and MUST NOT be merged: a template owns a whole document's structure, a library item is a fragment. A library item is a WEB-DOMAIN primitive and is distinct from a `StudioComponent` ([STU-DS-102]); the design-to-web pipeline of [STU-WEB-135] maps a component to a library item or to a framework component, and the mapping is recorded.

**[STU-WEB-097] Snippet record (normative shape).** `{snippet_name, description, folder_path, top_level_group, snippet_kind, preview_mode, inserts_before_selection, inserts_after_selection}`. `snippet_kind` ∈ {`block`, `wrap`}: a `block` snippet inserts at the cursor and its `inserts_after_selection` is emitted immediately after the before-text; a `wrap` snippet wraps the current selection between the two texts. `preview_mode` ∈ {`code`, `design`}. Insert texts are VERBATIM: whitespace, newlines and indentation are part of the snippet and MUST be preserved exactly.

**[STU-WEB-098] Snippet library sizing and grouping.** The shipped baseline is 647 snippets in 9 top-level groups: a framework-component group of 501 across three version subtrees, a CSS-animation-and-transition group of 12, a CSS-effects group of 17, a CSS group of 13, a markup group of 19, a script group of 58 across twelve subfolders, a server-language group of 4, a preprocessor group of 20, and a responsive-design group of 3. 628 are `block` and 18 are `wrap` (one entry is unnamed and MUST be reported as a load diagnostic rather than dropped). Folder paths nest arbitrarily and are authority for picker organisation.

**[STU-WEB-099] Snippet keyboard triggers.** A snippet MAY declare a keyboard trigger word that expands it in the code view. The trigger map is a document-independent site resource. Triggers MUST be unique within a site; a duplicate is a registration error.

---

### 14.28.7 The Insert catalogue, behaviours and the event model

**[STU-WEB-100] Insert catalogue structure.** Studio MUST provide an Insert catalogue of authorable objects organised into categories. The shipped baseline is 12 categories holding 444 entries (378 buttons, 32 menu-buttons, 34 separators), backed by 358 object implementations of which 339 are wired into the catalogue and 19 are reachable only by command.

**[STU-WEB-101] Insert category set (normative, twelve rows).** `common_html` (40 entries), `form` (30), `templates` (7), `framework_components` (39), `mobile_components` (22), `ui_widgets` (11), `data` (28), `server_asp` (11), `server_cfml_basic` (15), `server_cfml_form` (10), `server_php` (9), `favorites` (operator-populated, ships empty). Category membership and entry order are authority for the panel.

**[STU-WEB-102] Object implementation record (normative shape).** `{object_id, folder, display_title, requires_document_model (boolean), inserts, dialog_controls[], api_hooks[], window_dimensions?}` where `inserts` is exactly one of: `literal_markup` (a verbatim string, optionally with named substitution points), `framework_component` (a key resolved against the pinned framework's component table), or `command_only` (the object performs an edit rather than inserting markup).

**[STU-WEB-103] Insertion-mode distribution (sizing contract).** In the shipped baseline, 268 objects insert literal markup, 7 use their own document body as the inserted markup, and 131 insert a framework component of which 129 resolve against the shipped framework resource files. The three mechanisms are all normative; an implementer must support all three, and the framework-component path must degrade with a typed diagnostic when the pinned framework version does not declare the requested key.

**[STU-WEB-104] Object API hooks (normative, closed set of four).** An object implementation declares at most: `requires_document_model()` (whether the object needs a parsed document), `object_markup()` (returns the markup to insert), `window_dimensions()` (the size of its parameter dialog), and `display_help()`. Any other hook is not part of the contract. 47 of the shipped objects open a parameter dialog and 48 declare their own dialog controls.

**[STU-WEB-105] Behaviour record (normative shape).** A behaviour is `{behavior_id, group, display_title, safe_in_templates (boolean), parameter_controls[], page_functions_emitted[], event_handler_template, shared_helpers[]}`. `event_handler_template` is the literal handler text written onto the target element, with substitution points marked; the literal-only form (the template with every substitution removed) MUST also be stored so a document can be scanned for the behaviour without evaluating anything.

**[STU-WEB-106] Behaviour catalogue (normative, thirty entries in two groups).** The shipped baseline is 30 behaviours carrying 165 parameter controls: 18 in the root group and 12 in an effects group. Root-group behaviours and their control counts: call-script (1), change-property (7), check-plugin (9), drag-element (22), go-to-url (3), jump-menu (10), jump-menu-go (1), open-browser-window (11), popup-message (1), preload-images (5), set-text-of-frame (4), set-text-of-container (2), set-text-of-status-bar (1), set-text-of-text-field (2), show-hide-elements (4), swap-image (5), swap-image-restore (0), validate-form (8). Effects-group behaviours: appear-fade (4), blind (5), bounce (7), clip (5), drop (5), fold (6), scale (9), highlight (6), puff (5), pulsate (5), shake (6), slide (6). Studio ships Handshake-native equivalents of these behaviours; it MUST NOT ship the source suite's names ([STU-WEB-005]).

**[STU-WEB-107] Template safety of behaviours (shipped distribution).** 14 of the 30 behaviours are template-safe and 16 are not; every effects-group behaviour is template-unsafe because it writes into the document head. Studio MUST carry the flag per behaviour and MUST enforce it per [STU-WEB-095].

**[STU-WEB-108] Behaviour parameter-control kinds (normative, closed).** A behaviour parameter control is exactly one of: `text`, `textarea`, `select`, `checkbox`, `radio`, `button`, `image_button`, `color_button`. Each declares a name, an optional default, and its handlers.

**[STU-WEB-109] Shared page helpers.** A behaviour may emit shared helper functions into the page rather than duplicating code per instance. The shipped baseline declares 13 shared helpers. Studio MUST emit each helper at most once per document, MUST remove it when the last behaviour depending on it is removed, and MUST record helper reference counts on the document so removal is exact rather than heuristic.

**[STU-WEB-110] Event model (normative).** Studio MUST ship a declarative event model mapping element names to the events they accept. The shipped baseline declares one model with 79 element rows over 18 distinct events: `onBlur`, `onChange`, `onClick`, `onDblClick`, `onError`, `onFocus`, `onKeyDown`, `onKeyPress`, `onKeyUp`, `onLoad`, `onMouseDown`, `onMouseMove`, `onMouseOut`, `onMouseOver`, `onMouseUp`, `onReset`, `onSubmit`, `onUnload`. Each row carries an ORDERED event list and an optional `default_event` (the anchor row's default is `onClick`). Order is authority for picker order; the default event is what a behaviour binds to when the operator does not choose.

**[STU-WEB-111] Event-model extensibility.** Studio MUST support more than one event model, selectable per site by target-browser baseline, and MUST report in the export receipt any behaviour bound to an event the selected model does not declare for that element.

**[STU-WEB-112] Insert and behaviour commands.** Every catalogue entry and every behaviour MUST be invocable as a typed `StudioCommand` with a stable id and typed parameters, independent of the panel. `insert.object(object_id, params, target)` and `behavior.apply(behavior_id, params, target, event)` are the required minimum, plus `behavior.list_for_node`, `behavior.edit`, `behavior.remove` and `behavior.reorder` (handler order on one element is authority).

---

### 14.28.8 Site definition, transports and server models

**[STU-WEB-113] Site record (normative shape).** A site is an authority record carrying `site_id`, `display_name`, `local_root` (a path), `default_images_folder`, `site_relative_link_base` (`document` \| `site_root`), `cache_enabled` (boolean), `remote_targets[]` ([STU-WEB-114]), `testing_target?`, `server_model_id?`, `preprocessor_config?`, `framework_pin?` ([STU-WEB-085]), `linter_overrides?`, `browser_baseline?`, and `version_control_binding?`.

**[STU-WEB-114] Remote-target record (normative shape).** `{target_id, display_name, transport ([STU-WEB-115]), host, port, root_path, credentials_ref, passive_mode?, use_ipv6?, encryption?, proxy_ref?, keepalive?, connection_timeout, transfer_timeout, save_before_put (boolean), enable_check_in_out (boolean), check_out_name?, check_out_email?}`. `credentials_ref` is a reference into the kernel credential store; a site record MUST NOT contain a password, token or private key. Publishing credentials are capability-gated and MUST be scrubbed from every receipt, log and Flight-Recorder span.

**[STU-WEB-115] Transport enumeration (normative, closed, six members).** `local_or_network` (a filesystem path), `ftp`, `ftps` (FTP over TLS), `sftp` (SSH file transfer), `webdav` (HTTP/WebDAV), and `version_control` (a repository binding, [STU-WEB-122]). The shipped transport baseline provides an HTTP/WebDAV core plus discrete WebDAV, FTP and SFTP implementations.

**[STU-WEB-116] SFTP algorithm policy.** SFTP host-key and public-key algorithm sets MUST be an operator-editable policy on the site, not a hard-coded list, because interoperating with older servers requires re-enabling algorithms modern defaults exclude. The shipped baseline re-enables one legacy signature algorithm for both host keys and public-key authentication; Studio MUST make such a choice explicit and MUST warn in the publish receipt when a weakened algorithm was negotiated. Studio MUST NOT silently downgrade.

**[STU-WEB-117] Transfer-mode map (normative, sixty-eight rows).** FTP-family transports MUST transfer each file in text or binary mode according to a declarative extension map, because a binary file sent as text is corrupted. The shipped map declares 68 extensions. Text-mode extensions: `as`, `ascx`, `asmx`, `asp`, `aspx`, `cfm`, `cfml`, `cgi`, `cs`, `css`, `dwt`, `htm`, `html`, `inc`, `js`, `lbi`, `mxml`, `php`, `php3`, `php4`, `php5`, `pl`, `shtm`, `shtml`, `text`, `txt`, `vb`, `xhtm`, `xhtml`. Binary-mode extensions: `aif`, `aiff`, `aifc`, `bin`, `bmp`, `dcr`, `dir`, `dmg`, `doc`, `dxr`, `exe`, `fla`, `gif`, `jpg`, `jpeg`, `mno`, `mov`, `mpeg`, `mpg`, `pdf`, `pic`, `pict`, `png`, `psd`, `qt`, `ra`, `ram`, `readme`, `rm`, `rtf`, `sea`, `sit`, `snd`, `swf`, `tif`, `tiff`, `tpl`, `wav`, `zip`. An unlisted extension defaults to BINARY, which is the safe default, and MUST be reported once per publish so the map can be extended.

**[STU-WEB-118] Link management.** Studio MUST maintain a site-wide link graph and MUST offer: update-links-on-move (rewriting every referring document when a file is moved or renamed, in one transaction with a receipt), change-link-site-wide, a broken-link report, an orphaned-file report, and an external-link report. Link rewriting MUST honour the site's `site_relative_link_base`.

**[STU-WEB-119] Synchronisation contract.** Studio MUST offer put, get, and a two-way synchronise that compares local and remote by modification time and size, presents the resulting action list for review, and applies it transactionally per file with a per-file receipt. Dependent-file inclusion (images, stylesheets, scripts a page references) MUST be an explicit option, defaulting to prompt. Synchronisation MUST run as a headless bounded job under the quiet law (14.20): no foreground window, no focus steal, progress and per-file outcome observable through structured job state ([STU-AUT-012]).

**[STU-WEB-120] Web output binding.** Web output produced by Studio — a whole site, a document, a compiled stylesheet, or HTML generated from a design document — is produced through `StudioExportRecipe` (14.13) into matrix row 46, never by a web-domain-private writer. The recipe declares: output root, which document set, whether to compile preprocessors, whether to minify, whether to emit source maps, the asset-rewriting policy, and the link-base policy.

**[STU-WEB-121] Site reports.** Studio MUST offer a site report surface producing typed, machine-readable results for at minimum: broken links, orphaned files, missing alternative text, untitled documents, redundant nested tags, removable empty tags, files with lint diagnostics above a threshold, and files checked out by another operator. Reports run on the batch runner and are readable through the command surface.

**[STU-WEB-122] Version-control binding.** A site MAY bind to a repository. Studio MUST support at minimum: initialise, clone (with credential handling per [STU-WEB-114]), stage, commit, revert, branch create / switch / merge / delete, remote add / edit / remove, pull, push, per-file history, repository history, and a diff view. Version control is an alternative to a remote transport for the same purpose and MUST be selectable as the site's publishing mechanism.

**[STU-WEB-123] Browser capability profiles.** Studio MUST support declarative browser capability profiles used to warn about constructs a target browser does not support. The shipped baseline is 26 profiles plus one exception file. A site declares its `browser_baseline` as an ordered list of profiles; a construct unsupported by any listed profile is a lint warning naming the profile.

**[STU-WEB-124] Server-model record (normative shape and eight rows).** A server model is `{model_id, display_name, server_name, server_language, server_version, file_extensions[], language_signatures[], code_delimiters[], supports_charset (boolean)}`. The eight shipped models:

| model_id | Display name | server_name | server_language | server_version |
|---|---|---|---|---|
| `aspnet_csharp` | ASP.NET C# | ASP.NET | C# | 1.0 |
| `aspnet_vb` | ASP.NET VB | ASP.NET | VB | 1.0 |
| `asp_js` | ASP JavaScript | ASP | JavaScript | 2.0 |
| `asp_vbs` | ASP VBScript | ASP | VBScript | 2.0 |
| `coldfusion` | ColdFusion | Cold Fusion | CFML | 4.5 |
| `jsp` | JSP | JSP | Java | 1.0 |
| `php_mysql` | PHP | PHP | (none declared) | 4.0 |
| `xslt` | XSLT | XSLT | XSLT | 2.0 |

**[STU-WEB-125] Server-model hook set (normative, closed, eleven members).** A server model declares at most: `can_recognize_document`, `get_file_extensions`, `get_language_signatures`, `get_server_info`, `get_server_model_delimiters`, `get_server_model_display_name`, `get_server_model_folder_name`, `get_server_supports_charset`, `update_page_directive`, `inspect_dynamic_data_reference`, `charset_to_code_page`. `get_server_model_delimiters` is the hook that tells the tokeniser, the formatter, the validator and the commenter where server code begins and ends; without it every other web capability mis-parses a dynamic document, so it is mandatory for every `DYNAMIC` document type.

**[STU-WEB-126] Server-behaviour posture (declared reduction).** The source application ships 116 server behaviours over 243 participant files carrying 1,176 real code blocks, 128 parameter dialogs with 387 controls, 41 database connection surfaces with 310 setting controls, 377 server-format entries, 28 data-source entries and 16 component definitions — a code-generation system that writes server-side data-access code into the page. **Studio does NOT port that system.** Studio's posture is: recognise these constructs for parsing, colouring, validation, formatting, round-trip and publishing fidelity ([STU-WEB-002]); do not generate them, do not open database connections, and do not ship a data-access code generator. A document containing them MUST round-trip byte-for-byte and MUST be editable as text and through the contextual panel mechanism where an inspector exists. This is a deliberate scope edge recorded per [STU-SECTION-003], not an omission. If the operator later brings server-side code generation into scope, it is a new work packet with its own refinement, not an extension of this clause.

**[STU-WEB-127] Publishing receipts.** Every publish, put, get, synchronise or version-control operation MUST emit an EventLedger-bound receipt naming the site, the target, the transport, the file set with per-file outcome, the transfer mode used per file ([STU-WEB-117]), any negotiated algorithm weakening ([STU-WEB-116]), and the elapsed time. Credentials MUST NOT appear. A failed publish MUST leave the remote in a described state: Studio MUST record which files were transferred before the failure so the operator or a model can resume rather than restart.

---

### 14.28.9 Views, commands and obligations

**[STU-WEB-128] View-mode enumeration (normative, closed, four members).** `code` (source only), `split` (source and rendered, with a per-site orientation preference), `live` (rendered, editable in place), and `inspect` (rendered with element inspection and the computed-style surface). Switching views MUST NOT alter document bytes.

**[STU-WEB-129] Live-view editing contract.** An edit made in the live view is a text edit on the authority source ([STU-WEB-003]) and MUST produce the same `StudioCommand`, receipt and undo entry as the equivalent code-view edit. There MUST NOT be a separate live-view edit path with different semantics.

**[STU-WEB-130] Element inspection surface.** The inspect view MUST expose, per hovered or selected element: its element path, its matched CSS rules in cascade order with the winning declaration marked, its computed box model, and the source location of each matched rule. All of it MUST be readable through the command surface so a model can answer "why is this element this colour" without a screenshot.

**[STU-WEB-131] Command-surface obligation.** Every capability in this sub-section MUST be a typed `StudioCommand` per [STU-AUT-001] and MUST satisfy [STU-CON-007]: model-invokable, parallel-safe, deterministic and visually verifiable. Specifically:

- **Parallel-safe:** two model lanes editing two different documents in one site MUST both succeed; two lanes editing the same document MUST produce one success and one typed conflict under the expected-revision precondition of [STU-SDB-004]. Site-wide operations (link update, synchronise, site report) MUST take a site-level advisory lock and MUST report the lock holder on contention rather than blocking indefinitely.
- **Deterministic:** formatting ([STU-WEB-028]), validation ([STU-WEB-055]), linting ([STU-WEB-058]), preprocessor compilation ([STU-WEB-086]), template propagation ([STU-WEB-093]) and export ([STU-WEB-120]) MUST each be pure functions of input bytes plus declared configuration. No output may depend on filesystem enumeration order, locale, or wall-clock time.
- **Visually verifiable:** the live view MUST be renderable to bytes at a requested width through `web.capture_view(document_id, width, height?)` with no foreground window and no focus steal ([STU-WEB-087], 14.20).

**[STU-WEB-132] Shortcut and menu surface.** The web domain contributes commands to the shared Studio command corpus. The source baseline for this domain is 2,176 invocable entries across 301 menus, 508 of which carry a shortcut, plus 264 scripted command surfaces; three complete keyboard-shortcut sets (776, 701 and 509 bindings) and 23 locale-adaptive layout sets over 138 keyboard layouts. Studio MUST support multiple named shortcut sets with one active per operator, locale-adaptive remapping, and per-command rebinding; a conflicting binding within a set is a registration error naming both commands.

**[STU-WEB-133] Enabler predicate contract.** A command MAY declare an ENABLER: a pure predicate over the current selection and document state that decides whether the command is currently applicable. In the source baseline 1,274 of the invocable entries declare one. An enabler MUST be side-effect-free and MUST be evaluable headlessly, because the model command surface uses it to answer "can I do this here" without attempting the edit. Whether a disabled command is hidden or shown greyed is an operator preference and is NOT settled by this clause; Studio MUST implement both and MUST default to shown-and-greyed, matching the source convention, until the operator decides otherwise.

**[STU-WEB-134] Asset library binding.** Every asset a web document references — images, video, audio, fonts, downloadable files, framework bundles, favicon — MUST be resolved through CKC as a placed-asset link per [STU-ASSET-005]. The web domain MUST NOT maintain its own asset catalog, its own thumbnail cache, or its own metadata store. Site-relative path resolution ([STU-WEB-118]) operates over CKC-resolved asset identities, and publishing materialises the bytes from CKC's artifact tier at transfer time ([STU-ASSET-009]).

**[STU-WEB-135] Design-to-web pipeline binding.** A design document (14.10) MUST be convertible to a web document. The conversion contract:

- A `component` maps to a library item ([STU-WEB-096]) or a framework component ([STU-WEB-102]); the choice is per component and is recorded on the component.
- A `component_set` maps to a base library item plus one class per variant property value; the variant-property names become the class-name segments.
- A `StudioVariable` maps to a CSS custom property; its `code_syntax` value for the `WEB` platform ([STU-DS-129]) is the emitted name, falling back to a slug of the variable name.
- A variable collection MODE maps to a selector scope (a class or a media query); the mapping is declared per collection.
- `StudioAutoLayout` maps to flex or grid declarations; `layout_mode` `HORIZONTAL`/`VERTICAL` map to flex, `GRID` maps to CSS grid, `layout_wrap` maps to `flex-wrap`, and the sizing tokens `FIXED`/`HUG`/`FILL` ([STU-DS-148]) map to explicit size, content size, and grow respectively.
- Every unmapped construct MUST appear in the generation receipt with the construct id and the reason. The receipt is the contract: silent approximation is forbidden.

**[STU-WEB-136] Round-trip posture.** Generation is one-directional by default. Studio MUST NOT claim design-from-web round-trip unless a fixture set proves it under the gate of [STU-IO-004]. Until such a fixture set exists, the design-to-web direction is declared `X` (export) only, and this is stated in the UserManual.

**[STU-WEB-137] Validation descriptor set.** This sub-section contributes at minimum: `document_type_extension_conflict`, `doctype_missing`, `tag_unknown_in_active_libraries`, `attribute_value_outside_enumeration`, `attribute_required_set_unsatisfied`, `tag_context_violation`, `cross_tag_attribute_collision`, `panel_binding_unresolvable_tie`, `panel_discriminator_does_not_outrank`, `panel_author_id_collision`, `breakpoint_below_minimum`, `breakpoint_range_below_minimum_span`, `framework_class_invalid_for_pinned_version`, `template_edit_in_locked_region`, `template_behavior_not_template_safe`, `snippet_trigger_duplicate`, `behavior_event_not_in_model`, `link_broken`, `file_orphaned`, `transfer_mode_unmapped_extension`, `sftp_algorithm_weakened`, `credential_in_site_record`.

**[STU-WEB-138] Diagnostic-tier obligation.** Every failure mode in [STU-WEB-137] MUST be surfaced at all three diagnostic tiers wired against the current kernel base: the in-process structured diagnostic, the operator-facing diagnostics surface, and the external watcher. None of the three may be deferred; all three exist in the base.

**[STU-WEB-139] Resource-privacy obligation.** Site records, credentials references, remote-target definitions, publish receipts, link graphs and site reports are resource-scoped authority records. Every read and write passes the kernel `ResourceBroker` and the record-level SurrealDB permissions of [STU-SDB-005]. A publish receipt MUST NOT be readable across accounts or projects, and cross-account and cross-project adversarial cases MUST be part of the acceptance proof, not an afterthought.

**[STU-WEB-140] GUI / Argus / UserManual obligation.** Every panel, view, control and visible state in this sub-section — the document-type picker, the code and live and split and inspect views, the tag and attribute inspectors, every contextual panel resolved by 14.28.2, the CSS authoring surface, the breakpoint ruler, the template and snippet surfaces, the Insert catalogue, the behaviour editor, the site manager, the synchronise and publish surfaces, the site reports, and the version-control surface — MUST be model-visible and typed-steerable through the Studio command surface (14.16); MUST be headlessly inspectable, steerable and screenshot-capturable through Argus with no foreground focus steal (14.20, HBR-VIS/HBR-QUIET); and MUST ship dual-audience UserManual entries — operator layer (task-oriented) plus model layer (command ids, typed I/O, receipts, undo semantics, Argus targets, failure and recovery) — kept same-change current (14.22). Every enumeration in this sub-section MUST appear in the model-facing UserManual as its literal token list, not as prose.

---

### 14.28.10 Microtask Derivation

**[STU-WEB-141] Derivation rule (NORMATIVE).** The web-authoring microtask set is derived from this module mechanically, not editorially. ONE microtask corresponds to ONE of the following units, and to nothing else:

1. Each numbered clause that states a **document-type or vocabulary contract** ([STU-WEB-011]-[STU-WEB-029]), a **panel binding record or a resolution rule** ([STU-WEB-031]-[STU-WEB-044] — including the nine-field binding record of [STU-WEB-033], the closed selector grammar of [STU-WEB-034], the selection-scope set of [STU-WEB-035], the priority bands of [STU-WEB-036], the deterministic resolution algorithm of [STU-WEB-040], the resolution receipt of [STU-WEB-041], the registry static analysis of [STU-WEB-042] and the four lifecycle hooks of [STU-WEB-044]), a **code-intelligence rule set** ([STU-WEB-045]-[STU-WEB-059]), a **CSS property or unit contract** ([STU-WEB-060]-[STU-WEB-075]), a **responsive constant set, framework descriptor or breakpoint contract** ([STU-WEB-076]-[STU-WEB-089]), a **template, region, locking or snippet contract** ([STU-WEB-090]-[STU-WEB-099]), an **Insert-object, behaviour or event-model contract** ([STU-WEB-100]-[STU-WEB-112]), a **site, transport, server-model or publishing contract** ([STU-WEB-113]-[STU-WEB-127]), a **view or inspection contract** ([STU-WEB-128], [STU-WEB-129], [STU-WEB-130]), a **scope tripwire** ([STU-WEB-001], [STU-WEB-002]), a **document-model rule** ([STU-WEB-003], [STU-WEB-004]), or an **execution guarantee or pipeline binding** ([STU-WEB-120], [STU-WEB-131], [STU-WEB-132], [STU-WEB-135]), where that clause can be implemented and proven independently of its siblings.
2. Each **validation-descriptor clause** in sub-section 14.28.11, [STU-WEB-146] through [STU-WEB-167]. Each of the 22 descriptors named in [STU-WEB-137] is stated as its own clause precisely so it yields its own microtask: a check is a unit of implementable, independently provable work, and one microtask reading "implement 22 checks" is not implementable by the small models these contracts are sized for. A descriptor list inside a single clause, whether as prose or as a table, is one unit to any derivation tool and therefore loses 21 units of real work.
3. Each **declared open item** — in this module exactly two, [STU-WEB-133] and [STU-WEB-136]. Each yields a microtask under [STU-WEB-142], not nothing.

No other unit yields a microtask. Exactly 6 clauses in this module yield nothing, and they are:

- **Pure pointer clauses** — [STU-WEB-005]. Each restates a clause that already carries the contract; the microtask lives there.
- **This derivation sub-section itself** — its five clauses yield nothing.

Every other clause yields at least one unit. This list is the module's declared non-yielding set and is the authority a derivation tool reconciles against.

**[STU-WEB-142] Open items and blocked dependencies.** This module declares two, and each YIELDS a microtask whose FIRST acceptance criterion is resolving the named dependency:

| Declared open item | Clause | First acceptance criterion of its microtask |
|---|---|---|
| Whether a command whose enabler is false is hidden or shown greyed is an unresolved operator preference | [STU-WEB-133] | Implement BOTH behaviours behind the preference and obtain the operator decision on the default. The clause already fixes a safe interim default (shown and greyed, matching the source convention), so implementation is not blocked; the microtask closes when the preference is recorded. |
| Design-from-web round-trip is claimed nowhere and has no fixture set | [STU-WEB-136] | Either build the fixture set and pass the five-artifact gate of [STU-IO-004], or record the direction as export-only in the UserManual and in matrix row 46. Claiming round-trip without fixtures is forbidden by [STU-IO-011]. |

A declared open item MUST NOT be dropped from the yields index, because an item that yields nothing disappears silently and is rediscovered at implementation time. The same rule governs any BLOCKED dependency a later amendment introduces: its microtask first acceptance criterion is resolving the dependency, or raising a BLOCKED record naming the exact blocker.

**[STU-WEB-143] Microtask content obligation.** A microtask derived under [STU-WEB-141] MUST carry into its own body: the clause anchor; the COMPLETE row set of every normative table it touches — all thirty-nine document types with their extensions, MIME types, byte-order-mark flags and DTD contexts; all ten tag libraries with their applies-to sets; all twenty attribute value types; all fifteen length units; all twenty-three media features; all sixty-eight transfer-mode rows; all eight server models with their three declared literals; the exact shipped default of every linter rule it touches, enabled and disabled alike; the verbatim numeric constants of [STU-WEB-079] and [STU-WEB-084], with the four source-declared grid bounds distinguished from the Studio-declared ones; and, for any panel microtask, the full nine-field binding record of [STU-WEB-033] plus the tiebreak order of [STU-WEB-040]. A microtask that says "implement the CSS property surface" without the eighty-two entries, their control types, their defaults and their negative-value flags does not satisfy this clause.

**[STU-WEB-144] Yields index (NORMATIVE).** The counts below are the derivation surface of this module under [STU-WEB-141]. They are not estimates: they are the measured output of applying that rule to this module's text, and every row states which unit kinds it contributes.

| Unit group | Clauses | Units by kind | Yields |
|---|---|---|---|
| 28.0 Scope, posture and dedup | [STU-WEB-001]-[STU-WEB-010] | 9 clause | 9 |
| 28.1 Document types, tag libraries and the attribute vocabulary | [STU-WEB-011]-[STU-WEB-029] | 19 clause, 2 enumeration | 21 |
| 28.2 Contextual property-panel binding (Studio-wide contract) | [STU-WEB-030]-[STU-WEB-044] | 15 clause | 15 |
| 28.3 Code intelligence | [STU-WEB-045]-[STU-WEB-059] | 15 clause, 1 enumeration | 16 |
| 28.4 The CSS property surface | [STU-WEB-060]-[STU-WEB-075] | 16 clause, 2 enumeration, 2 parameter table | 20 |
| 28.5 Responsive authoring: media queries and breakpoints | [STU-WEB-076]-[STU-WEB-089] | 14 clause, 2 enumeration, 2 parameter table | 18 |
| 28.6 Templates, library items and snippets | [STU-WEB-090]-[STU-WEB-099] | 10 clause, 1 enumeration | 11 |
| 28.7 The Insert catalogue, behaviours and the event model | [STU-WEB-100]-[STU-WEB-112] | 13 clause | 13 |
| 28.8 Site definition, transports and server models | [STU-WEB-113]-[STU-WEB-127] | 15 clause, 1 enumeration | 16 |
| 28.9 Views, commands and obligations | [STU-WEB-128]-[STU-WEB-140] | 13 clause | 13 |
| 28.11 Validation Descriptor Catalogue | [STU-WEB-146]-[STU-WEB-167] | 22 validator | 22 |
| Clauses yielding nothing | 6 clauses, listed in [STU-WEB-141] | — | 0 |
| **Module total** | | **167 clauses** | **174** |

Of this module's 167 clauses, 6 yield nothing and 161 yield at least one unit; tables inside yielding clauses contribute the remainder. The module total is **174**. The last numeric column is the yields count.

**[STU-WEB-145] Anchor binding.** A microtask derived from this module cites its clause anchor directly. Because 14.28 is a NEW domain, no staged microtask predates it: every web-authoring microtask is derived from this module at authoring time and carries a real anchor from the outset, never `spec_anchor_status = "PROVISIONAL"`. A microtask that cannot cite an anchor in [STU-WEB-001]-[STU-WEB-167] is out of scope for the web-authoring domain and MUST be re-derived or retired, not activated. Microtasks derived from 14.28.2 ([STU-WEB-031]-[STU-WEB-044]) are shell-scoped rather than web-scoped and MUST be scheduled with the shell work, not behind the web domain; when those clauses are re-anchored under a shell prefix per [STU-WEB-030], their microtasks are re-bound to the new anchors and are NOT re-derived.

---

### 14.28.11 Validation Descriptor Catalogue

Each descriptor below is its own clause because each is its own unit of implementable, independently provable work: feed the runtime a document that violates the rule and assert the check fires with the stated diagnostic. [STU-WEB-137] names the set; the clauses in this sub-section state what each member catches, which clause it enforces, its severity, and what its diagnostic MUST name. Every one is a `StudioValidationDescriptor` in the catalogue of 14.24.

**[STU-WEB-146] `document_type_extension_conflict`.** The web-authoring validator MUST reject, with severity `error`, a document or command in which two document types claim the same extension with no disambiguating rule, so an opened file has no determinate type, enforcing [STU-WEB-013]. The diagnostic MUST name both types and the shared extension.

**[STU-WEB-147] `doctype_missing`.** The web-authoring validator MUST reject, with severity `warning`, a document or command in which an HTML-family document carries no doctype declaration, enforcing [STU-WEB-015]. The diagnostic MUST name the document.

**[STU-WEB-148] `tag_unknown_in_active_libraries`.** The web-authoring validator MUST reject, with severity `warning`, a document or command in which a tag resolves against none of the libraries active for the document's type, enforcing [STU-WEB-018]. The diagnostic MUST name the tag, the document type and the libraries searched.

**[STU-WEB-149] `attribute_value_outside_enumeration`.** The web-authoring validator MUST reject, with severity `warning`, a document or command in which an attribute value falls outside its declared `allowed_values`, enforcing [STU-WEB-023]. The diagnostic MUST name the attribute, the offending value and the allowed set; the operator's text stands and is never silently rewritten.

**[STU-WEB-150] `attribute_required_set_unsatisfied`.** The web-authoring validator MUST reject, with severity `error`, a document or command in which a tag fails an attribute-requirement set declared by the active validator rule set, enforcing [STU-WEB-053]. The diagnostic MUST name the tag, the requirement set and which members were present.

**[STU-WEB-151] `tag_context_violation`.** The web-authoring validator MUST reject, with severity `error`, a document or command in which a tag appears in a parent or ancestor context its rule set forbids, enforcing [STU-WEB-053]. The diagnostic MUST name the tag, its context and the rule.

**[STU-WEB-152] `cross_tag_attribute_collision`.** The web-authoring validator MUST reject, with severity `warning`, a document or command in which a cross-tag attribute group declares an attribute a tag also declares itself, enforcing [STU-WEB-024]. The diagnostic MUST name the group, the tag and the attribute; the tag's own definition wins.

**[STU-WEB-153] `panel_binding_unresolvable_tie`.** The web-authoring validator MUST reject, with severity `error`, a document or command in which two panels declare identical binding, scope, priority, discriminator and document types, so only the `panel_id` tiebreak separates them, enforcing [STU-WEB-042]. The diagnostic MUST name both panels; this is a registry design defect reported at load, never at selection.

**[STU-WEB-154] `panel_discriminator_does_not_outrank`.** The web-authoring validator MUST reject, with severity `error`, a document or command in which a discriminated panel does not outrank the generic panel it refines, so the refinement can never win, enforcing [STU-WEB-037]. The diagnostic MUST name both panels and their priorities.

**[STU-WEB-155] `panel_author_id_collision`.** The web-authoring validator MUST reject, with severity `error`, a document or command in which two panels claim the same `author_id_prefix`, enforcing [STU-WEB-042]. The diagnostic MUST name both panels and the prefix.

**[STU-WEB-156] `breakpoint_below_minimum`.** The web-authoring validator MUST reject, with severity `error`, a document or command in which a breakpoint value falls below the declared minimum or above the declared maximum, enforcing [STU-WEB-080]. The diagnostic MUST name the breakpoint, its value and the declared bound.

**[STU-WEB-157] `breakpoint_range_below_minimum_span`.** The web-authoring validator MUST reject, with severity `error`, a document or command in which a min-and-max breakpoint's two edges are closer than the declared minimum span, enforcing [STU-WEB-081]. The diagnostic MUST name both edges and the required span; the span is never silently widened.

**[STU-WEB-158] `framework_class_invalid_for_pinned_version`.** The web-authoring validator MUST reject, with severity `error`, a document or command in which a visual edit emits a grid, offset or hide class that the site's pinned framework version does not declare, enforcing [STU-WEB-085]. The diagnostic MUST name the class, the pinned version and the element.

**[STU-WEB-159] `template_edit_in_locked_region`.** The web-authoring validator MUST reject, with severity `error`, a document or command in which a text edit targets a region locked by an attached template, enforcing [STU-WEB-094]. The diagnostic MUST name the region and the owning template; detach is offered as the remedy.

**[STU-WEB-160] `template_behavior_not_template_safe`.** The web-authoring validator MUST reject, with severity `error`, a document or command in which a behaviour that is not template-safe is applied inside a template instance, enforcing [STU-WEB-095]. The diagnostic MUST name the behaviour and the target region; this is a hard gate, not a warning.

**[STU-WEB-161] `snippet_trigger_duplicate`.** The web-authoring validator MUST reject, with severity `error`, a document or command in which two snippets in one site claim the same keyboard trigger word, enforcing [STU-WEB-099]. The diagnostic MUST name both snippets and the trigger.

**[STU-WEB-162] `behavior_event_not_in_model`.** The web-authoring validator MUST reject, with severity `warning`, a document or command in which a behaviour binds to an event the active event model does not declare for that element, enforcing [STU-WEB-111]. The diagnostic MUST name the behaviour, the event, the element and the active model; it is reported in the export receipt rather than emitted.

**[STU-WEB-163] `link_broken`.** The web-authoring validator MUST reject, with severity `warning`, a document or command in which a link in the site graph resolves to no file, asset or anchor, enforcing [STU-WEB-118]. The diagnostic MUST name the referring document, the link target and the line.

**[STU-WEB-164] `file_orphaned`.** The web-authoring validator MUST reject, with severity `info`, a document or command in which a file in the local root is referenced by no document in the site graph, enforcing [STU-WEB-118]. The diagnostic MUST name the file.

**[STU-WEB-165] `transfer_mode_unmapped_extension`.** The web-authoring validator MUST reject, with severity `warning`, a document or command in which a published file's extension appears in neither the text nor the binary transfer-mode list, enforcing [STU-WEB-117]. The diagnostic MUST name the extension; it transfers as binary, which is the safe default, and is reported once per publish.

**[STU-WEB-166] `sftp_algorithm_weakened`.** The web-authoring validator MUST reject, with severity `warning`, a document or command in which a publish negotiated a host-key or public-key algorithm the site policy re-enabled beyond modern defaults, enforcing [STU-WEB-116]. The diagnostic MUST name the algorithm and the target; a silent downgrade is forbidden.

**[STU-WEB-167] `credential_in_site_record`.** The web-authoring validator MUST reject, with severity `error`, a document or command in which a site or remote-target record contains a password, token or private key rather than a credential-store reference, enforcing [STU-WEB-114]. The diagnostic MUST name the record and the field name only, never the secret value.
