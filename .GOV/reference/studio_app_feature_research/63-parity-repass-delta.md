---
file_id: studio-app-feature-research-parity-repass-delta
topic_id: SFR-REPASS
title: "Parity Re-Pass Delta (2026-07-20, ACTION-A5)"
status: draft
summary: "Targeted vendor re-pass rows closing the highest-value verified parity gaps from the 58-register: Figma Sites accessibility (CRITICAL), Camera Raw, InDesign fonts, Affinity option depth, PS AVIF/JXL+HDR. Online-source; NON-AI."
sources: 106
updated_at: "2026-07-20"
---


## [SFR-REPASS] Parity Re-Pass Delta

### [SFR-REPASS.summary] Summary

```json
{
  "action": "ACTION-A5 (see 61-parity-audit-action-register.md)",
  "method": "Online-source targeted re-pass per lane; each row cites an authoritative vendor URL. NON-AI scope.",
  "total_rows": 106,
  "by_lane": {
    "figma-sites-a11y": 27,
    "camera-raw": 19,
    "indesign-fonts": 24,
    "affinity-option-depth": 21,
    "ps-formats-hdr": 15
  },
  "closes_critical": "Figma Sites per-element Accessibility panel + semantic HTML + landmarks + ARIA roles/labels/current/hidden — the sole CRITICAL gap (SFR-PGAP-FG) is now rowed at promotable depth.",
  "authority": "Reference/provenance only; not product authority. Feeds WP-KERNEL-STUDIO refinement Section-14 coverage decisions."
}
```

### [SFR-REPASS.figma-sites-a11y] Figma Sites Accessibility + SEO + Website Settings (closes the sole CRITICAL gap)

