## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Canvas-Typography-v2
- CREATED_AT: 2026-01-19T00:30:35.1208584Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- SPEC_TARGET_SHA1: cf2f5305fc8eec517d577d87365bd9c072a99b0f
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja190120260138
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Canvas-Typography-v2

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE (Master Spec provides sufficient normative requirements for Phase 1 font packs, font registry behavior, deterministic loading, and Tauri security posture).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE (no Flight Recorder requirements are specified in the scoped anchors for this WP).

### RED_TEAM_ADVISORY (security failure modes)
- CSP widening: weakening CSP beyond what is required for `asset:` / `http://asset.localhost` increases attack surface in WebView.
- Path traversal: font import or manifest handling that accepts arbitrary paths can escape `{APP_DATA}/fonts/**`.
- CSS injection: unsanitized font family names can inject CSS (e.g., quotes/semicolons/newlines) when used in styles or `FontFace`.
- Malicious font parsing: do not execute or trust metadata; treat font files as untrusted input; keep parsing minimal and fail-safe.

### PRIMITIVES (traits/structs/enums)
- Tauri/Rust:
  - Commands (required): `fonts_bootstrap_pack`, `fonts_rebuild_manifest`, `fonts_list`, `fonts_import`, `fonts_remove`
  - Data: `Manifest` (schemaVersion, generatedAt, packVersion, fonts[]), `ImportResult`
- Frontend:
  - Runtime font loader using `invoke("fonts_list")`, `convertFileSrc`, `FontFace`, and `document.fonts.ready`

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec provides explicit requirements for font runtime root, CSP/asset protocol scoping, bootstrapping Design Pack 40, import behavior, backend command set + manifest schema, deterministic loading steps, security constraints, and acceptance criteria/tests.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Anchors define concrete command names, manifest schema, file layout, and deterministic loading behavior sufficient for an implementable task packet.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.10.2 Canvas Typography & Font Registry
- CONTEXT_START_LINE: 49526
- CONTEXT_END_LINE: 49534
- CONTEXT_TOKEN: Design Pack 40
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 11.10.2 Canvas Typography & Font Registry

  1.  **Runtime Root:** All fonts must be served from `{APP_DATA}/fonts/`.
  2.  **CSP Policy:** Tauri `asset:` protocol MUST be restricted to the `{APP_DATA}/fonts/` directory.
  3.  **Bootstrap:** On first run, the "Design Pack 40" (see list in \\u00A710.6.1.4.1) must be copied from embedded resources (`app/src-tauri/resources/fonts/`) to `{APP_DATA}/fonts/bundled/`.
  4.  **Import UI:** The system settings or a dedicated Font Manager UI MUST provide an "Import Font" action.
      -   **Behavior:** Allows the user to select local `.ttf`, `.otf`, or `.woff2` files.
      -   **Backend:** Moves files to `{APP_DATA}/fonts/user/`, deduplicates by hash, and updates the `manifest.json`.
  5.  **Loading:** Use the `FontFace` API in the frontend to load fonts dynamically from the `asset:` URL.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 10.6.1 Font Packs + Canvas Typography Support Spec v0.1 (Licensing + Design Pack 40 inventory)
