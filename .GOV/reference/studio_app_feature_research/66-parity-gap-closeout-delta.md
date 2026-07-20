---
file_id: studio-app-feature-research-parity-gap-closeout-delta
topic_id: SFR-CLOSE
title: "Parity Gap Closeout Delta (2026-07-21)"
status: draft
summary: "Gap-closing research unblocked by operator decisions STUDIO-DEC-001/003: CJK + RTL/ME typography (now in scope), accessibility-EXCEED bar + strategy, print/prepress depth, automation capability model, performance NFR targets. Online-source, NON-AI, local-first."
sources: 152
updated_at: "2026-07-21"
---


## [SFR-CLOSE] Parity Gap Closeout Delta

### [SFR-CLOSE.summary] Summary

```json
{
  "date": "2026-07-21",
  "driver": "Operator scope decisions STUDIO-DEC-001 (CJK/RTL/ME IN), STUDIO-DEC-003 (accessibility EXCEED), STUDIO-DEC-004 (local-first open-source full-parity positioning).",
  "method": "Online-source research, NON-AI, no vendor apps. Each row cites an authoritative source URL. Accessibility lane rows carry an exceed_strategy.",
  "total_rows": 152,
  "by_lane": {
    "cjk-typography": 33,
    "rtl-me-complex": 24,
    "accessibility-exceed": 23,
    "print-prepress-depth": 30,
    "automation-model-depth": 26,
    "performance-nfr-targets": 16
  },
  "authority": "Reference/provenance only; feeds WP-KERNEL-STUDIO refinement Section-14 coverage decisions."
}
```

### [SFR-CLOSE.cjk-typography] CJK typography full depth (composite fonts / kinsoku / mojikumi / ruby / warichu / kenten / tate-chu-yoko / vertical) — STUDIO-DEC-001 IN SCOPE (33 rows)