```json
{
  "rows": [
    {
      "feature": "Per-element Accessibility panel in right sidebar",
      "app_behavior": "Dedicated Accessibility section on the selected element exposing Alt text, Label, Hidden, Current item, and Role controls at 1-click depth on the design canvas",
      "primitive_domain": "accessibility.panel",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242789265431-Improve-the-accessibility-of-your-site",
      "closes_gap": "SFR-PGAP-FG accessibility panel (CRITICAL)",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-01"
    },
    {
      "feature": "Image/video alt text with decorative toggle",
      "app_behavior": "Alt text field on media elements to write a concise functional description OR mark the image as decorative (emits empty alt / aria-hidden)",
      "primitive_domain": "accessibility.alt_text",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242789265431-Improve-the-accessibility-of-your-site",
      "closes_gap": "SFR-PGAP-FG accessibility panel (CRITICAL)",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-02"
    },
    {
      "feature": "ARIA label for elements without visible text",
      "app_behavior": "Label field mapping to aria-label, naming icon buttons and controls that have no visible text",
      "primitive_domain": "accessibility.aria_label",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242789265431-Improve-the-accessibility-of-your-site",
      "closes_gap": "SFR-PGAP-FG accessibility panel (CRITICAL)",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-03"
    },
    {
      "feature": "Hidden (aria-hidden) toggle",
      "app_behavior": "Hidden control that marks decorative content and removes it from the accessibility tree via aria-hidden",
      "primitive_domain": "accessibility.aria_hidden",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242789265431-Improve-the-accessibility-of-your-site",
      "closes_gap": "SFR-PGAP-FG accessibility panel (CRITICAL)",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-04"
    },
    {
      "feature": "Current item (aria-current) state",
      "app_behavior": "Current item dropdown with values page, step, location, date, time, true, false mapping to aria-current for active nav/step states",
      "primitive_domain": "accessibility.aria_current",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242789265431-Improve-the-accessibility-of-your-site",
      "closes_gap": "SFR-PGAP-FG accessibility panel (CRITICAL)",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-05"
    },
    {
      "feature": "ARIA Role assignment (Regions/Document/Interactive)",
      "app_behavior": "Role picker mapping to the role attribute, grouped Regions (banner, complementary, contentinfo, form, main, navigation, region, search), Document (article, figure, list, listitem, table, tooltip, etc.), Interactive (button, checkbox, link, tab, tabpanel, slider, combobox, dialog, alert, etc.)",
      "primitive_domain": "accessibility.aria_role",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242789265431-Improve-the-accessibility-of-your-site",
      "closes_gap": "SFR-PGAP-FG accessibility panel (CRITICAL)",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-06"
    },
    {
      "feature": "Semantic HTML tag selection per element",
      "app_behavior": "HTML tag dropdown letting an element emit Container(div), Section, Article, Aside, Navigation(nav), Header, Footer, Main Content, Button, Media container(figure), Ordered list(ol), Unordered list(ul)",
      "primitive_domain": "semantic.html_tag",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242789265431-Improve-the-accessibility-of-your-site",
      "closes_gap": "SFR-PGAP-FG accessibility panel (CRITICAL)",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-07"
    },
    {
      "feature": "Heading level tag (h1-h6)",
      "app_behavior": "Heading tag selector assigning h1 through h6 to text elements to define document outline; guidance enforces single h1 per page and hierarchical h2-h6 subheadings",
      "primitive_domain": "semantic.heading_level",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242789265431-Improve-the-accessibility-of-your-site",
      "closes_gap": "SFR-PGAP-FG accessibility panel (CRITICAL)",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-08"
    },
    {
      "feature": "Landmark tags on top-level frames",
      "app_behavior": "Assigning landmark HTML tags (nav, main, footer, header, aside) to top-level auto layout frames so screen readers can jump between page regions",
      "primitive_domain": "semantic.landmarks",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242789265431-Improve-the-accessibility-of-your-site",
      "closes_gap": "SFR-PGAP-FG accessibility panel (CRITICAL)",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-09"
    },
    {
      "feature": "Site title metadata",
      "app_behavior": "Website settings Title field that appears in browser tabs, search engine results, and social media; overridable per-page",
      "primitive_domain": "seo.meta_title",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242875661591-Edit-website-settings",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-10"
    },
    {
      "feature": "Meta description",
      "app_behavior": "Meta description field (site-wide and per-page) providing a short summary for search results and social cards",
      "primitive_domain": "seo.meta_description",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242875661591-Edit-website-settings",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-11"
    },
    {
      "feature": "Social sharing (OG) image",
      "app_behavior": "Social sharing image setting (recommended 1200x630px) rendered when the page is shared on social media; overridable per-page",
      "primitive_domain": "seo.og_image",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242875661591-Edit-website-settings",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-12"
    },
    {
      "feature": "Favicon",
      "app_behavior": "Favicon upload for the small browser-tab representation of the site",
      "primitive_domain": "site_settings.favicon",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242875661591-Edit-website-settings",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-13"
    },
    {
      "feature": "Language code (lang attribute)",
      "app_behavior": "Language code field using ISO codes to declare the page primary language for assistive tech; overridable per-page",
      "primitive_domain": "seo.language",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242875661591-Edit-website-settings",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-14"
    },
    {
      "feature": "Search engine indexing toggle",
      "app_behavior": "Enable/disable search-engine indexing, emitting meta name=robots content=noindex when disabled; per-page controllable",
      "primitive_domain": "seo.robots_noindex",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242875661591-Edit-website-settings",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-15"
    },
    {
      "feature": "Google Analytics ID",
      "app_behavior": "Field to connect a Google Analytics property ID for site traffic insights",
      "primitive_domain": "site_settings.analytics",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242875661591-Edit-website-settings",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-16"
    },
    {
      "feature": "Custom code injection (head/body)",
      "app_behavior": "Custom code setting to insert HTML/script snippets into the head or body tags site-wide",
      "primitive_domain": "site_settings.custom_code",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242875661591-Edit-website-settings",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-17"
    },
    {
      "feature": "Custom domain + subdomain",
      "app_behavior": "Add a personalized custom domain and choose a subdomain for the published site",
      "primitive_domain": "publish.custom_domain",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242875661591-Edit-website-settings",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-18"
    },
    {
      "feature": "Published site access control",
      "app_behavior": "Published site access setting to restrict to an internal audience or open to the web (paid plans)",
      "primitive_domain": "publish.access_control",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242875661591-Edit-website-settings",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-19"
    },
    {
      "feature": "Password protection (site + per-page)",
      "app_behavior": "Password protection applied site-wide or per individual page to gate content (paid plans)",
      "primitive_domain": "publish.password",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242875661591-Edit-website-settings",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-20"
    },
    {
      "feature": "Cookie consent banner",
      "app_behavior": "Built-in cookie consent banner toggle in website settings (paid plans)",
      "primitive_domain": "site_settings.cookie_consent",
      "source_url": "https://help.figma.com/hc/en-us/articles/31242875661591-Edit-website-settings",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-21"
    },
    {
      "feature": "CMS collection creation with typed fields",
      "app_behavior": "Create a CMS collection (spreadsheet model: fields=columns, items=rows) with field types Title(required), Slug(required, auto from title, editable), Plain text, Rich text, Link, Image (JPEG/PNG/GIF), Date; up to 200 items per collection in beta",
      "primitive_domain": "cms.collection_fields",
      "source_url": "https://help.figma.com/hc/en-us/articles/36165345510551-Create-a-CMS-collection",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-22"
    },
    {
      "feature": "CMS rich text field editing + Rich text styles",
      "app_behavior": "Rich text field editor with six heading styles + one body style, bold/italic/underline, links, left/center/right alignment, single-level numbered/bulleted lists, inline images (align + fill-width) with alt text, Markdown input; Figma auto-generates Rich text styles for typography",
      "primitive_domain": "cms.rich_text",
      "source_url": "https://help.figma.com/hc/en-us/articles/36165352090775-Work-with-rich-text-fields-in-CMS",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-23"
    },
    {
      "feature": "Bind CMS field / variable to canvas layer",
      "app_behavior": "Select a text/image layer and Apply variable or CMS field (via right sidebar or the Connect view) to wire collection data into the design, producing a rich text layer bound to the field",
      "primitive_domain": "cms.field_binding",
      "source_url": "https://help.figma.com/hc/en-us/articles/36165352090775-Work-with-rich-text-fields-in-CMS",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-24"
    },
    {
      "feature": "CMS list and CMS page templating",
      "app_behavior": "CMS list repeats one design across all items in a collection; CMS page is a dedicated detail webpage template for a single item (opened when a visitor clicks a list item)",
      "primitive_domain": "cms.list_and_page",
      "source_url": "https://help.figma.com/hc/en-us/articles/35222938006679-Create-a-CMS-page",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-25"
    },
    {
      "feature": "CMS CSV import",
      "app_behavior": "Import a CMS collection from a CSV file to bulk-populate items",
      "primitive_domain": "cms.csv_import",
      "source_url": "https://help.figma.com/hc/en-us/articles/35691883305879-Import-a-CMS-collection-from-a-CSV",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-26"
    },
    {
      "feature": "CMS republish gate",
      "app_behavior": "Collection edits stay in draft and do not appear on the live site until the site is republished (explicit publish action separates draft content state from live)",
      "primitive_domain": "publish.republish_draft",
      "source_url": "https://help.figma.com/hc/en-us/articles/35995403973783-Guide-to-Figma-Sites-CMS",
      "closes_gap": "",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-figma-sites-a11y-27"
    }
  ]
}
```

### [SFR-REPASS.camera-raw] Adobe Camera Raw 2024-2026 NON-AI develop surface

