---
file_id: affinity-format-research-20261016
file_kind: research_note
updated_at: 2026-10-16
lane: SMB-AFFINITY-FORMAT-RESEARCH
scope: parity rows AF-02, AF-06, AF-07 (affinity app); prose context, not authority
legend: "[V] verified at the cited source on 2026-10-16 | [U] UNVERIFIED | [I] verified locally in installed-app note app-inspect-affinity.md"
---

<topic id="basis" status="active" summary="Policy, matrix rows and sources checked">

## Basis

- Matrix `research/parity-matrix-20261016.json` app `affinity`: AF-02 native documents MISSING (no reader or writer); AF-06 DWG/DXF MISSING; AF-07 SVG/EPS PARTIAL (SVG VALIDATED, EPS MISSING). Interop section: native open/save MISSING; DWG/DXF/EPS open/save MISSING.
- START_LOG policy: permissive licences only (MIT/Apache-2.0/BSD/Zlib/ISC, CC0); GPL/LGPL/MPL sources are not read (2026-10-14 entries). 2026-10-11 entry: no decompilation of vendor binaries; public specs plus black-box testing as a licensed user (EU Directive 2009/24/EC art. 5(3) and 6 as read by the lead, not legal advice). 2026-10-14 operator instruction: installed Affinity (Canva.Affinity 3.2.3.4646) may be inspected and used; no launch without an Operator-attended session (D5).
- Local container facts: `research/app-inspect-affinity.md` (KA header `00 FF 4B 41`, version u16 11 for V2 and 12 for V3, type `Prsn`, `#FT4` stream directory, `doc.dat` zstd, `Thmb` PNG).

Sources checked (all fetched 2026-10-16):

- https://github.com/VMDevCpp/afread (MIT, 9 commits, 27 stars)
- https://github.com/NickBeeuwsaert/AFDesignLoad (MIT, resource extraction only)
- https://crates.io/api/v1/crates/afphoto and https://github.com/Asset-Hoard/afphoto (MIT, 0.1.0, 2026-06-15, 74 downloads)
- https://gitlab.com/inkscape/extras/extension-afdesign (GPL-2.0-or-later, 54 commits, created 2024-05-30)
- https://summerofcode.withgoogle.com/archive/2024/projects/sR7nHXTJ (GSoC 2024 Inkscape afdesign import)
- https://github.com/storytold/photocraft/pull/1607, https://github.com/storytold/vectorcraft/pull/667, https://github.com/storytold/designcraft/pull/291, https://github.com/storytold/photocraft/issues/1605
- https://affinity.help/designer2/en-US.lproj/pages/Appendix/fileformat.html (Designer 2 format table), https://www.canva.com/newsroom/news/all-new-affinity/, https://www.canva.com/help/upload-formats-requirements-variantb/ (Canva lists `.af` plus legacy `.afdesign/.afphoto/.afpub`)
- https://crates.io/api/v1/crates/dxf, /dxf-tools-rs, /acadrust, /stet; https://github.com/ixmilia/dxf-rs, https://github.com/hakanaktt/acadrust, https://github.com/AndyCappDev/stet
- Studio code: `git grep -i eps origin/codex/handshake-studio-assets -- crates/hsk-studio-interop-vector` after `git fetch`.

</topic>

<topic id="q1-native-findings" status="active" summary="AF-02 findings: documentation, readers, licences, maturity, interchange">

## Q1 findings: native Affinity files

**Public documentation.** None found. Forum and Esri threads say the format is undocumented and Serif/Canva do not intend to publish it [V: search results for Esri idea thread and VectorStyler forum, secondary]. No official spec was located.

**Versions.** V1/V2 per-app files (`.afphoto`, `.afdesign`, `.afpub`) and the V3 unified `.af` all share the KA container; the document type is `Prsn` in every sample, version u16 11 (V2) or 12 (V3) [I]. Canva's upload help lists `.af` and the three legacy extensions as accepted [V]. `PersonaTagForLegacy` in libpersona maps legacy files to studios [I].

**Open-source readers.**