```json
{
  "rows": [
    {
      "feature": "Composite fonts (per-script font mixing)",
      "app_behavior": "Composite Font Editor assigns separate fonts to component classes Kanji (base), Kana, Punctuation, Symbols, Roman, and Numbers; each non-Kanji class takes independent Size (% or Q), Baseline shift, and Vertical/Horizontal Scale relative to the Kanji base; small characters can be aligned to the base via Roman Baseline, Embox, or ICF. Only in CJK/Middle East (J/ME) SKUs, not Western.",
      "app_or_standard": "InDesign (J/ME SKU); InCopy",
      "primitive_domain": "composite-fonts",
      "source_url": "https://helpx.adobe.com/indesign/using/formatting-cjk-characters.html",
      "verification": "VERIFIED",
      "closes_gap": "No Western/Affinity/Figma/Photoshop-standard tool exposes per-script size/baseline/scale font mixing as a saved font object; Handshake ships composite-font primitive natively.",
      "id": "SFR-CLOSE-cjk-typography-01"
    },
    {
      "feature": "Composite fonts in Illustrator",
      "app_behavior": "Type > Composite Fonts combines Asian and Roman fonts into one named composite; component rows (Kanji/Kana/Punctuation/Symbol/Roman/Number) each get size and baseline; East Asian options must first be enabled in Preferences to expose the menu.",
      "app_or_standard": "Illustrator (with Show Asian Options)",
      "primitive_domain": "composite-fonts",
      "source_url": "https://helpx.adobe.com/illustrator/using/formatting-asian-characters.html",
      "verification": "VERIFIED",
      "closes_gap": "Confirms composite-font is an Adobe-suite primitive Handshake must match across both layout and vector surfaces.",
      "id": "SFR-CLOSE-cjk-typography-02"
    },
    {
      "feature": "Kinsoku shori line-break prohibition",
      "app_behavior": "Kinsoku sets classify characters that may not start a line (no-start: closing brackets, small kana, sound marks, punctuation like 、。) or end a line (no-end: opening brackets). Resolution via Push In First (fit onto current line) vs Push Out First (move to next line); editable Kinsoku set with Can Not Begin/Can Not End/Hanging lists; based on JIS X 4051.",
      "app_or_standard": "InDesign/InCopy; Illustrator; standard JIS X 4051",
      "primitive_domain": "kinsoku",
      "source_url": "https://helpx.adobe.com/indesign/using/composing-cjk-characters.html",
      "verification": "VERIFIED",
      "closes_gap": "Affinity and Figma lack any line-break-prohibition engine; Handshake needs a JIS X 4051 kinsoku classifier with push-in/push-out.",
      "id": "SFR-CLOSE-cjk-typography-03"
    },
    {
      "feature": "Burasagari (hanging punctuation)",
      "app_behavior": "Burasagari hangs line-ending periods/commas (、。) outside the text-frame edge; modes None / Regular / Force. Not defined in JIS X 4051 body (explained sec. 8.1c), so it is an app-level extension of kinsoku.",
      "app_or_standard": "InDesign/InCopy; Illustrator",
      "primitive_domain": "kinsoku",
      "source_url": "https://helpx.adobe.com/indesign/using/composing-cjk-characters.html",
      "verification": "VERIFIED",
      "closes_gap": "Hanging-punctuation for CJK glyphs (not just Latin optical margin) is absent outside Adobe; Handshake ships burasagari None/Regular/Force.",
      "id": "SFR-CLOSE-cjk-typography-04"
    },
    {
      "feature": "Mojikumi aki spacing tables",
      "app_behavior": "Mojikumi Settings dialog defines per-class aki (spacing): desired/minimum/maximum value plus priority order for each of ~ line-start, line-end, punctuation, brackets, middle-dot, numbers, Roman-adjacent classes; min/max used when justifying by kinsoku; Indicate Differences highlights values differing from a compared set; Use CID-Based Mojikumi derives JIS X 4051 class from font glyph (jikei) not Unicode.",
      "app_or_standard": "InDesign/InCopy",
      "primitive_domain": "mojikumi",
      "source_url": "https://helpx.adobe.com/indesign/using/composing-cjk-characters.html",
      "verification": "VERIFIED",
      "closes_gap": "Editable per-class aki spacing tables with priority ordering are unique to Adobe J SKU; Handshake needs full mojikumi table editor.",
      "id": "SFR-CLOSE-cjk-typography-05"
    },
    {
      "feature": "Mojikumi preset sets",
      "app_behavior": "Predefined JIS X 4051-1995 sets selectable per paragraph: YakumonoHankaku (half-width punctuation), YakumonoZenkaku (full-width punctuation), GyoumatsuYakumonoHankaku (default: full-width except line-end punctuation to half), GyoumatsuYakumonoZenkaku (full-width including line-end).",
      "app_or_standard": "InDesign/InCopy; Photoshop; Illustrator",
      "primitive_domain": "mojikumi",
      "source_url": "https://helpx.adobe.com/indesign/using/composing-cjk-characters.html",
      "verification": "VERIFIED",
      "closes_gap": "Named punctuation-spacing presets ship in all Adobe J apps but nowhere else; Handshake bundles the standard JIS presets.",
      "id": "SFR-CLOSE-cjk-typography-06"
    },
    {
      "feature": "Ruby type: mono vs group",
      "app_behavior": "Ruby Type = Per-Character (Mono) ruby aligns each ruby char over its individual parent char; Group Ruby centers the whole ruby string over the whole parent string. Placement above horizontal / right of vertical text.",
      "app_or_standard": "InDesign/InCopy; standard (JLREQ)",
      "primitive_domain": "ruby",
      "source_url": "https://helpx.adobe.com/indesign/desktop/language-and-proofing/chinese-japanese-and-korean/add-and-format-ruby-text-annotations.html",
      "verification": "VERIFIED",
      "closes_gap": "Figma/Affinity have no ruby; Handshake ships mono + group ruby as a first-class annotation object.",
      "id": "SFR-CLOSE-cjk-typography-07"
    },
    {
      "feature": "Ruby alignment and spacing",
      "app_behavior": "Alignment menu (center, flush left/right, 1-2-1 JIS rule, equal/full justification), Auto Align At line ends, Char Width Scaling to auto-condense oversized ruby, XOffset/YOffset for parent-to-ruby distance, and ruby Font/Size overrides.",
      "app_or_standard": "InDesign/InCopy; standard JISx4051-1995 / JLREQ",
      "primitive_domain": "ruby",
      "source_url": "https://helpx.adobe.com/indesign/desktop/language-and-proofing/chinese-japanese-and-korean/add-and-format-ruby-text-annotations.html",
      "verification": "VERIFIED",
      "closes_gap": "Promotable ruby needs alignment rules + offsets + auto-scaling, not just superscript text; Handshake must implement JIS ruby distribution.",
      "id": "SFR-CLOSE-cjk-typography-08"
    },
    {
      "feature": "Ruby overhang (jukugo-style overflow)",
      "app_behavior": "When ruby is wider than its parents, Overhang lets ruby overflow into the space above adjacent characters; overhang-eligible neighbor character types comply with JISx4051-1995; per-side overhang amount configurable (ruby-overhang property in DOM).",
      "app_or_standard": "InDesign/InCopy; standard JISx4051-1995",
      "primitive_domain": "ruby",
      "source_url": "https://developer.adobe.com/indesign/uxp/dom/api/r/ruby-overhang/",
      "verification": "VERIFIED",
      "closes_gap": "Ruby overhang into neighboring cells is a subtle JIS behavior missing from web/Figma ruby; Handshake ships overhang controls.",
      "id": "SFR-CLOSE-cjk-typography-09"
    },
    {
      "feature": "Warichu (inline two-line note)",
      "app_behavior": "Warichu sets a run as 2+ stacked mini-lines within one line height; options: number of Lines, Line Gap, Alignment, Char Size scale, and Auto Adjust with a Line Break/character-count threshold controlling how many chars go before/after the split.",
      "app_or_standard": "InDesign/InCopy; Illustrator",
      "primitive_domain": "warichu",
      "source_url": "https://helpx.adobe.com/indesign/using/formatting-cjk-characters.html",
      "verification": "VERIFIED",
      "closes_gap": "Inline multi-line intercalary notes exist only in Adobe J apps; Handshake ships warichu with line count/gap/alignment/auto-split.",
      "id": "SFR-CLOSE-cjk-typography-10"
    },
    {
      "feature": "Kenten emphasis marks: mark set",
      "app_behavior": "Kenten (bouten) marks attach one glyph per character. Preset characters include sesame dot / white sesame dot, black circle / white circle, black triangle / white triangle, bullseye, fisheye, small black circle / small white circle, plus Custom (enter char or char-code).",
      "app_or_standard": "InDesign/InCopy (KentenCharacter enum)",
      "primitive_domain": "kenten",
      "source_url": "https://developer.adobe.com/indesign/dom/api/k/KentenCharacter",
      "verification": "VERIFIED",
      "closes_gap": "No emphasis-dot primitive outside Adobe J; Handshake ships the standard kenten glyph set plus custom char.",
      "id": "SFR-CLOSE-cjk-typography-11"
    },
    {
      "feature": "Kenten position and size",
      "app_behavior": "Kenten placement: Above/Right (above horizontal, right of vertical) or Below/Left; horizontal position Center or Left/above within the embox; independent kenten Font, Size, and Aki (spacing to parent) settings.",
      "app_or_standard": "InDesign/InCopy",
      "primitive_domain": "kenten",
      "source_url": "https://helpx.adobe.com/indesign/using/formatting-cjk-characters.html",
      "verification": "VERIFIED",
      "closes_gap": "Promotable kenten needs writing-mode-aware position + size + aki, not a static overline; Handshake must model kenten geometry.",
      "id": "SFR-CLOSE-cjk-typography-12"
    },
    {
      "feature": "Tate-chu-yoko (upright-in-vertical)",
      "app_behavior": "Rotates a short run of half-width chars (numbers/latin) to stay upright inside vertical text; manual apply from Character panel with XOffset (up/down; + = up) and YOffset (left/right) fine-positioning.",
      "app_or_standard": "InDesign/InCopy; Illustrator; Photoshop",
      "primitive_domain": "tate-chu-yoko",
      "source_url": "https://helpx.adobe.com/indesign/desktop/language-and-proofing/chinese-japanese-and-korean/apply-tate-chu-yoko-in-vertical-text.html",
      "verification": "VERIFIED",
      "closes_gap": "Upright runs inside vertical text are impossible in Figma/Affinity; Handshake ships tate-chu-yoko with offsets.",
      "id": "SFR-CLOSE-cjk-typography-13"
    },
    {
      "feature": "Auto Tate-chu-yoko",
      "app_behavior": "Paragraph attribute auto-uprights runs of up to N consecutive half-width characters (Numbers field sets max run length); applied automatically during composition in vertical frames.",
      "app_or_standard": "InDesign/InCopy",
      "primitive_domain": "tate-chu-yoko",
      "source_url": "https://helpx.adobe.com/indesign/desktop/language-and-proofing/chinese-japanese-and-korean/apply-tate-chu-yoko-in-vertical-text.html",
      "verification": "VERIFIED",
      "closes_gap": "Automatic uprighting by run-length threshold is Adobe-only; Handshake ships auto-TCY paragraph rule.",
      "id": "SFR-CLOSE-cjk-typography-14"
    },
    {
      "feature": "Vertical writing mode",
      "app_behavior": "Story-level vertical (tategaki) composition: columns fill top-to-bottom, right-to-left; frame Grid/text frames carry a writing-direction toggle; punctuation, brackets and kana take vertical forms; interacts with tate-chu-yoko, ruby-on-right, kenten-on-right.",
      "app_or_standard": "InDesign/InCopy; Illustrator; Photoshop",
      "primitive_domain": "vertical-writing",
      "source_url": "https://en.wikipedia.org/wiki/Vertical_writing",
      "verification": "VERIFIED",
      "closes_gap": "Figma has no native vertical text (plugin-only) and Affinity has none; Handshake ships vertical writing mode as core.",
      "id": "SFR-CLOSE-cjk-typography-15"
    },
    {
      "feature": "Vertical OpenType metrics/forms",
      "app_behavior": "Vertical composition consumes font features: vert (vertical alternates), vrt2 (vertical rotation, GSUB type-1 required for CFF vertical), vkna (vertical kana alternates), vpal (proportional vertical metrics), vhal (vertical half-width), vkrn (vertical kerning), plus VORG vertical-origin table.",
      "app_or_standard": "OpenType spec (Microsoft); InDesign consumes",
      "primitive_domain": "vertical-writing",
      "source_url": "https://learn.microsoft.com/en-us/typography/opentype/spec/features_uz",
      "verification": "VERIFIED",
      "closes_gap": "Correct vertical layout requires GSUB/GPOS vert/vrt2/vpal handling; Handshake shaping engine must honor these tags, unlike naive rotate-the-frame workarounds.",
      "id": "SFR-CLOSE-cjk-typography-16"
    },
    {
      "feature": "Shatai (oblique slanting)",
      "app_behavior": "Shatai slants glyphs by Angle 30/45/60 degrees and also scales them (film-lens emulation); Magnification field (10% = lens 1, 40% = lens 4) controls skew degree; distinct from plain skew because it rescales glyphs.",
      "app_or_standard": "InDesign/InCopy",
      "primitive_domain": "shatai",
      "source_url": "https://helpx.adobe.com/indesign/using/formatting-cjk-characters.html",
      "verification": "VERIFIED",
      "closes_gap": "Lens-style oblique-plus-scale is a photo-typesetting behavior absent elsewhere; Handshake ships shatai angle+magnification.",
      "id": "SFR-CLOSE-cjk-typography-17"
    },
    {
      "feature": "Character rotation (East Asian)",
      "app_behavior": "Per-character rotation in 90/180/-90 style values (+ = counterclockwise, - = clockwise); Adjust Rotation keeps horizontal strokes horizontal in horizontal text and vertical in vertical text; combinable with shatai.",
      "app_or_standard": "InDesign/InCopy",
      "primitive_domain": "char-rotation",
      "source_url": "https://helpx.adobe.com/indesign/desktop/language-and-proofing/chinese-japanese-and-korean/rotate-characters-in-east-asian-text.html",
      "verification": "VERIFIED",
      "closes_gap": "Glyph-level rotation aware of writing mode is Adobe-only; Handshake ships per-character rotation with Adjust Rotation.",
      "id": "SFR-CLOSE-cjk-typography-18"
    },
    {
      "feature": "Character alignment (mojisoroe)",
      "app_behavior": "Aligns mixed-size chars on a line to: Roman Baseline, Embox Top/Right, Embox Center, Embox Bottom/Left, ICF Top/Right, or ICF Bottom/Left (ICF = Ideographic Character Face, the designer's average ideograph box); direction labels swap for vertical frames.",
      "app_or_standard": "InDesign/InCopy; Illustrator",
      "primitive_domain": "character-alignment",
      "source_url": "https://helpx.adobe.com/indesign/desktop/language-and-proofing/chinese-japanese-and-korean/set-character-alignment-for-east-asian-text.html",
      "verification": "VERIFIED",
      "closes_gap": "Embox/ICF alignment references have no equivalent in Latin-only tools; Handshake must model embox + ICF metrics per font.",
      "id": "SFR-CLOSE-cjk-typography-19"
    },
    {
      "feature": "Kurikaeshi moji shori (repeat-char handling)",
      "app_behavior": "Controls iteration marks (odoriji, e.g. 々) at line breaks: when enabled both characters are shown if separated by a line break instead of using the repeat mark.",
      "app_or_standard": "InDesign/InCopy; Illustrator",
      "primitive_domain": "kurikaeshi",
      "source_url": "https://helpx.adobe.com/indesign/desktop/language-and-proofing/chinese-japanese-and-korean/set-character-alignment-for-east-asian-text.html",
      "verification": "VERIFIED",
      "closes_gap": "Iteration-mark expansion at breaks is a niche JIS rule Handshake includes for full parity.",
      "id": "SFR-CLOSE-cjk-typography-20"
    },
    {
      "feature": "Aki / tsume character spacing",
      "app_behavior": "CJK-specific spacing set in bu units (e.g. 2bu = half em, 4bu = quarter em); Tsume condenses per-glyph side bearings; Adjust Tsume applies jidori; Adjust Tracking With CJK Grid ties tracking to grid aki; distinct from Latin kerning/tracking.",
      "app_or_standard": "InDesign/InCopy",
      "primitive_domain": "character-spacing",
      "source_url": "https://helpx.adobe.com/incopy/using/changing-spacing-characters-cjk-text.html",
      "verification": "VERIFIED",
      "closes_gap": "Em-fraction (bu) spacing and tsume are absent from Latin-centric tools; Handshake ships bu-based aki + tsume.",
      "id": "SFR-CLOSE-cjk-typography-21"
    },
    {
      "feature": "OpenType CJK glyph-form features",
      "app_behavior": "OpenType/CJK panel exposes jp78 (JIS78 forms), jp83, jp90, jp04 form sets, expt (expert forms), nlck (NLC kanji), hojo (JIS X 0212), trad/smpl, plus half/full-width hwid/fwid/pwid; drives glyph substitution over CID-keyed fonts.",
      "app_or_standard": "InDesign/Illustrator/Photoshop; OpenType spec",
      "primitive_domain": "opentype-cjk",
      "source_url": "https://helpx.adobe.com/indesign/using/formatting-cjk-characters.html",
      "verification": "VERIFIED",
      "closes_gap": "Full JIS form-set switching needs GSUB feature access over CID fonts; Handshake shaping must expose jp78/jp90/expt/hojo etc.",
      "id": "SFR-CLOSE-cjk-typography-22"
    },
    {
      "feature": "CID-keyed font / Adobe-Japan1 access",
      "app_behavior": "CJK typesetting relies on CID-keyed CFF fonts (Adobe-Japan1 ROS) with large glyph inventories; Use CID-Based Mojikumi selects glyph-based (jikei) JIS X 4051 class; glyph access via CID/Unicode in Glyphs panel.",
      "app_or_standard": "InDesign/InCopy; Adobe CID-keyed font tech",
      "primitive_domain": "opentype-cjk",
      "source_url": "https://helpx.adobe.com/indesign/using/composing-cjk-characters.html",
      "verification": "VERIFIED",
      "closes_gap": "CID-keyed glyph addressing and Adobe-Japan1 mapping are prerequisites Handshake's font engine must support for pro CJK.",
      "id": "SFR-CLOSE-cjk-typography-23"
    },
    {
      "feature": "Half-width / full-width handling",
      "app_behavior": "Distinguishes hankaku (half-width) vs zenkaku (full-width) forms; hwid/fwid/pwid OpenType features and mojikumi presets convert punctuation/latin/kana between widths; half-width chars are the auto-TCY / tate-chu-yoko candidates.",
      "app_or_standard": "InDesign/Illustrator/Photoshop; OpenType",
      "primitive_domain": "halfwidth-fullwidth",
      "source_url": "https://en.wikipedia.org/wiki/List_of_typographic_features",
      "verification": "VERIFIED",
      "closes_gap": "Width-form conversion and awareness underpin CJK spacing; Handshake must track half/full-width state per run.",
      "id": "SFR-CLOSE-cjk-typography-24"
    },
    {
      "feature": "Layout grid (genko-yoshi document frame)",
      "app_behavior": "CJK document Layout Grid defines page by characters-per-line and lines-per-page; sets font, Char Aki, Line Aki, char size (Q/bu), and derives margins from grid; available only in Asian-language versions; foundation of manuscript (genko yoshi) layout.",
      "app_or_standard": "InDesign (J/ME SKU)",
      "primitive_domain": "grid",
      "source_url": "https://helpx.adobe.com/indesign/using/layout-grids.html",
      "verification": "VERIFIED",
      "closes_gap": "Character-count-driven page grids don't exist in Western DTP; Handshake ships genko-yoshi layout grid as CJK page model.",
      "id": "SFR-CLOSE-cjk-typography-25"
    },
    {
      "feature": "Frame grid (genko yoshi text frame)",
      "app_behavior": "Frame Grid text frames render a per-cell manuscript grid; properties: Font, Size, Vertical/Horizontal Scale, Char Aki, Line Aki, Char count (chars/line), Line count, Line Align, and grid view (grid/border/off); text snaps one glyph per cell.",
      "app_or_standard": "InDesign (J/ME SKU); InCopy",
      "primitive_domain": "grid",
      "source_url": "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/grids/set-frame-grid-properties.html",
      "verification": "VERIFIED",
      "closes_gap": "Per-cell one-glyph manuscript frames are unique to Adobe J; Handshake ships frame-grid text objects.",
      "id": "SFR-CLOSE-cjk-typography-26"
    },
    {
      "feature": "Named grids (reusable grid styles)",
      "app_behavior": "Named Grids panel saves grid formats as style objects (Font incl. variable-font sliders, Size, Vertical/Horizontal Scale, Char Aki, Line Aki, Line Align, Grid settings); apply consistently like paragraph styles; requires East Asian options.",
      "app_or_standard": "InDesign (J/ME SKU)",
      "primitive_domain": "grid",
      "source_url": "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/grids/create-apply-named-grids.html",
      "verification": "VERIFIED",
      "closes_gap": "Grid-as-style reuse has no analog outside Adobe J; Handshake ships named grid styles for consistent manuscript layout.",
      "id": "SFR-CLOSE-cjk-typography-27"
    },
    {
      "feature": "Japanese composer (JIS X 4051 engine)",
      "app_behavior": "Dedicated Adobe Japanese Paragraph Composer and Japanese Single-line Composer apply JIS X 4051 to run mojikumi (spacing classes) and kinsoku (break prohibition) together during line breaking; selectable per paragraph.",
      "app_or_standard": "InDesign/InCopy; Illustrator; standard JIS X 4051",
      "primitive_domain": "kinsoku",
      "source_url": "https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/mojikumi-kinsoku-overview.html",
      "verification": "VERIFIED",
      "closes_gap": "A JIS-X-4051 line-breaking composer is the core CJK engine Handshake must build; Latin composers cannot substitute.",
      "id": "SFR-CLOSE-cjk-typography-28"
    },
    {
      "feature": "Requirements for Japanese Text Layout (JLREQ) standard",
      "app_behavior": "W3C JLREQ codifies the target rules Handshake must implement: kinsoku classes, mojikumi spacing categories, ruby placement (mono/group/jukugo), warichu, tate-chu-yoko, emphasis dots, vertical writing, and line-adjustment (oidashi/oikomi) algorithms.",
      "app_or_standard": "Standard: W3C JLREQ",
      "primitive_domain": "kinsoku",
      "source_url": "https://www.w3.org/TR/jlreq/",
      "verification": "VERIFIED",
      "closes_gap": "JLREQ is the open, vendor-neutral spec Handshake can implement fully offline, exceeding closed Adobe-only behavior with a documented standard.",
      "id": "SFR-CLOSE-cjk-typography-29"
    },
    {
      "feature": "Simple ruby placement rules (W3C)",
      "app_behavior": "W3C Simple Ruby defines jukugo (per-character with group fallback), mono, and group placement, overhang eligibility, and 1-2-1 / 2-3-2 distribution rules for open-source ruby engines.",
      "app_or_standard": "Standard: W3C Simple Ruby",
      "primitive_domain": "ruby",
      "source_url": "https://w3c.github.io/simple-ruby/",
      "verification": "VERIFIED",
      "closes_gap": "Open ruby spec gives Handshake an implementable, offline reference for jukugo ruby that Figma/Affinity never shipped.",
      "id": "SFR-CLOSE-cjk-typography-30"
    },
    {
      "feature": "Affinity CJK gap (vertical/ruby absent)",
      "app_behavior": "Affinity Publisher accepts Unicode CJK input but has no vertical text, no RTL, no ruby, no kinsoku/mojikumi engine; vertical is only faked by rotating a text frame 90 degrees.",
      "app_or_standard": "Affinity Publisher",
      "primitive_domain": "vertical-writing",
      "source_url": "https://forum.affinity.serif.com/index.php?/topic/68887-japanese-vertical-text/",
      "verification": "UNVERIFIED",
      "closes_gap": "Confirms a real incumbent hole; Handshake local-first CJK exceeds Affinity by shipping vertical + ruby + kinsoku natively.",
      "id": "SFR-CLOSE-cjk-typography-31"
    },
    {
      "feature": "Figma CJK gap (no native vertical/ruby)",
      "app_behavior": "Figma renders horizontal CJK via web fonts but has no native vertical writing mode or ruby; vertical is community-plugin-only (e.g. Vertja) bounded by Plugin API limits.",
      "app_or_standard": "Figma",
      "primitive_domain": "vertical-writing",
      "source_url": "https://forum.figma.com/suggest-a-feature-11/east-asian-vertical-text-31111",
      "verification": "UNVERIFIED",
      "closes_gap": "Another incumbent gap; Handshake's desktop-class local module provides vertical+ruby without plugins.",
      "id": "SFR-CLOSE-cjk-typography-32"
    },
    {
      "feature": "Photoshop East Asian type parity",
      "app_behavior": "Photoshop (East Asian Features enabled via Character/Paragraph panel menu) supports tate-chu-yoko, mojikumi presets (JIS X 4051-1995), burasagari, kinsoku, and vertical type, sharing the CJK type engine with InDesign at a reduced feature set.",
      "app_or_standard": "Photoshop",
      "primitive_domain": "mojikumi",
      "source_url": "https://helpx.adobe.com/in/photoshop/using/asian-type.html",
      "verification": "VERIFIED",
      "closes_gap": "Shows CJK type primitives extend even to the raster app; Handshake's Studio module should expose the same across surfaces.",
      "id": "SFR-CLOSE-cjk-typography-33"
    }
  ]
}
```

### [SFR-CLOSE.rtl-me-complex] RTL / Middle-East / complex-script typography (bidi / kashida / digit shaping / Indic / SEA) — STUDIO-DEC-001 IN SCOPE (24 rows)

