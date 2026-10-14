---
file_id: STUDIO-APP-INSPECT-AFFINITY
file_kind: research_note
updated_at: 2026-10-14T16:25:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-APP-INSPECT (read-only static inspector)
scope: Affinity 3.2.3.4646 (Canva.Affinity MSIX) document format and unified-app architecture; prose context, not authority
legend: "[V] read in an installed file at the cited path | [U] UNVERIFIED inference | [P] already in prep parser affinity_ka.py"
---

<topic id="method" status="active" summary="Access and inputs">

## Method

- Install root: `C:/Program Files/WindowsApps/Canva.Affinity_3.2.3.4646_x64__8a0j1tnjnt4a4` (`$AF` below).
- Access: listing and file reads work, `cd` into `App` is denied, and no ACL was changed [V].
- Nothing was launched. Bytes were read with `od` and `python -I`.
- Inputs: vendor-shipped files only:
  - `AppxManifest.xml`
  - `App/Resources/Affinity/*.afstudio`, `*.propcol`, `*.lproj/studios.strings`
  - `App/Resources/JSLib/*.js` (BSD-3-Clause, "Copyright (c) 2026, Canva Pty Ltd" [V: `JSLib/application.js` header])
  - `App/Resources/JSLib/tests/*.af|*.afdesign|*.affinity` (Canva test fixtures)
  - string tables of `Thumbnail.dll`, `libpersona.dll`, `Serif.Affinity.dll`, `Serif.Interop.Persona.dll`
- No Operator documents were opened.
- Prior work: prep corpus `.GOV/reference/studio_app_feature_research/_tools/greenroom/affinity_ka.py` already parses the KA container for `.propcol`/`.afstudio` [P]. Its harvest outputs (`installed_exports/affinity/offline/*.json`) are listed in `_greenroom_20260903/installed_exports_manifest.json` but are not present on disk. This note adds document-file facts and corrections.

</topic>

<topic id="file-types" status="active" summary="Registered file types">

## File associations (`AppxManifest.xml`) [V]

- Native types: `.af`, `.afdesign`, `.afphoto`, `.afpub`, `.aftemplate`, `.afbook`, `.afpackage`. These share one thumbnail handler: COM class `233d80c6-b8aa-4b99-88da-ee96551d4ca1`, `App/Thumbnail.dll`.
- Resource types: `.afassets`, `.afbrushes`, `.affont`, `.afluts`, `.afmacros`, `.afpalette`, `.afstyles`, `.afstudio`, `.afshort`.
- Import types include `.psd/.psb`, `.ai`, `.eps`, `.pdf`, `.idml`, `.dwg/.dxf`, `.fh10/.fh11`, `.exr/.hdr`, `.jxl`, `.heic` and the RAW family (`.cr2/.cr3/.nef/.arw/.dng/.raf/.orf/.pef/.iiq/.3fr/...`).
- Studio relevance: Affinity opens PSD, RAW and PDF directly, the interop set the Operator asked for. Implementation DLLs: `libpsd.dll`, `libpdfimport.dll`, `libidmlimport.dll`, `libdwgimport.dll`, `libheif.dll` [V: `App/` listing].

</topic>

<topic id="container" status="active" summary="KA container layout incl. document files">

## KA container: one format for documents, presets and studios

Header (little-endian) [V: hexdumps of the 10 test documents, 9 `.afstudio` and 19 `.propcol` files; layout matches [P]]:

| Offset | Field |
|---|---|
| 0x00 | magic `00 FF 4B 41` ("\0\xFFKA") |
| 0x04 | u16 format version |
| 0x06 | u16 flags |
| 0x08 | 4CC type, stored byte-reversed |
| 0x0C | `#Inf` |
| 0x10 | u64 offset of the `#FT4` directory |
| 0x18 | u64 end of the `#FT4` directory |
| 0x20 | u64 stored length of the first stream |
| 0x30 | u32 (see below) |
| 0x38 | u32 |
| 0x3C | u32 |
| 0x40 | `Prot` + u32 |
| 0x48 | `#Fil`, followed by the first stream |