| Project | Licence | Reads | Maturity |
|---|---|---|---|
| VMDevCpp/afread | MIT | `.afdesign/.afphoto/.afpub` to JSON (C++) | 9 commits, 27 stars; v3 `.af` support not stated; README has no format description [V] |
| NickBeeuwsaert/AFDesignLoad | MIT | extracts resources only | author states parsing beyond that is not done [V via search] |
| Asset-Hoard/afphoto crate | MIT | KA header detect + embedded PNG thumbnail only | 0.1.0, 2026-06-15, 74 downloads [V] |
| Inkscape extension-afdesign | GPL-2.0-or-later | `.afdesign` to Inkscape import | 54 commits, created 2024-05-30, GSoC 2024; EXCLUDED, source must not be read [V] |
| storytold PhotoCraft/VectorCraft/DesignCraft | repo licence NOT stated on the pages read [U] | layers, artboards, vectors, text, images; container `#FAT/#FT2/#FT3/#FT4`, zlib/zstd, CRC, `doc.dat` tagged object stream; PR claims 176 public documents decode with matching CRC and no leftover bytes | claims layout re-described from afread with no code translated; the project already records the PhotoCraft clean-room claim as disputed (START_LOG 2026-10-11, `research/craft-case-study.md`); read support only, "writing Affinity files is not implemented", no output ever opened in Affinity (issue 1605) [V] |

**Takeaways.** The container is largely solved publicly (header, directory, per-stream zstd/zlib, CRC). The hard part is the `doc.dat` tagged object graph: node types, property tags, per-type versions, and the vector/pixel/text payloads. Only the disputed-provenance storytold work claims to go that far; no permissively licensed, trustworthy, complete reader exists. No reader found can write.

**Interchange.** Affinity Designer 2 exports PSD, PDF (text rasterised on export in that version), SVG and EPS, and imports PSD, PDF, SVG, EPS, AI, DXF, DWG [V, Designer 2 table]. The installed v3 DLL set (`libpsd`, `libpdfimport`, `libidmlimport`, `libdwgimport`) is consistent with that [I]. Canva's v3 announcement says IDML is supported, direction unstated [V, marketing]. The v3 export list was NOT confirmed from official help and needs a one-time check in the installed app [U].

</topic>

<topic id="q1-recommendation" status="active" summary="AF-02 recommendation: (c) both, interchange first">

## Q1 recommendation: option (c), sequenced