```json
{
  "rows": [
    {
      "feature": "Unicode Bidirectional Algorithm (bidi) resolution",
      "app_behavior": "Implement UAX #9: assign paragraph embedding level (rules P2/P3), resolve explicit embeddings/overrides (LRE/RLE/LRO/RLO + PDF), resolve weak types (numbers, separators), neutral types, and implicit levels (rules L1-L4); reorder runs by resolved level for display. Directional status/override stack with max_depth 125.",
      "primitive_domain": "bidi",
      "app_or_standard": "Unicode Standard Annex #9 (UAX #9)",
      "source_url": "https://unicode.org/reports/tr9/",
      "verification": "VERIFIED",
      "closes_gap": "Core RTL/mixed-direction layout; nothing in a Latin-only engine handles level resolution.",
      "id": "SFR-CLOSE-rtl-me-complex-01"
    },
    {
      "feature": "Bidi isolate controls preferred over legacy embeddings",
      "app_behavior": "Support isolate initiators LRI (U+2066), RLI (U+2067), FSI (U+2068) with terminator PDI (U+2069); these are NOT stripped like LRE/RLE/LRO/RLO/PDF and define isolating run sequences, preventing spillover of surrounding direction into embedded runs. Unicode recommends RLI/LRI over RLE/LRE.",
      "primitive_domain": "bidi",
      "app_or_standard": "UAX #9 / W3C i18n bidi controls",
      "source_url": "https://www.w3.org/International/questions/qa-bidi-unicode-controls",
      "verification": "VERIFIED",
      "closes_gap": "Correct isolation of runs (e.g. RTL name inside LTR sentence) without direction bleed.",
      "id": "SFR-CLOSE-rtl-me-complex-02"
    },
    {
      "feature": "Bidi mirroring + directional marks",
      "app_behavior": "Apply glyph mirroring for characters with Bidi_Mirrored=Yes (brackets, parens, angle quotes) when resolved to RTL level (rule L4 / Bidi_Mirroring_Glyph); support zero-width RLM (U+200F) / LRM (U+200E) / ALM (U+061C) to force directionality at boundaries.",
      "primitive_domain": "bidi",
      "app_or_standard": "UAX #9 (BidiMirroring.txt)",
      "source_url": "https://unicode.org/reports/tr9/",
      "verification": "VERIFIED",
      "closes_gap": "Brackets/quotes visually correct in RTL; boundary control marks.",
      "id": "SFR-CLOSE-rtl-me-complex-03"
    },
    {
      "feature": "Kashida (tatweel) justification levels",
      "app_behavior": "Arabic text justified by elongating letters with kashida rather than only inter-word spacing. Per-paragraph Insert Kashida control with discrete levels: None, Short, Medium, Long, Stylistic. Uses U+0640 ARABIC TATWEEL / font kashida behavior.",
      "primitive_domain": "justification",
      "app_or_standard": "InDesign MENA (World-Ready)",
      "source_url": "https://helpx.adobe.com/indesign/desktop/language-and-proofing/arabic-and-hebrew/justify-arabic-text.html",
      "verification": "VERIFIED",
      "closes_gap": "Native Arabic justification method absent from Latin spacing-only justifier.",
      "id": "SFR-CLOSE-rtl-me-complex-04"
    },
    {
      "feature": "Kashida insertion placement rules",
      "app_behavior": "Kashida elongation must be inserted only at letter connections that legally accept tatweel, governed by joining rules and priority ordering (preferred kashida positions per letter/context) rather than arbitrary stretching; automatic vs manual (typed U+0640) insertion. Exact priority/eligibility table per script is font/engine-defined.",
      "primitive_domain": "justification",
      "app_or_standard": "Arabic shaping / InDesign kashida",
      "source_url": "https://helpx.adobe.com/indesign/desktop/language-and-proofing/arabic-and-hebrew/justify-arabic-text.html",
      "verification": "UNVERIFIED",
      "closes_gap": "Prevents illegal/ugly elongation; needs a documented kashida-eligibility ruleset.",
      "id": "SFR-CLOSE-rtl-me-complex-05"
    },
    {
      "feature": "Non-kashida glyph justification (jalt)",
      "app_behavior": "Where kashida is unavailable/undesired, use OpenType 'jalt' (Justification Alternates) alternate glyph forms and script-appropriate width variation for justification of non-Latin runs.",
      "primitive_domain": "justification",
      "app_or_standard": "OpenType feature registry (jalt)",
      "source_url": "https://harfbuzz.github.io/shaping-opentype-features.html",
      "verification": "UNVERIFIED",
      "closes_gap": "Justification path for fonts/scripts lacking kashida; jalt specifics need confirmation.",
      "id": "SFR-CLOSE-rtl-me-complex-06"
    },
    {
      "feature": "Arabic/Hebrew digit type selection",
      "app_behavior": "Three numeral renderings selectable for RTL text: Arabic (European 0-9), Hindi (Arabic-Indic U+0660-0669), and Farsi (Extended/Persian Arabic-Indic U+06F0-06F9); plus find/replace transliteration between digit systems.",
      "primitive_domain": "digit-shaping",
      "app_or_standard": "InDesign MENA",
      "source_url": "https://helpx.adobe.com/indesign/desktop/language-and-proofing/arabic-and-hebrew/choose-digit-types.html",
      "verification": "VERIFIED",
      "closes_gap": "National digit shaping (Arabic vs Hindi vs Farsi) not in Latin engine.",
      "id": "SFR-CLOSE-rtl-me-complex-07"
    },
    {
      "feature": "Contextual digit shaping / directionality",
      "app_behavior": "Default digit-type resolves by surrounding script context (national digit substitution) and digits keep LTR run order inside RTL text per bidi weak-type resolution; toggle to force a digit system regardless of context.",
      "primitive_domain": "digit-shaping",
      "app_or_standard": "InDesign MENA / UAX #9 weak types",
      "source_url": "https://helpx.adobe.com/indesign/using/arabic-hebrew.html",
      "verification": "VERIFIED",
      "closes_gap": "Correct numeric run order and contextual national digits.",
      "id": "SFR-CLOSE-rtl-me-complex-08"
    },
    {
      "feature": "Paragraph / character / story direction levels",
      "app_behavior": "Set base direction independently at story level (RTL/LTR story direction toggle), paragraph level (paragraph direction), and character level (character direction override), enabling mixed bidi within one frame.",
      "primitive_domain": "text-direction",
      "app_or_standard": "InDesign MENA",
      "source_url": "https://helpx.adobe.com/indesign/desktop/language-and-proofing/arabic-and-hebrew/change-text-direction.html",
      "verification": "VERIFIED",
      "closes_gap": "Three-tier direction model vs single global direction flag.",
      "id": "SFR-CLOSE-rtl-me-complex-09"
    },
    {
      "feature": "RTL document binding / spread direction",
      "app_behavior": "Document-level binding direction (right-bound spine) for RTL books so pages progress right-to-left, spreads mirror, and default new-frame direction is RTL; set at document creation (Binding = Right).",
      "primitive_domain": "document-direction",
      "app_or_standard": "InDesign MENA",
      "source_url": "https://helpx.adobe.com/indesign/desktop/language-and-proofing/arabic-and-hebrew/change-text-direction.html",
      "verification": "VERIFIED",
      "closes_gap": "Page/spread progression and spine side for RTL publications.",
      "id": "SFR-CLOSE-rtl-me-complex-10"
    },
    {
      "feature": "Arabic cursive contextual shaping (joining forms)",
      "app_behavior": "Select isolated/initial/medial/final glyph form per letter by tracking join_type (dual/right/left/non/transparent) and join causing; apply init/medi/fina/isol GSUB features. HarfBuzz Arabic shaper covers Arabic, Persian, Urdu, Syriac, N'Ko.",
      "primitive_domain": "arabic-shaping",
      "app_or_standard": "HarfBuzz Arabic shaper / OpenType",
      "source_url": "https://harfbuzz.github.io/why-do-i-need-a-shaping-engine.html",
      "verification": "VERIFIED",
      "closes_gap": "Cursive joining is mandatory; without it Arabic renders as disconnected isolated letters.",
      "id": "SFR-CLOSE-rtl-me-complex-11"
    },
    {
      "feature": "Arabic required ligatures + contextual alternates",
      "app_behavior": "Activate 'rlig' by default (mandatory lam-alef ligature and others) always, and 'calt' contextual alternates by default for horizontal runs; support discretionary 'liga'/'dlig' as opt-in.",
      "primitive_domain": "arabic-shaping",
      "app_or_standard": "HarfBuzz / OpenType feature registry",
      "source_url": "https://harfbuzz.github.io/shaping-opentype-features.html",
      "verification": "VERIFIED",
      "closes_gap": "lam-alef and required ligatures are non-optional for correct Arabic.",
      "id": "SFR-CLOSE-rtl-me-complex-12"
    },
    {
      "feature": "Mark-to-base / mark-to-mark diacritic positioning (GPOS)",
      "app_behavior": "Attach combining marks with GPOS Lookup 4 (MarkToBase) and Lookup 6 (MarkToMark) via anchor points, for Arabic harakat, Hebrew niqqud/cantillation, Vietnamese stacked tones; MarkToLigature (Lookup 5) for marks on ligatures.",
      "primitive_domain": "mark-positioning",
      "app_or_standard": "OpenType GPOS spec",
      "source_url": "https://learn.microsoft.com/en-us/typography/opentype/spec/gpos",
      "verification": "VERIFIED",
      "closes_gap": "Correctly stacked vowels/diacritics vs default-advance overlap.",
      "id": "SFR-CLOSE-rtl-me-complex-13"
    },
    {
      "feature": "Mark reordering before positioning",
      "app_behavior": "Reorder adjacent combining marks into canonical visual order (per canonical combining class / script rules) prior to applying GPOS mark-to-base and mark-to-mark, so stacked diacritics attach in correct sequence.",
      "primitive_domain": "mark-positioning",
      "app_or_standard": "OpenType Arabic shaping docs",
      "source_url": "https://github.com/n8willis/opentype-shaping-documents/blob/master/opentype-shaping-arabic-general.md",
      "verification": "VERIFIED",
      "closes_gap": "Prevents mis-stacked diacritics when input mark order varies.",
      "id": "SFR-CLOSE-rtl-me-complex-14"
    },
    {
      "feature": "HarfBuzz-class shaping engine core",
      "app_behavior": "Ground a native offline shaper on the HarfBuzz model: buffer of Unicode + script/language/direction -> hb_shape() -> glyph IDs + positions; read GSUB (substitution) / GPOS (positioning) / GDEF (glyph classes) tables; cache reusable shape plans per (face, script, features).",
      "primitive_domain": "shaping-engine",
      "app_or_standard": "HarfBuzz Manual (open source, MIT/Old-MIT)",
      "source_url": "https://harfbuzz.github.io/shaping-and-shape-plans.html",
      "verification": "VERIFIED",
      "closes_gap": "Foundational offline shaping stage for all complex scripts; reusable proven OSS.",
      "id": "SFR-CLOSE-rtl-me-complex-15"
    },
    {
      "feature": "Indic reordering shaping model",
      "app_behavior": "Cluster-based syllable model: apply rphf (reph above-base Ra), pref, then blwf/half/pstf/abvf/vatu/cjct; decompose 2-3 part matras and reposition matras/reph/syllable modifiers relative to base consonant; feature stages locl, ccmp, nukt, akhn first.",
      "primitive_domain": "indic-shaping",
      "app_or_standard": "Microsoft OpenType Indic dev spec (Devanagari)",
      "source_url": "https://learn.microsoft.com/en-us/typography/script-development/devanagari",
      "verification": "VERIFIED",
      "closes_gap": "Conjuncts, reph, matra reordering essential for Devanagari/Bengali/Tamil etc.",
      "id": "SFR-CLOSE-rtl-me-complex-16"
    },
    {
      "feature": "Universal Shaping Engine (USE) for other complex scripts",
      "app_behavior": "Single generic shaper driven by Unicode character properties + font's USE feature model to cover complex scripts lacking a dedicated shaper; ordered feature application (locl/ccmp/nukt/akhn, then rphf/pref, then rkrf/abvf/blwf/half/pstf/vatu/cjct).",
      "primitive_domain": "indic-shaping",
      "app_or_standard": "USE (Microsoft/HarfBuzz)",
      "source_url": "https://simoncozens.github.io/use/",
      "verification": "VERIFIED",
      "closes_gap": "Broad complex-script coverage (Javanese, Khmer, Tibetan, etc.) without per-script code.",
      "id": "SFR-CLOSE-rtl-me-complex-17"
    },
    {
      "feature": "UAX #14 line-breaking algorithm",
      "app_behavior": "Assign line-break class per char, apply pair-table to find mandatory breaks (BK/LF/CR/NL), no-break/opportunity rules, non-breaking/CJK rules; SA (South-East Asian) class marked as requiring higher-level analysis.",
      "primitive_domain": "line-breaking",
      "app_or_standard": "Unicode Standard Annex #14 (UAX #14)",
      "source_url": "http://www.unicode.org/reports/tr14/tr14-39.html",
      "verification": "VERIFIED",
      "closes_gap": "Standards-correct break opportunities across scripts.",
      "id": "SFR-CLOSE-rtl-me-complex-18"
    },
    {
      "feature": "Thai/SEA dictionary-based line breaking",
      "app_behavior": "Thai/Lao/Khmer/Myanmar have no inter-word spaces; break opportunities require dictionary/word-segmentation (SA class is out of UAX #14 scope). Break at syllable/word boundaries via dictionary lookup analogous to hyphenation.",
      "primitive_domain": "line-breaking",
      "app_or_standard": "UAX #14 (SA class) / ICU-class dictionary segmentation",
      "source_url": "https://www.unicode.org/reports/tr14/tr14-15.html",
      "verification": "VERIFIED",
      "closes_gap": "Correct wrapping for spaceless SEA scripts; needs bundled dictionary.",
      "id": "SFR-CLOSE-rtl-me-complex-19"
    },
    {
      "feature": "Text segmentation (grapheme/word/cluster boundaries)",
      "app_behavior": "UAX #29 grapheme-cluster and word boundaries for cursor movement, selection, and deletion over complex clusters (e.g. Indic aksara, emoji ZWJ sequences, Arabic mark clusters) so caret does not split a shaped cluster.",
      "primitive_domain": "text-segmentation",
      "app_or_standard": "Unicode Standard Annex #29 (UAX #29)",
      "source_url": "https://www.unicode.org/reports/tr29/tr29-22.html",
      "verification": "VERIFIED",
      "closes_gap": "Correct caret/selection over complex clusters in the editor.",
      "id": "SFR-CLOSE-rtl-me-complex-20"
    },
    {
      "feature": "Diacritic position + color adjustment (Arabic)",
      "app_behavior": "Manual Adjust Horizontal Diacritic Position and Adjust Vertical Diacritic Position values in the Character panel; plus find/change Arabic diacritic color to color harakat differently for styling/legibility.",
      "primitive_domain": "mark-positioning",
      "app_or_standard": "InDesign MENA",
      "source_url": "https://helpx.adobe.com/indesign/using/arabic-hebrew.html",
      "verification": "VERIFIED",
      "closes_gap": "Fine manual diacritic control beyond font default anchors.",
      "id": "SFR-CLOSE-rtl-me-complex-21"
    },
    {
      "feature": "Bidi-aware line composer (World-Ready Composer)",
      "app_behavior": "Adobe World-Ready Paragraph/Single-line Composers are the bidi-aware composition engine required for Arabic/Hebrew and other complex scripts; they perform bidi reordering + shaping + justification as an integrated line-breaking pass (vs Latin-only Adobe Paragraph Composer).",
      "primitive_domain": "shaping-engine",
      "app_or_standard": "InDesign World-Ready Composer",
      "source_url": "https://helpx.adobe.com/indesign/desktop/language-and-proofing/language-settings/adobe-world-ready-composer-overview.html",
      "verification": "VERIFIED",
      "closes_gap": "Composer must integrate bidi+shaping+justify, not bolt on after.",
      "id": "SFR-CLOSE-rtl-me-complex-22"
    },
    {
      "feature": "Hebrew shaping (niqqud + cantillation)",
      "app_behavior": "Position Hebrew vowel points (niqqud) and cantillation (te'amim) marks contextually via GPOS mark attachment; correct mark position depends on presence of other marks and must be handled contextually by the font/shaper; support final-form letters and RTL bidi.",
      "primitive_domain": "mark-positioning",
      "app_or_standard": "OpenType Hebrew shaping docs / Microsoft",
      "source_url": "https://learn.microsoft.com/en-us/typography/script-development/hebrew",
      "verification": "VERIFIED",
      "closes_gap": "Biblical/pointed Hebrew stacking correctness.",
      "id": "SFR-CLOSE-rtl-me-complex-23"
    },
    {
      "feature": "Middle East / MENA edition capability gating",
      "app_behavior": "Incumbent gates all above RTL/complex features behind the Middle East & North Africa (MENA) SKU / World-Ready composer install; standard InDesign lacks kashida, digit types, RTL direction, and binding. Handshake ships these in the base local build (no separate SKU).",
      "primitive_domain": "text-direction",
      "app_or_standard": "InDesign MENA edition requirement",
      "source_url": "https://helpx.adobe.com/indesign/using/arabic-hebrew.html",
      "verification": "VERIFIED",
      "closes_gap": "Parity target: RTL/complex-script support as default, not a paid regional SKU.",
      "id": "SFR-CLOSE-rtl-me-complex-24"
    }
  ]
}
```