- **Document type**: `.af`, `.afdesign` and `.affinity` all carry type `Prsn` (stored `nsrP`) [V]. The file type does not depend on the extension or on the authoring persona or studio: one document type for all.
- **Version**: u16 at 0x04 = 11 in V2-era `.afdesign` fixtures and 12 in V3 `.af`/`.affinity` fixtures [V]. `.propcol` files use 11 or 12, and all `.afstudio` files use 12 [V].
- **Flags** at 0x06 vary (0, 4, 8, 0x200, 0x0304); their meaning is [U].
- **Correction to [P]**: 0x18 is NOT the total file size for documents. For example, `A4Empty.afdesign` has 7514 at 0x18 but is 9443 bytes. It equals the end of the `#FT4` directory, and after it a `0xFFFFFFFF` marker and a `Thmb` block holding a PNG thumbnail follow [V]. For thumbnail-less `.afstudio` files 0x18 does equal the file size [V].
- **Correction to [P]**: 0x30, labelled "container uid" in `affinity_ka.py`, decodes as a Unix timestamp in every sample, and the same value repeats inside `#FT4` [V]. Examples: 2023-08-30 and 2024-01-04 for the V2 `.afdesign` fixtures, 2025-03-21 for `.affinity`, 2026-01-07 for `.af`, 2026-01-22 for `01 Vector.afstudio`. Timestamp semantics [U, strong].
- **Streams**: the `#FT4` directory lists named streams. Each record holds a stored offset (0x48 for the first stream), an inflated size, a stored size, a 32-bit check value, and a u16-length name [V: bytes; field roles U]. Names seen:
  - `doc.dat`: the main document object stream, in every document
  - `d/<hex>`: data streams, many of them in `EmbDocWithPDF.afdesign` and `CyanisePixel.affinity` (raster tiles and embedded files [U])
  - `studio`: the stream name in `.afstudio` files
- **Compression**: per stream. `doc.dat` in every document and `studio` in every `.afstudio` starts with a zstd frame (`28 B5 2F FD`) [V]. The `adjustments.propcol` stream is a stored, uncompressed `00 FF 4B 53` ("KS") object stream [V].
- **Inner object stream** (KS) [P]: typed property records with byte-reversed 4CC tags, object references with inline or reused type definitions, and per-type u16 versions. This is a self-describing tagged object graph, comparable to a Kiwi or protobuf message with per-type schema versions.
- **Thumbnail**: `Thumbnail.dll` contains `\0\xFFKA`, `#Inf` and `Prot` plus libpng [V]. The shell thumbnail is read from the container's `Thmb` PNG without decoding the document [U, strong].

</topic>

<topic id="unified-model" status="active" summary="One document model across Vector/Pixel/Layout studios">

## Unified app: one document model, studios as tool and panel sets

Facts [V]:
- Affinity 3 names its modes "Studios". `App/Resources/Affinity/` ships nine `.afstudio` files: `01 Vector`, `02 Pixel`, `03 Layout`, `04 Slice`, `05 Canva AI`, `06 Retouching`, `07 Colour Grading`, `08 Typography`, `09 Compositing`. Each is a KA container of type `prss` (stored `ssrp`) holding one zstd `studio` stream.
- `en-US.lproj/studios.strings` describes three of them:
  - Vector: vector design tools and panels for graphic design and illustration.
  - Pixel: raster photo-editing tools, filters, adjustments and panels.
  - Layout: typography and layout tools for publishing.
  These are the former Designer, Photo and Publisher apps re-expressed as studios over one app (wording paraphrased).
- `libpersona.dll` (359 MB, the engine) exports `PersonaController` (366 refs), `PersonaTag`, `SetPersonaTag`/`GetPersonaTag`/`AssignPersonaTag`, `PersonaTagForLegacy`, `GetLastEditingStudio`/`SetLastEditingStudio`, `ImmediatePersona` and `EmfPersona`.
- `Serif.Affinity.dll` (WPF shell) has `Studio`/`StudioPage`/`StudioBook`, `ApplyStudioPresetCommand`, `StudioPresetsManagerDialog`, `PersonasToolbar` and `ToneMappingPersona`.