```json
{
  "rows": [
    {
      "feature": "Point Color per-swatch selective color",
      "app_behavior": "Color Mixer > Point Color tab: eyedropper samples a color into up to 8 stored swatches; each swatch exposes independent Hue, Saturation, Luminance shift sliders plus Hue Range, Saturation Range, Luminance Range sliders and a 'Visualize Range' toggle to preview affected pixels",
      "primitive_domain": "color-selective",
      "source_url": "https://helpx.adobe.com/camera-raw/using/make-color-tonal-adjustments-camera.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-01"
    },
    {
      "feature": "Point Color Variance slider",
      "app_behavior": "Variance slider inside Point Color edit controls (ACR 17.4+): decreasing pulls in-range colors toward the sampled color, increasing pushes them away, tightening or widening the selective-color falloff around each swatch",
      "primitive_domain": "color-selective",
      "source_url": "https://helpx.adobe.com/camera-raw/using/make-color-tonal-adjustments-camera.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-02"
    },
    {
      "feature": "HDR Optimization / HDR Output editing",
      "app_behavior": "HDR mode edits raw in high-dynamic-range with HDR-display preview; a 'High Dynamic Range' section at bottom of Basic panel with a 'Preview for SDR' toggle that reveals a separate set of SDR-only tone sliders; Visualize gamut/clipping overlays; 'HDR Output' checkbox in the Save dialog controls whether the exported file carries HDR",
      "primitive_domain": "hdr-tone",
      "source_url": "https://helpx.adobe.com/camera-raw/using/hdr-output.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-03"
    },
    {
      "feature": "Gain Map HDR embedding",
      "app_behavior": "On save, 'Maximize Compatibility' writes an ISO gain map so one file stores a base rendition (SDR or HDR) + gain map + metadata; display device/browser interpolates between SDR and HDR renditions using its own HDR headroom",
      "primitive_domain": "hdr-io",
      "source_url": "https://helpx.adobe.com/camera-raw/using/gain-map.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-04"
    },
    {
      "feature": "AVIF save/open support",
      "app_behavior": "ACR can open and save AVIF (added ACR 15.1); AVIF is selectable in the Save dialog with image-dimension and quality-level controls, and supports HDR output with gain map when 'Maximize Compatibility' + 'HDR Output' are enabled",
      "primitive_domain": "io-export",
      "source_url": "https://helpx.adobe.com/camera-raw/using/navigate-open-save-images-camera.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-05"
    },
    {
      "feature": "JPEG XL (JXL) save/open support",
      "app_behavior": "ACR can open and save JPEG XL; JXL selectable in Save dialog and supports HDR output with embedded gain map for web/gallery sharing",
      "primitive_domain": "io-export",
      "source_url": "https://helpx.adobe.com/camera-raw/using/navigate-open-save-images-camera.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-06"
    },
    {
      "feature": "WebP save support",
      "app_behavior": "Native WebP export in the ACR Save dialog is NOT confirmed in current ACR docs (open-format request in Adobe community only); WebP appears unsupported as a native ACR save format as of 2025-2026",
      "primitive_domain": "io-export",
      "source_url": "https://helpx.adobe.com/camera-raw/using/navigate-open-save-images-camera.html",
      "verification": "UNVERIFIED",
      "id": "SFR-REPASS-camera-raw-07"
    },
    {
      "feature": "Global three-way Color Grading wheels",
      "app_behavior": "Color Grading panel with Shadows / Midtones / Highlights + Global color wheels (drag for hue around the wheel, in/out for saturation), a Luminance slider under each wheel, plus Blending and Balance sliders to control overlap and weighting between tonal ranges",
      "primitive_domain": "color-grade",
      "source_url": "https://helpx.adobe.com/camera-raw/using/make-color-tonal-adjustments-camera.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-08"
    },
    {
      "feature": "Local Color Grading inside masks",
      "app_behavior": "Color Grading three-way wheels (Shadows/Midtones/Highlights hue+sat, per-wheel Luminance, Blending) are now available per-mask in the Masking panel, so a graded look can be applied to one masked region and left off the rest of the frame",
      "primitive_domain": "masking-color",
      "source_url": "https://helpx.adobe.com/camera-raw/using/masking.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-09"
    },
    {
      "feature": "Mask Edge and Feather refinement sliders",
      "app_behavior": "Masking panel adds dedicated Edge and Feather refinement sliders to soften/harden and shift mask boundaries (introduced May 2026 masking-refinements update); exact slider ranges/labels not yet confirmed from primary helpx doc",
      "primitive_domain": "masking-refine",
      "source_url": "https://helpx.adobe.com/camera-raw/using/masking.html",
      "verification": "UNVERIFIED",
      "id": "SFR-REPASS-camera-raw-10"
    },
    {
      "feature": "Anamorphic desqueeze",
      "app_behavior": "Crop/Geometry panel adds anamorphic desqueeze that fixes aspect ratio of anamorphic-lens captures with selectable squeeze factors (1.33x, 1.6x, 2.0x) supporting factors up to 2.0 (ACR 18.3)",
      "primitive_domain": "geometry-lens",
      "source_url": "https://helpx.adobe.com/camera-raw/using/correct-lens-distortions-camera-raw.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-11"
    },
    {
      "feature": "Projection Correction slider",
      "app_behavior": "Crop/Geometry > Manual Transforms adds a Projection Correction slider that bends edges/corners to reduce 'stretched faces' near the borders of wide-angle group shots and selfies (ACR 18.3)",
      "primitive_domain": "geometry-lens",
      "source_url": "https://helpx.adobe.com/camera-raw/using/automatic-perspective-correction-camera-raw.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-12"
    },
    {
      "feature": "Show Autofocus Points overlay",
      "app_behavior": "'Show Autofocus Points' overlays the camera's capture-time AF points on the preview for supported Canon, Nikon, and Sony raw files; toggled from the preview right-click menu or Ctrl+Alt+Shift+O (Win)/Cmd+Opt+Shift+O (Mac); non-destructive, does not alter edits or metadata (ACR 18.4.1, July 2026)",
      "primitive_domain": "metadata-overlay",
      "source_url": "https://helpx.adobe.com/camera-raw/using/whats-new/release-notes.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-13"
    },
    {
      "feature": "Extended white-balance Temperature range to 1500K",
      "app_behavior": "Basic panel Temperature slider for raw files now reaches down to 1500 Kelvin (previously 2000K, top still ~50000K) for finer white-balance control of candlelit/low-light scenes (May 2026)",
      "primitive_domain": "white-balance",
      "source_url": "https://helpx.adobe.com/camera-raw/using/make-color-tonal-adjustments-camera.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-14"
    },
    {
      "feature": "Filmstrip Synchronize Settings",
      "app_behavior": "With multiple filmstrip images selected, Synchronize button (or Alt+S/Opt+S) opens a Synchronize dialog to push the active image's chosen edit categories (WB, tone, color, masks, crop, etc.) to all selected images",
      "primitive_domain": "batch-workflow",
      "source_url": "https://helpx.adobe.com/camera-raw/using/camera-raw-settings.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-15"
    },
    {
      "feature": "Copy/Paste Edit Settings with granular selection",
      "app_behavior": "Ctrl+C/Ctrl+V copies and pastes all edit settings between filmstrip images; Ctrl+Alt+C opens a 'Copy Edit Settings' dialog to choose exactly which adjustment groups are copied before pasting to one or many images",
      "primitive_domain": "batch-workflow",
      "source_url": "https://helpx.adobe.com/camera-raw/using/camera-raw-settings.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-16"
    },
    {
      "feature": "Merge to Panorama from filmstrip",
      "app_behavior": "Select multiple raw frames in the filmstrip and choose Merge to Panorama (Ctrl+M) or the thumbnail ellipsis 'Merge to' menu; stitches into a raw DNG panorama with projection (Spherical/Cylindrical/Perspective), Boundary Warp and Auto Crop options",
      "primitive_domain": "merge",
      "source_url": "https://helpx.adobe.com/camera-raw/using/create-panoramas.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-17"
    },
    {
      "feature": "Merge to HDR from filmstrip",
      "app_behavior": "Select exposure-bracketed raw frames and choose Merge to HDR to produce a single floating-point raw DNG with Auto Align, Auto Settings, and Deghost (Amount/Overlay) controls for moving-subject artifacts",
      "primitive_domain": "merge",
      "source_url": "https://helpx.adobe.com/camera-raw/using/create-panoramas.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-18"
    },
    {
      "feature": "Merge to HDR Panorama from filmstrip",
      "app_behavior": "For bracketed sequences across multiple positions, Merge to HDR Panorama combines HDR merge and panorama stitching in one step, outputting a single high-bit raw DNG",
      "primitive_domain": "merge",
      "source_url": "https://helpx.adobe.com/camera-raw/using/create-panoramas.html",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-camera-raw-19"
    }
  ]
}
```