### [SFR-CLOSE.accessibility-exceed] Accessible-output EXCEED bar (PDF/UA-1/2, WCAG 2.2, EN 301 549/EAA, EPUB3, semantic web) + exceed strategy — STUDIO-DEC-003 (23 rows)

```json
{
  "rows": [
    {
      "feature": "PDF/UA-1 logical structure tree (tagged PDF)",
      "app_behavior": "ISO 14289-1:2014 (on PDF 1.7 / ISO 32000-1) requires a tag tree where every piece of real content is tagged in logical reading order and tags fully represent the author's semantic (not literal) intent; standard structure types (Document, Part, Sect, H1-H6/H, P, Figure, Table, L/LI) must carry correct semantics and role mapping so AT can announce roles and relationships.",
      "primitive_domain": "document-model-structure",
      "source_url": "https://pdfa.org/resource/iso-14289-pdfua/",
      "app_or_standard": "PDF/UA-1 (ISO 14289-1)",
      "closes_gap": "Illustrator and Figma emit untagged/flattened PDF with no structure tree at all",
      "exceed_strategy": "Make the tag tree a first-class primitive in Studio's native document model (roles bound to content objects, not a post-export afterthought) so structure survives edits AND re-export deterministically instead of being re-authored in Acrobat each round-trip",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-01"
    },
    {
      "feature": "Artifact vs real-content separation",
      "app_behavior": "Every marked-content sequence is classified as EITHER real content (in the tag tree) OR an /Artifact (excluded from tags); pagination artifacts (page numbers, running headers/footers, table rules, decorative rules/images) MUST be marked as Artifact so screen readers do not repeat them on every page; content can never be both.",
      "primitive_domain": "reading-order",
      "source_url": "https://pdfa.org/understanding-pdf-accessibility-techniques/",
      "app_or_standard": "PDF/UA-1 (ISO 14289-1)",
      "closes_gap": "Flattened exporters leak decorative/pagination content into reading order or drop the artifact distinction entirely",
      "exceed_strategy": "Auto-classify master-page/running-header/footer and decorative objects as artifacts at layout time (derive from the object's role in the page model) so born-accessible artifact marking is the default, with a design-time toggle rather than manual per-object marking",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-02"
    },
    {
      "feature": "Alternative text and ActualText",
      "app_behavior": "Non-text content (Figure and other non-text structure elements) requires an /Alt entry; text represented as glyphs/graphics requires /ActualText so the real Unicode is available to AT and copy/extract; alt is added per object and must export into the PDF (and to HTML/EPUB) tag stream.",
      "primitive_domain": "alt-text",
      "source_url": "https://www.pdflib.com/pdf-knowledge-base/pdfua/requirements/",
      "app_or_standard": "PDF/UA-1 (ISO 14289-1)",
      "closes_gap": "Figma outlines text to vectors (no ActualText, unreadable); Illustrator has zero alt-text authoring",
      "exceed_strategy": "Bind alt text and ActualText to the source object once and reuse across every export target (PDF, EPUB, HTML) so a single authored description flows to all born-accessible outputs; surface a missing-alt lint inline on canvas during design",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-03"
    },
    {
      "feature": "Logical reading order independent of layout",
      "app_behavior": "Reading order is defined by tag-tree order, not by x/y position on the page; multi-column, sidebars, and floated content must be ordered by author intent in the structure, matching InDesign's Articles-panel 'Use for Tagging Order' mechanic.",
      "primitive_domain": "reading-order",
      "source_url": "https://dap.berkeley.edu/documents-forms/pdfs/create-accessible-pdfs-indesign",
      "app_or_standard": "PDF/UA-1 (ISO 14289-1) / InDesign Articles panel",
      "closes_gap": "Positional-order exporters produce scrambled reading order for multi-column layouts",
      "exceed_strategy": "Derive default reading order from the linked text-flow/thread graph in the document model (InDesign requires manual Articles-panel dragging) and offer a live reading-order overlay that reads back the exact tag sequence before export",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-04"
    },
    {
      "feature": "Accessible table structure (Scope / Headers-IDs)",
      "app_behavior": "Tables tag as Table > TR > TH/TD (THead/TBody/TFoot optional); every TH must carry a Scope attribute (Row/Column/Both) or, for complex/spanning tables, TD cells associate to header cells via Headers + ID attributes; missing Scope fails Matterhorn Protocol checkpoint 15-003.",
      "primitive_domain": "tables",
      "source_url": "https://pdfix.net/how-to-automatically-set-table-cell-scope-in-tagged-pdfs/",
      "app_or_standard": "PDF/UA-1 (ISO 14289-1)",
      "closes_gap": "Design tools export tables as positioned text with no TH/TD semantics or scope",
      "exceed_strategy": "Model tables as real table objects with header rows/cols as data (auto-emit Scope, and auto-generate Headers/ID wiring for merged/spanning cells) so complex-table remediation that is manual in Acrobat is automatic and validated in-app",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-05"
    },
    {
      "feature": "Natural-language declaration (/Lang, BCP 47)",
      "app_behavior": "A document-level default /Lang (BCP 47 tag) is required, plus span/structure-level Lang on any content whose language differs, so AT switches pronunciation/voice correctly; also governs metadata and outline language.",
      "primitive_domain": "language",
      "source_url": "https://pdfa.org/glossary-of-accessibility-terminology-in-pdf/",
      "app_or_standard": "PDF/UA-1 (ISO 14289-1)",
      "closes_gap": "Exporters omit document language and never tag inline language changes",
      "exceed_strategy": "Attach language at the character-style/paragraph-style level (with CJK and RTL/Arabic-Hebrew in scope) so mixed-language and bidi runs emit span-level Lang automatically to PDF, EPUB and HTML; lint any run with no language resolved",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-06"
    },
    {
      "feature": "Document title & DisplayDocTitle",
      "app_behavior": "PDF/UA requires a human document title in XMP metadata and the ViewerPreferences /DisplayDocTitle flag set true so AT and the title bar show the title rather than the filename.",
      "primitive_domain": "metadata",
      "source_url": "https://www.pdflib.com/pdf-knowledge-base/pdfua/requirements/",
      "app_or_standard": "PDF/UA-1 (ISO 14289-1)",
      "closes_gap": "Generic exporters ship filename-only titles and leave DisplayDocTitle false",
      "exceed_strategy": "Populate title/metadata from the project's document properties and set DisplayDocTitle by default in every PDF/UA export preset, blocking export if title is empty",
      "verification": "UNVERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-07"
    },
    {
      "feature": "Heading hierarchy without skipped levels",
      "app_behavior": "Headings tag as H1-H6 (or nested H) forming a correct, gap-free outline that expresses document hierarchy; paragraph styles map to heading tags via export-tagging so AT can build a navigable heading tree.",
      "primitive_domain": "document-model-structure",
      "source_url": "https://www.pubcom.com/blog/2020_05-02_tags/pdf-ua-tags.shtml",
      "app_or_standard": "PDF/UA-1 (ISO 14289-1) / InDesign Export Tagging",
      "closes_gap": "Visually-styled 'headings' with no tag mapping produce a flat, unnavigable outline",
      "exceed_strategy": "Bind heading level to the paragraph style once (InDesign-style Export Tagging) and run a live outline lint that flags skipped levels (e.g. H2 to H4) at design time rather than at post-export validation",
      "verification": "UNVERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-08"
    },
    {
      "feature": "List semantics (L/LI/Lbl/LBody)",
      "app_behavior": "Ordered/unordered lists tag as L containing LI, each with optional Lbl (bullet/number marker) and LBody; proper nesting is required so AT announces list membership, position and count.",
      "primitive_domain": "document-model-structure",
      "source_url": "https://pdfa.org/wp-content/uploads/2015/12/StructureElementsBestPracticeGuide_2016-01-19.pdf",
      "app_or_standard": "PDF/UA-1 (ISO 14289-1)",
      "closes_gap": "Manually-formatted 'lists' (typed bullets) carry no list semantics",
      "exceed_strategy": "Emit L/LI/Lbl/LBody automatically from real list objects (InDesign auto-recognizes lists on export; extend to nested and mixed lists) and mark the bullet/number glyph as Lbl so it is not read as body text",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-09"
    },
    {
      "feature": "PDF/UA-2 on PDF 2.0",
      "app_behavior": "ISO 14289-2:2024 (released 2024-03-15, built on PDF 2.0 / ISO 32000-2, derived from the PDF Association WTPDF spec) adds comprehensive structure-element attribute requirements, comprehensive annotation requirements, native math via MathML, intra-document links using the structure-destinations feature, and the new PDF 2.0 structure element types.",
      "primitive_domain": "math",
      "source_url": "https://pdfa.org/iso-14289-2-pdfua-2/",
      "app_or_standard": "PDF/UA-2 (ISO 14289-2:2024)",
      "closes_gap": "No incumbent design tool targets PDF/UA-2 / PDF 2.0 tagged output or embeds MathML",
      "exceed_strategy": "Target PDF/UA-2 as the default modern preset (MathML-backed equations, structure-destination links) so Studio ships the 'gold standard' PDF 2.0 accessibility level that Acrobat-era workflows do not yet author natively",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-10"
    },
    {
      "feature": "Native math (MathML) across PDF & EPUB",
      "app_behavior": "PDF/UA-2 and EPUB Accessibility 1.1 both require/support MathML for equations so math is machine-readable, navigable and can be spoken or brailled, rather than rendered as an inaccessible image.",
      "primitive_domain": "math",
      "source_url": "https://www.quadient.com/en/blog/pdf-ua-2",
      "app_or_standard": "PDF/UA-2 + EPUB Accessibility 1.1",
      "closes_gap": "Illustrator/Figma/most DTP export equations as flattened images with no math semantics",
      "exceed_strategy": "Store equations as MathML in the document model and emit the same MathML to both PDF/UA-2 and EPUB3 exports, giving a single accessible math source across print and reflowable output",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-11"
    },
    {
      "feature": "Matterhorn Protocol machine-checkable failure conditions",
      "app_behavior": "The Matterhorn Protocol restates PDF/UA as 136 failure conditions grouped in 31 checkpoints; 89 are machine-checkable and the remainder require human judgment — this is the concrete checklist that governs automated PDF/UA validation.",
      "primitive_domain": "validation-engine",
      "source_url": "https://www.pdflib.com/pdf-knowledge-base/pdfua/matterhorn-protocol/",
      "app_or_standard": "Matterhorn Protocol (PDF Association)",
      "closes_gap": "Incumbents offer no in-tool conformance checklist; validation happens externally after export",
      "exceed_strategy": "Ship the 89 machine-checkable conditions as a live in-app linter that runs continuously during design (not only at export), plus a guided human-check queue for the ~47 judgment items, so authors reach conformance before export",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-12"
    },
    {
      "feature": "veraPDF-class open-source validation engine",
      "app_behavior": "veraPDF is the open-source industry validator that formalizes each 'shall' statement of PDF/A-1..4, PDF/UA-1, PDF/UA-2 and WTPDF 1.0 as XML validation profiles applied at runtime; for PDF/UA it runs the machine-verifiable Matterhorn conditions, matching PAC and axesPDF coverage.",
      "primitive_domain": "validation-engine",
      "source_url": "https://docs.verapdf.org/validation/",
      "app_or_standard": "veraPDF (Open Preservation Foundation)",
      "closes_gap": "No native design app embeds a PDF/UA validator; users must round-trip through PAC/Acrobat/axesPDF",
      "exceed_strategy": "Embed a veraPDF-class engine locally and offline inside Studio for live, in-app PDF/UA-1 and PDF/UA-2 validation with one-click jump-to-offending-object remediation — turning a separate external checker into an always-on design surface",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-13"
    },
    {
      "feature": "WCAG 2.2 AA baseline for exported web/EPUB",
      "app_behavior": "WCAG 2.2 (Oct 2023) is the conformance baseline; new criteria include 2.4.11 Focus Not Obscured Minimum (AA), 2.5.8 Target Size Minimum 24x24 CSS px (AA), 2.5.7 Dragging Movements (AA), 3.3.8 Accessible Authentication Minimum (AA, no cognitive-function test, allow paste/password managers), 3.2.6 Consistent Help (A), 3.3.7 Redundant Entry (A), with AAA enhancements 2.4.12 and 3.3.9.",
      "primitive_domain": "semantic-html",
      "source_url": "https://dequeuniversity.com/resources/wcag-2.2/",
      "app_or_standard": "WCAG 2.2 (W3C)",
      "closes_gap": "Design exporters ignore interaction-level criteria (target size, focus visibility) for generated web output",
      "exceed_strategy": "Apply WCAG 2.2 AA (and offer AAA) as design-time defaults for interactive/web export — enforce 24x24px minimum hit targets, focus-not-obscured, and contrast at layout time so generated HTML is born-conformant rather than remediated",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-14"
    },
    {
      "feature": "WCAG contrast at design time",
      "app_behavior": "WCAG 2.x AA requires 4.5:1 text contrast (3:1 for large text) and 3:1 for non-text/UI components and graphical objects; these are the color-choice gates every accessible visual output must pass.",
      "primitive_domain": "design-time-linting",
      "source_url": "https://getwcag.com/en/wcag-2-2-guidelines",
      "app_or_standard": "WCAG 2.2 (W3C)",
      "closes_gap": "Color pickers in design tools give no contrast verdict against WCAG thresholds",
      "exceed_strategy": "Compute WCAG contrast ratios live in the swatch/color picker and flag failing text/UI pairs on canvas as they are chosen, with a one-click 'nearest passing color' suggestion — accessibility linting during design, not after",
      "verification": "UNVERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-15"
    },
    {
      "feature": "EN 301 549 non-web documents chapter (EAA)",
      "app_behavior": "EN 301 549 (current harmonised v3.2.1, 2021) maps to WCAG 2.1 AA and structures requirements as Ch.9 Web, Ch.10 Non-web documents (covers PDF and other electronic documents), Ch.11 Non-web software; the European Accessibility Act enforcement began 2025-06-28 with penalties/market removal for non-compliance; v4.1.1 (expected 2026) adopts WCAG 2.2.",
      "primitive_domain": "conformance-mapping",
      "source_url": "https://www.deque.com/en-301-549-compliance/",
      "app_or_standard": "EN 301 549 / European Accessibility Act",
      "closes_gap": "Design tools give no EN 301 549 / EAA conformance mapping for exported documents",
      "exceed_strategy": "Ship an EN 301 549 Ch.10 document-conformance report generated locally from the same validation engine, mapping each PDF/UA and WCAG check to its EN clause, so an EU-market export carries an offline conformance record without cloud services",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-16"
    },
    {
      "feature": "EPUB Accessibility 1.1 discovery metadata",
      "app_behavior": "Every EPUB MUST expose schema.org accessibility metadata in the package document regardless of conformance: accessMode, accessModeSufficient, accessibilityFeature (e.g. alternativeText, longDescription, structuralNavigation, MathML), accessibilityHazard, accessibilitySummary; plus dcterms:conformsTo pointing at the WCAG level met (or 'none'); minimum bar is WCAG 2.0 Level A.",
      "primitive_domain": "epub-metadata",
      "source_url": "https://www.w3.org/TR/epub-a11y-11/",
      "app_or_standard": "EPUB Accessibility 1.1 (W3C)",
      "closes_gap": "Design tools that export EPUB rarely write complete schema.org a11y metadata",
      "exceed_strategy": "Auto-derive and write accurate accessibility metadata from what the document actually contains (detected alt text, MathML, structural nav, page-list) rather than leaving authors to hand-write it, keeping conformsTo honest and verifiable",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-17"
    },
    {
      "feature": "EPUB3 born-accessible structure (nav, page-list, media overlays)",
      "app_behavior": "EPUB3 accessible structure requires a nav document with toc/landmarks, a page-list mapped to print page breaks (page markers), structural navigation via real HTML sectioning/headings, optional SMIL media overlays for synchronized text-audio, and MathML for math.",
      "primitive_domain": "epub-packaging",
      "source_url": "https://kb.daisy.org/publishing/docs/conformance/epub.html",
      "app_or_standard": "EPUB Accessibility 1.1 / EPUB 3.3",
      "closes_gap": "Illustrator has no EPUB path; Figma has none; generic EPUB exporters omit page-list/landmarks/media overlays",
      "exceed_strategy": "Generate nav, landmarks, and print-equivalent page-list automatically from the same structured document model used for print, so a single Studio source yields both a PDF/UA file and a born-accessible reflowable EPUB3 without re-authoring",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-18"
    },
    {
      "feature": "Ace by DAISY EPUB validation coverage",
      "app_behavior": "Ace by DAISY (free, open-source) is the reference EPUB accessibility checker: it verifies presence/validity/coherence of accessibility metadata, checks MathML usage (even commented-out), evaluates SMIL media-overlay structure and metadata, and checks page-marker rules — while noting a clean report is not equal to full WCAG conformance.",
      "primitive_domain": "validation-engine",
      "source_url": "https://daisy.github.io/ace/",
      "app_or_standard": "Ace by DAISY (DAISY Consortium)",
      "closes_gap": "No native design tool embeds EPUB accessibility validation; authors run Ace externally after export",
      "exceed_strategy": "Embed an Ace-class EPUB checker locally in Studio so metadata/MathML/media-overlay/page-marker validation runs in-app and offline, and expose the human-judgment gaps Ace cannot machine-check as a guided review queue",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-19"
    },
    {
      "feature": "Semantic HTML / WAI-ARIA web output",
      "app_behavior": "Accessible web output prioritizes native HTML5 sectioning (header->banner, nav->navigation, main->main, aside->complementary, footer->contentinfo, plus search/form/region) and uses WAI-ARIA roles/states/properties only as fallback (first rule of ARIA: use native HTML when it suffices); interactive widgets follow ARIA Authoring Practices Guide keyboard+role patterns.",
      "primitive_domain": "semantic-html",
      "source_url": "https://www.w3.org/WAI/ARIA/apg/practices/landmark-regions/",
      "app_or_standard": "WAI-ARIA 1.x / ARIA APG (W3C)",
      "closes_gap": "Figma/DTP web exports emit div-soup with no landmarks, headings, or ARIA widget semantics",
      "exceed_strategy": "Emit native HTML5 landmarks and heading structure from the document model by default (ARIA only where HTML is insufficient), so generated web output is screen-reader navigable by landmark and heading out of the box instead of flat positioned divs",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-20"
    },
    {
      "feature": "Incumbent gap: Adobe Illustrator cannot export tagged PDF",
      "app_behavior": "Adobe Illustrator provides no facility to export a tagged, accessible PDF (open feature request since 2017); Illustrator-produced PDFs must be manually tagged afterward in Acrobat, and Illustrator has effectively zero built-in accessibility tooling unlike InDesign.",
      "primitive_domain": "export-pipeline",
      "source_url": "https://illustrator.uservoice.com/forums/333657-illustrator-desktop-feature-requests/suggestions/31494562-make-illustrator-pdfs-more-automatically-accessibl",
      "app_or_standard": "Adobe Illustrator (incumbent failure)",
      "closes_gap": "Vector/graphic-design work in Illustrator produces inaccessible PDFs with no remediation path in-app",
      "exceed_strategy": "Make Studio's vector/graphic design surface itself accessibility-aware (roles, alt text, artifact marking on vector art and infographics) so even graphic-led documents export tagged PDF/UA directly — closing the exact gap Illustrator has left open for years",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-21"
    },
    {
      "feature": "Incumbent gap: Figma flattens text to vectors on PDF export",
      "app_behavior": "Figma's native PDF export converts text layers to vector outlines, producing PDFs whose text is not selectable, searchable, or readable by screen readers and cannot be reliably tagged afterward; Figma has no born-accessible output path beyond limited Sites tooling.",
      "primitive_domain": "export-pipeline",
      "source_url": "https://jen.dev/blog/pdf-exports/",
      "app_or_standard": "Figma (incumbent failure)",
      "closes_gap": "UI/design work in Figma cannot become an accessible document without full rebuild in another tool",
      "exceed_strategy": "Preserve live text (with embedded fonts, ActualText, and tags) through export by design — never outline text — so Studio output is screen-reader-readable where Figma's is inert, giving accessible PDF/HTML from the same design surface",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-22"
    },
    {
      "feature": "InDesign accessibility toolset (BAR to match and exceed)",
      "app_behavior": "InDesign is the incumbent bar: Articles panel with 'Use for Tagging Order in Tagged PDF' for reading order, Object Export Options > Alt Text (Custom or from XMP/structure) for images, Export Tagging to map paragraph styles to PDF/heading tags, Table header-row recognition, auto list recognition, and 'Create Tagged PDF' on Adobe PDF export.",
      "primitive_domain": "export-pipeline",
      "source_url": "https://helpx.adobe.com/indesign/desktop/interactive-elements-and-forms/forms-and-pdfs/use-tags-for-accessible-pdfs.html",
      "app_or_standard": "Adobe InDesign (incumbent bar)",
      "closes_gap": "InDesign requires manual Articles-panel curation and external PAC/Acrobat validation; no in-app conformance verdict",
      "exceed_strategy": "Match every InDesign mechanic natively but add what InDesign lacks: structure that survives re-export in the document model, live embedded PDF/UA + EPUB validation, one-click remediation, and born-accessible defaults — so authors reach conformance inside Studio without the Acrobat round-trip",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-accessibility-exceed-23"
    }
  ]
}
```