Reading [U]:
- The document stores a tag of the last editing studio. `PersonaTagForLegacy` maps V1/V2 per-app files (`.afphoto`/`.afdesign`/`.afpub`) onto studios.
- Studio switching swaps tools, panels and context bars over the same document; it does not convert the document.
- Special-purpose modes stay personas inside a studio: Tone Mapping, Export/Slice, Liquify and Develop.

The scripting DOM is one node tree for every discipline [V: `JSLib/nodes.js` class hierarchy]:
- `Node` → `LogicalNode` (`DocumentNode`, `GroupNode`, `ContainerNode`, `SpreadNode`) and `PhysicalNode`.
- `PhysicalNode` → `RasterNode` → `EnclosureRasterNode` → `AdjustmentRasterNode`. Levels, Curves, Exposure, HSLShift, Threshold, Posterise, SelectiveColour, SplitToning, WhiteBalance, ToneCompression/Stretch, ... are adjustment nodes.
- `EnclosureRasterNode` → `FilterRasterNode`: live filters such as Gaussian/Box/Motion/Lens blur, Clarity, Denoise, HighPass, Halftone, Voronoi, ...
- Also under `PhysicalNode`: `PatternRasterNode`, `EmbeddedDocumentNode`, `ImageNode`, `VectorNode` (`PolyCurveNode`, `ShapeNode`), and text nodes (`ArtTextNode`, `FrameTextNode`, `CurvePathTextNode`, `ShapePathTextNode`, `TableTextNode`, `ShapeTextNode`).
- Node children are typed (`NodeChildType.Main` vs `NodeChildType.Enclosure`). Adjustments and live filters attach as "enclosure" children of the layer they affect, not as layer properties [V: `nodes.js` lines 461-489].

Other DOM facts [V]:
- Pixel format is a document-level property, `Document.format`, converted by command (`createConvertDocumentFormat`) [V: `document.js` 413-420]. Formats: `RGBA8`, `RGBA16`, `RGBAUF` (32-bit HDR), `IA8`, `IA16`, `CMYKA8`, `LABA16` [V: `JSLib/examples/setDocumentFormat.js`].
- Save forms: `save`, `saveAs` and `saveAsPackage(path, policy)` (the `.afpackage` = document + linked resources) [V: `document.js` 372-381].
- Blend-If is `BlendOptions{gamma, masterSourceLayerRanges: Spline, masterUnderlyingCompositionRanges: Spline}`. Ranges are curves, not Photoshop's 4-point ramps [V: `JSLib/blendmodeinterface.js` 42-76].
- Blend-mode names in the interop metadata, in string-heap order (ordinal values [U]) [V: `Serif.Interop.Persona.dll` offset ~51.64 MB]:
  - `Passthrough, Normal, Darken, Multiply, ColorBurn, LinearBurn, DarkerColor, Lighten, Screen, ColorDodge, LighterColor, Overlay, SoftLight, HardLight, VividLight, LinearLight, PinLight, HardMix`
  - `Difference, Exclusion, Subtract, Divide, Luminosity, Average, Negation, Reflect, Glow, ContrastInvert, Erase`
  - mask modes `MaskAdd, MaskSubract` (sic), `MaskIntersect, MaskXor`
  - `Unset`
  - `Hue`, `Saturation`, `Colour` and `Add` occur elsewhere in the heap.

Studio cohesion takeaways (lead to weigh):
- One container, one document type tag and one node graph serve all disciplines. The UI mode (studio) is metadata on the document plus a tool and panel preset file, never a format fork.
- Adjustments and filters are graph nodes attached to their target.
- Colour format is a document property with an explicit conversion command.
- Studio presets and user resources reuse the same container (KA) as documents, so one reader and one writer serve both.

</topic>

<topic id="open" status="active" summary="Open items">

## Open / not done

- Field roles in `#FT4` records and the header flags are [U]. Fixing them needs a byte diff of documents saved under controlled changes, in an Operator-attended session (CX-SAFE-002).
- The KS object stream of `doc.dat` was not decompressed here: no zstd library was used in this lane. `affinity_ka.py` can do it with a system `libzstd` [P]. It was not run, because it loads a native DLL via ctypes.
- Interop value for Studio: low priority. Affinity is not on the Operator's must-support format list (PSD/TIFF/RAW/PDF); this note serves the architecture question only.

</topic>
