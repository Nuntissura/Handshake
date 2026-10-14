---
file_id: STUDIO-APP-INSPECT-FIGMA
file_kind: research_note
updated_at: 2026-10-14T16:15:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-APP-INSPECT (read-only static inspector)
scope: Figma desktop 126.10.7 static inspection for the .fig format and interop-fig; prose context, not authority
legend: "[V] read in an installed file at the cited path | [U] UNVERIFIED inference | [NI] NOT_INSPECTED"
---

<topic id="method" status="active" summary="What was read, what was not, and why">

## Method and limits

- Nothing was launched or executed. Inputs were read as bytes, and strings were extracted with `python -I` scripts kept in the session scratch directory (`asar_extract.py`, `strs.py`, `kiwi_parse.py`).
- Install: `%LOCALAPPDATA%/Figma/app-126.10.7` (Figma.exe FileVersion 126.10.7) [V].
- `resources/app.asar` was copied to `Handshake_Artifacts/WP-KERNEL-STUDIO/app-inspect/figma/asar-src/` and unpacked there to `app/` (100 files) [V]. `build.json`: channel `stable`, bundle id `com.figma.Desktop` [V].
- The desktop shell does NOT contain the .fig reader or writer. Searches of `main.js`, `desktop_shell.js`, `lazy/external/*.js`, `desktop_rust.node`, `bindings.node` and `FigmaAgent/figma_agent.exe` for `fig-kiwi`, `nodeChanges`, `zstd` and `kiwi` found no format code [V]. The editor is loaded from figma.com.
- Source of the format facts below: the V8 code cache of the editor's public webpack bundles, `%APPDATA%/Figma/DesktopProfile/v43/Code Cache/js/`. These entries hold compiled script and string constants only, with no user documents. Three entries were copied to `app-inspect/figma/codecache/`:
  - `9b51719368527443_0`: `https://www.figma.com/webpack-artifacts/assets/7621-90db6066689112de.min.js.br`
  - `23f2edc08b808f93_0`: `.../5519-dc0559c4467060ce.min.js.br`
  - `9a879d52ddbb9c14_0`: `.../figma_app-95b50598e6046255.min.js.br`
- NOT read: the HTTP `Cache/Cache_Data`, which holds the editor WASM but also user content (thumbnails and images), and cookies, Local Storage and IndexedDB. The container codec (header, chunk table and compression) almost certainly lives in the editor WASM. Every container-layout claim below therefore stays [U].
- Clean-room: identifiers, enum values and field numbers only. The extracted schema text, 102,499 chars, is kept machine-local at `app-inspect/figma/fig-kiwi-schema.23f2.txt`, with a parsed index at `fig-kiwi-schema.23f2.json`. It is not copied into Studio sources.

</topic>

<topic id="preludes" status="active" summary="8-byte preludes and file extensions">

## Preludes and extensions

- The editor bundle 7621 holds a constants module (webpack id 923181) with exactly three 8-byte strings: `fig-diff`, `fig-kiwi`, `fig-jam.` [V: `codecache/9b51719368527443_0.strings.txt` line 30050].
- `fig-deck` occurs in NO cached bundle, and none of `fig-make`, `fig-site`, `fig-buzz` or `fig-slides` occurs either [V: grep over all 286 code-cache entries]. Absence from the code cache is weak evidence, because only executed functions are cached.
- The desktop shell's "save local copy" path writes buffers it receives from the editor. It accepts only the extensions `.fig .jam .deck .jpg .pdf .png .svg .json .lottie` [V: `app/main.js`, extension allow-list next to `writeFileSync`]. So Slides files are saved as `.deck`, but their prelude bytes are [U].
- The shell knows about checkpoint URLs of the form `https://s3-alpha.figma.com/checkpoints/.../*.fig` [V: 5519 strings]. The server's own storage unit is also called a `.fig`.

</topic>

<topic id="kiwi-schema" status="active" summary="Current Fig kiwi schema: root, node, blob, enums">

## Kiwi schema embedded in the editor (package `Fig`)