### [SFR-CLOSE.print-prepress-depth] Print / prepress dialog option depth (closes XAPP-03) (30 rows)

```json
{
  "rows": [
    {
      "feature": "Print color handling mode",
      "app_or_standard": "Photoshop",
      "app_behavior": "Print Settings > Color Management > Color Handling dropdown: exact modes 'Photoshop Manages Colors', 'Printer Manages Colors', 'Separations' (for CMYK/Duotone docs), and 'No Color Management'. Manages-by-Photoshop requires disabling printer-driver color management to avoid double conversion.",
      "primitive_domain": "color-management",
      "source_url": "https://helpx.adobe.com/photoshop/using/printing-color-management-photoshop1.html",
      "closes_gap": "XAPP-03 app-manages vs printer-manages parity",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-01"
    },
    {
      "feature": "Rendering intent + black point compensation",
      "app_or_standard": "Photoshop",
      "app_behavior": "Print dialog Rendering Intent dropdown: 'Perceptual', 'Saturation', 'Relative Colorimetric', 'Absolute Colorimetric'; plus 'Black Point Compensation' checkbox that scales source black to destination black to preserve shadow gradation.",
      "primitive_domain": "color-management",
      "source_url": "https://helpx.adobe.com/photoshop/using/printing-color-management-photoshop1.html",
      "closes_gap": "per-app rendering-intent parity",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-02"
    },
    {
      "feature": "Hard proof / simulate paper+ink at print time",
      "app_or_standard": "Photoshop",
      "app_behavior": "Print dialog 'Hard Proofing' with a 'Proofing Profile' selector, plus 'Simulate Paper Color' (restricts tonal range to paper white) and 'Simulate Black Ink' (dulls black point to press black) checkboxes; selecting Simulate Paper Color auto-enables Simulate Black Ink. Mirrors View > Proof Setup > Custom 'Device to Simulate'.",
      "primitive_domain": "proofing",
      "source_url": "https://helpx.adobe.com/photoshop/using/proofing-colors.html",
      "closes_gap": "hard proof / simulate paper+ink",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-03"
    },
    {
      "feature": "Printer/ICC profile selection",
      "app_or_standard": "Photoshop",
      "app_behavior": "'Printer Profile' dropdown lists installed output/paper ICC profiles; chosen profile must match the paper/printer combination and match the soft-proof profile for WYSIWYG.",
      "primitive_domain": "color-management",
      "source_url": "https://helpx.adobe.com/photoshop/using/printing-color-management-photoshop1.html",
      "closes_gap": "printer/ICC profile selection",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-04"
    },
    {
      "feature": "Host-based vs In-RIP separations mode",
      "app_or_standard": "Illustrator",
      "app_behavior": "Print > Output panel 'Mode' dropdown: 'Composite', 'Separations (Host-Based)' (Illustrator builds one PostScript stream per plate), 'In-RIP Separations' (composite PostScript sent, RIP performs separation/trap/CM).",
      "primitive_domain": "separations",
      "source_url": "https://helpx.adobe.com/illustrator/using/printing-color-separations.html",
      "closes_gap": "host-based vs in-RIP separations",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-05"
    },
    {
      "feature": "Emulsion + image exposure controls",
      "app_or_standard": "Illustrator",
      "app_behavior": "Output panel 'Emulsion' (Up (Right Reading) / Down (Right Reading)) and 'Image' (Positive / Negative) for film/plate output; plus 'Printer Resolution' lpi/dpi pairs read from the PPD.",
      "primitive_domain": "screening-halftone",
      "source_url": "https://helpx.adobe.com/illustrator/using/printing-color-separations.html",
      "closes_gap": "emulsion/negative + printer resolution",
      "verification": "UNVERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-06"
    },
    {
      "feature": "Per-ink halftone screen frequency/angle/dot",
      "app_or_standard": "Illustrator",
      "app_behavior": "Output panel 'Document Ink Options' list: double-click an ink name to edit that plate's screen Frequency (lpi), Angle (degrees), and halftone Dot Shape; click the printer icon to disable a plate; click the process icon to convert an individual spot to CMYK.",
      "primitive_domain": "screening-halftone",
      "source_url": "https://helpx.adobe.com/illustrator/using/printing-color-separations.html",
      "closes_gap": "screening/halftone frequency/angle/dot",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-07"
    },
    {
      "feature": "Spot-to-process + overprint black at output",
      "app_or_standard": "Illustrator",
      "app_behavior": "Output panel checkboxes 'Convert All Spot Colors to Process' (folds spot plates into CMYK) and 'Overprint Black' (forces 100% K to overprint rather than knock out).",
      "primitive_domain": "separations",
      "source_url": "https://helpx.adobe.com/illustrator/using/printing-color-separations.html",
      "closes_gap": "overprint + separations control",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-08"
    },
    {
      "feature": "Ink Manager ink aliasing + density + type",
      "app_or_standard": "InDesign",
      "app_behavior": "Output > Ink Manager: per-ink 'Neutral Density' (drives trap conservatism), 'Ink Aliasing' (map a spot to another spot's plate / '<No Alias>'), 'All Spots to Process' checkbox (removes aliases, converts spots to CMYK), 'Ink Sequence' / trapping order, ink 'Type' (Normal / Transparent / Opaque / OpaqueIgnore), and 'Use Standard Lab Values for Spots'.",
      "primitive_domain": "ink-management",
      "source_url": "https://helpx.adobe.com/indesign/using/inks-separations-screen-frequency.html",
      "closes_gap": "ink manager / ink aliasing",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-09"
    },
    {
      "feature": "Composite vs separation output color modes",
      "app_or_standard": "InDesign",
      "app_behavior": "Print > Output 'Color' dropdown exact values: 'Composite Leave Unchanged', 'Composite Gray', 'Composite RGB', 'Composite CMYK', 'Separations' (on-host), 'In-RIP Separations'.",
      "primitive_domain": "separations",
      "source_url": "https://helpx.adobe.com/indesign/using/inks-separations-screen-frequency.html",
      "closes_gap": "host-based vs in-RIP separations",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-10"
    },
    {
      "feature": "Trapping engine selection",
      "app_or_standard": "InDesign",
      "app_behavior": "Output panel 'Trapping' dropdown: 'Off', 'Application Built-In' (InDesign's raster trapping engine), 'Adobe In-RIP' (only valid targeting a device that supports Adobe In-RIP Trapping).",
      "primitive_domain": "trapping",
      "source_url": "https://helpx.adobe.com/indesign/using/inks-separations-screen-frequency.html",
      "closes_gap": "trapping",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-11"
    },
    {
      "feature": "Flip (emulsion) + Negative output",
      "app_or_standard": "InDesign",
      "app_behavior": "Output panel 'Flip' dropdown (None / Horizontal / Vertical / Horizontal & Vertical) and 'Negative' checkbox for imagesetter film/plate output.",
      "primitive_domain": "screening-halftone",
      "source_url": "https://helpx.adobe.com/indesign/using/inks-separations-screen-frequency.html",
      "closes_gap": "emulsion/negative",
      "verification": "UNVERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-12"
    },
    {
      "feature": "Screening (frequency/angle) from PPD",
      "app_or_standard": "InDesign",
      "app_behavior": "Output panel 'Screening' lists lpi/dpi halftone pairs available from the selected PPD; the Inks list shows per-plate Frequency and Angle (editable only for PostScript devices / custom screening).",
      "primitive_domain": "screening-halftone",
      "source_url": "https://helpx.adobe.com/indesign/using/inks-separations-screen-frequency.html",
      "closes_gap": "screening/halftone frequency/angle",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-13"
    },
    {
      "feature": "Trap preset width + black width",
      "app_or_standard": "InDesign",
      "app_behavior": "Trap Presets Options: 'Default' trap width (default 0p0.25) for all non-black traps and 'Black' trap width (default 0p0.5, typically 1.5-2x default) = inkspread/holdback into solid black.",
      "primitive_domain": "trapping",
      "source_url": "https://helpx.adobe.com/indesign/using/trap-presets.html",
      "closes_gap": "trapping depth",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-14"
    },
    {
      "feature": "Trap image placement + image trapping toggles",
      "app_or_standard": "InDesign",
      "app_behavior": "Trap preset 'Images' section: 'Trap Placement' = Center / Choke / Neutral Density / Spread where vector abuts bitmap; plus 'Trap Objects to Images', 'Trap Images to Images', 'Trap Images Internally', 'Trap 1-bit Images' checkboxes.",
      "primitive_domain": "trapping",
      "source_url": "https://helpx.adobe.com/indesign/using/trap-presets.html",
      "closes_gap": "trapping depth",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-15"
    },
    {
      "feature": "Trap thresholds: sliding trap, black density, color reduction",
      "app_or_standard": "InDesign",
      "app_behavior": "Trap preset 'Thresholds': 'Step' %, 'Black Color' %, 'Black Density' (neutral density at/above which ink counts as black, default ~1.6), 'Sliding Trap' % (default 70; 100 disables), 'Trap Color Reduction' % (0 makes trap ND equal to darker color).",
      "primitive_domain": "trapping",
      "source_url": "https://helpx.adobe.com/indesign/using/trap-presets.html",
      "closes_gap": "trapping depth",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-16"
    },
    {
      "feature": "Printer marks set",
      "app_or_standard": "InDesign",
      "app_behavior": "Marks and Bleed panel: 'Crop Marks', 'Bleed Marks', 'Registration Marks', 'Color Bars', 'Page Information' (filename/page/date-time/separation name in 6pt Helvetica, lower-left); plus 'Type' (Default/custom e.g. Japanese), 'Weight' (line weight of crop/bleed marks), 'Offset' (distance from page edge, not bleed).",
      "primitive_domain": "printer-marks",
      "source_url": "https://helpx.adobe.com/indesign/using/printers-marks-bleeds.html",
      "closes_gap": "printer marks (crop/registration/bleed/color bars/page info)",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-17"
    },
    {
      "feature": "Bleed + slug controls",
      "app_or_standard": "InDesign",
      "app_behavior": "Marks and Bleed 'Bleed and Slug' section: 'Use Document Bleed Settings', independent Top/Bottom/Inside(Left)/Outside(Right) bleed values with link toggle, and 'Include Slug Area'.",
      "primitive_domain": "printer-marks",
      "source_url": "https://helpx.adobe.com/indesign/using/printers-marks-bleeds.html",
      "closes_gap": "bleed/slug output",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-18"
    },
    {
      "feature": "Transparency flattener preset controls",
      "app_or_standard": "InDesign / Illustrator",
      "app_behavior": "Flattener Preset options: 'Raster/Vector Balance' slider (0-100, higher keeps more vector), 'Line Art and Text Resolution' (up to 9600 ppi), 'Gradient and Mesh Resolution' (up to 1200 ppi), 'Convert All Text to Outlines', 'Convert All Strokes to Outlines', 'Clip Complex Regions'. Built-ins: Low / Medium / High Resolution.",
      "primitive_domain": "transparency-flattening",
      "source_url": "https://helpx.adobe.com/indesign/desktop/apply-color/advanced-color-techniques/transparency-flattener-preset-options.html",
      "closes_gap": "transparency flattener presets",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-19"
    },
    {
      "feature": "Illustrator flatten transparent artwork presets",
      "app_or_standard": "Illustrator",
      "app_behavior": "Print > Advanced and Document Setup use same flattener preset model (raster/vector balance, LineArt+Text resolution, Gradient+Mesh resolution, convert text/strokes to outlines, clip complex regions); '[High Resolution]' preset preserves most vector for offset output.",
      "primitive_domain": "transparency-flattening",
      "source_url": "https://helpx.adobe.com/illustrator/using/printing-saving-transparent-artwork.html",
      "closes_gap": "transparency flattener presets",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-20"
    },
    {
      "feature": "PDF/X output intent standards",
      "app_or_standard": "PDF/X (ISO 15930) via InDesign",
      "app_behavior": "Export Adobe PDF 'Standard': 'PDF/X-1a:2001/2003' (CMYK+spot, no color mgmt, transparency flattened, Acrobat 4), 'PDF/X-3:2002/2003' (adds embedded RGB profiles/CM), 'PDF/X-4:2010' (live transparency + CM, Adobe PDF Print Engine). Output Intent set via Output panel 'Destination'/'Output Intent Profile Name'.",
      "primitive_domain": "output-intent-pdfx",
      "source_url": "https://helpx.adobe.com/indesign/using/pdf-options.html",
      "closes_gap": "PDF/X output intents",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-21"
    },
    {
      "feature": "PDF export color conversion + output intent profile",
      "app_or_standard": "InDesign",
      "app_behavior": "Export PDF Output panel: 'Color Conversion' = No Color Conversion / Convert to Destination / Convert to Destination (Preserve Numbers); 'Profile Inclusion Policy'; 'Destination'; 'Output Intent Profile Name'; 'Simulate Overprint' (for X-1a); 'Ink Manager' button reachable at export time.",
      "primitive_domain": "output-intent-pdfx",
      "source_url": "https://helpx.adobe.com/indesign/using/pdf-options.html",
      "closes_gap": "PDF/X output intents + color conversion",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-22"
    },
    {
      "feature": "Separations preview + ink coverage warning",
      "app_or_standard": "InDesign",
      "app_behavior": "Window > Output > Separations Preview panel: 'View' = Separations (isolate/toggle each CMYK+spot plate) or 'Ink Limit' (highlights areas exceeding a settable total area coverage %, e.g. 300%).",
      "primitive_domain": "separations",
      "source_url": "https://helpx.adobe.com/indesign/desktop/print/color-output-and-separations/preview-color-separations-and-ink-coverage.html",
      "closes_gap": "overprint + separations preview",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-23"
    },
    {
      "feature": "Overprint preview on-screen",
      "app_or_standard": "InDesign / Illustrator / Acrobat",
      "app_behavior": "View > Overprint Preview simulates on screen how overprinting/knockout inks, ink aliasing, and spot overprints will actually composite on press (approximation of blended overprints).",
      "primitive_domain": "separations",
      "source_url": "https://helpx.adobe.com/indesign/desktop/print/color-output-and-separations/about-overprinting.html",
      "closes_gap": "overprint + separations preview",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-24"
    },
    {
      "feature": "PostScript printing graphics/data options",
      "app_or_standard": "Illustrator / InDesign",
      "app_behavior": "Print Graphics panel: 'Send Data' (All / Optimized Subsampling), 'Font Downloading' (None / Complete / Subset), 'PostScript' (Level 2 / Level 3), 'Data Format' (ASCII / Binary); path 'Flatness' via output settings for complex-path RIP tolerance.",
      "primitive_domain": "postscript-output",
      "source_url": "https://helpx.adobe.com/illustrator/using/postscript-printing.html",
      "closes_gap": "flatness/PostScript options",
      "verification": "UNVERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-25"
    },
    {
      "feature": "Print colour-handling toggle",
      "app_or_standard": "Affinity Photo/Designer/Publisher",
      "app_behavior": "Print dialog 'Colour Management' section: colour handling 'Performed by the App' vs 'Performed by the Printer', a 'Printer Profile' ICC selector (match soft-proof profile), and a 'Rendering Intent' selector (Perceptual / Relative Colorimetric / Saturation / Absolute Colorimetric).",
      "primitive_domain": "color-management",
      "source_url": "https://affinity.help/publisher2/English.lproj/pages/Clr/ClrProfiles.html",
      "closes_gap": "app-manages vs printer-manages parity (Affinity)",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-26"
    },
    {
      "feature": "Soft proof adjustment layer",
      "app_or_standard": "Affinity Photo/Publisher",
      "app_behavior": "'Soft Proof' adjustment: 'Proof Profile' (paper/output ICC), 'Rendering Intent', 'Black Point Compensation', and 'Gamut Check' overlay; supports multiple stacked soft-proof layers for different output devices.",
      "primitive_domain": "proofing",
      "source_url": "https://affinity.help/publisher2/English.lproj/pages/Clr/ClrProfiles.html",
      "closes_gap": "hard proof / simulate paper+ink (Affinity)",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-27"
    },
    {
      "feature": "PDF/X export presets + CMYK/overprint",
      "app_or_standard": "Affinity Publisher",
      "app_behavior": "Export PDF presets 'PDF/X-1a:2003' (PDF 1.4, flatten transparency, no colour mgmt, all colour to CMYK), 'PDF/X-3:2003' (spot colours + colour management, RGB allowed), 'PDF/X-4' (PDF 1.6, live transparency); export embeds document ICC as colour space and can force overprint.",
      "primitive_domain": "output-intent-pdfx",
      "source_url": "https://www.affinity.studio/help/sharing-pdf-presets/",
      "closes_gap": "PDF/X output intents (Affinity)",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-28"
    },
    {
      "feature": "Affinity print bleed + printer marks + spot handling",
      "app_or_standard": "Affinity Publisher",
      "app_behavior": "Export/Print supports bleed, printer marks (crop, registration, colour bars, page info), embedded fonts, >300dpi image threshold, and preservation of spot colours under PDF/X-3/X-4 (spots kept as separate plates rather than forced to CMYK).",
      "primitive_domain": "printer-marks",
      "source_url": "https://www.affinity.studio/help/sharing-publish-pdffiles/",
      "closes_gap": "printer marks + spot colour separations (Affinity)",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-29"
    },
    {
      "feature": "Spot color separate-as-process + Lab display",
      "app_or_standard": "InDesign",
      "app_behavior": "Ink Manager per-spot toggle to output a spot as process (CMYK build) without altering the swatch, and 'Use Standard Lab Values for Spots' to display/output named spots via Lab (Pantone) rather than the stored CMYK approximation.",
      "primitive_domain": "ink-management",
      "source_url": "https://helpx.adobe.com/indesign/using/inks-separations-screen-frequency.html",
      "closes_gap": "ink manager / spot handling",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-print-prepress-depth-30"
    }
  ]
}
```