### [SFR-REPASS.indesign-fonts] InDesign variable fonts + OpenType + font management

```json
{
  "rows": [
    {
      "feature": "Variable font axis sliders",
      "app_behavior": "For variable OpenType fonts, expose live slider controls for the registered axes Weight, Width, Slant, and Optical Size in the Control panel, Character panel, Properties panel, Character Styles, and Paragraph Styles; sliders map to font-variation named/custom axes and persist as text-run attributes.",
      "primitive_domain": "typography.variable-fonts",
      "source_url": "https://helpx.adobe.com/indesign/using/using-fonts.html",
      "closes_gap": "variable-fonts",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-01"
    },
    {
      "feature": "Auto optical-size matching",
      "app_behavior": "Option to automatically match a variable font's Optical Size axis to the current font size; when enabled optical size tracks font-size changes, when disabled optical size is decoupled and held constant.",
      "primitive_domain": "typography.variable-fonts",
      "source_url": "https://helpx.adobe.com/indesign/using/using-fonts.html",
      "closes_gap": "variable-fonts",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-02"
    },
    {
      "feature": "Custom (non-registered) variable axes",
      "app_behavior": "Support foundry-defined custom axes beyond the 5 registered axes (width/weight/slant/italic/optical-size), rendering a generated slider per exposed axis (e.g. Acumin Variable exposes Slant/Weight/Width).",
      "primitive_domain": "typography.variable-fonts",
      "source_url": "https://helpx.adobe.com/fonts/using/using-variable-fonts.html",
      "closes_gap": "variable-fonts",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-03"
    },
    {
      "feature": "Variable font discovery in font list",
      "app_behavior": "In the font dropdown (Control/Properties panel) allow typing 'variable' or a dedicated marker/icon to surface only active variable fonts so the axis-slider UI is discoverable.",
      "primitive_domain": "font-management.browsing",
      "source_url": "https://helpx.adobe.com/indesign/using/using-fonts.html",
      "closes_gap": "variable-fonts",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-04"
    },
    {
      "feature": "OpenType ligature/alternate toggles",
      "app_behavior": "Character panel flyout > OpenType submenu with toggles for Standard Ligatures, Discretionary Ligatures, Contextual Alternates, Stylistic Alternates, Titling Alternates, and Swash; applied as run-level feature flags.",
      "primitive_domain": "typography.opentype-features",
      "source_url": "https://support.fontspring.com/hc/en-us/articles/10243482085019-How-to-Access-Opentype-Features-in-InDesign",
      "closes_gap": "opentype-features",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-05"
    },
    {
      "feature": "OpenType Stylistic Sets picker",
      "app_behavior": "Stylistic Sets control letting the user enable one or more of up to 20 named stylistic sets (ss01-ss20) per font; sets a font supports fewer than 20 show only available entries.",
      "primitive_domain": "typography.opentype-features",
      "source_url": "https://support.fontspring.com/hc/en-us/articles/10243482085019-How-to-Access-Opentype-Features-in-InDesign",
      "closes_gap": "opentype-features",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-06"
    },
    {
      "feature": "OpenType Fractions and Ordinals",
      "app_behavior": "OpenType submenu options Fractions (stacks numerals into proper fractions) and Ordinals; applied to selected text where the font has the feature.",
      "primitive_domain": "typography.opentype-features",
      "source_url": "https://support.fontspring.com/hc/en-us/articles/10243482085019-How-to-Access-Opentype-Features-in-InDesign",
      "closes_gap": "opentype-features",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-07"
    },
    {
      "feature": "Unavailable-feature bracket indicator",
      "app_behavior": "OpenType feature entries the current font does not support are rendered in square brackets (e.g. [Fractions]) so the user sees which features are inert for the selected font.",
      "primitive_domain": "typography.opentype-features",
      "source_url": "https://support.fontspring.com/hc/en-us/articles/10243482085019-How-to-Access-Opentype-Features-in-InDesign",
      "closes_gap": "opentype-features",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-08"
    },
    {
      "feature": "OpenType figure/number styles",
      "app_behavior": "OpenType submenu figure-style options: Tabular Lining, Proportional Lining, Proportional Oldstyle, Tabular Oldstyle, plus Slashed Zero, applied as mutually-exclusive numeral rendering per run.",
      "primitive_domain": "typography.opentype-features",
      "source_url": "https://helpx.adobe.com/indesign/using/using-fonts.html",
      "closes_gap": "opentype-features",
      "verification": "UNVERIFIED",
      "id": "SFR-REPASS-indesign-fonts-09"
    },
    {
      "feature": "OpenType-SVG color font rendering",
      "app_behavior": "Render OpenType-SVG fonts where a single glyph carries multiple colors and gradients baked into the font; treat as a font, not artwork, so text remains editable.",
      "primitive_domain": "typography.color-fonts",
      "source_url": "https://helpx.adobe.com/fonts/using/ot-svg-color-fonts.html",
      "closes_gap": "ot-svg",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-10"
    },
    {
      "feature": "Emoji font composite glyphs",
      "app_behavior": "For OpenType-SVG emoji fonts (e.g. EmojiOne), support composing glyphs from multiple base glyphs, including building country flags and changing skin-tone modifiers on people/body-part glyphs.",
      "primitive_domain": "typography.color-fonts",
      "source_url": "https://helpx.adobe.com/indesign/using/using-fonts.html",
      "closes_gap": "ot-svg",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-11"
    },
    {
      "feature": "OT-SVG font list marker",
      "app_behavior": "Flag OpenType-SVG (color/emoji) fonts with a distinct icon in the font list so they are visually distinguishable from monochrome fonts.",
      "primitive_domain": "font-management.browsing",
      "source_url": "https://helpx.adobe.com/indesign/using/using-fonts.html",
      "closes_gap": "ot-svg",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-12"
    },
    {
      "feature": "Glyphs panel color/alternate access",
      "app_behavior": "Glyphs panel (Type > Glyphs or Window > Type & Tables > Glyphs) to select specific color/emoji glyphs and glyph alternates from an OpenType-SVG or feature-rich font.",
      "primitive_domain": "typography.glyph-selection",
      "source_url": "https://helpx.adobe.com/indesign/using/using-fonts.html",
      "closes_gap": "ot-svg",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-13"
    },
    {
      "feature": "Document Fonts folder auto-install",
      "app_behavior": "Fonts placed in a 'Document Fonts' folder co-located with the .indd file are temporarily installed on document open and uninstalled on close; scoped to that document only, not other documents.",
      "primitive_domain": "font-management.install-scope",
      "source_url": "https://helpx.adobe.com/indesign/desktop/fonts/install-and-activate-fonts.html",
      "closes_gap": "document-fonts",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-14"
    },
    {
      "feature": "Document-font PostScript supersede rule",
      "app_behavior": "A Document Fonts font supersedes any installed font with the same PostScript name, but only within that document's scope, so per-document font pinning is deterministic.",
      "primitive_domain": "font-management.install-scope",
      "source_url": "https://helpx.adobe.com/indesign/desktop/fonts/install-and-activate-fonts.html",
      "closes_gap": "document-fonts",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-15"
    },
    {
      "feature": "Application-only Fonts folder",
      "app_behavior": "Support an application-level Fonts folder inside the app install directory whose fonts are available only to the app (not the OS), as a distinct install scope from Document Fonts and OS-installed fonts.",
      "primitive_domain": "font-management.install-scope",
      "source_url": "https://helpx.adobe.com/indesign/desktop/fonts/install-and-activate-fonts.html",
      "closes_gap": "document-fonts",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-16"
    },
    {
      "feature": "Package generates Document Fonts folder",
      "app_behavior": "The Package command collects all document fonts into a generated Document Fonts folder alongside the packaged document for hand-off/relocation.",
      "primitive_domain": "font-management.packaging",
      "source_url": "https://helpx.adobe.com/indesign/desktop/fonts/install-and-activate-fonts.html",
      "closes_gap": "document-fonts",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-17"
    },
    {
      "feature": "Auto-activate cloud fonts preference",
      "app_behavior": "Preferences > File Handling toggle 'Auto-activate Adobe Fonts' that silently activates missing cloud-synced fonts in the background when a document opens; no interruption if all missing fonts resolve.",
      "primitive_domain": "font-management.activation",
      "source_url": "https://helpx.adobe.com/indesign/desktop/fonts/install-and-activate-fonts.html",
      "closes_gap": "font-activation",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-18"
    },
    {
      "feature": "In-app font marketplace (Find More)",
      "app_behavior": "A 'Find More' tab in the font list browses the full cloud font library inline; clicking the cloud icon next to a font activates it instantly for use across all apps without leaving the app.",
      "primitive_domain": "font-management.activation",
      "source_url": "https://helpx.adobe.com/indesign/desktop/fonts/install-and-activate-fonts.html",
      "closes_gap": "font-activation",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-19"
    },
    {
      "feature": "Font classification filter",
      "app_behavior": "Filter the font list by classification: Serif, Slab Serif, Sans Serif, Script, Blackletter, Monospace, Handwritten, Decorative.",
      "primitive_domain": "font-management.browsing",
      "source_url": "https://helpx.adobe.com/be_en/indesign/using/using-fonts.html",
      "closes_gap": "font-browsing",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-20"
    },
    {
      "feature": "Favorite fonts star filter",
      "app_behavior": "Per-font star toggle (appears on hover) to mark favorites, plus a top-level star filter to show only starred fonts.",
      "primitive_domain": "font-management.browsing",
      "source_url": "https://helpx.adobe.com/be_en/indesign/using/using-fonts.html",
      "closes_gap": "font-browsing",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-21"
    },
    {
      "feature": "Similar-fonts visual filter",
      "app_behavior": "Similarity filter that finds and lists fonts visually similar to the currently selected font; mutually exclusive with other filters.",
      "primitive_domain": "font-management.browsing",
      "source_url": "https://helpx.adobe.com/be_en/indesign/using/using-fonts.html",
      "closes_gap": "font-browsing",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-22"
    },
    {
      "feature": "Activated-cloud-fonts filter",
      "app_behavior": "Toggle to restrict the font list to only activated cloud (Adobe Fonts) fonts vs. locally installed fonts.",
      "primitive_domain": "font-management.browsing",
      "source_url": "https://helpx.adobe.com/be_en/indesign/using/using-fonts.html",
      "closes_gap": "font-browsing",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-23"
    },
    {
      "feature": "Composite fonts editor (per-script mixing)",
      "app_behavior": "Composite Fonts editor assigning distinct fonts to predefined Unicode character ranges Kanji, Kana, Punctuation, Symbols, Latin (Roman), and Numbers, saved as a named composite font usable like a single font; gated to CJK/Japanese feature set.",
      "primitive_domain": "typography.cjk-composite",
      "source_url": "https://helpx.adobe.com/indesign/using/composing-cjk-characters.html",
      "closes_gap": "composite-fonts",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-indesign-fonts-24"
    }
  ]
}
```