- The 5519 bundle carries the whole text schema as one string constant beginning `package Fig;` [V: `codecache/23f2edc08b808f93_0` offset 12192308]. It has 653 definitions: 213 enums, 30 structs and 410 messages [V: `kiwi_parse.py`].
- The same bundle references `require("kiwi-schema").ByteBuffer` [V]: the editor uses the evanw `kiwi-schema` npm package. This matches the wire facts kiwi.rs cites.
- Native types used anywhere in the schema are exactly `bool byte int uint float string int64 uint64` [V]. 165 fields carry `[deprecated]` [V].
- Root `message Message` has 49 fields [V]:

  | Field | Number |
  |---|---|
  | `type: MessageType` | 1 |
  | `sessionID` | 2 |
  | `ackID` | 3 |
  | `nodeChanges: NodeChange[]` | 4 |
  | `userChanges: UserChange[]` | 5 |
  | `blobs: Blob[]` | 6 |
  | `fileVersion: uint` | 18 |
  | `blobBaseIndex: uint` | 30 |
  | `encodedOffsetsIndex: EncodedOffsetsIndex` | 34 |
  | `nodeChangesMetadata` (deprecated; holds `blobsFieldOffset`) | 17 |
  | `localUndoStack: Message[]` | 22 |
  | `localRedoStack: Message[]` | 23 |

  Paste fields (`pasteID`, `pasteFileKey`, ...) are also present.
- `MessageType`: `JOIN_START`=0, `NODE_CHANGES`=1, ..., `DIFF`=13, ..., 25 members up to `BUZZ_APPROVAL_CHANGE`=24 [V].
- `message NodeChange` has 616 fields, and the highest field number is 616 [V]. Census-relevant fields: `guid: GUID`=1, `phase: NodePhase`=2, `parentIndex: ParentIndex`=3, `type: NodeType`=4, `name`=5, `visible`=6, `opacity`=8, `blendMode`=9, `size: Vector`=11, `transform: Matrix`=12, `mask`=16, `cornerRadius`=20, `strokeWeight`=26, `fillPaints: Paint[]`=38, `strokePaints`=39, `effects: Effect[]`=43, `vectorData`=48, `fillGeometry: Path[]`=51, `strokeGeometry`=52, `symbolData`=113, `derivedSymbolData`=125, `overriddenSymbolID`=143, `componentKey`=216 [V].
- Structs [V]:
  - `GUID{uint sessionID; uint localID}`
  - `ParentIndex{GUID guid; string position}`
  - `Vector{float x,y}`
  - `Matrix{m00 m01 m02 m10 m11 m12}`
  - `Color{float r,g,b,a}`
  - `Blob{byte[] bytes}`
- `NodeType`, 65 members [V]:
  - 0-10: `NONE`=0, `DOCUMENT`=1, `CANVAS`=2, `GROUP`=3, `FRAME`=4, `BOOLEAN_OPERATION`=5, `VECTOR`=6, `STAR`=7, `LINE`=8, `ELLIPSE`=9, `RECTANGLE`=10
  - 11-20: `REGULAR_POLYGON`=11, `ROUNDED_RECTANGLE`=12, `TEXT`=13, `SLICE`=14, `SYMBOL`=15, `INSTANCE`=16, `STICKY`=17, `SHAPE_WITH_TEXT`=18, `CONNECTOR`=19, `CODE_BLOCK`=20
  - 21-30: `WIDGET`=21, `STAMP`=22, `MEDIA`=23, `HIGHLIGHT`=24, `SECTION`=25, `SECTION_OVERLAY`=26, `WASHI_TAPE`=27, `VARIABLE`=28, `TABLE`=29, `TABLE_CELL`=30
  - 31-40: `VARIABLE_SET`=31, `SLIDE`=32, `ASSISTED_LAYOUT`=33, `INTERACTIVE_SLIDE_ELEMENT`=34, `VARIABLE_OVERRIDE`=35, `MODULE`=36, `SLIDE_GRID`=37, `SLIDE_ROW`=38, `RESPONSIVE_SET`=39, `CODE_COMPONENT`=40
  - 41-50: `TEXT_PATH`=41, `CODE_INSTANCE`=42, `CODE_LIBRARY`=43, `CODE_FILE`=44, `CODE_LAYER`=45, `BRUSH`=46, `MANAGED_STRING`=47, `TRANSFORM`=48, `CMS_RICH_TEXT`=49, `REPEATER`=50
  - 51-64: `JSX`=51, `EMBEDDED_PROTOTYPE`=52, `REACT_FIBER`=53, `RESPONSIVE_NODE_SET`=54, `WEBPAGE`=55, `KEYFRAME`=56, `KEYFRAME_TRACK`=57, `ANIMATION_PRESET_INSTANCE`=58, `CODE_EMBED`=59, `BINARY_FILE`=60, `SPEC_BLOCK`=61, `TOOL_INSTANCE`=62, `CUSTOM_EFFECT_INSTANCE`=63, `NATIVE_CODE_LAYER_INSTANCE`=64