### [SFR-CLOSE.automation-model-depth] Automation/scripting capability model depth (closes XAPP-02 at capability level) (26 rows)

```json
{
  "rows": [
    {
      "feature": "Command-descriptor execution bus",
      "app_behavior": "action.batchPlay(descriptors[], options) executes one-or-more Photoshop commands, async by default returning Promise<result[]>; each descriptor is actionJSON with _obj (command id, e.g. 'make','hide','get') and _target (element ref). Options: synchronousExecution, continueOnError, immediateRedraw.",
      "primitive_domain": "command-descriptor-bus",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/ps_reference/media/batchplay/",
      "closes_gap": "XAPP-02: Studio must expose a single low-level descriptor bus so any native command is scriptable by JS/TS without per-feature bindings",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-01"
    },
    {
      "feature": "Typed action references (element addressing)",
      "app_behavior": "_target uses _ref forms in reverse order (most→least specific): by ID {_ref,_id}, Index {_ref,_index} (1-based), Name {_ref,_name}, Enumeration {_ref,_enum:'ordinal',_value} with targetEnum/first/last/front. Studio Rust-ABI must expose the same 4 addressing modes.",
      "primitive_domain": "command-descriptor-bus",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/ps_reference/media/batchplay/",
      "closes_gap": "XAPP-02: deterministic object addressing for scripts across ID/index/name/ordinal",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-02"
    },
    {
      "feature": "Per-command dialog + progress control",
      "app_behavior": "Descriptor _options.dialogOptions = 'silent' (no UI, errors throw) | 'dontDisplay' (UI only on error) | 'display' (show command UI); _options.suppressProgressBar suppresses the native progress bar. Enables headless batch without modal popups.",
      "primitive_domain": "command-descriptor-bus",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/ps_reference/media/batchplay/",
      "closes_gap": "XAPP-02: scripts must run fully silent for batch, per GLOBAL-BUILD-QUIET non-intrusive operation",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-03"
    },
    {
      "feature": "State read via get / multiGet",
      "app_behavior": "Read state with {_obj:'get',_target:[{_property:name}...]}; multiGet batches many properties in one call via extendedReference [[propNames],{_obj,index,count}] with options.failOnMissingProperty. One round-trip bulk read of layer/document props.",
      "primitive_domain": "command-descriptor-bus",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/ps_reference/media/batchplay/",
      "closes_gap": "XAPP-02: efficient bulk state introspection for scripts; avoids N calls per property",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-04"
    },
    {
      "feature": "Modal execution scope for mutations",
      "app_behavior": "core.executeAsModal(callback, {commandName, interactive}) is mandatory (apiVersion 2) before any state-changing command; guarantees exclusive control vs other plugins; callback receives executionContext. Studio needs an equivalent modal/transaction lock primitive.",
      "primitive_domain": "scripting-runtime",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/ps_reference/media/executeasmodal",
      "closes_gap": "XAPP-02: concurrency-safe mutation scope for parallel model/agent scripts (GLOBAL-BUILD-PARALLEL)",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-05"
    },
    {
      "feature": "History coalescing / undo grouping",
      "app_behavior": "executionContext.hostControl.suspendHistory({documentID, name}) / resumeHistory({...,commit}) collapses all edits between calls into one named, undoable history state; state only created if document actually changed. Studio must expose named atomic undo grouping to scripts.",
      "primitive_domain": "scripting-runtime",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/ps_reference/media/executeasmodal",
      "closes_gap": "XAPP-02: script-authored edits appear as a single clean undo step, not dozens",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-06"
    },
    {
      "feature": "Direct pixel get/put imaging API",
      "app_behavior": "imaging.getPixels / putPixels with options documentID, layerID, historyStateID, sourceBounds, targetSize (scaled read), colorSpace, colorProfile, componentSize, applyAlpha, replace, targetBounds, commandName. Direct raster read/write outside command bus.",
      "primitive_domain": "pixel-imaging",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/ps_reference/media/imaging/",
      "closes_gap": "XAPP-02: native pixel-buffer I/O so scripts manipulate raster data directly",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-07"
    },
    {
      "feature": "Mask and selection pixel I/O",
      "app_behavior": "getLayerMask/putLayerMask with kind:'user'|'vector'; getSelection/putSelection returns/writes active selection as pixel data (sourceBounds, targetSize, replace, targetBounds). Studio must let scripts read/write masks and marquee selections as buffers.",
      "primitive_domain": "pixel-imaging",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/ps_reference/media/imaging/",
      "closes_gap": "XAPP-02: programmatic mask/selection editing without menu commands",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-08"
    },
    {
      "feature": "Image buffer construct + encode",
      "app_behavior": "createImageDataFromBuffer({width,height,components,chunky,colorProfile,colorSpace,fullRange}) builds PhotoshopImageData from an ArrayBuffer; encodeImageData({imageData,base64}) serializes for UXP UI/export. Round-trips external buffers into the document.",
      "primitive_domain": "pixel-imaging",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/ps_reference/media/imaging/",
      "closes_gap": "XAPP-02: interop between raw memory buffers and document pixels for generative/procedural scripts",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-09"
    },
    {
      "feature": "Pixel data format contract",
      "app_behavior": "PhotoshopImageData exposes colorSpace RGB/Grayscale/Lab, componentSize 8/16/32-bit, hasAlpha, components, chunky (interleaved per-pixel) vs planar (grouped) memory layout, pixelFormat. Studio ABI must document identical format/layout enums for cross-language buffers.",
      "primitive_domain": "pixel-imaging",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/ps_reference/media/imaging/",
      "closes_gap": "XAPP-02: unambiguous pixel buffer contract for Rust<->JS interop (bit depth, channel order, layout)",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-10"
    },
    {
      "feature": "Document/app event notification bus",
      "app_behavior": "action.addNotificationListener([{event}], (eventName, descriptor)=>void) subscribes to document-altering events (e.g. 'select','open','make','set'); core.addNotificationListener handles UI/OS-level events; central Event Codes registry lists subscribable event ids. Studio needs a hookable event bus.",
      "primitive_domain": "event-hooks",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2021/ps_reference/media/advanced/event-listener/",
      "closes_gap": "XAPP-02: scripts react to user/app events (GLOBAL-BUILD-PARALLEL observable actions)",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-11"
    },
    {
      "feature": "Event code registry",
      "app_behavior": "Event names for addNotificationListener are drawn from a documented Event Codes table (same string ids as batchPlay _obj commands, e.g. historyStateChanged, close, save, tool changes). Studio should ship a canonical machine-readable event-id registry.",
      "primitive_domain": "event-hooks",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/ps_reference/media/eventcodes/",
      "closes_gap": "XAPP-02: stable event id vocabulary for no-context models to bind hooks",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-12"
    },
    {
      "feature": "Plugin entrypoints (panels + commands)",
      "app_behavior": "entrypoints.setup({ panels:{...}, commands:{...} }) registers UI panels and invokable menu commands; manifest 'entrypoints' array declares type 'panel'|'command' with id/label/shortcut. Studio plugin surface must register panels and headless commands the same way.",
      "primitive_domain": "plugin-entrypoints",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/scripting/getting-started/",
      "closes_gap": "XAPP-02: declarative registration of script-invokable commands and panels",
      "verification": "UNVERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-13"
    },
    {
      "feature": "Standalone .psjs headless scripts",
      "app_behavior": "A single ES6+ JavaScript file with .psjs extension runs via File>Scripts>Browse or drag-onto-app icon, no panel/plugin needed; uses full UXP+Photoshop APIs; available Photoshop 23.5+/UDT 1.6+. Studio must support single-file run-and-exit scripting.",
      "primitive_domain": "headless-batch",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/scripting/how-it-works/",
      "closes_gap": "XAPP-02: no-ceremony single-file automation for batch jobs and agents",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-14"
    },
    {
      "feature": "Manifest v5 permission model",
      "app_behavior": "requiredPermissions declares: localFileSystem 'plugin'(sandbox)|'request'(user picker)|'fullAccess'; network.domains allowlist; launchProcess with allowed schemes/extensions (openExternal/openPath); webview.domains allowlist. Deny-by-default capability declaration.",
      "primitive_domain": "permission-model",
      "app_or_standard": "Photoshop UXP",
      "source_url": "https://developer.adobe.com/photoshop/uxp/2022/guides/uxp-guide/uxp-misc/manifest-v5/",
      "closes_gap": "XAPP-02: Studio needs a declarative deny-by-default plugin capability/permission manifest for untrusted scripts",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-15"
    },
    {
      "feature": "Menu-command bridge for non-DOM tasks",
      "app_behavior": "app.executeMenuCommand(stringID) invokes menu items lacking direct DOM APIs (Illustrator, since CS6); ~500+ documented string ids extracted from SDK (some obsolete). Studio should expose a string-id menu-command fallback so 100% of UI actions are scriptable.",
      "primitive_domain": "menu-command-bridge",
      "app_or_standard": "Illustrator scripting (ExtendScript)",
      "source_url": "https://community.adobe.com/t5/illustrator-discussions/executemenucommand-command-list/td-p/13131490",
      "closes_gap": "XAPP-02: guarantees no UI action is unreachable from scripts even without a typed DOM binding",
      "verification": "UNVERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-16"
    },
    {
      "feature": "Recorded-action playback + loadable action files",
      "app_behavior": "app.doScript(actionName, actionSet) plays an Actions-panel action from script; app.loadAction(file)/unloadAction load/remove .aia action files at runtime so scripts don't depend on pre-installed user actions. Studio needs portable, script-loadable macro files.",
      "primitive_domain": "action-playback",
      "app_or_standard": "Illustrator scripting (ExtendScript)",
      "source_url": "https://community.adobe.com/t5/illustrator/javascript-s-quot-do-script-quot-documentation-for-aia-no-gui/m-p/9227875",
      "closes_gap": "XAPP-02: portable recorded-macro playback bundled with a script, not relying on user's local actions",
      "verification": "UNVERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-17"
    },
    {
      "feature": "Typed vector DOM (paths/layers/text)",
      "app_behavior": "ExtendScript DOM exposes app.activeDocument, document.pathItems, layers, textFrames, groupItems, selection for structured vector object model access. (Illustrator UXP remains Adobe-internal / not public to third parties as of 2026; CEP+ExtendScript is the production path.)",
      "primitive_domain": "dom-object-model",
      "app_or_standard": "Illustrator scripting (ExtendScript/CEP)",
      "source_url": "https://community.adobe.com/questions-652/clarification-needed-is-uxp-publicly-available-for-illustrator-in-2026-1548811",
      "closes_gap": "XAPP-02: Studio must ship a first-class typed vector DOM day one (something Illustrator still lacks in UXP)",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-18"
    },
    {
      "feature": "Multi-language doScript host",
      "app_behavior": "app.doScript(scriptText|file, ScriptLanguage.JAVASCRIPT|APPLESCRIPT|VISUAL_BASIC, argsArray, undoMode, undoName) runs foreign scripts and wraps them in a named undo. Studio's JS/TS host should support pluggable languages + undo-mode wrapping.",
      "primitive_domain": "scripting-runtime",
      "app_or_standard": "InDesign scripting",
      "source_url": "https://helpx.adobe.com/in/indesign/using/scripting.html",
      "closes_gap": "XAPP-02: language-agnostic script host with undo-mode control",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-19"
    },
    {
      "feature": "Multi-target event listener model",
      "app_behavior": "eventListeners.add(eventType, handler) attaches at app, document, or menuAction scope; events include afterOpen, beforeClose, beforePrint, afterExport, afterImport; listeners are session-scoped (non-persistent) unless placed in the startup scripts folder. Studio needs scoped, persistable hooks.",
      "primitive_domain": "event-hooks",
      "app_or_standard": "InDesign scripting",
      "source_url": "https://developer.adobe.com/indesign/uxp/resources/recipes/indesign-events/",
      "closes_gap": "XAPP-02: lifecycle hooks (open/close/print/export) plus a startup-script auto-load mechanism",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-20"
    },
    {
      "feature": "Idle-task background scheduler",
      "app_behavior": "app.idleTasks.add(); task.addEventListener(IdleEvent.ON_IDLE, callback) runs deferred work during app idle without blocking the UI thread. Studio should expose an idle/background task queue for non-intrusive long-running script work.",
      "primitive_domain": "scripting-runtime",
      "app_or_standard": "InDesign scripting",
      "source_url": "https://developer.adobe.com/indesign/uxp/resources/recipes/indesign-events/",
      "closes_gap": "XAPP-02: quiet background scheduling (GLOBAL-BUILD-QUIET bounded/observable background work)",
      "verification": "UNVERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-21"
    },
    {
      "feature": "GREP / text find-change engine",
      "app_behavior": "Set app.findGrepPreferences/changeGrepPreferences (or findTextPreferences), then doc.findGrep()/changeGrep(); search modes cover GREP (regex), TEXT, GLYPH, and OBJECT (find/change formatting). Studio must ship a scriptable regex find-change over text+formatting+objects.",
      "primitive_domain": "text-find-change",
      "app_or_standard": "InDesign scripting",
      "source_url": "https://helpx.adobe.com/indesign/using/find-replace-grep-queries.html",
      "closes_gap": "XAPP-02: programmatic regex find/replace across text, glyphs, and object attributes",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-22"
    },
    {
      "feature": "Structured multi-format export",
      "app_behavior": "doc.exportFile(ExportFormat.PDF_TYPE, file, showingOptions, PDFExportPreset) exports; ExportFormat covers PDF_TYPE, INDESIGN_MARKUP (IDML), EPUB (reflow/FXL), INCML (ICML), XHTML, PNG, JPG. Preset-driven, options-suppressible. Studio needs preset-parameterized scriptable export.",
      "primitive_domain": "structured-export",
      "app_or_standard": "InDesign scripting DOM",
      "source_url": "https://developer.adobe.com/indesign/dom/api/e/ExportFormat/",
      "closes_gap": "XAPP-02: one export call parameterized by named presets across PDF/IDML/EPUB/XHTML/image",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-23"
    },
    {
      "feature": "Package-for-output automation",
      "app_behavior": "doc.packageForPrint(toFolder, copyingFonts, copyingLinkedGraphics, copyingProfiles, updatingGraphics, includingHiddenLayers, ignorePreflightErrors, creatingReport, includeIdml, includePdf, pdfStyle, ...) collects fonts+links+profiles+report in one call. Studio should offer a scriptable collect/package.",
      "primitive_domain": "preflight-package",
      "app_or_standard": "InDesign scripting DOM",
      "source_url": "https://developer.adobe.com/indesign/dom/api/d/Document/",
      "closes_gap": "XAPP-02: one-call asset collection + optional IDML/PDF + report for handoff",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-24"
    },
    {
      "feature": "DOM version pinning + collection access",
      "app_behavior": "InDesign DOM is a require()-able JS module (18.4+); scripts pin a DOM version for forward compatibility so future releases don't break them; collections use item(i) (subscript [] removed in UXP). Studio must version its scripting DOM and use explicit accessors.",
      "primitive_domain": "dom-versioning",
      "app_or_standard": "InDesign UXP",
      "source_url": "https://developer.adobe.com/indesign/uxp/resources/fundamentals/dom-versioning/",
      "closes_gap": "XAPP-02: stable, version-pinned scripting DOM so old scripts keep running (long-term maintenance)",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-25"
    },
    {
      "feature": "Data-merge (variable-data) scripting",
      "app_behavior": "dataMergeProperties / dataMergeOptions drive merge of a delimited data source into placeholders; scripts can merge all records to a new document or export single records as PDF/INDD with filenames derived from database fields, and combine with GREP find-change. Studio needs scriptable variable-data.",
      "primitive_domain": "data-merge",
      "app_or_standard": "InDesign scripting",
      "source_url": "https://community.adobe.com/t5/indesign-discussions/grep-find-change-combined-with-data-merge/m-p/14438256",
      "closes_gap": "XAPP-02: programmatic variable-data/mail-merge producing per-record named outputs",
      "verification": "UNVERIFIED",
      "id": "SFR-CLOSE-automation-model-depth-26"
    }
  ]
}
```