### [SFR-REPASS.affinity-option-depth] Affinity V2 dialog option depth (brush / soft proof / performance / PDF import)

```json
{
  "rows": [
    {
      "feature": "Brush editor General tab core parameters",
      "app_behavior": "Brush Editor > General tab exposes Size (default stroke width), Hardness (edge softness %), Spacing (distance between nozzle points), Flow (rate colour builds up), Accumulation (deviation in opacity/visibility of stroke), Shape (nozzle diameter), Rotation (nozzle draw angle), Blend Mode, and Associated Tool (auto-selected tool when brush chosen)",
      "primitive_domain": "raster-brush-engine",
      "source_url": "https://affinity.help/photo2/en-US.lproj/pages/Painting/pixel_modify.html",
      "closes_gap": "brush-parameter-depth",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-01"
    },
    {
      "feature": "Brush editor Dynamics tab pressure/velocity ramps",
      "app_behavior": "Brush Editor > Dynamics tab drives Size, Opacity, Flow and Rotation via per-parameter Pressure and Velocity ramp curves, plus Scatter X (horizontal position deviation), Scatter Y (vertical position deviation), and Hue/Saturation/Luminosity jitter that vary brush colour per nozzle stamp",
      "primitive_domain": "raster-brush-engine",
      "source_url": "https://affinity.help/photo2/en-US.lproj/pages/Painting/pixel_modify.html",
      "closes_gap": "brush-dynamics-ramps",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-02"
    },
    {
      "feature": "Brush editor Sub Brushes",
      "app_behavior": "Brush Editor > Sub Brushes tab layers additional brushes onto the main stroke with Drawing (sub-brush position relative to main brush), Blending (how sub-brush blends with main brush), and Sync size / Sync spacing checkboxes to inherit the main brush's size and nozzle spacing",
      "primitive_domain": "raster-brush-engine",
      "source_url": "https://affinity.help/photo2/en-US.lproj/pages/Painting/pixel_modify.html",
      "closes_gap": "brush-sub-brush-compositing",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-03"
    },
    {
      "feature": "Brush editor Nozzle and Texture management",
      "app_behavior": "Brush Editor > Texture tab manages Brush Nozzles (Add / Remove nozzle images per preset) with per-nozzle ramp controllers and an Interpolate option for smoother tips; Base Texture supports Set Texture / Remove / Invert, a Mode of None / Nozzle / Final, and Scale to size the texture",
      "primitive_domain": "raster-brush-engine",
      "source_url": "https://affinity.help/photo2/en-US.lproj/pages/Painting/pixel_modify.html",
      "closes_gap": "brush-nozzle-texture",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-04"
    },
    {
      "feature": "Brush editor Wet Edges toggle",
      "app_behavior": "Brush Editor > General exposes a Wet Edges option setting the default wet-edge behaviour of the brush (colour pooling toward stroke edges), stored per brush preset rather than only as a live tool toggle",
      "primitive_domain": "raster-brush-engine",
      "source_url": "https://affinity.help/photo2/en-US.lproj/pages/Painting/pixel_modify.html",
      "closes_gap": "brush-wet-edges",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-05"
    },
    {
      "feature": "Soft Proof adjustment target ICC profile",
      "app_behavior": "Soft Proof adjustment layer > Proof Profile menu selects the target output colour profile to preview against (cycle with up/down arrow keys); behaves as an adjustment layer that must be hidden/removed before export or it bakes into output",
      "primitive_domain": "color-management-proofing",
      "source_url": "https://affinity.help/publisher2/en-US.lproj/pages/Adjustments/adjustment_softProof.html",
      "closes_gap": "soft-proof-target-profile",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-06"
    },
    {
      "feature": "Soft Proof rendering intent selector",
      "app_behavior": "Soft Proof adjustment > Rendering Intent pop-up menu sets the visual purpose (colour mapping method) applied when previewing the proof profile; the setting exists as a distinct control per adjustment layer",
      "primitive_domain": "color-management-proofing",
      "source_url": "https://affinity.help/publisher2/en-US.lproj/pages/Adjustments/adjustment_softProof.html",
      "closes_gap": "soft-proof-rendering-intent",
      "verification": "UNVERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-07"
    },
    {
      "feature": "Soft Proof black point compensation",
      "app_behavior": "Soft Proof adjustment > Black Point Compensation checkbox (default on) adjusts the design's black point to honour contrast within the current proof profile; when off, black point is not adjusted and image contrast may not be honoured",
      "primitive_domain": "color-management-proofing",
      "source_url": "https://affinity.help/publisher2/en-US.lproj/pages/Adjustments/adjustment_softProof.html",
      "closes_gap": "soft-proof-bpc",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-08"
    },
    {
      "feature": "Soft Proof gamut check overlay",
      "app_behavior": "Soft Proof adjustment > Gamut Check checkbox renders RGB colours that have no CMYK equivalent in the target profile as flat gray, highlighting out-of-gamut regions before print/convert",
      "primitive_domain": "color-management-proofing",
      "source_url": "https://affinity.help/publisher2/en-US.lproj/pages/Adjustments/adjustment_softProof.html",
      "closes_gap": "soft-proof-gamut-overlay",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-09"
    },
    {
      "feature": "Performance preferences RAM usage limit",
      "app_behavior": "Preferences > Performance > RAM Usage Limit sets the memory ceiling the app may consume to optimise performance for the current project size",
      "primitive_domain": "engine-performance-config",
      "source_url": "https://affinity.help/photo2/en-US.lproj/pages/Workspace/preferences.html",
      "closes_gap": "perf-ram-limit",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-10"
    },
    {
      "feature": "Performance preferences renderer selection",
      "app_behavior": "Preferences > Performance > Renderer selects the rendering device: Default (primary graphics card), a named graphics adapter, or WARP (Windows Advanced Rasterization Platform) as a software-rasterizer fallback for troubleshooting standard-renderer problems",
      "primitive_domain": "engine-performance-config",
      "source_url": "https://affinity.help/photo2/en-US.lproj/pages/Workspace/preferences.html",
      "closes_gap": "perf-renderer-warp",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-11"
    },
    {
      "feature": "Performance preferences hardware acceleration mode",
      "app_behavior": "Preferences > Performance > Display chooses the compute/hardware acceleration backend: Metal, OpenGL, OpenGL (Basic), or Software acceleration; separate Enable Metal / Enable OpenCL compute-acceleration toggles boost some task performance when a compatible GPU is present",
      "primitive_domain": "engine-performance-config",
      "source_url": "https://affinity.help/photo2/en-US.lproj/pages/Workspace/preferences.html",
      "closes_gap": "perf-hardware-accel",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-12"
    },
    {
      "feature": "Performance preferences Retina rendering quality",
      "app_behavior": "Preferences > Performance > Retina Rendering offers Automatic (Best) (balanced), Low Quality (Fastest) for max performance, and High Quality (Slowest) for superior fidelity on high-DPI displays",
      "primitive_domain": "engine-performance-config",
      "source_url": "https://affinity.help/photo2/en-US.lproj/pages/Workspace/preferences.html",
      "closes_gap": "perf-retina-quality",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-13"
    },
    {
      "feature": "Performance preferences view quality and gradient handling",
      "app_behavior": "Preferences > Performance exposes View Quality (image display quality during edits), Dither Gradients (toggle to speed up gradient performance), and Use Precise Clipping (clipping-accuracy vs performance option)",
      "primitive_domain": "engine-performance-config",
      "source_url": "https://affinity.help/photo2/en-US.lproj/pages/Workspace/preferences.html",
      "closes_gap": "perf-view-quality",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-14"
    },
    {
      "feature": "Performance preferences undo, recovery and disk warnings",
      "app_behavior": "Preferences > Performance sets Undo Limit (history depth accessible), File Recovery Interval (auto-save interval for temp data of open documents), and Disk Usage Warning At (threshold for disk-usage warnings)",
      "primitive_domain": "engine-performance-config",
      "source_url": "https://affinity.help/photo2/en-US.lproj/pages/Workspace/preferences.html",
      "closes_gap": "perf-undo-recovery",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-15"
    },
    {
      "feature": "PDF import page range selection",
      "app_behavior": "PDF import dialog > Load all pages / Load pages lets you import every page or a specific page-number range from a multi-page PDF into the document",
      "primitive_domain": "pdf-import-pipeline",
      "source_url": "https://affinity.help/publisher2/en-US.lproj/pages/GetStarted/importPDF.html",
      "closes_gap": "pdf-page-range",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-16"
    },
    {
      "feature": "PDF import DPI with estimate",
      "app_behavior": "PDF import dialog > DPI sets the resolution used for the imported document; the Estimate option reads and applies the PDF file's own resolution",
      "primitive_domain": "pdf-import-pipeline",
      "source_url": "https://affinity.help/publisher2/en-US.lproj/pages/GetStarted/importPDF.html",
      "closes_gap": "pdf-dpi-estimate",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-17"
    },
    {
      "feature": "PDF import colour space with estimate",
      "app_behavior": "PDF import dialog > Color space sets the document colour space (e.g. RGB or CMYK) applied to PDF contents; the Estimate option senses and adopts the PDF file's native colour space",
      "primitive_domain": "pdf-import-pipeline",
      "source_url": "https://affinity.help/publisher2/en-US.lproj/pages/GetStarted/importPDF.html",
      "closes_gap": "pdf-colorspace-estimate",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-18"
    },
    {
      "feature": "PDF import editable-text vs fidelity tradeoff",
      "app_behavior": "PDF import dialog > Favor editable text over fidelity keeps imported text more editable at the expense of exact design reproduction, versus preserving accurate layout fidelity",
      "primitive_domain": "pdf-import-pipeline",
      "source_url": "https://affinity.help/publisher2/en-US.lproj/pages/GetStarted/importPDF.html",
      "closes_gap": "pdf-text-fidelity-tradeoff",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-19"
    },
    {
      "feature": "PDF import text-frame grouping",
      "app_behavior": "PDF import dialog > Group lines of text into text frames merges separate imported text lines into single text frames to aid text flow and reflow after import",
      "primitive_domain": "pdf-import-pipeline",
      "source_url": "https://affinity.help/publisher2/en-US.lproj/pages/GetStarted/importPDF.html",
      "closes_gap": "pdf-text-grouping",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-20"
    },
    {
      "feature": "PDF import missing-font substitution",
      "app_behavior": "PDF import dialog > Replace missing fonts, when checked, substitutes fonts absent on the system with a suggested replacement family/style or one you choose; leaving it unchecked defers substitution so fonts can be swapped later via the Font Manager",
      "primitive_domain": "pdf-import-pipeline",
      "source_url": "https://affinity.help/publisher2/en-US.lproj/pages/GetStarted/importPDF.html",
      "closes_gap": "pdf-missing-font-substitution",
      "verification": "VERIFIED",
      "id": "SFR-REPASS-affinity-option-depth-21"
    }
  ]
}
```