1. **Slice 1, interchange import (b).** An Affinity "import" is: user exports PSD (layers preserved, Studio's own PSD parser) or SVG/PDF from Affinity, and Studio opens that through existing interop-psd, interop-vector and the PDF path. No new format risk. Add a typed `affinity_native_use_export` refusal for `.af*` files that names the exact export path, detected by the KA magic. Optionally read the `Thmb` PNG as a preview only (KA header facts are already ours).
2. **Slice 2, KA container reader (container only).** Build the container layer (header, directory, per-stream decompression, CRC check, caps for bombs) clean-room from our own hexdump facts in `app-inspect-affinity.md` plus the MIT afread description; zstd via a permissive crate (ruzstd is MIT per the PR text [V]; licence gate to confirm before adoption). Output: named streams plus typed `object_graph_not_decoded` loss. This also makes `.afstudio` and `.propcol` readable.
3. **Slice 3+, black-box object-graph study (a), Operator-attended.** Create controlled documents in the installed Affinity (one feature per file: rectangle, bezier, fill, gradient, text, raster layer, group, mask, artboard) in an Operator-attended session (D5), and decode `doc.dat` by differential analysis, recording facts, never vendor code. Read-only first; each decoded node type becomes a proposal with typed losses. Validate against Affinity's own PSD/SVG export of the same file (oracle) rather than against third-party readers.
4. **Write support: defer.** No one has verified a written file opens in Affinity; writing risks corrupt-looking documents. Interchange export (PSD/SVG/PDF) is the supported route. Revisit only after read-only parity and an Operator-attended open test.

Clean-room rules for the lane: builders may read MIT afread; must not read the Inkscape extension (GPL); must not read storytold code beyond prose until their licence is recorded, and then only as behaviour reference (consistent with the PhotoCraft decision); no decompilation or disassembly of Affinity binaries; no vendor strings or tables copied into product code; every decoded field gets a provenance note naming the controlled test file.

Legal and interop considerations (factual, not legal advice): the project reading of EU Directive 2009/24/EC art. 5(3) allows observing, studying and testing a licensed program's functioning; art. 6 allows decompilation only for interoperability and not to build a substantially similar program. Black-box study of files we create ourselves avoids art. 6 entirely. Affinity's EULA terms on reverse engineering were NOT read [U]. Jurisdiction-specific counsel review is the Operator's, outside the assistant's scope.

</topic>

<topic id="q2-dwg-dxf" status="active" summary="AF-06: DXF via dxf crate, DWG has no permissive route">

## Q2: DWG / DXF (AF-06)

| Option | Licence | Recency | Verdict |
|---|---|---|---|
| `dxf` (ixmilia/dxf-rs) | MIT | 0.6.2, updated 2026-10-03, 308,099 downloads [V] | USE for DXF read (and write if needed) |
| `dxf-tools-rs` | MIT | 0.1.3, 2026-01-20, 432 downloads [V] | reject: immature |
| `acadrust` (DXF + DWG R13-R2018+) | MPL-2.0 | 0.6.4, 2026-10-11, 34,929 downloads [V] | EXCLUDED by policy (MPL, source unreadable) |
| LibreDWG | GPL-3.0-or-later | n/a | EXCLUDED |
| ODA Drawings SDK | proprietary, paid | n/a | EXCLUDED |
| `ezdwg` | licence not shown, Python | n/a | not Rust; UNVERIFIED |

No permissive DWG reader found. Open Design Alliance has historically published a free "Open Design Specification for .dwg files" [U, not re-verified here]; a clean-room Rust reader from that public spec is possible but large (bit-coded streams, Reed-Solomon, LZ77 variants); defer.

**Recommendation.** Slice 1: DXF import (ASCII and binary) through `dxf` into a vector proposal (LINE, ARC, CIRCLE, LWPOLYLINE, POLYLINE, SPLINE, ELLIPSE, TEXT/MTEXT as outlines or typed loss, INSERT/BLOCK flattening, layers to groups, units from `$INSUNITS`), with caps and typed losses for HATCH, DIMENSION, 3D entities, proxy objects. DWG: typed `dwg_unavailable` refusal that tells the user to export DXF from the CAD tool; Affinity itself opens DWG and exports DXF [V, Designer 2 table], a documented fallback. Verify the `dxf` dependency closure and unsafe use through the licence gate and steward before adoption. Place in the interop-vector crate or a sibling `hsk-studio-interop-cad` (builder lane decision; a sibling keeps the dependency optional).

</topic>

<topic id="q3-eps" status="active" summary="AF-07 EPS: current crate state and options">

## Q3: EPS import (AF-07)

**What Studio already does** (origin/codex/handshake-studio-assets, crates/hsk-studio-interop-vector):

- `ai_adapter.rs:19` detects the `%!PS-Adobe` header as legacy AI or EPS (`identify_ai`).
- `tests/ai_import.rs:100` asserts that `%!PS-Adobe-3.0 EPSF` returns `VectorError::NativeEditabilityBlocked`.
- `DESCRIPTOR native_gap`: "EPS and PostScript are not decoded; PDF decoding is a Press provider". No EPS exporter exists. `import_ai_with(.., &dyn PdfDrawingProvider, ..)` and `import_ai_document` already turn a PDF drawing into layers, swatches, text and ICC records, so any route that yields PDF bytes or a `PdfDrawing` reuses them.

**Options.**

| Option | Licence / state | Notes |
|---|---|---|
| `stet` pure-Rust PostScript Level 3 interpreter, renders PS/EPS to raster or PDF | Apache-2.0 OR MIT; 0.8.7, updated 2026-10-15, 220,375 downloads; pre-1.0, MSRV 1.92 may rise; embeds OFL fonts, CC0 ICC, BSD-3 CMaps; uses hayro crates, a tiny-skia fork, moxcms [V] | Honors `%%BoundingBox` for EPSF. The page does not describe a sandbox, unsafe use or binary-EPS (DOS header) handling [U]. PostScript is Turing-complete: needs timeout and VM cap, file-operator denial and worker isolation, same pattern as the AV1 worker decision. |
| Own minimal PostScript subset interpreter | own code | Practical for Illustrator/Inkscape/Affinity-written EPS (path ops, fills, strokes, clip, shfill gradients, images) but unbounded for hand-written PS; large effort. |
| DOS EPS binary header (magic `C5 D0 D3 C6`) preview: embedded TIFF or WMF | public Adobe EPSF spec | Raster fallback only, no vector editing. TIFF via `hsk-studio-interop-tiff`. Header layout from the Adobe EPSF 3.0 spec [U, not re-fetched]. |
| Embedded PDF/AI private data in EPS | n/a | Illustrator EPS carries `%AI*` private comments, not a PDF [U]; do not plan on it. |
| Ghostscript | AGPL | EXCLUDED |

**Recommendation.** Two stages. (1) DOS-header and DSC parser (`%%BoundingBox`, `%%HiResBoundingBox`, `%%BeginPreview`, `%%DocumentFonts`) giving a bounded raster preview import and typed `eps_vector_not_decoded`. (2) Vector import by running `stet` EPS to PDF in an isolated, resource-capped worker (feature default-off, trusted-input-first like AV1, hard timeout, VM cap, no file or network operators, process-abort containment), then feed the PDF to the existing `import_ai_with` path; losses `eps_via_pdf_conversion`, `eps_text_as_outlines` where fonts are not embedded. Export EPS: write DSC-conformant EPSF 3.0 from the same path model as the existing SVG export (paths, solid and gradient fills, strokes, bounding box, optional TIFF preview) with a typed loss list; no interpreter needed. Run `stet` through the licence gate and steward (its closure includes a tiny-skia fork) before adoption, and fuzz the worker boundary.

</topic>

<topic id="risks" status="active" summary="Risks and mitigations">

## Risks

- **Provenance contamination (AF-02):** reading storytold or GPL Inkscape code taints the reusable asset. Mitigation: read-list in the builder brief, provenance note per decoded field, MIT afread only.
- **Format drift:** v3 is new (3.2.3.4646); object graph versions change per release. Mitigation: per-type version check, refuse unknown versions with typed errors, keep a controlled-file corpus per app version.
- **Silent corruption on write:** unverified writer. Mitigation: no native write until an Operator-attended open test passes.
- **Hostile input:** zstd bombs, CRC-valid garbage, PostScript infinite loops or file operators, DXF huge entity counts. Mitigation: caps, timeouts, worker isolation, fuzz targets.
- **Attended-session dependency (D5):** black-box study needs Affinity launched in the foreground. Mitigation: batch the file-authoring script into the next Operator session; Affinity scripting through its JSLib is NOT confirmed as a way to automate this [U].
- **Dependency churn:** stet pre-1.0 and MSRV bumps; dxf crate single-maintainer. Mitigation: pin exact versions, gate in CI, keep adapters behind port traits.
- **v3 export set unverified:** Affinity v3 export formats come from the v2 help table. Mitigation: first builder step is a one-time check in the installed app.

</topic>

<topic id="lane-outline" status="proposed" summary="Proposed builder lanes">

## Proposed builder lanes (AF-02, AF-06, AF-07)

1. **SMB-AFFINITY-INTEROP-1 (AF-02 slice 1):** KA-magic detection, `Thmb` preview, typed refusal naming the PSD/SVG/PDF export route; descriptor `native_gap` text; test with the installed fixture documents as test data (not committed).
2. **SMB-AFFINITY-KA-CONTAINER (AF-02 slice 2):** new `hsk-studio-interop-affinity` crate: header, `#FT4` directory, streams, zstd/zlib, CRC, caps, fuzz; outputs streams plus loss; also `.afstudio` and `.propcol`. Licence-gate the zstd crate.
3. **SMB-AFFINITY-ORACLE-SESSION (D5 item):** authored controlled-file script and one attended session in installed Affinity; saves `.af` plus PSD/SVG/PDF export of each; decode notes recorded as facts under `research/`.
4. **SMB-AFFINITY-GRAPH-1..n (AF-02 slice 3):** decode node types incrementally (artboard/group/rect/bezier/fill, then text, raster tiles, masks), validate against the oracle export.
5. **SMB-INTEROP-DXF (AF-06):** `dxf` 0.6.2 pinned, entity-to-vector mapping, units, layers, typed losses, `dwg_unavailable` refusal; deferred follow-up SMB-DWG-CLEANROOM only if the Operator wants native DWG.
6. **SMB-INTEROP-EPS-1 (AF-07):** DSC/DOS-header parser, TIFF preview, typed refusal replaced by raster import; **SMB-INTEROP-EPS-2:** isolated `stet` worker to PDF to `import_ai_with`; **SMB-INTEROP-EPS-EXPORT:** EPSF 3.0 writer from the SVG-export path model.

Ordering: 1, 5, EPS-1, EPS-export, 2, EPS-2, 3, 4. Operator-gated decisions: the D5 session and the licence gate for new crates.

</topic>
