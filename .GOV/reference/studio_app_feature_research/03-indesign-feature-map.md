---
file_id: studio-app-feature-research-indesign
topic_id: SFR-INDESIGN
title: "Adobe InDesign Feature Map"
status: draft
summary: "InDesign page layout, typography, styles, prepress, export, collaboration, automation, and AI feature families."
sources: 10
updated_at: "2026-07-05"
---

## [SFR-INDESIGN] Adobe InDesign Feature Map

### [SFR-INDESIGN.inventory] Feature Inventory

```yaml
as_of: "2026-07-05"
app: "Adobe InDesign desktop"
features:
  - { id: "indesign.document_setup", name: "Document setup, margins, columns", app_behavior: "Create/modify page size, margins, columns, bleed, and slug settings per document/page/spread.", primitive_domain: page_layout, source_ids: [ID-S01, ID-S05] }
  - { id: "indesign.pages_spreads", name: "Pages and multi-page spreads", app_behavior: "Manage document pages, spreads, shuffling behavior, and island spreads up to documented spread limits.", primitive_domain: page_layout, source_ids: [ID-S01] }
  - { id: "indesign.parent_pages", name: "Parent pages", app_behavior: "Reusable page foundations for shared headers, footers, page furniture, and layout elements inherited by document pages.", primitive_domain: master_pages, source_ids: [ID-S01] }
  - { id: "indesign.page_numbering_sections", name: "Page numbers, sections, markers", app_behavior: "Auto-updating page numbers, section markers, jump lines, chapter numbers, and absolute/section numbering modes.", primitive_domain: page_layout, source_ids: [ID-S01] }
  - { id: "indesign.book_files", name: "Book files", app_behavior: "Group multiple InDesign documents as a book with coordinated page numbering, shared style source, and output workflows.", primitive_domain: page_layout, source_ids: [ID-S01] }
  - { id: "indesign.guides_grids", name: "Guides, layout grids, baseline grids", app_behavior: "Create ruler guides, layout grids, named grids, and baseline grids for alignment and repeatable composition.", primitive_domain: page_layout, source_ids: [ID-S01] }
  - { id: "indesign.layers", name: "Layers", app_behavior: "Organize objects with document-wide visibility, editability, and stacking controls.", primitive_domain: page_layout, source_ids: [ID-S01] }
  - { id: "indesign.adjust_layout", name: "Adjust Layout", app_behavior: "Reflow/adapt existing layouts when page size, margins, or bleed settings change.", primitive_domain: page_layout, source_ids: [ID-S01] }
  - { id: "indesign.flex_layout", name: "Flex Layout", app_behavior: "Container layout system that adapts child item direction, spacing, padding, alignment, wrapping, and resizing as content changes.", primitive_domain: page_layout, source_ids: [ID-S01, ID-S02] }
  - { id: "indesign.text_frames", name: "Text frames", app_behavior: "Text lives in resizable/positionable containers with properties, columns, counts, and Story Editor access.", primitive_domain: typography, source_ids: [ID-S01] }
  - { id: "indesign.threaded_text", name: "Threaded text flow", app_behavior: "Connect text frames so a story flows across frames/pages and exposes overset text state.", primitive_domain: typography, source_ids: [ID-S01] }
  - { id: "indesign.text_wrap", name: "Text wrap", app_behavior: "Flow text around frames, images, and shapes using wrap boundaries and offsets.", primitive_domain: page_layout, source_ids: [ID-S01] }
  - { id: "indesign.frame_fitting", name: "Frame fitting", app_behavior: "Fit, crop, align, and auto-fit placed content inside frames.", primitive_domain: page_layout, source_ids: [ID-S01] }
  - { id: "indesign.object_transform", name: "Object transforms", app_behavior: "Rotate, scale, skew, position, and transform objects from reference points.", primitive_domain: page_layout, source_ids: [ID-S01] }
  - { id: "indesign.anchored_objects", name: "Anchored objects", app_behavior: "Attach objects to text positions so layout objects move with story content.", primitive_domain: page_layout, source_ids: [ID-S01] }
  - { id: "indesign.fonts_opentype", name: "Fonts and OpenType attributes", app_behavior: "Manage font usage and expose OpenType features such as ligatures, fractions, ordinals, swashes, and font-specific alternates.", primitive_domain: typography, source_ids: [ID-S01, ID-S02] }
  - { id: "indesign.text_styles", name: "Paragraph and character styles", app_behavior: "Reusable text styling definitions, loadable from InDesign/InCopy documents.", primitive_domain: style_system, source_ids: [ID-S01, ID-S02] }
  - { id: "indesign.nested_styles", name: "Nested styles", app_behavior: "Automatically apply character styles inside paragraphs according to word/sentence/character rules.", primitive_domain: style_system, source_ids: [ID-S01] }
  - { id: "indesign.grep_styles", name: "GREP styles", app_behavior: "Automatically apply character styles to regex/GREP text patterns inside paragraph styles.", primitive_domain: style_system, source_ids: [ID-S01] }
  - { id: "indesign.object_styles", name: "Object styles", app_behavior: "Reusable formatting for frames/graphics/text objects including stroke, fill, transparency, effects, paragraph style, and text wrap.", primitive_domain: style_system, source_ids: [ID-S01] }
  - { id: "indesign.lists_numbering", name: "Lists and numbering", app_behavior: "Define numbered-list behavior that can remain consistent across stories, pages, and book documents.", primitive_domain: typography, source_ids: [ID-S01] }
  - { id: "indesign.text_variables", name: "Text variables", app_behavior: "Auto-updating text tokens for dates, file names, page numbers, running headers, and related dynamic document text.", primitive_domain: typography, source_ids: [ID-S01] }
  - { id: "indesign.spell_hyphenation", name: "Spell check, autocorrect, dictionaries", app_behavior: "Manual spell check, dynamic spelling, autocorrect, language dictionaries, and Hunspell dictionary expansion.", primitive_domain: typography, source_ids: [ID-S01] }
  - { id: "indesign.cjk_text", name: "CJK text composition", app_behavior: "CJK-specific text formatting and composition controls including Japanese composition settings.", primitive_domain: typography, source_ids: [ID-S01] }
  - { id: "indesign.find_change", name: "Find/Change and saved queries", app_behavior: "Find/replace text, formatting, GREP patterns, and saved query workflows.", primitive_domain: typography, source_ids: [ID-S01] }
  - { id: "indesign.footnotes_endnotes", name: "Footnotes and endnotes", app_behavior: "Create, format, lay out, span, wrap, and convert footnotes/endnotes while preserving numbering and note text.", primitive_domain: typography, source_ids: [ID-S01] }
  - { id: "indesign.toc", name: "Table of contents", app_behavior: "Generate and maintain TOCs from styled headings, keeping page numbers and document changes current.", primitive_domain: typography, source_ids: [ID-S01] }
  - { id: "indesign.index", name: "Indexing", app_behavior: "Create, tag, manage, format, update, and cross-reference index entries.", primitive_domain: typography, source_ids: [ID-S01] }
  - { id: "indesign.cross_references", name: "Cross-references", app_behavior: "Link references to paragraph styles or text anchors and update references as targets move or change.", primitive_domain: typography, source_ids: [ID-S01] }
  - { id: "indesign.bookmarks", name: "PDF bookmarks", app_behavior: "Create, nest, rename, and delete bookmarks that export as PDF navigation entries.", primitive_domain: pdf, source_ids: [ID-S01] }
  - { id: "indesign.swatches_color", name: "Swatches, tints, mixed inks", app_behavior: "Create and manage process, spot, RGB, LAB, tint, gradient, and mixed-ink swatches for document-wide reuse.", primitive_domain: style_system, source_ids: [ID-S01] }
  - { id: "indesign.gradients_transparency", name: "Gradients and transparency effects", app_behavior: "Apply linear/radial gradients, gradient swatches, opacity, blend/effects, and transparency settings.", primitive_domain: style_system, source_ids: [ID-S01] }
  - { id: "indesign.tables", name: "Tables", app_behavior: "Create/import tables, add text/images/headers/footers, embed tables, format cells, break tables across frames, and convert tables/text.", primitive_domain: tables, source_ids: [ID-S01] }
  - { id: "indesign.table_cell_styles", name: "Table and cell styles", app_behavior: "Define, apply, import, manage, and base reusable table/cell formatting styles.", primitive_domain: style_system, source_ids: [ID-S01] }
  - { id: "indesign.data_merge", name: "Data Merge", app_behavior: "Merge CSV/TXT records into a target layout for personalized documents, labels, letters, images, previews, and merged output.", primitive_domain: automation, source_ids: [ID-S01] }
  - { id: "indesign.qr_codes", name: "QR codes", app_behavior: "Generate scalable vector QR codes for URLs, plain text, SMS, email, and business-card data.", primitive_domain: interactive, source_ids: [ID-S01] }
  - { id: "indesign.hyperlinks", name: "Hyperlinks", app_behavior: "Create/manage hyperlink sources and destinations for URLs, files, email, pages, and text anchors.", primitive_domain: interactive, source_ids: [ID-S01] }
  - { id: "indesign.buttons_forms", name: "Buttons and fillable forms", app_behavior: "Convert objects into buttons, assign events/actions, and create basic PDF form fields.", primitive_domain: interactive, source_ids: [ID-S01] }
  - { id: "indesign.media", name: "Audio/video media", app_behavior: "Place movie and sound files as linked frame content for interactive documents/PDFs.", primitive_domain: interactive, source_ids: [ID-S01] }
  - { id: "indesign.interactivity_preview", name: "EPUB Interactivity Preview", app_behavior: "Preview current spread or full document interactions, buttons, animations, and multimedia before output.", primitive_domain: interactive, source_ids: [ID-S01] }
  - { id: "indesign.interactive_pdf_export", name: "Interactive PDF export", app_behavior: "Export interactive PDFs with page/view settings, security, forms, media, buttons, and static appearance options.", primitive_domain: pdf, source_ids: [ID-S01] }
  - { id: "indesign.accessible_pdf", name: "Accessible PDF tagging", app_behavior: "Use structure tags, reading order, metadata, alt text, headings, bookmarks, hyperlinks, and form labels for accessible PDFs.", primitive_domain: pdf, source_ids: [ID-S01] }
  - { id: "indesign.epub_export", name: "EPUB export", app_behavior: "Export fixed-layout or reflowable EPUB with digital reading/accessibility options.", primitive_domain: epub, source_ids: [ID-S01] }
  - { id: "indesign.epub_accessibility", name: "EPUB ARIA/accessibility enhancements", app_behavior: "Add ARIA roles/labels for hyperlinks/cross-references/objects and preserve accessible structures for EPUB export.", primitive_domain: epub, source_ids: [ID-S01, ID-S02] }
  - { id: "indesign.publish_online", name: "Publish Online and HTML5 package", app_behavior: "Publish browser-viewable documents, update existing published URLs, include interactivity, and export HTML5 packages.", primitive_domain: interactive, source_ids: [ID-S01] }
  - { id: "indesign.html_export", name: "HTML export", app_behavior: "Export InDesign content to HTML/CSS, preserving style names as CSS classes where documented.", primitive_domain: file_io, source_ids: [ID-S01] }
  - { id: "indesign.print_pdf_export", name: "Print PDF export", app_behavior: "Export print-ready PDFs using presets/custom settings, compression, standards, compatibility, security, and output controls.", primitive_domain: pdf, source_ids: [ID-S01, ID-S03] }
  - { id: "indesign.preflight", name: "Live preflight and profiles", app_behavior: "Check documents/books for output issues such as missing fonts, links, and profile violations; manage/export preflight profiles.", primitive_domain: prepress, source_ids: [ID-S01, ID-S03] }
  - { id: "indesign.package_output", name: "Package for output", app_behavior: "Collect InDesign file, linked graphics, fonts, and report into a handoff folder after preflight.", primitive_domain: prepress, source_ids: [ID-S01] }
  - { id: "indesign.print_marks_bleed_slug", name: "Printer marks, bleed, slug", app_behavior: "Configure crop/registration/color marks and include bleed/slug areas for print/PDF output.", primitive_domain: print, source_ids: [ID-S01] }
  - { id: "indesign.booklet_imposition", name: "Print Booklet imposition", app_behavior: "Generate printer spreads for folded/bound booklets and preview imposition, margins, marks, bleeds, and creep.", primitive_domain: print, source_ids: [ID-S01] }
  - { id: "indesign.separations_inks_overprint", name: "Separations, inks, overprint", app_behavior: "Prepare/process CMYK/spot separations, manage inks, preview separation readiness, and control overprinting.", primitive_domain: prepress, source_ids: [ID-S01] }
  - { id: "indesign.transparency_flattening", name: "Transparency flattening", app_behavior: "Flatten transparency for print/export workflows where live transparency is unsupported.", primitive_domain: prepress, source_ids: [ID-S01] }
  - { id: "indesign.links_panel", name: "Links panel", app_behavior: "Track placed graphics/files, link status, instances, nested dependencies, relinking, and production handoff context.", primitive_domain: file_io, source_ids: [ID-S01] }
  - { id: "indesign.import_place_assets", name: "Place/import graphics and Adobe files", app_behavior: "Place graphics and Adobe files with format-specific import options, pages/layers/crop/transparency controls, and metadata.", primitive_domain: file_io, source_ids: [ID-S01] }
  - { id: "indesign.pdf_to_indesign", name: "PDF to editable InDesign conversion", app_behavior: "Open or place PDFs and choose conversion to editable InDesign content or static placement.", primitive_domain: file_io, source_ids: [ID-S01] }
  - { id: "indesign.idml_saveback", name: "IDML/saveback compatibility", app_behavior: "Use Simple Saveback or IDML export to open newer documents in older InDesign versions.", primitive_domain: file_io, source_ids: [ID-S03] }
  - { id: "indesign.xml_structure", name: "XML structure, import, export", app_behavior: "Create/load XML tags, structure documents, import XML in append/merge modes, and export tagged content as XML.", primitive_domain: file_io, source_ids: [ID-S01] }
  - { id: "indesign.share_for_review", name: "Share for Review", app_behavior: "Generate browser review links, collect stakeholder comments, and manage/reply/resolve feedback inside InDesign.", primitive_domain: collaboration, source_ids: [ID-S01, ID-S04] }
  - { id: "indesign.cloud_invite_edit", name: "Invite to edit cloud documents", app_behavior: "Invite Creative Cloud collaborators to edit cloud documents.", primitive_domain: collaboration, source_ids: [ID-S01] }
  - { id: "indesign.incopy_workflows", name: "InCopy assignments and web editing", app_behavior: "Use assignment files and InCopy on the web beta for designer/editor collaboration while preserving layout control.", primitive_domain: collaboration, source_ids: [ID-S01] }
  - { id: "indesign.editorial_notes", name: "Editorial notes", app_behavior: "Add color-coded user notes for review/collaboration; notes are searchable in Story Editor under documented limits.", primitive_domain: collaboration, source_ids: [ID-S01] }
  - { id: "indesign.cc_libraries", name: "Creative Cloud Libraries", app_behavior: "Share reusable colors, character styles, paragraph styles, and graphics across InDesign and other Adobe apps.", primitive_domain: collaboration, source_ids: [ID-S01] }
  - { id: "indesign.scripting_panels", name: "Scripts and Script Label panels", app_behavior: "Run scripts and attach labels to page items for automation workflows.", primitive_domain: automation, source_ids: [ID-S01] }
  - { id: "indesign.uxp_dom", name: "UXP/DOM scripting APIs", app_behavior: "Use InDesign DOM APIs to create/modify/query application documents and content from UXP scripts/plugins.", primitive_domain: automation, source_ids: [ID-S06, ID-S07, ID-S08] }
  - { id: "indesign.event_scripting", name: "Event scripting", app_behavior: "Attach event listeners to InDesign objects so scripts respond to application/document events.", primitive_domain: automation, source_ids: [ID-S09] }
  - { id: "indesign.server_automation", name: "InDesign Server automation", app_behavior: "Headless/server-side document automation via scripts/plugins with server-specific object-model constraints.", primitive_domain: automation, source_ids: [ID-S10] }
  - { id: "indesign.express_edit", name: "Adobe Express image editing", app_behavior: "Edit linked/embedded images with Adobe Express tools from InDesign and manage Save as New behavior.", primitive_domain: file_io, source_ids: [ID-S02] }
  - { id: "indesign.export_to_express", name: "Export to Adobe Express", app_behavior: "Export InDesign documents to Adobe Express for downstream editing/sharing.", primitive_domain: file_io, source_ids: [ID-S01] }
  - { id: "indesign.ai_rewrite", name: "Rewrite / Generate Text", app_behavior: "Generate or refine text variations, tones, Fit Text, and language matching prompts/existing content.", primitive_domain: ai, source_ids: [ID-S02, ID-S04] }
  - { id: "indesign.ai_alt_text", name: "AI-generated alt text", app_behavior: "Generate/review image alt text and receive update indicators when image crop/refocus may make alt text stale.", primitive_domain: ai, source_ids: [ID-S02, ID-S04] }
  - { id: "indesign.ai_text_to_image", name: "Text to Image", app_behavior: "Generate Photo or Art image variants from text prompts using Adobe Firefly inside InDesign.", primitive_domain: ai, source_ids: [ID-S04] }
  - { id: "indesign.ai_generative_expand", name: "Generative Expand", app_behavior: "Extend existing images beyond current borders/aspect ratio, optionally with prompt-guided background generation.", primitive_domain: ai, source_ids: [ID-S04] }
  - { id: "indesign.ai_generative_fill_beta", name: "Generative Fill beta", app_behavior: "Apply prompt-based generated textures/effects to SVGs, shapes, or text.", primitive_domain: ai, verification_status: "BETA", source_ids: [ID-S04] }
  - { id: "indesign.ai_assistant_beta", name: "AI Assistant beta", app_behavior: "Beta prompt-to-progress assistant capability.", primitive_domain: ai, verification_status: "UNVERIFIED_DETAIL", source_ids: [ID-S04] }
```