### [SFR-REPASS.ps-formats-hdr] Photoshop native AVIF/JXL + HDR/gain-map export

```json
{
  "rows": [
    {
      "feature": "Native AVIF open/import",
      "app_behavior": "File > Open reads .avif natively (no plugin) in Photoshop desktop v26.8; decodes 8/10/12-bit and HDR AVIF including gain-map base rendition",
      "primitive_domain": "raster-codec/decode",
      "source_url": "https://www.cgchannel.com/2025/06/adobe-releases-photoshop-26-8/",
      "verification": "VERIFIED",
      "closes_gap": "PS AVIF open gap",
      "id": "SFR-REPASS-ps-formats-hdr-01"
    },
    {
      "feature": "Native AVIF save/export",
      "app_behavior": "Save a Copy > AVIF writes native AVIF with lossy and lossless compression, wider color depths, and native HDR; replaces prior HEIF-only export path",
      "primitive_domain": "raster-codec/encode",
      "source_url": "https://www.cgchannel.com/2025/06/adobe-releases-photoshop-26-8/",
      "verification": "VERIFIED",
      "closes_gap": "PS AVIF save gap",
      "id": "SFR-REPASS-ps-formats-hdr-02"
    },
    {
      "feature": "AVIF gain-map HDR encoding",
      "app_behavior": "AVIF save embeds an HDR gain map (SDR base + gain map + metadata) so the image adapts across SDR/HDR displays; gain map added when 'Maximize Compatibility' is enabled via Camera Raw path",
      "primitive_domain": "hdr/gain-map-encode",
      "source_url": "https://helpx.adobe.com/camera-raw/using/gain-map.html",
      "verification": "VERIFIED",
      "closes_gap": "PS HDR gain-map gap",
      "id": "SFR-REPASS-ps-formats-hdr-03"
    },
    {
      "feature": "Skip-gain-map HDR encode option",
      "app_behavior": "Native HDR support lets the encoder emit a smaller gain map OR skip the gain map entirely when file size matters more than cross-display fidelity",
      "primitive_domain": "hdr/gain-map-encode",
      "source_url": "https://www.cgchannel.com/2025/06/adobe-releases-photoshop-26-8/",
      "verification": "VERIFIED",
      "closes_gap": "PS HDR gain-map gap",
      "id": "SFR-REPASS-ps-formats-hdr-04"
    },
    {
      "feature": "JPEG XL (JXL) open/import",
      "app_behavior": "File > Open reads .jxl natively in v26.8 with wide color depth and HDR support",
      "primitive_domain": "raster-codec/decode",
      "source_url": "https://www.cgchannel.com/2025/06/adobe-releases-photoshop-26-8/",
      "verification": "VERIFIED",
      "closes_gap": "PS JXL open gap",
      "id": "SFR-REPASS-ps-formats-hdr-05"
    },
    {
      "feature": "JPEG XL (JXL) save/export",
      "app_behavior": "Save a Copy > JPEG XL writes native JXL with lossy/lossless compression and HDR; supports high bit-depth (reported 8-32bit) export from v26.8",
      "primitive_domain": "raster-codec/encode",
      "source_url": "https://gregbenzphotography.com/photoshop/photoshop-now-natively-supports-avif-for-50-smaller-files-than-jpg/",
      "verification": "UNVERIFIED",
      "closes_gap": "PS JXL save gap",
      "id": "SFR-REPASS-ps-formats-hdr-06"
    },
    {
      "feature": "JXL/TIF gain-map encoding",
      "app_behavior": "Gain-map HDR metadata can also be written into JPEG XL and TIFF containers (in addition to AVIF), preserving HDR base+gain-map for compatible viewers",
      "primitive_domain": "hdr/gain-map-encode",
      "source_url": "https://gregbenzphotography.com/hdr-setup-and-troubleshooting/",
      "verification": "UNVERIFIED",
      "closes_gap": "PS HDR gain-map container gap",
      "id": "SFR-REPASS-ps-formats-hdr-07"
    },
    {
      "feature": "Precise Color Management for HDR Display (Tech Preview)",
      "app_behavior": "Preferences > Technology Previews > 'Precise color management for HDR Display' toggle (requires restart) drives clip-free 32-bit HDR on-canvas display; on by default where supported",
      "primitive_domain": "hdr/display-colormgmt",
      "source_url": "https://helpx.adobe.com/photoshop/kb/hdr-display-support.html",
      "verification": "VERIFIED",
      "closes_gap": "PS HDR display preview gap",
      "id": "SFR-REPASS-ps-formats-hdr-08"
    },
    {
      "feature": "HDR display OS/hardware gating",
      "app_behavior": "HDR display preview requires macOS or Windows 11 with an HDR-capable display; the preference is greyed out on Windows 10",
      "primitive_domain": "hdr/display-colormgmt",
      "source_url": "https://helpx.adobe.com/photoshop/kb/hdr-display-support.html",
      "verification": "VERIFIED",
      "closes_gap": "PS HDR display preview gap",
      "id": "SFR-REPASS-ps-formats-hdr-09"
    },
    {
      "feature": "Gain-map HDR view via Camera Raw HDR mode",
      "app_behavior": "Photoshop shows only the SDR base for gain-map files unless opened through Adobe Camera Raw with HDR mode on; ACR gear default 'Enable HDR editing by default for HDR photos' auto-enables it",
      "primitive_domain": "hdr/import-pipeline",
      "source_url": "https://helpx.adobe.com/camera-raw/using/hdr-output.html",
      "verification": "VERIFIED",
      "closes_gap": "PS HDR ingest gap",
      "id": "SFR-REPASS-ps-formats-hdr-10"
    },
    {
      "feature": "32-bit HDR editing tool coverage",
      "app_behavior": "PS 2025 extended ~20 retouch/editing tools (Spot Healing Brush, Remove, etc.) to operate in 32-bit HDR documents for full HDR editing rather than SDR-only tooling",
      "primitive_domain": "hdr/edit-ops",
      "source_url": "https://gregbenzphotography.com/hdr-setup-and-troubleshooting/",
      "verification": "UNVERIFIED",
      "closes_gap": "PS HDR edit-op coverage gap",
      "id": "SFR-REPASS-ps-formats-hdr-11"
    },
    {
      "feature": "Export Color Lookup Tables (3D LUT authoring)",
      "app_behavior": "File > Export > Color Lookup Tables writes 3DLUT formats 3DL, CUBE, CSP and an RGB device-link from an RGB document; from a CMYK document it exports an ICC CMYK device-link profile",
      "primitive_domain": "color/lut-export",
      "source_url": "https://helpx.adobe.com/photoshop/using/export-color-lookup-tables.html",
      "verification": "VERIFIED",
      "closes_gap": "PS 3D LUT export/authoring gap",
      "id": "SFR-REPASS-ps-formats-hdr-12"
    },
    {
      "feature": "Color Lookup adjustment layer (3D LUT apply)",
      "app_behavior": "Layers panel > Color Lookup adjustment layer applies a LUT via the '3DLUT File' option in Properties (load .3dl/.cube/.csp); option requires RGB Color mode",
      "primitive_domain": "color/lut-apply",
      "source_url": "https://helpx.adobe.com/photoshop/how-to/edit-photo-color-lookup-adjustment.html",
      "verification": "VERIFIED",
      "closes_gap": "PS LUT apply gap",
      "id": "SFR-REPASS-ps-formats-hdr-13"
    },
    {
      "feature": "Merge to HDR Pro (32-bit HDR build)",
      "app_behavior": "File > Automate > Merge to HDR Pro combines bracketed exposures into a 32-bit HDR document with tone-mapping/HDR Toning controls before conversion to 16/8-bit",
      "primitive_domain": "hdr/merge-tonemap",
      "source_url": "https://helpx.adobe.com/photoshop/using/high-dynamic-range-images.html",
      "verification": "UNVERIFIED",
      "closes_gap": "PS HDR merge gap",
      "id": "SFR-REPASS-ps-formats-hdr-14"
    },
    {
      "feature": "HDR Toning / bit-depth conversion",
      "app_behavior": "Image > Mode 32>16/8-bit and Image > Adjustments > HDR Toning apply tone mapping when down-converting HDR to LDR, preserving highlight/shadow detail",
      "primitive_domain": "hdr/merge-tonemap",
      "source_url": "https://helpx.adobe.com/photoshop/using/adjusting-hdr-exposure-toning.html",
      "verification": "UNVERIFIED",
      "closes_gap": "PS HDR tone-map gap",
      "id": "SFR-REPASS-ps-formats-hdr-15"
    }
  ]
}
```