### [SFR-CLOSE.performance-nfr-targets] Production-volume performance NFR targets to beat (16 rows)

```json
{
  "rows": [
    {
      "feature": "Per-document memory ceiling (canvas engine)",
      "app_or_standard": "Figma",
      "app_behavior": "Hard ~2GB active memory limit PER BROWSER TAB (WASM heap), enforced even in the desktop app because it is browser/Chromium-hosted; nearing the wall causes long load times, 'file almost out of browser memory' warnings, and tab crashes. Figma recommends splitting files to stay under the wall.",
      "primitive_domain": "canvas-memory / document-model",
      "source_url": "https://help.figma.com/hc/en-us/articles/360040528173-Reduce-memory-usage-in-files",
      "closes_gap": "Studio target: native 64-bit process addressing, NO 2GB wall; disk-backed document streaming (mmap / paged scene graph) so working-set RAM is decoupled from document size — open 10GB+ documents on 16GB RAM by paging cold pages/artboards/layers to SSD. Target: document size limited only by disk, not by process address space.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-01"
    },
    {
      "feature": "Image/asset decode budget separation",
      "app_or_standard": "Figma",
      "app_behavior": "Image decoding consumes JS-heap memory that is NOT counted in the reported memory percentage, and decoded images additionally consume WASM canvas memory when rendered — a hidden second allocation path that pushes files over the 2GB wall unpredictably.",
      "primitive_domain": "asset-pipeline / raster-cache",
      "source_url": "https://help.figma.com/hc/en-us/articles/360040528173-Reduce-memory-usage-in-files",
      "closes_gap": "Studio target: single unified GPU-uploaded texture cache with an explicit, bounded, evictable budget (LRU eviction to disk); decoded-image residency is tracked in one accounting path with a hard cap, so RAM pressure is deterministic and observable, not split across hidden JS+WASM pools.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-02"
    },
    {
      "feature": "Component/variant swap cost",
      "app_or_standard": "Figma",
      "app_behavior": "Variant sets above ~1000 variants degrade performance; Figma must load ALL variants in the background to enable instance swapping, so a single large set plus many instances multiplies memory/CPU and can crash on variant click. Figma explicitly discourages sets this large and steers users to component properties instead.",
      "primitive_domain": "component-system / symbol-instancing",
      "source_url": "https://forum.figma.com/suggest-a-feature-11/optimize-performance-in-large-variant-sets-28624",
      "closes_gap": "Studio target: lazy variant resolution — only the active variant plus a small predicted set are resident; variant definitions are indexed on disk and materialized on demand. Target: O(1) swap cost independent of set size, 10,000+ variants with no background full-load.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-03"
    },
    {
      "feature": "High-megapixel raster editing headroom",
      "app_or_standard": "Affinity Photo",
      "app_behavior": "Memory management becomes less stable than Photoshop's at 60MP+; the gap is most apparent on 100MP+ medium-format files, where slowdown and unpredictable memory reclamation set in. Large RAW stacks, 10+ image HDR merges, and hundreds-of-layers documents compound the degradation.",
      "primitive_domain": "raster-engine / tile-memory",
      "source_url": "https://www.inkydesignworks.com/posts/affinity-photo-problems",
      "closes_gap": "Studio target: tiled, disk-backed raster with parallel tile codecs — edit 500MP+ / gigapixel rasters at interactive rates on commodity RAM by paging tiles; explicit deterministic tile-memory budget with predictable reclamation (no GC-style unpredictability). Target: interactive edit at 4x the 100MP+ threshold.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-04"
    },
    {
      "feature": "Long-document composition throughput",
      "app_or_standard": "Affinity Publisher",
      "app_behavior": "Text flowing over hundreds of pages makes the whole app extremely slow — text entry, font selection, applying styles, and changing visual options all lag on long documents (reported bug, Windows). Recomposition appears to re-run over the whole flow rather than incrementally.",
      "primitive_domain": "text-layout / incremental-reflow",
      "source_url": "https://forum.affinity.serif.com/index.php?/topic/98601-publisher-extremely-slow-on-long-documents/",
      "closes_gap": "Studio target: incremental/localized text recomposition — a keystroke reflows only the affected story region and dirty pages, not the whole document; off-screen pages composed lazily and cached. Target: constant-time-per-edit typing latency (<16ms) at 2000+ pages.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-05"
    },
    {
      "feature": "GPU compute stability (raster acceleration)",
      "app_or_standard": "Affinity (Photo/Publisher/Designer)",
      "app_behavior": "OpenCL hardware ('Compute') acceleration is the #1 cause of instability/crash-on-splash unless on a high-end workstation GPU; it conflicts with specific GPU drivers, and Serif's own documented workaround is to DISABLE hardware acceleration entirely.",
      "primitive_domain": "gpu-backend / driver-abstraction",
      "source_url": "https://support.serif.com/hc/en-us/articles/10414847589647-How-do-I-disable-OpenCL-Compute-Acceleration-Hardware-Acceleration-on-Windows",
      "closes_gap": "Studio target: wgpu abstraction over Vulkan/Metal/DX12 (not legacy OpenCL) with a validated device allow/deny path and automatic, graceful CPU fallback on driver fault — GPU acceleration stays ON safely on consumer GPUs. Target: zero crash-on-launch from GPU path; acceleration default-on, not default-off.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-06"
    },
    {
      "feature": "GPU stability across OS updates",
      "app_or_standard": "Affinity",
      "app_behavior": "Windows 24H2 can cause Affinity app instability (system hangs/freezes) with OpenCL Acceleration enabled; Serif's guidance is again to disable acceleration — i.e., an OS update silently breaks the GPU path with no in-app recovery.",
      "primitive_domain": "gpu-backend / os-compat",
      "source_url": "https://support.serif.com/hc/en-us/articles/12344117915663-Windows-24H2-update-can-cause-Affinity-app-Instability-with-OpenCL-Acceleration-Enabled",
      "closes_gap": "Studio target: runtime GPU capability probe + crash-loop detector that auto-quarantines a faulting backend and continues on a fallback adapter, surfacing a diagnostic — OS/driver regressions degrade gracefully instead of hanging the app. UNVERIFIED source text (403 on fetch); confirmed via search snippet.",
      "verification": "UNVERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-07"
    },
    {
      "feature": "Linked-image scaling (long doc, many placements)",
      "app_or_standard": "InDesign",
      "app_behavior": "Documents with ~3300 images per chapter become very slow after import; updating modified images, relinking, moving text frames, and paging take 5-10 seconds longer per action. High-Quality display forces hi-res color-managed rendering of every placed graphic; 'Typical' downgrades to screen-res to recover speed.",
      "primitive_domain": "linked-asset-manager / display-proxy",
      "source_url": "https://community.adobe.com/t5/indesign-discussions/indesign-getting-slow-due-to-more-number-of-images/td-p/12380748",
      "closes_gap": "Studio target: lazy/amortized link loading with persistent multi-resolution proxy cache — only on-screen placements decode at full res; link status checks are async/batched off the UI thread. Target: sub-100ms page turns and relinks at 10,000+ linked hi-res images.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-08"
    },
    {
      "feature": "GPU acceleration availability floor",
      "app_or_standard": "InDesign",
      "app_behavior": "GPU Performance requires >=1024MB VRAM and, on Windows, DirectX 12 (Feature Level 12_0), OpenGL 4.4+, AND a monitor resolution greater than 2K; on macOS it requires Metal (M1+/Intel Metal GPU). Sub-2K monitors and older GPUs get no acceleration at all — historically macOS-only before v20.4.",
      "primitive_domain": "gpu-backend / capability-gate",
      "source_url": "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/gpu-performance.html",
      "closes_gap": "Studio target: GPU-native canvas via wgpu with NO monitor-resolution gate and a low VRAM floor; identical accelerated pipeline on Windows/macOS/Linux from one codebase, with CPU raster fallback below the floor. Target: accelerated rendering on any GPU wgpu supports, any display resolution.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-09"
    },
    {
      "feature": "Vector engine parallelism",
      "app_or_standard": "Illustrator",
      "app_behavior": "Core vector processing is bound to a SINGLE CPU thread and has been for many years; multi-core is used only for some ancillary tasks, with diminishing returns past 8 cores. Adobe only announced a multithreading effort in Sept 2024 — complex documents remain single-thread-bound today.",
      "primitive_domain": "vector-engine / render-scheduler",
      "source_url": "https://illustrator.uservoice.com/forums/333657-illustrator-desktop-feature-requests/suggestions/20423296-make-illustrator-multi-threaded-on-cpu",
      "closes_gap": "Studio target: Rust data-parallel geometry pipeline (rayon-style work-stealing) — tessellation, boolean ops, path effects, and rendering fan out across all cores and scale past 8. Target: near-linear speedup to core count on batch vector ops vs single-thread incumbent.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-10"
    },
    {
      "feature": "Canvas coordinate range (standard)",
      "app_or_standard": "Illustrator",
      "app_behavior": "Canvas is capped at 2^14 = 16384 points (16383 addressable from zero) ≈ 227 inches / 5779mm per axis, because coordinates are stored in a fixed representation the whole engine is built on; Adobe calls it costly/foundational to change.",
      "primitive_domain": "coordinate-system / scene-precision",
      "source_url": "https://community.adobe.com/t5/illustrator/add-the-ability-to-scale-the-canvas-beyond-it-s-archaic-227-inch-limits/m-p/4182920",
      "closes_gap": "Studio target: unbounded 64-bit (f64 / fixed-64) world coordinates from day one — effectively no practical canvas cap; large-format signage, maps, and architectural plates fit without mode-switching. Target: >10,000 inch canvas at full precision, no coordinate-overflow artifacts.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-11"
    },
    {
      "feature": "Large-canvas mode ceiling",
      "app_or_standard": "Illustrator",
      "app_behavior": "Opt-in Large Canvas mode raises the cap 10x to ~2,275 inches / 163,822 pixels per axis, but must be chosen at document creation, cannot be toggled on an existing standard doc, and trades precision — it is a bolt-on mode, not the default coordinate space.",
      "primitive_domain": "coordinate-system / precision-mode",
      "source_url": "https://pixelandbracket.com/how-to-change-max-canvas-size-in-illustrator/",
      "closes_gap": "Studio target: one uniform 64-bit coordinate space — no separate 'large canvas' mode, no create-time decision, no precision cliff; every document already spans the full range. Target: eliminate the mode entirely as a UX/precision failure class.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-12"
    },
    {
      "feature": "Artboards per document",
      "app_or_standard": "Illustrator",
      "app_behavior": "Hard cap of 1000 artboards per document (raised from 100 in Oct 2017); icon-library and storyboard users hit it and there are standing feature requests to exceed it. Artboards are a fixed-count structure, not a streamed collection.",
      "primitive_domain": "document-model / page-artboard-collection",
      "source_url": "https://illustrator.uservoice.com/forums/333657-illustrator-desktop-feature-requests/suggestions/33333385-be-available-to-add-more-than-1000-artboards",
      "closes_gap": "Studio target: artboards/pages as a virtualized, disk-indexed collection with lazy materialization and view culling — no fixed cap; only visible artboards are resident. Target: 100,000+ artboards/pages with constant memory and instant navigation.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-13"
    },
    {
      "feature": "Save cost on large documents",
      "app_or_standard": "Affinity / InDesign (long-doc class)",
      "app_behavior": "Serif guidance for documents over 500MB is to switch resources from Embedded to Linked specifically to cut file bloat and corruption risk — i.e., saving large embedded docs is expensive and corruption-prone; incumbents rewrite whole-file on save rather than journaling deltas.",
      "primitive_domain": "persistence / save-io",
      "source_url": "https://softexpo.com/fixes/633-serif-affinity-publisher-solving-major-problems.html",
      "closes_gap": "Studio target: incremental/append-only save — persist only changed pages/objects as a journal + periodic compaction, with crash-safe atomic commit. Target: save latency proportional to edit size, not document size; sub-second saves on multi-GB documents, no corruption on kill-mid-save.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-14"
    },
    {
      "feature": "Working-set vs document-size decoupling",
      "app_or_standard": "Figma / Affinity / InDesign (cross-cut)",
      "app_behavior": "All incumbents load enough of the document into RAM that document growth drives RAM growth toward a hard wall (Figma 2GB tab; Affinity 100MP+/hundreds-of-pages instability; InDesign thousands-of-images lag) — none stream a document larger than available RAM without degradation.",
      "primitive_domain": "document-model / streaming-io",
      "source_url": "https://www.linkedin.com/pulse/figma-memory-limit-file-almost-out-browser-nabeel-saleem",
      "closes_gap": "Studio target: memory-mapped, paged scene graph where resident set = visible viewport + working region, independent of total document size. Target: open and edit documents 10x larger than physical RAM with graceful page-in/out, no hard wall.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-15"
    },
    {
      "feature": "Core-count scaling on raster/effects",
      "app_or_standard": "Illustrator (and effect pipelines generally)",
      "app_behavior": "Even where Illustrator uses multiple cores, benefit tails off after 8 cores — the engine does not scale to modern 16/24/32-core desktops, so high-end hardware is wasted on complex vector/effect workloads.",
      "primitive_domain": "render-scheduler / parallel-effects",
      "source_url": "https://community.adobe.com/t5/illustrator-discussions/does-illustrator-use-multiple-cores/td-p/9537873",
      "closes_gap": "Studio target: tile- and object-parallel effect/render scheduler that keeps scaling past 8 cores on desktop-class CPUs; GPU compute for filterable effects. Target: measurable speedup at 16/32 cores (no plateau) on blur/mesh/tessellation batch workloads.",
      "verification": "VERIFIED",
      "id": "SFR-CLOSE-performance-nfr-targets-16"
    }
  ]
}
```