- CONTEXT_START_LINE: 42977
- CONTEXT_END_LINE: 43053
- CONTEXT_TOKEN: THIRD_PARTY_NOTICES
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 3. Licensing & compliance model (non-negotiable)

  ##### 3.1 Baseline principle
  Even \\u201cfree\\u201d fonts are licensed; shipping a font means shipping licensed software. Google Fonts explicitly states the most common license is **SIL OFL**, with some fonts under **Apache** or **Ubuntu Font License**, and that you can use and redistribute fonts under the license conditions.

  ##### 3.2 Packaging requirements
  When Handshake redistributes fonts (bundled pack), Handshake MUST:
  - Keep a **per-font license file** (e.g., `OFL.txt`, `LICENSE`, etc.)
  - Maintain a project-level **THIRD_PARTY_NOTICES** file listing:
    - font family name
    - license type (SPDX identifier if known, e.g. `OFL-1.1`)
    - source (Google Fonts page / upstream repo)
    - checksum (sha256) of shipped binary
  - If any font uses OFL Reserved Font Names (RFNs): do **not** modify the font files (format conversion, subsetting, rebuilding) unless the RFN rules are satisfied (rename) or explicit permission exists.
    - RFNs are a formal OFL concept and are declared in the OFL text / metadata.
    - If you do modify and redistribute, include original copyright statements, RFN declarations, and license text.

  Notes:
  - This spec assumes **no modification** of shipped fonts beyond \\u201ccopy as-is.\\u201d
  - Prefer shipping vendor-provided `.woff2` if available, otherwise ship `.ttf/.otf` unmodified.

  #### 4. Font pack inventory

  ##### 4.1 Design Pack 40 (default full pack)

  These are selected to match \\u201cmodern design studio + editorial + architectural annotation\\u201d patterns. Many are directly present in Typewolf\\u2019s curated \\u201cbest free Google Fonts\\u201d list (used as a popularity proxy for design usage).

  ###### Sans / UI / grotesk (20)
  1. Inter
  2. DM Sans
  3. Manrope
  4. Space Grotesk
  5. Work Sans
  6. IBM Plex Sans
  7. Plus Jakarta Sans
  8. Outfit
  9. Urbanist
  10. Montserrat
  11. Poppins
  12. Open Sans
  13. Source Sans 3
  14. Libre Franklin
  15. Fira Sans
  16. Karla
  17. Lato
  18. PT Sans
  19. Chivo
  20. Rubik

  ###### Serif / editorial (12)
  21. Playfair Display
  22. Lora
  23. Source Serif 4
  24. Spectral
  25. Cormorant
  26. Alegreya
  27. Libre Baskerville
  28. Eczar
  29. Fraunces
  30. Inknut Antiqua
  31. Merriweather
  32. BioRhyme

  ###### Mono / annotation (4)
  33. JetBrains Mono (OFL; upstream confirms OFL 1.1)
  34. Space Mono
  35. Inconsolata
  36. Archivo Narrow (use as condensed label font; not mono but often used similarly in diagrams)

  ###### \\u201cArchitectural handwriting / sketch\\u201d accents (4)
  37. Architects Daughter (architectural-note vibe)
  38. Syne (display)
  39. Proza Libre
  40. Alegreya Sans
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 10.6.1 Font Packs + Canvas Typography Support Spec v0.1 (Font Registry + determinism + security + Tauri config + acceptance)
- CONTEXT_START_LINE: 43124
- CONTEXT_END_LINE: 43279
- CONTEXT_TOKEN: fonts_rebuild_manifest
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 7. Backend (Rust/Tauri) specification

  ##### 7.1 Commands (required)

  1) `fonts_bootstrap_pack(pack_id?: string)`
  - Ensures runtime folders exist
  - Copies packaged fonts from `resources/fonts/<pack_id>` into `{APP_DATA}/fonts/bundled/`
  - Writes/updates `fonts_pack_version.json` to detect future upgrades

  2) `fonts_rebuild_manifest() -> Manifest`
  - Scans `{APP_DATA}/fonts/bundled/` and `{APP_DATA}/fonts/user/`
  - For each font file:
    - validate extension allowlist: `.ttf`, `.otf`, `.woff2` (optional `.woff`)
    - compute sha256
    - extract metadata: family, style, weight range, postscript name (best effort)
  - Writes `{APP_DATA}/fonts/cache/manifest.json`

  3) `fonts_list() -> Manifest`
  - Returns cached manifest (rebuild only if missing or invalid)

  4) `fonts_import(paths: string[]) -> ImportResult`
  - Copies user-selected files into `{APP_DATA}/fonts/user/`
  - Enforces size limit (configurable)
  - Deduplicates by sha256
  - Rebuilds manifest

  5) `fonts_remove(font_id: string)`
  - Removes from `user/` only
  - Rebuilds manifest

  ##### 7.2 Manifest schema

  ```json
  {
    "schemaVersion": 1,
    "generatedAt": "ISO-8601",
    "packVersion": "design-pack-40@1",
    "fonts": [
      {
        "id": "sha256:...",
        "family": "Inter",
        "style": "normal",
        "weight": { "type": "variable", "min": 100, "max": 900 },
        "source": "bundled|user",
        "format": "woff2|ttf|otf",
        "path": "absolute-device-path",
        "license": {
          "spdx": "OFL-1.1|Apache-2.0|UFL-1.0|UNKNOWN",
          "licenseFile": "absolute-device-path-or-null"
        }
      }
    ]
  }
  ```

  Validation rules:
  - `family` must be sanitized for CSS usage (see 8.3)
  - `path` must be under `{APP_DATA}/fonts/**` (reject otherwise)
  - `id` uniqueness enforced

  ##### 7.3 Security notes (backend)
  - If using `@tauri-apps/plugin-fs`, enforce strict scope globs and keep font IO in Rust commands anyway.
  - Do not expose the \\u201copen\\u201d endpoint to untrusted input without validation regex.

  #### 8. Frontend (React/WebView) specification

  ##### 8.1 Font URL conversion (required)
  Use `convertFileSrc(font.path)` to convert a device path into a WebView-loadable URL.

  Important: Tauri requires `asset:` and `http://asset.localhost` to be included in CSP when using `convertFileSrc()`.

  ##### 8.2 Deterministic loading (FontFace API)
  When entering Canvas (or any surface needing fonts):
  1) `manifest = await invoke("fonts_list")`
  2) For each font:
     - `url = convertFileSrc(font.path)`
     - `face = new FontFace(family, `url(${url})`, { style, weight })`
     - `await face.load(); document.fonts.add(face)`
  3) `await document.fonts.ready`
  4) Then render canvas text to avoid fallback/layout shift.

  ##### 8.3 Safe CSS + name handling
  To prevent CSS injection via font family names:
  - Allow only `[A-Za-z0-9 _-]` for display names
  - Strip/replace quotes, semicolons, newlines
  - Maintain an internal `cssFamily` name if the font\\u2019s true family contains unsafe characters.

  ##### 8.4 CSS generation options
  Option A (preferred):
  - Backend generates `{APP_DATA}/fonts/cache/fonts.css`
  - Frontend `<link rel="stylesheet" href={convertFileSrc(cssPath)} />`

  Option B:
  - Frontend injects a `<style>` tag at runtime (only if CSP permits)

  This spec recommends **FontFace API** as primary; CSS file generation is optional.

  #### 9. Tauri configuration requirements

  ##### 9.1 CSP
  Handshake must configure CSP so that:
  - `asset:` and `http://asset.localhost` are allowed where needed (fonts, styles if used)
  - `font-src` includes `asset:` and `http://asset.localhost`

  Tauri CSP is intentionally restrictive; do not weaken it broadly\\u2014add only what is required for fonts.

  ##### 9.2 Asset protocol
  Enable `assetProtocol` and scope it narrowly to the fonts directory under app data, not `**/*`.

  #### 10. Performance strategy

  Problems:
  - Bundling 40+ fonts increases app size.
  - Loading all fonts at runtime wastes memory and delays first draw.

  Required mitigations:
  - **Lazy register:** only load a font when it is selected in UI, plus a small default set.
  - Keep a small \\u201cCanvas default set\\u201d preloaded (e.g., Inter, Space Grotesk, JetBrains Mono).
  - Cache \\u201cloaded families\\u201d in-memory per session.

  Optional:
  - A \\u201cpack toggle\\u201d UI to install extras on demand (download or copy from resources).

  #### 11. Test plan

  ##### 11.1 Backend tests
  - Import a valid font -> appears in manifest, sha256 stable
  - Import duplicate -> dedup works
  - Import invalid file -> rejected
  - Path traversal attempt -> rejected
  - Manifest rebuild on missing cache works

  ##### 11.2 Frontend tests
  - Font selection changes canvas rendering
  - First render after loading uses correct font (no fallback flash)
  - Editing overlay preserves cursor/selection/IME
  - Export (PNG/SVG) uses the selected font and matches on-screen rendering (within tolerance)

  #### 12. Acceptance criteria

  1) Handshake ships with Design Pack 40 available offline.
  2) Canvas can render and edit text objects using those fonts.
  3) Users can import additional fonts without granting the UI arbitrary filesystem access.
  4) Font loading is deterministic (no \\u201crandom fallback\\u201d on first draw).
  5) Licensing artifacts are present (per-font license files + THIRD_PARTY_NOTICES).
  6) CSP and asset protocol scopes remain narrow and security-conscious.
  ```