- `BlendMode`, 20 members [V]:
  - 0-9: `PASS_THROUGH`=0, `NORMAL`=1, `DARKEN`=2, `MULTIPLY`=3, `LINEAR_BURN`=4, `COLOR_BURN`=5, `LIGHTEN`=6, `SCREEN`=7, `LINEAR_DODGE`=8, `COLOR_DODGE`=9
  - 10-19: `OVERLAY`=10, `SOFT_LIGHT`=11, `HARD_LIGHT`=12, `DIFFERENCE`=13, `EXCLUSION`=14, `HUE`=15, `SATURATION`=16, `COLOR`=17, `LUMINOSITY`=18, `UNSET`=19
- Other enums [V]:
  - `PaintType`: `SOLID`=0, `GRADIENT_LINEAR`..`GRADIENT_DIAMOND`=1-4, `IMAGE`=5, `EMOJI`=6, `VIDEO`=7, `PATTERN`=8, `NOISE`=9, `CUSTOM`=10
  - `EffectType`: `INNER_SHADOW`=0, `DROP_SHADOW`=1, `FOREGROUND_BLUR`=2, `BACKGROUND_BLUR`=3, `REPEAT`=4, `SYMMETRY`=5, `GRAIN`=6, `NOISE`=7, `GLASS`=8, `CUSTOM`=9
  - `ImageScaleMode`: `STRETCH`=0, `FIT`=1, `FILL`=2, `TILE`=3
  - `EditorType`: `DESIGN`=0, `WHITEBOARD`=1, `SLIDES`=2, `DEV_HANDOFF`=3, `SITES`=4, `COOPER`=5, `ILLUSTRATION`=6, `FIGMAKE`=7, `FIGSPEC`=8
  - `ImageType` (export): `PNG`=0, `JPEG`=1, `SVG`=2, `PDF`=3, `MP4`=4, `GIF`=5, `WEBM`=6, `LOTTIE`=7, `DOTLOTTIE`=8

</topic>

<topic id="blobs-images" status="active" summary="How images, vector networks and paths are stored">

## Blobs and images

- Exactly six fields reference blobs [V]:
  - `Message.blobs: Blob[]` (6)
  - `DiffPayload.blobs: Blob[]` (2)
  - `Image.dataBlob: uint` (3)
  - `VectorData.vectorNetworkBlob: uint` (1)
  - `Path.commandsBlob: uint` (2)
  - `Glyph.commandsBlob: uint` (1)
- The `uint` fields are indices into the message's `blobs` array. `Message.blobBaseIndex` (30) is presumably an offset for split or incremental messages [U].
- `message Image{byte[] hash=1; string name=2; uint dataBlob=3}` [V]. `Paint` has `image`=8, `imageThumbnail`=9, `animatedImage`=16, `imageScaleMode`=10, `originalImageWidth`/`Height`=19/20, `thumbHash`=25 [V].
- A paint therefore references an image by content hash. In ZIP `.fig` files the bytes are expected at `images/<hex(hash)>` [U: community notes; no fixture]. The `dataBlob` path carries inline bytes, as in paste [U]. The hash algorithm and length (SHA-1, 20 bytes) are [U].
- `Video{byte[] hash; string s3Url}` [V]: video bytes are not inline.
- `EncodedOffsetsIndex{nodeChangesFieldOffset, nodeChangesFieldLength, blobsFieldOffset, GUIDAndEncodedOffset[] nodeChangeOffsets}` [V]. This is a random-access index into an encoded message, used for lazy loading [U].
- `DiffPayload{nodeChanges, blobs, diffChunks, diffBasis, basisParentNodeChanges, parentNodeChanges, diffType}` [V]. It is the likely root of `fig-diff` files [U].

</topic>

<topic id="container-unverified" status="active" summary="Container layout items that stay UNVERIFIED">