### [SFR-INDESIGN.implementation-notes] Implementation Notes

```text
InDesign parity is primarily a document-layout engine problem, not a canvas drawing problem. The core Rust model should start with pages, spreads, frames, stories, style systems, linked assets, generated references, tables, and export recipes.

Preflight/package/export should be treated as first-class deterministic workflows with receipts, not UI-only dialogs. This is especially important for Handshake model agents, because a model needs machine-readable missing-font, missing-link, overset-text, accessibility, and print-readiness failures.
```

### [SFR-INDESIGN.gaps] Gaps

```yaml
gaps:
  - id: ID-GAP-001
    detail: "The category map is broad; the generated InDesign leaf index now enumerates official help-topic leaves, but typography, CJK, PDF, EPUB, scripting, and prepress leaves still need promotion into implementation command contracts."
    next_step: "Use 07-indesign-leaf-index.md to promote selected leaves into Studio command schemas with inputs, outputs, undo semantics, state mutations, diagnostics, and tests."
  - id: ID-GAP-002
    detail: "Cloud/collaboration and AI rows are adapter-dependent, not local Rust primitives by default."
    next_step: "Add local/provider/omitted posture before implementation."
```

### [SFR-INDESIGN.sources] Sources

```yaml
sources:
  - { id: ID-S01, url: "https://helpx.adobe.com/indesign/desktop.html", note: "InDesign desktop help index/categories." }
  - { id: ID-S02, url: "https://helpx.adobe.com/indesign/desktop/whats-new/whats-new.html", note: "Current InDesign feature summaries." }
  - { id: ID-S03, url: "https://helpx.adobe.com/indesign/desktop/whats-new/release-notes.html", note: "InDesign release notes." }
  - { id: ID-S04, url: "https://helpx.adobe.com/indesign/desktop/generative-ai-features/generative-ai-faq.html", note: "InDesign generative AI FAQ." }
  - { id: ID-S05, url: "https://www.adobe.com/products/indesign/features.html", note: "Adobe InDesign product features page." }
  - { id: ID-S06, url: "https://developer.adobe.com/indesign/", note: "Adobe InDesign developer portal." }
  - { id: ID-S07, url: "https://developer.adobe.com/indesign/uxp/resources/fundamentals/dom-versioning/", note: "InDesign UXP DOM versioning." }
  - { id: ID-S08, url: "https://developer.adobe.com/indesign/uxp/resources/fundamentals/object-model/", note: "InDesign UXP object model." }
  - { id: ID-S09, url: "https://developer.adobe.com/indesign/uxp/resources/recipes/indesign-events/", note: "InDesign event scripting." }
  - { id: ID-S10, url: "https://www.adobe.com/products/indesignserver.html", note: "InDesign Server automation." }
```