## Container items NOT resolvable from installed files

All of these live in the editor WASM, which was not inspected:
- the header version field (u32 LE at offset 8) and its current value
- the chunk length width and endianness
- chunk order (schema, then message)
- per-chunk compression (raw DEFLATE vs zstd)
- `meta.json` / `thumbnail.png` / `canvas.fig` part names (none of these strings is in the code cache)

To resolve them, either:
- (a) use the Operator-exported fixtures (D17, still pending), the cheapest and decisive option; or
- (b) let the Operator authorise reading the cached editor WASM entry from `Cache_Data`, keyed by its static.figma.com URL only. Mixed user content makes that an Operator decision; this lane did not do it.

`Message.fileVersion` (18) is a second, in-message version number [V]. How it relates to the header u32 is [U].

</topic>

<topic id="interop-fig-mismatches" status="active" summary="interop-fig vs installed-app evidence, with file:line">

## interop-fig comparison

Compared tree: `wt-studio-assets/handshake-studio-assets` HEAD 241df1f; interop-fig last touched at e7de0ae.

| # | Location | Code says | App evidence | Verdict |
|---|---|---|---|---|
| 1 | `container.rs:42-79` (`FigKind`, `from_prelude`) | Classifies `fig-kiwi`, `fig-jam.`, `fig-deck` | Constants module has `fig-kiwi`, `fig-jam.`, `fig-diff` | MISMATCH: `fig-diff` is unclassified, so it is `Unknown`; a bare `fig-diff` is `NotFig` (`container.rs:344-346`). Add a `Diff` kind, or document it as a deliberate Unknown. |
| 2 | `container.rs:48-49,57,66` | `fig-deck` = Deck | Not in any cached bundle; the shell saves `.deck` | UNVERIFIED: keep, but label `fig-deck` as community-only until a `.deck` fixture exists. |
| 3 | `container.rs:198-199,242,253-256`; `version_map.rs:11` | u32 LE version at 8; u32 LE chunk lengths; `OBSERVED_VERSIONS` empty | No container code in installed files | NO EVIDENCE either way; stays UNVERIFIED (D17 fixtures). |
| 4 | `kiwi.rs:401-432` (census) | Root `Message`, array `nodeChanges` of message `NodeChange`; `guid` struct with uint `sessionID`/`localID`; enum `type` | Message.nodeChanges=4 (`NodeChange[]`), NodeChange.guid=1 (struct GUID{uint sessionID; uint localID}), type=4 (enum NodeType) | MATCH. The comment at `kiwi.rs:391-394` ("community-documented, UNVERIFIED") can cite this note: verified against the 2026 app schema, not against a file. |
| 5 | `kiwi.rs:487` (`DOCUMENT` = root) | Root ids are nodes typed `DOCUMENT` | `NodeType.DOCUMENT=1` | MATCH. |
| 6 | `kiwi.rs:42-51` (natives) | 8 natives | Schema uses exactly those 8 | MATCH. |
| 7 | `container.rs:122-123` (Limits) | `max_definitions` 4096, `max_fields` 1024 | 653 definitions; NodeChange 616 fields | OK. NodeChange already uses 60% of `max_fields` and grows with the product; keep a test pinned near the real count. |
| 8 | `kiwi.rs:577-601,644-659` | Schema = first raw-DEFLATE chunk of 0/1; message = zstd by magic, else DEFLATE | No evidence | UNVERIFIED, unchanged. |
| 9 | `kiwi.rs:446-449` | Skips every root field except `nodeChanges` | `blobs`=6, `blobBaseIndex`=30, `fileVersion`=18 exist | Not a bug for the census. Suggest recording `fileVersion` (cheap) as a second version witness in `Census`/`FigImport`. Any later geometry or image decoder must resolve blob indices with `blobBaseIndex` [U]. |
| 10 | `import.rs:129-130` (`images/` loss code) | `images/<hash>` parts preserved opaque | `Image.hash` bytes + `dataBlob` | Consistent. Hash-to-name mapping [U]. |
| 11 | `container.rs:43-51` | Kinds Design/Jam/Deck | `EditorType` has 9 editors (Sites, Make, Buzz/`COOPER`, ...) | Informational: other editors may share `fig-kiwi` and differ only inside the message [U]. |

</topic>
