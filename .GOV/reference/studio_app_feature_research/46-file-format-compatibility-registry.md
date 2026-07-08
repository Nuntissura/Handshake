---
file_id: file-format-compatibility-registry
file_kind: source_distilled_file_format_compatibility_registry
topic_id: SFR-FILE-FORMAT-COMPATIBILITY-REGISTRY
title: File Format Compatibility Registry
status: draft
updated_at: '2026-07-05'
compatibility_record_count: 410
format_family_count: 38
native_format_record_count: 15
domain_format_record_count: 23
feature_format_record_count: 372
---

## [SFR-FILE-FORMAT-COMPATIBILITY-REGISTRY] File Format Compatibility Registry

<topic id="compatibility-coverage" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Coverage and policy for source-distilled file-format compatibility records.">

### [SFR-FILE-FORMAT-COMPATIBILITY-REGISTRY.coverage] Coverage

```yaml
coverage:
  distillation_status: source_distilled_file_format_compatibility_registry
  compatibility_record_count: 410
  native_format_record_count: 15
  domain_format_record_count: 23
  feature_format_record_count: 372
  format_family_count: 38
  policy:
    format_compatibility_rule: Do not invent a replacement interchange format for Studio parity scope.
    fixture_rule: Every import/export/round-trip claim needs representative fixtures and receipts.
    native_rule: Native source formats are compatibility targets with explicit unsupported-feature diagnostics.
    local_first_rule: Provider/cloud publishing is optional adapter behavior; local import/export fixtures remain primary.
  source_files:
    feature_rows:
      photoshop: 39-photoshop-source-distilled-feature-rows.md
      indesign: 40-indesign-source-distilled-feature-rows.md
      illustrator: 41-illustrator-source-distilled-feature-rows.md
      affinity: 42-affinity-source-distilled-feature-rows.md
      figma: 43-figma-source-distilled-feature-rows.md
    domain_ledgers:
      photoshop: 34-photoshop-source-distilled-domain-ledger.md
      indesign: 35-indesign-source-distilled-domain-ledger.md
      illustrator: 36-illustrator-source-distilled-domain-ledger.md
      affinity: 37-affinity-source-distilled-domain-ledger.md
      figma: 38-figma-source-distilled-domain-ledger.md
```

</topic>

<topic id="compatibility-records" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Machine-readable format-family matrix and compatibility records.">

### [SFR-FILE-FORMAT-COMPATIBILITY-REGISTRY.records] Compatibility Records

```yaml
format_family_matrix:
- format_id: format.afdesign
  format_labels:
  - AFDESIGN
  source_apps_present:
  - affinity
  support_by_app:
    affinity:
    - fixture_required
    - round_trip
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.afphoto
  format_labels:
  - AFPHOTO
  source_apps_present:
  - affinity
  support_by_app:
    affinity:
    - fixture_required
    - round_trip
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.afpub
  format_labels:
  - AFPUB
  source_apps_present:
  - affinity
  support_by_app:
    affinity:
    - fixture_required
    - round_trip
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.ai
  format_labels:
  - AI
  source_apps_present:
  - affinity
  - illustrator
  support_by_app:
    affinity:
    - fixture_required
    illustrator:
    - fixture_required
    - round_trip
  record_count: 3
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.ait
  format_labels:
  - AIT
  source_apps_present:
  - illustrator
  support_by_app:
    illustrator:
    - fixture_required
    - round_trip
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.buzz
  format_labels:
  - BUZZ local copy
  source_apps_present:
  - figma
  support_by_app:
    figma:
    - fixture_required
    - round_trip
  record_count: 3
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.css
  format_labels:
  - CSS
  source_apps_present:
  - figma
  - illustrator
  support_by_app:
    figma:
    - fixture_required
    illustrator:
    - fixture_required
  record_count: 3
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.csv
  format_labels:
  - CSV
  source_apps_present:
  - figma
  support_by_app:
    figma:
    - fixture_required
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.deck
  format_labels:
  - DECK local copy
  source_apps_present:
  - figma
  support_by_app:
    figma:
    - fixture_required
    - round_trip
  record_count: 3
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.dng
  format_labels:
  - DNG
  source_apps_present:
  - photoshop
  support_by_app:
    photoshop:
    - fixture_required
  record_count: 1
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.dwg
  format_labels:
  - DWG
  source_apps_present:
  - illustrator
  support_by_app:
    illustrator:
    - fixture_required
  record_count: 1
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.dxf
  format_labels:
  - DXF
  source_apps_present:
  - illustrator
  support_by_app:
    illustrator:
    - fixture_required
  record_count: 1
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.eps
  format_labels:
  - EPS
  source_apps_present:
  - affinity
  - illustrator
  - indesign
  support_by_app:
    affinity:
    - fixture_required
    illustrator:
    - fixture_required
    indesign:
    - export
  record_count: 3
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.epub
  format_labels:
  - EPUB
  source_apps_present:
  - indesign
  support_by_app:
    indesign:
    - export
    - fixture_required
  record_count: 10
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.exr_hdr
  format_labels:
  - EXR/HDR
  source_apps_present:
  - affinity
  - photoshop
  support_by_app:
    affinity:
    - fixture_required
    photoshop:
    - fixture_required
  record_count: 4
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.fig
  format_labels:
  - FIG local copy
  source_apps_present:
  - figma
  support_by_app:
    figma:
    - fixture_required
    - round_trip
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.gif
  format_labels:
  - GIF
  source_apps_present:
  - affinity
  - figma
  - photoshop
  support_by_app:
    affinity:
    - fixture_required
    figma:
    - fixture_required
    photoshop:
    - fixture_required
  record_count: 4
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.html
  format_labels:
  - HTML
  source_apps_present:
  - indesign
  support_by_app:
    indesign:
    - export
    - fixture_required
  record_count: 8
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.idml
  format_labels:
  - IDML
  source_apps_present:
  - indesign
  support_by_app:
    indesign:
    - fixture_required
    - round_trip
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.indd
  format_labels:
  - INDD
  source_apps_present:
  - indesign
  support_by_app:
    indesign:
    - round_trip
  record_count: 1
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.jam
  format_labels:
  - JAM local copy
  source_apps_present:
  - figma
  support_by_app:
    figma:
    - fixture_required
    - round_trip
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.jpeg
  format_labels:
  - JPEG/JPG
  source_apps_present:
  - affinity
  - figma
  - indesign
  - photoshop
  support_by_app:
    affinity:
    - fixture_required
    figma:
    - fixture_required
    indesign:
    - export
    photoshop:
    - fixture_required
  record_count: 4
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.make
  format_labels:
  - MAKE local copy
  source_apps_present:
  - figma
  support_by_app:
    figma:
    - fixture_required
    - round_trip
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.pdf
  format_labels:
  - PDF
  source_apps_present:
  - affinity
  - figma
  - illustrator
  - indesign
  - photoshop
  support_by_app:
    affinity:
    - fixture_required
    figma:
    - fixture_required
    illustrator:
    - fixture_required
    - import
    indesign:
    - export
    - fixture_required
    - import
    photoshop:
    - export
    - fixture_required
  record_count: 26
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.png
  format_labels:
  - PNG
  source_apps_present:
  - affinity
  - figma
  - indesign
  - photoshop
  support_by_app:
    affinity:
    - fixture_required
    figma:
    - fixture_required
    indesign:
    - export
    photoshop:
    - fixture_required
  record_count: 4
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.pptx
  format_labels:
  - PPTX
  source_apps_present:
  - figma
  support_by_app:
    figma:
    - fixture_required
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.ps
  format_labels:
  - PostScript
  source_apps_present:
  - indesign
  support_by_app:
    indesign:
    - export
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.psb
  format_labels:
  - PSB
  source_apps_present:
  - photoshop
  support_by_app:
    photoshop:
    - fixture_required
    - round_trip
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.psd
  format_labels:
  - PSD
  source_apps_present:
  - affinity
  - illustrator
  - photoshop
  support_by_app:
    affinity:
    - fixture_required
    illustrator:
    - fixture_required
    photoshop:
    - fixture_required
    - round_trip
  record_count: 4
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.raw
  format_labels:
  - RAW camera formats
  source_apps_present:
  - affinity
  - photoshop
  support_by_app:
    affinity:
    - fixture_required
    photoshop:
    - fixture_required
  record_count: 8
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.site
  format_labels:
  - SITE local copy
  source_apps_present:
  - figma
  support_by_app:
    figma:
    - export
    - fixture_required
    - round_trip
  record_count: 4
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: native_round_trip_target
- format_id: format.sketch
  format_labels:
  - Sketch
  source_apps_present:
  - figma
  support_by_app:
    figma:
    - fixture_required
    - import
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.svg
  format_labels:
  - SVG
  source_apps_present:
  - affinity
  - figma
  - illustrator
  - photoshop
  support_by_app:
    affinity:
    - fixture_required
    figma:
    - fixture_required
    illustrator:
    - fixture_required
    photoshop:
    - fixture_required
  record_count: 8
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.tiff
  format_labels:
  - TIFF
  source_apps_present:
  - affinity
  - photoshop
  support_by_app:
    affinity:
    - fixture_required
    photoshop:
    - fixture_required
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.unspecified
  format_labels:
  - Unspecified source-format workflow
  source_apps_present:
  - affinity
  - figma
  - illustrator
  - indesign
  - photoshop
  support_by_app:
    affinity:
    - export
    - fixture_required
    - import
    - round_trip
    figma:
    - export
    - fixture_required
    - import
    - round_trip
    illustrator:
    - export
    - fixture_required
    - import
    indesign:
    - export
    - fixture_required
    - import
    - round_trip
    photoshop:
    - export
    - fixture_required
    - import
  record_count: 332
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.webp
  format_labels:
  - WebP
  source_apps_present:
  - affinity
  - photoshop
  support_by_app:
    affinity:
    - fixture_required
    photoshop:
    - fixture_required
  record_count: 2
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.xls_excel
  format_labels:
  - Excel spreadsheets
  source_apps_present:
  - indesign
  support_by_app:
    indesign:
    - fixture_required
  record_count: 1
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
- format_id: format.xml
  format_labels:
  - XML
  source_apps_present:
  - indesign
  support_by_app:
    indesign:
    - export
    - fixture_required
    - import
  record_count: 4
  fixture_policy: fixture_required_for_every_supported_app_and_direction
  compatibility_posture: source_observable_import_export_target
compatibility_records:
- compatibility_record_id: compat.native.photoshop.format-psd.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-PHOTOSHOP
  source_app_key: photoshop
  source_family: adobe
  support_kind: round_trip
  format_refs:
  - format_id: format.psd
    format_label: PSD
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.photoshop.native
- compatibility_record_id: compat.native.photoshop.format-psb.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-PHOTOSHOP
  source_app_key: photoshop
  source_family: adobe
  support_kind: round_trip
  format_refs:
  - format_id: format.psb
    format_label: PSB
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.photoshop.native
- compatibility_record_id: compat.native.indesign.format-indd.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-INDESIGN
  source_app_key: indesign
  source_family: adobe
  support_kind: round_trip
  format_refs:
  - format_id: format.indd
    format_label: INDD
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.indesign.native
- compatibility_record_id: compat.native.indesign.format-idml.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-INDESIGN
  source_app_key: indesign
  source_family: adobe
  support_kind: round_trip
  format_refs:
  - format_id: format.idml
    format_label: IDML
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.indesign.native
- compatibility_record_id: compat.native.illustrator.format-ai.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-ILLUSTRATOR
  source_app_key: illustrator
  source_family: adobe
  support_kind: round_trip
  format_refs:
  - format_id: format.ai
    format_label: AI
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.illustrator.native
- compatibility_record_id: compat.native.illustrator.format-ait.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-ILLUSTRATOR
  source_app_key: illustrator
  source_family: adobe
  support_kind: round_trip
  format_refs:
  - format_id: format.ait
    format_label: AIT
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.illustrator.native
- compatibility_record_id: compat.native.affinity.format-afphoto.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-AFFINITY
  source_app_key: affinity
  source_family: affinity
  support_kind: round_trip
  format_refs:
  - format_id: format.afphoto
    format_label: AFPHOTO
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.affinity.native
- compatibility_record_id: compat.native.affinity.format-afdesign.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-AFFINITY
  source_app_key: affinity
  source_family: affinity
  support_kind: round_trip
  format_refs:
  - format_id: format.afdesign
    format_label: AFDESIGN
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.affinity.native
- compatibility_record_id: compat.native.affinity.format-afpub.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-AFFINITY
  source_app_key: affinity
  source_family: affinity
  support_kind: round_trip
  format_refs:
  - format_id: format.afpub
    format_label: AFPUB
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.affinity.native
- compatibility_record_id: compat.native.figma.format-fig.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-FIGMA
  source_app_key: figma
  source_family: figma
  support_kind: round_trip
  format_refs:
  - format_id: format.fig
    format_label: FIG local copy
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.figma.native
- compatibility_record_id: compat.native.figma.format-jam.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-FIGMA
  source_app_key: figma
  source_family: figma
  support_kind: round_trip
  format_refs:
  - format_id: format.jam
    format_label: JAM local copy
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.figma.native
- compatibility_record_id: compat.native.figma.format-deck.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-FIGMA
  source_app_key: figma
  source_family: figma
  support_kind: round_trip
  format_refs:
  - format_id: format.deck
    format_label: DECK local copy
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.figma.native
- compatibility_record_id: compat.native.figma.format-buzz.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-FIGMA
  source_app_key: figma
  source_family: figma
  support_kind: round_trip
  format_refs:
  - format_id: format.buzz
    format_label: BUZZ local copy
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.figma.native
- compatibility_record_id: compat.native.figma.format-site.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-FIGMA
  source_app_key: figma
  source_family: figma
  support_kind: round_trip
  format_refs:
  - format_id: format.site
    format_label: SITE local copy
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.figma.native
- compatibility_record_id: compat.native.figma.format-make.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-FIGMA
  source_app_key: figma
  source_family: figma
  support_kind: round_trip
  format_refs:
  - format_id: format.make
    format_label: MAKE local copy
  fixture_requirement: golden native document fixtures covering layers text color links effects masks and export settings
  round_trip_rule: native format compatibility requires import export diagnostics and explicit unsupported-feature receipts
  manual_topic_candidate: studio.manual.file-compatibility.figma.native
- compatibility_record_id: compat.domain.photoshop.psd-domain-document_file_io.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-PHOTOSHOP
  source_app_key: photoshop
  source_domain_id: psd.domain.document_file_io
  source_domain_name: Documents, presets, templates, file open/save/export, and metadata
  support_kind: fixture_required
  format_refs:
  - format_id: format.psd
    format_label: PSD
  - format_id: format.psb
    format_label: PSB
  - format_id: format.pdf
    format_label: PDF
  - format_id: format.svg
    format_label: SVG
  - format_id: format.png
    format_label: PNG
  - format_id: format.jpeg
    format_label: JPEG/JPG
  - format_id: format.gif
    format_label: GIF
  - format_id: format.webp
    format_label: WebP
  - format_id: format.tiff
    format_label: TIFF
  - format_id: format.raw
    format_label: RAW camera formats
  studio_primitive_domains:
  - file_io
  - export
  - color
  - asset_pipeline
  - metadata
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.file-compatibility.photoshop-class-documents
- compatibility_record_id: compat.domain.photoshop.psd-domain-color_tone_and_color_management.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-PHOTOSHOP
  source_app_key: photoshop
  source_domain_id: psd.domain.color_tone_and_color_management
  source_domain_name: Color, tone, adjustments, profiles, HDR, and color management
  support_kind: fixture_required
  format_refs:
  - format_id: format.raw
    format_label: RAW camera formats
  - format_id: format.exr_hdr
    format_label: EXR/HDR
  studio_primitive_domains:
  - color
  - layer
  - raster
  - prepress
  - camera_raw
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.color.photoshop-class-adjustments
- compatibility_record_id: compat.domain.photoshop.psd-domain-filters_effects_and_ai_filters.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-PHOTOSHOP
  source_app_key: photoshop
  source_domain_id: psd.domain.filters_effects_and_ai_filters
  source_domain_name: Filters, effects, liquify, neural filters, and procedural image operations
  support_kind: fixture_required
  format_refs:
  - format_id: format.raw
    format_label: RAW camera formats
  studio_primitive_domains:
  - raster
  - layer
  - ai
  - gpu_pipeline
  - diagnostics
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.filters.photoshop-class-effects
- compatibility_record_id: compat.domain.photoshop.psd-domain-camera_raw_development.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-PHOTOSHOP
  source_app_key: photoshop
  source_domain_id: psd.domain.camera_raw_development
  source_domain_name: Camera Raw development, profiles, optics, presets, and output
  support_kind: fixture_required
  format_refs:
  - format_id: format.raw
    format_label: RAW camera formats
  - format_id: format.dng
    format_label: DNG
  - format_id: format.exr_hdr
    format_label: EXR/HDR
  studio_primitive_domains:
  - camera_raw
  - color
  - raster
  - file_io
  - export
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.camera-raw.development
- compatibility_record_id: compat.domain.photoshop.psd-domain-camera_raw_masking_and_scopes.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-PHOTOSHOP
  source_app_key: photoshop
  source_domain_id: psd.domain.camera_raw_masking_and_scopes
  source_domain_name: Camera Raw masking, selection, healing, and local adjustments
  support_kind: fixture_required
  format_refs:
  - format_id: format.raw
    format_label: RAW camera formats
  studio_primitive_domains:
  - camera_raw
  - selection
  - mask
  - ai
  - batch
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.camera-raw.local-adjustments
- compatibility_record_id: compat.domain.indesign.idd-domain-graphics_objects_color.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-INDESIGN
  source_app_key: indesign
  source_domain_id: idd.domain.graphics_objects_color
  source_domain_name: Graphics, objects, links, color, transparency, and effects
  support_kind: fixture_required
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive_domains:
  - asset_pipeline
  - layout
  - vector
  - color
  - prepress
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.layout.graphics-and-color
- compatibility_record_id: compat.domain.indesign.idd-domain-interactive_and_accessible_outputs.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-INDESIGN
  source_app_key: indesign
  source_domain_id: idd.domain.interactive_and_accessible_outputs
  source_domain_name: Interactive documents, forms, hyperlinks, media, EPUB, and accessibility
  support_kind: fixture_required
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  - format_id: format.epub
    format_label: EPUB
  - format_id: format.html
    format_label: HTML
  studio_primitive_domains:
  - interactive
  - accessibility
  - pdf
  - epub
  - export
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.export.accessible-interactive-publications
- compatibility_record_id: compat.domain.indesign.idd-domain-import_export_publish_print.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-INDESIGN
  source_app_key: indesign
  source_domain_id: idd.domain.import_export_publish_print
  source_domain_name: Import, export, package, print, PDF, preflight, and publishing
  support_kind: fixture_required
  format_refs:
  - format_id: format.idml
    format_label: IDML
  - format_id: format.pdf
    format_label: PDF
  - format_id: format.epub
    format_label: EPUB
  - format_id: format.html
    format_label: HTML
  - format_id: format.xml
    format_label: XML
  - format_id: format.xls_excel
    format_label: Excel spreadsheets
  studio_primitive_domains:
  - file_io
  - export
  - pdf
  - epub
  - print
  - prepress
  - packaging
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.prepress.layout-export
- compatibility_record_id: compat.domain.illustrator.ail-domain-layers_assets_links.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-ILLUSTRATOR
  source_app_key: illustrator
  source_domain_id: ail.domain.layers_assets_links
  source_domain_name: Layers, assets, linked artwork, libraries, variables, and data-driven graphics
  support_kind: fixture_required
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive_domains:
  - asset_pipeline
  - layer
  - vector
  - data_binding
  - export
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.vector.assets-and-data
- compatibility_record_id: compat.domain.illustrator.ail-domain-effects_3d_web.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-ILLUSTRATOR
  source_app_key: illustrator
  source_domain_id: ail.domain.effects_3d_web
  source_domain_name: Effects, filters, 3D/materials, raster interop, SVG, CSS, and web/screen output
  support_kind: fixture_required
  format_refs:
  - format_id: format.svg
    format_label: SVG
  - format_id: format.css
    format_label: CSS
  studio_primitive_domains:
  - vector
  - raster
  - gpu_pipeline
  - export
  - web
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.vector.effects-and-web-export
- compatibility_record_id: compat.domain.illustrator.ail-domain-file_io_export_prepress.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-ILLUSTRATOR
  source_app_key: illustrator
  source_domain_id: ail.domain.file_io_export_prepress
  source_domain_name: File compatibility, import/export, packaging, print, PDF, SVG, and prepress
  support_kind: fixture_required
  format_refs:
  - format_id: format.psd
    format_label: PSD
  - format_id: format.ai
    format_label: AI
  - format_id: format.ait
    format_label: AIT
  - format_id: format.pdf
    format_label: PDF
  - format_id: format.svg
    format_label: SVG
  - format_id: format.eps
    format_label: EPS
  - format_id: format.dwg
    format_label: DWG
  - format_id: format.dxf
    format_label: DXF
  - format_id: format.css
    format_label: CSS
  studio_primitive_domains:
  - file_io
  - export
  - pdf
  - svg
  - prepress
  - print
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.file-compatibility.vector-formats
- compatibility_record_id: compat.domain.affinity.aff-domain-photo_imaging.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-AFFINITY
  source_app_key: affinity
  source_domain_id: aff.domain.photo_imaging
  source_domain_name: Photo raster editing, raw development, selections, masks, adjustments, live filters, and retouch
  support_kind: fixture_required
  format_refs:
  - format_id: format.raw
    format_label: RAW camera formats
  - format_id: format.exr_hdr
    format_label: EXR/HDR
  studio_primitive_domains:
  - raster
  - camera_raw
  - selection
  - mask
  - color
  - layer
  - brush_engine
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.photo.affinity-class-imaging
- compatibility_record_id: compat.domain.affinity.aff-domain-publishing_layout.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-AFFINITY
  source_app_key: affinity
  source_domain_id: aff.domain.publishing_layout
  source_domain_name: Publisher pages, spreads, masters, frames, preflight, package, and PDF
  support_kind: fixture_required
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive_domains:
  - page_layout
  - master_pages
  - typography
  - tables
  - prepress
  - export
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.layout.affinity-class-publishing
- compatibility_record_id: compat.domain.affinity.aff-domain-color_prepress_and_design_aids.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-AFFINITY
  source_app_key: affinity
  source_domain_id: aff.domain.color_prepress_and_design_aids
  source_domain_name: Color, swatches, gradients, effects, grids, snapping, resources, and prepress
  support_kind: fixture_required
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive_domains:
  - color
  - prepress
  - style_system
  - asset_pipeline
  - layout
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.color.affinity-class-design-aids
- compatibility_record_id: compat.domain.affinity.aff-domain-commands_and_workflow_surfaces.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-AFFINITY
  source_app_key: affinity
  source_domain_id: aff.domain.commands_and_workflow_surfaces
  source_domain_name: Commands, personas, macros, batch, export, resource management, and recovery
  support_kind: fixture_required
  format_refs:
  - format_id: format.raw
    format_label: RAW camera formats
  studio_primitive_domains:
  - automation
  - command_contracts
  - batch
  - export
  - versioning
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.automation.affinity-class-workflows
- compatibility_record_id: compat.domain.affinity.aff-domain-compatibility_and_formats.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-AFFINITY
  source_app_key: affinity
  source_domain_id: aff.domain.compatibility_and_formats
  source_domain_name: Native documents, PSD/PDF/SVG/EPS/AI-compatible import, raster formats, and export
  support_kind: fixture_required
  format_refs:
  - format_id: format.psd
    format_label: PSD
  - format_id: format.ai
    format_label: AI
  - format_id: format.pdf
    format_label: PDF
  - format_id: format.svg
    format_label: SVG
  - format_id: format.eps
    format_label: EPS
  - format_id: format.png
    format_label: PNG
  - format_id: format.jpeg
    format_label: JPEG/JPG
  - format_id: format.gif
    format_label: GIF
  - format_id: format.webp
    format_label: WebP
  - format_id: format.tiff
    format_label: TIFF
  - format_id: format.raw
    format_label: RAW camera formats
  - format_id: format.exr_hdr
    format_label: EXR/HDR
  - format_id: format.afphoto
    format_label: AFPHOTO
  - format_id: format.afdesign
    format_label: AFDESIGN
  - format_id: format.afpub
    format_label: AFPUB
  studio_primitive_domains:
  - file_io
  - export
  - pdf
  - svg
  - raster
  - vector
  - page_layout
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.file-compatibility.affinity-class
- compatibility_record_id: compat.domain.figma.fig-domain-motion.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-FIGMA
  source_app_key: figma
  source_domain_id: fig.domain.motion
  source_domain_name: Motion, timeline, keyframes, easing, animated prototypes, and video/GIF export
  support_kind: fixture_required
  format_refs:
  - format_id: format.gif
    format_label: GIF
  studio_primitive_domains:
  - motion
  - timeline
  - prototype
  - export
  - vector
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.motion.design-animation
- compatibility_record_id: compat.domain.figma.fig-domain-slides.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-FIGMA
  source_app_key: figma
  source_domain_id: fig.domain.slides
  source_domain_name: Slides, decks, presentation design, presenter workflows, and export
  support_kind: fixture_required
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  - format_id: format.pptx
    format_label: PPTX
  - format_id: format.deck
    format_label: DECK local copy
  studio_primitive_domains:
  - presentation
  - layout
  - typography
  - vector
  - export
  - collaboration
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.presentations.figma-slides-class
- compatibility_record_id: compat.domain.figma.fig-domain-sites.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-FIGMA
  source_app_key: figma
  source_domain_id: fig.domain.sites
  source_domain_name: Sites, web publishing, responsive pages, components, domains, and export
  support_kind: fixture_required
  format_refs:
  - format_id: format.site
    format_label: SITE local copy
  studio_primitive_domains:
  - web
  - layout
  - design_systems
  - export
  - provider_adapter
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.web.figma-sites-class
- compatibility_record_id: compat.domain.figma.fig-domain-buzz.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-FIGMA
  source_app_key: figma
  source_domain_id: fig.domain.buzz
  source_domain_name: Buzz, brand asset production, templates, bulk content, and marketing outputs
  support_kind: fixture_required
  format_refs:
  - format_id: format.csv
    format_label: CSV
  - format_id: format.buzz
    format_label: BUZZ local copy
  studio_primitive_domains:
  - brand_assets
  - templates
  - data_binding
  - export
  - collaboration
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.brand-assets.bulk-production
- compatibility_record_id: compat.domain.figma.fig-domain-make.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-FIGMA
  source_app_key: figma
  source_domain_id: fig.domain.make
  source_domain_name: Make, AI app generation, code layers, prototypes, and local/provider split
  support_kind: fixture_required
  format_refs:
  - format_id: format.make
    format_label: MAKE local copy
  studio_primitive_domains:
  - ai
  - app_generation
  - prototype
  - code_layers
  - provider_adapter
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.ai.app-generation
- compatibility_record_id: compat.domain.figma.fig-domain-dev_mode_api.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-FIGMA
  source_app_key: figma
  source_domain_id: fig.domain.dev_mode_api
  source_domain_name: Dev Mode, inspect, Code Connect, MCP, REST API, plugins, widgets, and webhooks
  support_kind: fixture_required
  format_refs:
  - format_id: format.css
    format_label: CSS
  studio_primitive_domains:
  - dev_mode
  - api
  - plugin_api
  - automation
  - design_systems
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.dev-mode.design-handoff
- compatibility_record_id: compat.domain.figma.fig-domain-collaboration_import_export_local_copies.v1
  source_ids:
  - COMPAT-S01
  - COMPAT-FIGMA
  source_app_key: figma
  source_domain_id: fig.domain.collaboration_import_export_local_copies
  source_domain_name: Collaboration, comments, permissions, branches, local copies, import/export, and compatibility
  support_kind: fixture_required
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  - format_id: format.svg
    format_label: SVG
  - format_id: format.png
    format_label: PNG
  - format_id: format.jpeg
    format_label: JPEG/JPG
  - format_id: format.gif
    format_label: GIF
  - format_id: format.csv
    format_label: CSV
  - format_id: format.pptx
    format_label: PPTX
  - format_id: format.sketch
    format_label: Sketch
  - format_id: format.fig
    format_label: FIG local copy
  - format_id: format.jam
    format_label: JAM local copy
  - format_id: format.deck
    format_label: DECK local copy
  - format_id: format.buzz
    format_label: BUZZ local copy
  - format_id: format.site
    format_label: SITE local copy
  studio_primitive_domains:
  - collaboration
  - file_io
  - export
  - versioning
  - permissions
  - diagnostics
  fixture_requirement: create representative source fixture set before implementation claim
  round_trip_rule: domain-level format mention must be refined into import export or round-trip command contracts
  manual_topic_candidate: studio.manual.file-compatibility.figma-class
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-adjust-color-selective-color-adjustments-save-and-apply-settings-in-match-color-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.adjust-color.selective-color-adjustments.save-and-apply-settings-in-match-color.v1
  feature_name: Save and apply settings in Match Color
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: layer
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioLayerGraph / Save and apply settings in Match Color
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-app-integrations-access-adobe-express-templates-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.app-integrations.access-adobe-express-templates.v1
  feature_name: Access Adobe Express Templates
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Access Adobe Express Templates
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-app-integrations-open-photoshop-files-in-illustrator-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.app-integrations.open-photoshop-files-in-illustrator.v1
  feature_name: Open Photoshop files in Illustrator
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Open Photoshop files in Illustrator
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-apply-painting-techniques-brushes-presets-import-brushes-brush-packs-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.brushes-presets.import-brushes-brush-packs.v1
  feature_name: Import brushes and brush packs
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: raster
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioRasterPipeline / Import brushes and brush packs
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-automation-settings-and-presets-actions-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.automation-settings-and-presets.actions-overview.v1
  feature_name: Overview of Actions
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Overview of Actions
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-automation-settings-and-presets-apply-actions-in-the-actions-panel-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.automation-settings-and-presets.apply-actions-in-the-actions-panel.v1
  feature_name: Apply actions in the Actions panel
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Apply actions in the Actions panel
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-automation-settings-and-presets-use-the-actions-panel-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.automation-settings-and-presets.use-the-actions-panel.v1
  feature_name: Use the Actions panel
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Use the Actions panel
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-create-record-actions-add-commands-to-an-action-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.add-commands-to-an-action.v1
  feature_name: Add commands to an action
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Add commands to an action
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-create-record-actions-change-settings-when-playing-an-action-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.change-settings-when-playing-an-action.v1
  feature_name: Change settings when playing an action
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Change settings when playing an action
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-create-record-actions-exclude-commands-from-an-action-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.exclude-commands-from-an-action.v1
  feature_name: Exclude commands from an action
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Exclude commands from an action
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-create-record-actions-insert-a-non-recordable-menu-command-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.insert-a-non-recordable-menu-command.v1
  feature_name: Insert a non-recordable menu command
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Insert a non-recordable menu command
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-create-record-actions-insert-a-stop-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.insert-a-stop.v1
  feature_name: Insert a stop
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Insert a stop
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-create-record-actions-overwrite-a-single-command-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.overwrite-a-single-command.v1
  feature_name: Overwrite a single command
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Overwrite a single command
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-create-record-actions-rearrange-commands-within-an-action-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.rearrange-commands-within-an-action.v1
  feature_name: Rearrange commands within an action
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Rearrange commands within an action
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-create-record-actions-record-an-action-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.record-an-action.v1
  feature_name: Record an action
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Record an action
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-create-record-actions-record-an-action-again-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.record-an-action-again.v1
  feature_name: Record an action again
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Record an action again
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-process-a-batch-of-files-batch-and-droplet-processing-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.process-a-batch-of-files.batch-and-droplet-processing-options.v1
  feature_name: Batch and droplet processing options
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Batch and droplet processing options
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-process-a-batch-of-files-batch-process-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.process-a-batch-of-files.batch-process-files.v1
  feature_name: Batch-process files
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Batch-process files
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-process-a-batch-of-files-convert-files-with-the-image-processor-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.process-a-batch-of-files.convert-files-with-the-image-processor.v1
  feature_name: Convert files with the Image Processor
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Convert files with the Image Processor
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-process-a-batch-of-files-create-a-droplet-from-an-action-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.process-a-batch-of-files.create-a-droplet-from-an-action.v1
  feature_name: Create a droplet from an action
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Create a droplet from an action
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-automate-tasks-process-a-batch-of-files-image-processor-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.automate-tasks.process-a-batch-of-files.image-processor-overview.v1
  feature_name: Image Processor overview
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Image Processor overview
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-create-manage-layers-apply-layer-effects-import-preset-style-libraries-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.import-preset-style-libraries.v1
  feature_name: Import preset style libraries
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: layer
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioLayerGraph / Import preset style libraries
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-create-manage-layers-layout-design-tools-place-image-frame-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.create-manage-layers.layout-design-tools.place-image-frame.v1
  feature_name: Place an image into a frame
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: layer
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioLayerGraph / Place an image into a frame
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-create-manage-layers-smart-objects-export-the-contents-of-an-embedded-smart-object-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.export-the-contents-of-an-embedded-smart-object.v1
  feature_name: Export the contents of an embedded Smart Object
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: layer
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioLayerGraph / Export the contents of an embedded Smart Object
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-create-manage-layers-smart-objects-package-linked-smart-objects-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.package-linked-smart-objects.v1
  feature_name: Package and locate linked Smart Objects
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: layer
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioLayerGraph / Package and locate linked Smart Objects
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-create-open-import-images-create-images-edit-images-with-generative-fill-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.create-open-import-images.create-images.edit-images-with-generative-fill.v1
  feature_name: Edit images with Generative Fill
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: ai
  provider_posture: provider_adapter
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioModelToolContract / Edit images with Generative Fill
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-create-open-import-images-create-images-explore-beyond-the-canvas-with-generative-expand-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.create-open-import-images.create-images.explore-beyond-the-canvas-with-generative-expand.v1
  feature_name: Explore beyond the canvas with Generative Expand
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: ai
  provider_posture: provider_adapter
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioModelToolContract / Explore beyond the canvas with Generative Expand
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-create-open-import-images-create-images-generate-image-with-descriptive-text-prompts-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.create-open-import-images.create-images.generate-image-with-descriptive-text-prompts.v1
  feature_name: Generate an image with descriptive text prompts
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Generate an image with descriptive text prompts
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-create-open-import-images-create-images-generate-images-using-reference-image-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.create-open-import-images.create-images.generate-images-using-reference-image.v1
  feature_name: Generate images guided by a reference image
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: ai
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioModelToolContract / Generate images guided by a reference image
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-create-open-import-images-create-images-generate-sharper-variations-with-enhance-detail-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.create-open-import-images.create-images.generate-sharper-variations-with-enhance-detail.v1
  feature_name: Generate sharper variations with Enhance Detail
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Generate sharper variations with Enhance Detail
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-create-open-import-images-create-images-use-reference-images-for-consistent-results-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.create-open-import-images.create-images.use-reference-images-for-consistent-results.v1
  feature_name: Use reference images for consistent results
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: ai
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioModelToolContract / Use reference images for consistent results
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-create-open-import-images-import-files-browse-select-and-import-adobe-stock-assets-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.create-open-import-images.import-files.browse-select-and-import-adobe-stock-assets.v1
  feature_name: Browse, select, and import Adobe Stock assets
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: selection
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioSelectionSet / Browse, select, and import Adobe Stock assets
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-crop-resize-transform-resize-adjust-resolution-change-print-dimensions-and-resolution-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.change-print-dimensions-and-resolution.v1
  feature_name: Change print dimensions and resolution
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Change print dimensions and resolution
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-access-discover-panel-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.access-discover-panel.v1
  feature_name: Access the Discover panel
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Access the Discover panel
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-collapse-expand-icons-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.collapse-expand-icons.v1
  feature_name: Expand or collapse panel icons
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Expand or collapse panel icons
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-delete-workspaces-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.delete-workspaces.v1
  feature_name: Delete workspaces
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Delete workspaces
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-dock-undock-panels-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.dock-undock-panels.v1
  feature_name: Dock or undock panels
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Dock or undock panels
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-hide-show-panels-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.hide-show-panels.v1
  feature_name: Hide or show all panels
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Hide or show all panels
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-homescreen-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.homescreen-overview.v1
  feature_name: Home screen overview
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Home screen overview
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-manipulate-panel-groups-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.manipulate-panel-groups.v1
  feature_name: Arrange and group panels
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Arrange and group panels
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-move-panels-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.move-panels.v1
  feature_name: Move panels
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Move panels
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-rearrange-document-windows-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.rearrange-document-windows.v1
  feature_name: Rearrange document windows
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Rearrange document windows
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-restore-workspaces-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.restore-workspaces.v1
  feature_name: Restore workspaces
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Restore workspaces
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-save-custom-workspaces-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.save-custom-workspaces.v1
  feature_name: Save custom workspaces
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Save custom workspaces
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-stack-floating-panels-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.stack-floating-panels.v1
  feature_name: Stack floating panels
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Stack floating panels
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-switch-workspaces-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.switch-workspaces.v1
  feature_name: Switch workspaces
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Switch workspaces
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-use-simple-math-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.use-simple-math.v1
  feature_name: Use simple math in number fields
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Use simple math in number fields
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-learn-the-basics-workspace-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.workspace-overview.v1
  feature_name: Workspace overview
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Workspace overview
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-set-up-toolbars-panels-create-tool-preset-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.create-tool-preset.v1
  feature_name: Create tool presets
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Create tool presets
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-set-up-toolbars-panels-create-work-snapshots-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.create-work-snapshots.v1
  feature_name: Use snapshots in the History panel
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Use snapshots in the History panel
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-set-up-toolbars-panels-customize-the-toolbar-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.customize-the-toolbar.v1
  feature_name: Customize toolbar
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Customize toolbar
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-set-up-toolbars-panels-history-panel-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.history-panel-overview.v1
  feature_name: History panel settings
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / History panel settings
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-set-up-toolbars-panels-manage-image-states-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.manage-image-states.v1
  feature_name: Manage image states
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Manage image states
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-set-up-toolbars-panels-restore-image-parts-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.restore-image-parts.v1
  feature_name: Restore parts of an image to a previous state
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Restore parts of an image to a previous state
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-set-up-toolbars-panels-show-or-hide-tool-tips-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.show-or-hide-tool-tips.v1
  feature_name: Tooltips overview
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Tooltips overview
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-set-up-toolbars-panels-spring-loaded-shortcuts-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.spring-loaded-shortcuts.v1
  feature_name: Use spring-loaded shortcuts
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Use spring-loaded shortcuts
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-set-up-toolbars-panels-use-undo-redo-commands-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.use-undo-redo-commands.v1
  feature_name: Use Undo and Redo commands
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Use Undo and Redo commands
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-get-started-set-up-toolbars-panels-view-history-logs-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.view-history-logs.v1
  feature_name: View history logs
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / View history logs
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-make-selections-freehand-selections-save-skin-tones-settings-as-a-preset-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.make-selections.freehand-selections.save-skin-tones-settings-as-a-preset.v1
  feature_name: Save Skin Tones settings as a preset
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: selection
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioSelectionSet / Save Skin Tones settings as a preset
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-enhance-animation-frames-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.enhance-animation-frames.v1
  feature_name: Enhance animation frames
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Enhance animation frames
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-export-artboards-as-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-artboards-as-files.v1
  feature_name: Export artboards as files
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export artboards as files
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-export-artboards-as-pdf-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-artboards-as-pdf.v1
  feature_name: Export artboards as PDF
  support_kind: export
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export artboards as PDF
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-export-files-in-different-sizes-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-files-in-different-sizes.v1
  feature_name: Export files in different sizes
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export files in different sizes
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-export-layers-as-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-layers-as-files.v1
  feature_name: Export layers as files
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export layers as files
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-export-settings-and-export-location-preferences-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-settings-and-export-location-preferences.v1
  feature_name: Export settings and export location preferences
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export settings and export location preferences
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-export-to-cloud-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-to-cloud.v1
  feature_name: Save and export to cloud
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Save and export to cloud
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-export-video-files-or-image-sequences-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-video-files-or-image-sequences.v1
  feature_name: Export video files or image sequences
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export video files or image sequences
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-export-your-work-using-the-quick-export-as-option-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-your-work-using-the-quick-export-as-option.v1
  feature_name: Export your work using the Quick Export as option
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export your work using the Quick Export as option
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-file-compression-in-photoshop-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.file-compression-in-photoshop.v1
  feature_name: File compression in Photoshop
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / File compression in Photoshop
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-fine-tune-your-export-settings-using-the-export-as-option-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.fine-tune-your-export-settings-using-the-export-as-option.v1
  feature_name: Fine-tune export settings with Export As
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Fine-tune export settings with Export As
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-flatten-frames-into-layers-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.flatten-frames-into-layers.v1
  feature_name: Flatten frames into layers
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Flatten frames into layers
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-photoshop-file-formats-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.photoshop-file-formats-overview.v1
  feature_name: Photoshop file formats overview
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Photoshop file formats overview
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-export-files-to-different-formats-video-and-animation-export-formats-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.video-and-animation-export-formats.v1
  feature_name: Video and animation export formats
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Video and animation export formats
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-metadata-content-credentials-export-your-work-with-content-credentials-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.metadata-content-credentials.export-your-work-with-content-credentials.v1
  feature_name: Export your work with Content Credentials
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Export your work with Content Credentials
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-metadata-content-credentials-preview-content-credentials-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.metadata-content-credentials.preview-content-credentials.v1
  feature_name: Preview Content Credentials
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Preview Content Credentials
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-metadata-content-credentials-use-content-credentials-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.metadata-content-credentials.use-content-credentials.v1
  feature_name: Use Content Credentials
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Use Content Credentials
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-save-files-common-questions-on-photoshop-cloud-documents-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.common-questions-on-photoshop-cloud-documents.v1
  feature_name: Common questions on Photoshop cloud documents
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Common questions on Photoshop cloud documents
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-save-files-file-saving-properties-and-preferences-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.file-saving-properties-and-preferences.v1
  feature_name: File saving properties and preferences
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / File saving properties and preferences
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-save-files-macos-image-preview-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.macos-image-preview-options.v1
  feature_name: macOS image preview options
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / macOS image preview options
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-save-files-revert-to-legacy-save-as-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.revert-to-legacy-save-as-options.v1
  feature_name: Revert to legacy Save As options
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Revert to legacy Save As options
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-save-files-save-as-photoshop-pdf-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.save-as-photoshop-pdf.v1
  feature_name: Save as Photoshop PDF
  support_kind: export
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Save as Photoshop PDF
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-save-files-save-for-web-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.save-for-web.v1
  feature_name: Save for Web
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Save for Web
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-save-files-save-large-documents-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.save-large-documents.v1
  feature_name: Save large documents
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Save large documents
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-save-and-export-save-files-save-your-work-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.save-your-work.v1
  feature_name: Save your work
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Save your work
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-share-and-collaborate-collaborate-and-edit-share-and-collaborate-with-projects-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.share-and-collaborate.collaborate-and-edit.share-and-collaborate-with-projects.v1
  feature_name: Share and collaborate with Projects
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Share and collaborate with Projects
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-share-and-collaborate-collaborate-and-edit-work-with-projects-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.share-and-collaborate.collaborate-and-edit.work-with-projects.v1
  feature_name: Create Projects and add files
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Create Projects and add files
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-text-typography-characters-glyphs-work-with-opentype-svg-fonts-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.text-typography.characters-glyphs.work-with-opentype-svg-fonts.v1
  feature_name: Work with OpenType SVG fonts
  support_kind: fixture_required
  format_refs:
  - format_id: format.svg
    format_label: SVG
  studio_primitive: vector
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Work with OpenType SVG fonts
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-whats-new-enable-and-use-technology-previews-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.whats-new.enable-and-use-technology-previews.v1
  feature_name: Use technology previews
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Use technology previews
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-whats-new-list-of-technology-preview-features-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.whats-new.list-of-technology-preview-features.v1
  feature_name: List of technology preview features
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / List of technology preview features
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-whats-new-photoshop-desktop-beta-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.whats-new.photoshop-desktop-beta-overview.v1
  feature_name: Overview of Adobe Photoshop (beta) on desktop
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Overview of Adobe Photoshop (beta) on desktop
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-whats-new-photoshop-on-desktop-release-notes-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.whats-new.photoshop-on-desktop-release-notes.v1
  feature_name: Adobe Photoshop on desktop release notes
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Adobe Photoshop on desktop release notes
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-whats-new-whats-new-in-adobe-photoshop-on-desktop-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.whats-new.whats-new-in-adobe-photoshop-on-desktop.v1
  feature_name: What's new in Adobe Photoshop on desktop
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / What's new in Adobe Photoshop on desktop
- compatibility_record_id: compat.feature.photoshop.osd-photoshop-photoshop-leaf-whats-new-whats-new-in-photoshop-beta-on-desktop-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: photoshop
  source_family: adobe
  source_feature_row_id: osd.photoshop.photoshop.leaf.whats-new.whats-new-in-photoshop-beta-on-desktop.v1
  feature_name: What's new in Adobe Photoshop (Beta) on desktop
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / What's new in Adobe Photoshop (Beta) on desktop
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-and-manage-text-add-and-import-text-add-text-to-documents-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-and-manage-text.add-and-import-text.add-text-to-documents.v1
  feature_name: Add text to documents
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Add text to documents
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-and-manage-text-add-and-import-text-drag-drop-text-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-and-manage-text.add-and-import-text.drag-drop-text.v1
  feature_name: Drag and drop text within InDesign
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Drag and drop text within InDesign
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-and-manage-text-add-and-import-text-find-replace-text-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-and-manage-text.add-and-import-text.find-replace-text.v1
  feature_name: Find and replace text
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Find and replace text
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-and-manage-text-add-and-import-text-generate-text-variations-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-and-manage-text.add-and-import-text.generate-text-variations.v1
  feature_name: Generate text variations
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Generate text variations
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-and-manage-text-add-and-import-text-import-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-and-manage-text.add-and-import-text.import-options.v1
  feature_name: Import options
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Import options
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-and-manage-text-add-and-import-text-import-text-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-and-manage-text.add-and-import-text.import-text-files.v1
  feature_name: Import text files
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Import text files
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-and-manage-text-add-and-import-text-set-up-smart-text-reflow-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-and-manage-text.add-and-import-text.set-up-smart-text-reflow.v1
  feature_name: Set up smart text reflow
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Set up smart text reflow
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-and-manage-text-add-and-import-text-thread-text-frames-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-and-manage-text.add-and-import-text.thread-text-frames.v1
  feature_name: Thread text frames
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Thread text frames
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-and-manage-text-add-and-import-text-use-cc-text-assets-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-and-manage-text.add-and-import-text.use-cc-text-assets.v1
  feature_name: Use Creative Cloud text assets in InDesign documents
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Use Creative Cloud text assets in InDesign documents
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-and-manage-text-add-and-manage-text-frames-open-and-use-story-editor-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-and-manage-text.add-and-manage-text-frames.open-and-use-story-editor.v1
  feature_name: Open and use Story Editor
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Open and use Story Editor
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-graphics-and-media-add-edit-graphics-import-firefly-assets-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-graphics-and-media.add-edit-graphics.import-firefly-assets.v1
  feature_name: Import Firefly assets
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: ai
  provider_posture: provider_adapter
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioModelToolContract / Import Firefly assets
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-graphics-and-media-add-edit-graphics-import-llustrator-graphics-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-graphics-and-media.add-edit-graphics.import-llustrator-graphics.v1
  feature_name: Import Illustrator graphics
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Import Illustrator graphics
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-graphics-and-media-add-edit-graphics-import-options-for-adobe-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-graphics-and-media.add-edit-graphics.import-options-for-adobe-files.v1
  feature_name: Import options for Adobe files
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Import options for Adobe files
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-graphics-and-media-add-edit-graphics-import-options-for-image-formats-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-graphics-and-media.add-edit-graphics.import-options-for-image-formats.v1
  feature_name: Import options for image formats
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Import options for image formats
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-graphics-and-media-manage-frames-and-objects-apply-object-export-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-graphics-and-media.manage-frames-and-objects.apply-object-export-options.v1
  feature_name: Apply object export options
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Apply object export options
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-graphics-and-media-manage-object-styles-define-and-apply-object-styles-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-graphics-and-media.manage-object-styles.define-and-apply-object-styles.v1
  feature_name: Create, import, and apply object styles
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Create, import, and apply object styles
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-graphics-and-media-movies-and-sound-change-media-settings-for-interactive-pdf-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-graphics-and-media.movies-and-sound.change-media-settings-for-interactive-pdf-files.v1
  feature_name: Change media settings for interactive PDF files
  support_kind: fixture_required
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Change media settings for interactive PDF files
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-graphics-and-media-object-libraries-and-snippets-open-close-delete-object-libraries-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-graphics-and-media.object-libraries-and-snippets.open-close-delete-object-libraries.v1
  feature_name: Open, close, or delete object libraries
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Open, close, or delete object libraries
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-graphics-and-media-page-transitions-view-page-transitions-in-pdfs-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-graphics-and-media.page-transitions.view-page-transitions-in-pdfs.v1
  feature_name: View page transitions in PDFs
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / View page transitions in PDFs
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-tables-and-data-create-tables-create-and-import-tables-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-tables-and-data.create-tables.create-and-import-tables.v1
  feature_name: Create and import tables
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: tables
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTableFrame / Create and import tables
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-add-tables-and-data-table-and-cell-styles-import-table-and-cell-styles-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.add-tables-and-data.table-and-cell-styles.import-table-and-cell-styles.v1
  feature_name: Import table and cell styles
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: tables
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTableFrame / Import table and cell styles
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-app-integrations-access-adobe-express-templates-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.app-integrations.access-adobe-express-templates.v1
  feature_name: Access Express templates from InDesign
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Access Express templates from InDesign
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-app-integrations-adobe-capture-plugin-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.app-integrations.adobe-capture-plugin.v1
  feature_name: Adobe Capture plugin
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Adobe Capture plugin
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-app-integrations-create-indesign-plugins-with-uxp-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.app-integrations.create-indesign-plugins-with-uxp.v1
  feature_name: Create InDesign plug-ins with UXP
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Create InDesign plug-ins with UXP
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-app-integrations-creative-cloud-add-on-integrations-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.app-integrations.creative-cloud-add-on-integrations.v1
  feature_name: Creative Cloud add-on integrations
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Creative Cloud add-on integrations
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-app-integrations-illustrator-integration-with-indesign-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.app-integrations.illustrator-integration-with-indesign.v1
  feature_name: Illustrator integration with InDesign
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Illustrator integration with InDesign
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-app-integrations-install-plugins-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.app-integrations.install-plugins.v1
  feature_name: Install plug-ins
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Install plug-ins
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-app-integrations-manage-assets-cc-libraries-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.app-integrations.manage-assets-cc-libraries.v1
  feature_name: Manage assets in Creative Cloud Libraries
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Manage assets in Creative Cloud Libraries
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-app-integrations-manage-project-assets-cc-libraries-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.app-integrations.manage-project-assets-cc-libraries.v1
  feature_name: Manage project assets with Creative Cloud Libraries
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Manage project assets with Creative Cloud Libraries
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-app-integrations-share-libraries-with-cc-users-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.app-integrations.share-libraries-with-cc-users.v1
  feature_name: Share libraries with Creative Cloud users
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Share libraries with Creative Cloud users
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-app-integrations-use-adobe-bridge-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.app-integrations.use-adobe-bridge.v1
  feature_name: Use Adobe Bridge with InDesign
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Use Adobe Bridge with InDesign
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-apply-color-define-and-manage-color-assets-import-and-share-swatch-libraries-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.apply-color.define-and-manage-color-assets.import-and-share-swatch-libraries.v1
  feature_name: Import and share swatch libraries
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Import and share swatch libraries
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-document-automation-automate-workflows-with-scripts-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.document-automation.automate-workflows-with-scripts.v1
  feature_name: Automate workflows with scripts
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Automate workflows with scripts
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-document-automation-automated-workflows-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.document-automation.automated-workflows-overview.v1
  feature_name: Automated workflows overview
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Automated workflows overview
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-document-automation-import-xml-data-into-indesign-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.document-automation.import-xml-data-into-indesign.v1
  feature_name: Import XML data into InDesign
  support_kind: import
  format_refs:
  - format_id: format.xml
    format_label: XML
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Import XML data into InDesign
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-document-automation-structure-and-tag-documents-for-xml-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.document-automation.structure-and-tag-documents-for-xml.v1
  feature_name: Structure and tag documents for XML
  support_kind: fixture_required
  format_refs:
  - format_id: format.xml
    format_label: XML
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Structure and tag documents for XML
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-merge-data-add-and-edit-data-fields-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.merge-data.add-and-edit-data-fields.v1
  feature_name: Add and edit data fields
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Add and edit data fields
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-merge-data-data-merging-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.merge-data.data-merging-overview.v1
  feature_name: Data merging overview
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Data merging overview
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-merge-data-data-source-files-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.merge-data.data-source-files-overview.v1
  feature_name: Data source files overview
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Data source files overview
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-merge-data-edit-data-field-placeholders-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.merge-data.edit-data-field-placeholders.v1
  feature_name: Edit data field placeholders
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Edit data field placeholders
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-merge-data-manage-data-source-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.merge-data.manage-data-source-files.v1
  feature_name: Update, remove, or replace data source files
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Update, remove, or replace data source files
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-merge-data-merge-data-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.merge-data.merge-data.v1
  feature_name: Merge data
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Merge data
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-merge-data-merge-records-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.merge-data.merge-records.v1
  feature_name: Merge records
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Merge records
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-merge-data-preview-records-in-the-target-document-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.merge-data.preview-records-in-the-target-document.v1
  feature_name: Preview records in the target document
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Preview records in the target document
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-merge-data-set-content-placement-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.merge-data.set-content-placement-options.v1
  feature_name: Set content placement options
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Set content placement options
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-automation-and-scripting-merge-data-set-up-target-documents-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.automation-and-scripting.merge-data.set-up-target-documents.v1
  feature_name: Set up target documents
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Set up target documents
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-edit-with-incopy-about-assignment-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.edit-with-incopy.about-assignment-files.v1
  feature_name: About assignment files between InDesign and InCopy
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / About assignment files between InDesign and InCopy
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-edit-with-incopy-check-in-and-check-out-content-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.edit-with-incopy.check-in-and-check-out-content.v1
  feature_name: Check in and check out content
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Check in and check out content
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-edit-with-incopy-create-and-manage-assignments-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.edit-with-incopy.create-and-manage-assignments.v1
  feature_name: Create and manage assignments
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Create and manage assignments
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-edit-with-incopy-open-and-update-managed-content-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.edit-with-incopy.open-and-update-managed-content.v1
  feature_name: Open and update managed content
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Open and update managed content
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-edit-with-incopy-set-up-assignment-packages-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.edit-with-incopy.set-up-assignment-packages.v1
  feature_name: Set up assignment packages for InCopy workflows
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Set up assignment packages for InCopy workflows
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-edit-with-incopy-set-user-identification-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.edit-with-incopy.set-user-identification.v1
  feature_name: Set user identification
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Set user identification
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-edit-with-incopy-update-and-save-managed-layout-changes-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.edit-with-incopy.update-and-save-managed-layout-changes.v1
  feature_name: Save managed layout changes
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Save managed layout changes
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-edit-with-incopy-workflow-icons-for-managed-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.edit-with-incopy.workflow-icons-for-managed-files.v1
  feature_name: Workflow icons for managed files
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Workflow icons for managed files
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-share-and-collaborate-add-files-to-projects-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.share-and-collaborate.add-files-to-projects.v1
  feature_name: Add files to projects
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Add files to projects
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-share-and-collaborate-create-and-share-projects-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.share-and-collaborate.create-and-share-projects.v1
  feature_name: Create and share projects
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Create and share projects
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-share-and-collaborate-edit-with-incopy-on-the-web-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.share-and-collaborate.edit-with-incopy-on-the-web.v1
  feature_name: Edit InDesign documents with InCopy on the web (beta)
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Edit InDesign documents with InCopy on the web (beta)
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-share-and-collaborate-import-pdf-comments-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.share-and-collaborate.import-pdf-comments.v1
  feature_name: Import PDF comments
  support_kind: import
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Import PDF comments
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-share-and-collaborate-invite-collaborators-to-edit-cloud-documents-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.share-and-collaborate.invite-collaborators-to-edit-cloud-documents.v1
  feature_name: Invite collaborators to edit cloud documents
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Invite collaborators to edit cloud documents
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-share-and-collaborate-manage-feedback-for-shared-documents-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.share-and-collaborate.manage-feedback-for-shared-documents.v1
  feature_name: Manage feedback for shared documents
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Manage feedback for shared documents
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-share-and-collaborate-review-a-shared-document-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.share-and-collaborate.review-a-shared-document.v1
  feature_name: Review a shared InDesign document
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Review a shared InDesign document
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-share-and-collaborate-share-for-review-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.share-and-collaborate.share-for-review-overview.v1
  feature_name: Share for review overview
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Share for review overview
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-collaborate-and-review-track-changes-and-review-add-editorial-notes-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.collaborate-and-review.track-changes-and-review.add-editorial-notes.v1
  feature_name: Add and manage editorial notes
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Add and manage editorial notes
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-create-and-organize-pages-create-and-manage-book-files-create-save-book-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.create-and-organize-pages.create-and-manage-book-files.create-save-book-files.v1
  feature_name: Create and save book files
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Create and save book files
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-create-and-organize-pages-create-and-manage-parent-pages-import-parent-pages-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.create-and-organize-pages.create-and-manage-parent-pages.import-parent-pages.v1
  feature_name: Import parent pages
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Import parent pages
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-create-and-organize-pages-create-documents-open-indesign-documents-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.create-and-organize-pages.create-documents.open-indesign-documents.v1
  feature_name: Open documents
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Open documents
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-create-and-organize-pages-import-and-convert-file-to-indesign-convert-pdfs-to-indesign-documents-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.create-and-organize-pages.import-and-convert-file-to-indesign.convert-pdfs-to-indesign-documents.v1
  feature_name: Convert PDFs to InDesign documents
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Convert PDFs to InDesign documents
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-create-and-organize-pages-import-and-convert-file-to-indesign-convert-quarkxpress-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.create-and-organize-pages.import-and-convert-file-to-indesign.convert-quarkxpress-files.v1
  feature_name: Convert and save QuarkXPress files
  support_kind: round_trip
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Convert and save QuarkXPress files
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-create-and-organize-pages-import-and-convert-file-to-indesign-import-files-graphics-metadata-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.create-and-organize-pages.import-and-convert-file-to-indesign.import-files-graphics-metadata.v1
  feature_name: Import files, graphics, and metadata
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Import files, graphics, and metadata
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-create-and-organize-pages-import-and-convert-file-to-indesign-import-from-cc-libraries-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.create-and-organize-pages.import-and-convert-file-to-indesign.import-from-cc-libraries.v1
  feature_name: Import files from Creative Cloud libraries
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Import files from Creative Cloud libraries
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-create-lines-and-shapes-edit-and-style-paths-apply-and-save-line-stroke-styles-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.create-lines-and-shapes.edit-and-style-paths.apply-and-save-line-stroke-styles.v1
  feature_name: Apply and save line stroke styles
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: raster
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioRasterPipeline / Apply and save line stroke styles
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-format-and-style-text-text-styles-map-styles-to-export-tags-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.format-and-style-text.text-styles.map-styles-to-export-tags.v1
  feature_name: Map styles to export tags
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Map styles to export tags
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-get-started-cloud-document-management-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.get-started.cloud-document-management-options.v1
  feature_name: Cloud document management options
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Cloud document management options
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-get-started-settings-and-preferences-export-and-import-user-settings-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.get-started.settings-and-preferences.export-and-import-user-settings.v1
  feature_name: Export and import user settings
  support_kind: round_trip
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Export and import user settings
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-get-started-system-and-product-info-create-deploy-indesign-server-packages-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.get-started.system-and-product-info.create-deploy-indesign-server-packages.v1
  feature_name: Create and deploy InDesign server packages
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Create and deploy InDesign server packages
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-indexes-and-references-add-a-table-of-contents-export-tocs-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.indexes-and-references.add-a-table-of-contents.export-tocs.v1
  feature_name: Export a table of contents
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Export a table of contents
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-indexes-and-references-references-and-bookmarks-create-bookmarks-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.indexes-and-references.references-and-bookmarks.create-bookmarks.v1
  feature_name: Create bookmarks for PDFs
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Create bookmarks for PDFs
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-interactive-elements-and-forms-forms-and-pdfs-accessible-pdfs-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.interactive-elements-and-forms.forms-and-pdfs.accessible-pdfs.v1
  feature_name: Accessible PDFs
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Accessible PDFs
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-interactive-elements-and-forms-forms-and-pdfs-create-fillable-forms-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.interactive-elements-and-forms.forms-and-pdfs.create-fillable-forms.v1
  feature_name: Create fillable forms
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Create fillable forms
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-interactive-elements-and-forms-forms-and-pdfs-export-to-interactive-pdfs-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.interactive-elements-and-forms.forms-and-pdfs.export-to-interactive-pdfs.v1
  feature_name: Export to interactive PDFs
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export to interactive PDFs
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-interactive-elements-and-forms-forms-and-pdfs-interactive-pdf-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.interactive-elements-and-forms.forms-and-pdfs.interactive-pdf-options.v1
  feature_name: Interactive PDF options
  support_kind: fixture_required
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Interactive PDF options
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-interactive-elements-and-forms-forms-and-pdfs-preview-and-present-interactive-documents-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.interactive-elements-and-forms.forms-and-pdfs.preview-and-present-interactive-documents.v1
  feature_name: Preview and present interactive documents
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Preview and present interactive documents
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-interactive-elements-and-forms-forms-and-pdfs-set-reading-order-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.interactive-elements-and-forms.forms-and-pdfs.set-reading-order.v1
  feature_name: Set reading order
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Set reading order
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-interactive-elements-and-forms-forms-and-pdfs-use-tags-for-accessible-pdfs-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.interactive-elements-and-forms.forms-and-pdfs.use-tags-for-accessible-pdfs.v1
  feature_name: Use tags to create accessible PDFs
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Use tags to create accessible PDFs
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-language-and-proofing-glyphs-characters-and-expressions-open-and-view-glyphs-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.language-and-proofing.glyphs-characters-and-expressions.open-and-view-glyphs.v1
  feature_name: Open and view glyphs
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Open and view glyphs
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-language-and-proofing-glyphs-characters-and-expressions-save-and-manage-find-and-replace-queries-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.language-and-proofing.glyphs-characters-and-expressions.save-and-manage-find-and-replace-queries.v1
  feature_name: Save and manage find and replace queries
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Save and manage find and replace queries
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-language-and-proofing-manage-language-dictionaries-import-or-export-word-lists-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.language-and-proofing.manage-language-dictionaries.import-or-export-word-lists.v1
  feature_name: Import or export word lists
  support_kind: round_trip
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Import or export word lists
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-layout-and-grid-tools-grids-import-grid-formats-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.layout-and-grid-tools.grids.import-grid-formats.v1
  feature_name: Import grid formats from other documents
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Import grid formats from other documents
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-about-color-separations-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.about-color-separations.v1
  feature_name: About color separations
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / About color separations
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-about-ink-trapping-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.about-ink-trapping.v1
  feature_name: About ink trapping
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / About ink trapping
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-about-overprinting-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.about-overprinting.v1
  feature_name: About overprinting
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / About overprinting
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-change-the-black-overprint-setting-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.change-the-black-overprint-setting.v1
  feature_name: Change the black overprint setting
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Change the black overprint setting
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-color-output-options-for-composites-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.color-output-options-for-composites.v1
  feature_name: Color output options
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Color output options
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-create-color-separations-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.create-color-separations.v1
  feature_name: Create color separations
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Create color separations
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-overprint-paragraph-and-footnote-rules-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.overprint-paragraph-and-footnote-rules.v1
  feature_name: Overprint paragraph and footnote rules
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Overprint paragraph and footnote rules
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-overprint-strokes-and-fills-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.overprint-strokes-and-fills.v1
  feature_name: Overprint strokes and fills
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Overprint strokes and fills
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-prepare-documents-for-color-separation-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.prepare-documents-for-color-separation.v1
  feature_name: Prepare documents for color separation
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Prepare documents for color separation
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-preview-color-separations-and-ink-coverage-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.preview-color-separations-and-ink-coverage.v1
  feature_name: Preview color separations and ink coverage
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Preview color separations and ink coverage
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-print-a-composite-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.print-a-composite.v1
  feature_name: Print a composite
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Print a composite
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-print-gradients-as-separations-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.print-gradients-as-separations.v1
  feature_name: Print gradients as separations
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Print gradients as separations
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-print-objects-on-all-color-plates-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.print-objects-on-all-color-plates.v1
  feature_name: Print objects on all color plates
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Print objects on all color plates
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-save-and-print-color-separations-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.save-and-print-color-separations.v1
  feature_name: Save and print color separations
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Save and print color separations
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-set-the-trap-width-for-colors-next-to-black-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.set-the-trap-width-for-colors-next-to-black.v1
  feature_name: Set the trap width for colors next to black
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Set the trap width for colors next to black
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-simulate-spot-ink-overprinting-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.simulate-spot-ink-overprinting.v1
  feature_name: Simulate spot ink overprinting
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Simulate spot ink overprinting
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-trap-a-document-or-a-book-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.trap-a-document-or-a-book.v1
  feature_name: Trap a document or a book
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Trap a document or a book
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-trap-preset-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.trap-preset-options.v1
  feature_name: Trap preset options
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Trap preset options
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-color-output-and-separations-use-color-management-when-printing-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.color-output-and-separations.use-color-management-when-printing.v1
  feature_name: Use color management when printing
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Use color management when printing
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-ink-and-color-management-customize-spot-color-appearance-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.ink-and-color-management.customize-spot-color-appearance.v1
  feature_name: Customize spot color appearance
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Customize spot color appearance
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-ink-and-color-management-emulsion-and-image-exposure-settings-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.ink-and-color-management.emulsion-and-image-exposure-settings.v1
  feature_name: Emulsion and image exposure settings
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Emulsion and image exposure settings
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-ink-and-color-management-manage-inks-for-separation-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.ink-and-color-management.manage-inks-for-separation.v1
  feature_name: Manage inks for separation
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Manage inks for separation
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-ink-and-color-management-process-colors-cmyk-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.ink-and-color-management.process-colors-cmyk-overview.v1
  feature_name: Process colors (CMYK) overview
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Process colors (CMYK) overview
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-ink-and-color-management-specify-halftone-frequency-and-resolution-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.ink-and-color-management.specify-halftone-frequency-and-resolution.v1
  feature_name: Specify halftone frequency and resolution
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Specify halftone frequency and resolution
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-ink-and-color-management-use-spot-colors-for-printing-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.ink-and-color-management.use-spot-colors-for-printing.v1
  feature_name: Use spot colors for printing
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Use spot colors for printing
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-page-set-up-and-printer-marks-change-page-position-on-media-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.page-set-up-and-printer-marks.change-page-position-on-media.v1
  feature_name: Change page position on media
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: interactive
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Change page position on media
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-page-set-up-and-printer-marks-mixed-page-size-printing-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.page-set-up-and-printer-marks.mixed-page-size-printing.v1
  feature_name: Print documents with mixed page sizes
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Print documents with mixed page sizes
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-page-set-up-and-printer-marks-object-printing-options-in-indesign-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.page-set-up-and-printer-marks.object-printing-options-in-indesign.v1
  feature_name: Object printing options
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Object printing options
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-page-set-up-and-printer-marks-print-bleed-and-slug-areas-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.page-set-up-and-printer-marks.print-bleed-and-slug-areas.v1
  feature_name: Print bleed and slug areas
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Print bleed and slug areas
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-page-set-up-and-printer-marks-set-printer-marks-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.page-set-up-and-printer-marks.set-printer-marks.v1
  feature_name: Set printer's marks
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Set printer's marks
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-page-set-up-and-printer-marks-specify-page-range-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.page-set-up-and-printer-marks.specify-page-range.v1
  feature_name: Specify page range
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Specify page range
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-page-set-up-and-printer-marks-specify-paper-size-and-orientation-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.page-set-up-and-printer-marks.specify-paper-size-and-orientation.v1
  feature_name: Specify paper size and orientation
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Specify paper size and orientation
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-page-set-up-and-printer-marks-specify-which-layers-to-print-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.page-set-up-and-printer-marks.specify-which-layers-to-print.v1
  feature_name: Specify which layers to print
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: layer
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioLayerGraph / Specify which layers to print
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-preflight-configure-and-use-the-preflight-panel-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.preflight.configure-and-use-the-preflight-panel.v1
  feature_name: Configure and use the preflight panel
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Configure and use the preflight panel
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-preflight-create-and-manage-preflight-profiles-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.preflight.create-and-manage-preflight-profiles.v1
  feature_name: Create and manage preflight profiles
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: color
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioColorPipeline / Create and manage preflight profiles
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-preflight-live-preflighting-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.preflight.live-preflighting.v1
  feature_name: Turn on or off live preflight
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Turn on or off live preflight
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-preflight-package-files-for-output-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.preflight.package-files-for-output.v1
  feature_name: Package InDesign files for output
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Package InDesign files for output
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-preflight-preflight-book-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.preflight.preflight-book-files.v1
  feature_name: Preflight book files
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Preflight book files
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-booklets-adjust-creep-settings-in-booklets-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-booklets.adjust-creep-settings-in-booklets.v1
  feature_name: Adjust creep settings in booklets
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Adjust creep settings in booklets
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-booklets-booklet-printing-settings-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-booklets.booklet-printing-settings.v1
  feature_name: Spacing, bleed, and margin options for booklet printing
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Spacing, bleed, and margin options for booklet printing
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-booklets-booklet-types-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-booklets.booklet-types.v1
  feature_name: Booklet types
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Booklet types
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-booklets-impose-documents-for-booklet-printing-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-booklets.impose-documents-for-booklet-printing.v1
  feature_name: Impose documents for booklet printing
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Impose documents for booklet printing
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-booklets-preview-booklet-printing-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-booklets.preview-booklet-printing.v1
  feature_name: Preview booklet printing
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Preview booklet printing
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-production-and-file-creation-about-postscript-and-eps-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-production-and-file-creation.about-postscript-and-eps-files.v1
  feature_name: About PostScript and EPS files
  support_kind: export
  format_refs:
  - format_id: format.eps
    format_label: EPS
  - format_id: format.ps
    format_label: PostScript
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / About PostScript and EPS files
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-production-and-file-creation-create-and-manage-print-presets-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-production-and-file-creation.create-and-manage-print-presets.v1
  feature_name: Create and manage print presets
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Create and manage print presets
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-production-and-file-creation-create-postscript-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-production-and-file-creation.create-postscript-files.v1
  feature_name: Create PostScript files
  support_kind: export
  format_refs:
  - format_id: format.ps
    format_label: PostScript
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Create PostScript files
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-production-and-file-creation-print-as-bitmap-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-production-and-file-creation.print-as-bitmap.v1
  feature_name: Print as bitmap
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Print as bitmap
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-production-and-file-creation-print-documents-and-books-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-production-and-file-creation.print-documents-and-books.v1
  feature_name: Print documents and books
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Print documents and books
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-production-and-file-creation-print-or-export-book-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-production-and-file-creation.print-or-export-book-files.v1
  feature_name: Print or export book files
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Print or export book files
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-production-and-file-creation-print-oversized-documents-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-production-and-file-creation.print-oversized-documents.v1
  feature_name: Print oversized documents
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Print oversized documents
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-production-and-file-creation-produce-print-ready-pdf-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-production-and-file-creation.produce-print-ready-pdf-files.v1
  feature_name: Produce print-ready PDF files
  support_kind: export
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Produce print-ready PDF files
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-production-and-file-creation-scale-documents-for-printing-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-production-and-file-creation.scale-documents-for-printing.v1
  feature_name: Scale documents for printing
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Scale documents for printing
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-print-print-production-and-file-creation-set-up-a-printer-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.print.print-production-and-file-creation.set-up-a-printer.v1
  feature_name: Set up a printer
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Set up a printer
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-epub-accessibility-for-indexes-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-epub.accessibility-for-indexes.v1
  feature_name: Accessibility enhancements for indexes
  support_kind: export
  format_refs:
  - format_id: format.epub
    format_label: EPUB
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Accessibility enhancements for indexes
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-epub-add-aria-labels-to-objects-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-epub.add-aria-labels-to-objects.v1
  feature_name: Add ARIA labels to objects
  support_kind: export
  format_refs:
  - format_id: format.epub
    format_label: EPUB
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Add ARIA labels to objects
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-epub-add-aria-role-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-epub.add-aria-role.v1
  feature_name: Add ARIA role while exporting to EPUB
  support_kind: export
  format_refs:
  - format_id: format.epub
    format_label: EPUB
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Add ARIA role while exporting to EPUB
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-epub-adjust-text-resizing-for-accessibility-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-epub.adjust-text-resizing-for-accessibility.v1
  feature_name: Adjust text resizing for accessibility
  support_kind: export
  format_refs:
  - format_id: format.epub
    format_label: EPUB
  studio_primitive: typography
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Adjust text resizing for accessibility
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-epub-create-accessible-glossaries-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-epub.create-accessible-glossaries.v1
  feature_name: Create accessible glossaries
  support_kind: export
  format_refs:
  - format_id: format.epub
    format_label: EPUB
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Create accessible glossaries
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-epub-epub-accessibility-features-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-epub.epub-accessibility-features.v1
  feature_name: Accessibility features while exporting to EPUB
  support_kind: export
  format_refs:
  - format_id: format.epub
    format_label: EPUB
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Accessibility features while exporting to EPUB
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-epub-epub-export-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-epub.epub-export-options.v1
  feature_name: EPUB export options in InDesign
  support_kind: export
  format_refs:
  - format_id: format.epub
    format_label: EPUB
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / EPUB export options in InDesign
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-epub-export-to-epub-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-epub.export-to-epub.v1
  feature_name: Export to EPUB
  support_kind: export
  format_refs:
  - format_id: format.epub
    format_label: EPUB
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Export to EPUB
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-html-and-web-create-and-manage-articles-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-html-and-web.create-and-manage-articles.v1
  feature_name: Create and manage articles
  support_kind: export
  format_refs:
  - format_id: format.html
    format_label: HTML
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Create and manage articles
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-html-and-web-export-content-as-html5-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-html-and-web.export-content-as-html5.v1
  feature_name: Export content as HTML5
  support_kind: export
  format_refs:
  - format_id: format.html
    format_label: HTML
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Export content as HTML5
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-html-and-web-export-to-adobe-express-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-html-and-web.export-to-adobe-express.v1
  feature_name: Export to Adobe Express
  support_kind: export
  format_refs:
  - format_id: format.html
    format_label: HTML
  studio_primitive: export
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Export to Adobe Express
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-html-and-web-export-to-html-legacy-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-html-and-web.export-to-html-legacy.v1
  feature_name: Export to HTML (Legacy)
  support_kind: export
  format_refs:
  - format_id: format.html
    format_label: HTML
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Export to HTML (Legacy)
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-html-and-web-html-export-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-html-and-web.html-export-options.v1
  feature_name: HTML export options
  support_kind: export
  format_refs:
  - format_id: format.html
    format_label: HTML
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / HTML export options
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-export-to-html-and-web-html5-export-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.export-to-html-and-web.html5-export-options.v1
  feature_name: HTML5 export options
  support_kind: export
  format_refs:
  - format_id: format.html
    format_label: HTML
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / HTML5 export options
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-publish-work-online-publish-indesign-documents-online-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.publish-work-online.publish-indesign-documents-online.v1
  feature_name: Publish InDesign documents online
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Publish InDesign documents online
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-publish-work-online-set-up-google-analytics-for-published-documents-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.publish-work-online.set-up-google-analytics-for-published-documents.v1
  feature_name: Set up Google Analytics for published documents
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Set up Google Analytics for published documents
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-save-and-export-add-edit-file-metadata-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.save-and-export.add-edit-file-metadata.v1
  feature_name: Add, edit, and export file metadata
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Add, edit, and export file metadata
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-save-and-export-adobe-pdf-export-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.save-and-export.adobe-pdf-export-options.v1
  feature_name: Adobe PDF export options in InDesign
  support_kind: export
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Adobe PDF export options in InDesign
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-save-and-export-export-as-incopy-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.save-and-export.export-as-incopy-files.v1
  feature_name: Export documents as separate InCopy files
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Export documents as separate InCopy files
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-save-and-export-export-pdfs-for-printing-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.save-and-export.export-pdfs-for-printing.v1
  feature_name: Export PDFs for printing
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Export PDFs for printing
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-save-and-export-export-to-jpeg-png-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.save-and-export.export-to-jpeg-png.v1
  feature_name: Export pages, objects, and text
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Export pages, objects, and text
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-save-and-export-export-to-xml-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.save-and-export.export-to-xml.v1
  feature_name: Export to XML
  support_kind: export
  format_refs:
  - format_id: format.xml
    format_label: XML
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Export to XML
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-save-and-export-jpeg-and-png-export-options-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.save-and-export.jpeg-and-png-export-options.v1
  feature_name: JPEG and PNG export options
  support_kind: export
  format_refs:
  - format_id: format.png
    format_label: PNG
  - format_id: format.jpeg
    format_label: JPEG/JPG
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / JPEG and PNG export options
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-save-and-export-manage-pdf-presets-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.save-and-export.manage-pdf-presets.v1
  feature_name: Manage PDF presets
  support_kind: export
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Manage PDF presets
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-save-export-and-publish-save-and-export-save-documents-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.save-export-and-publish.save-and-export.save-documents.v1
  feature_name: Save documents
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Save documents
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-whats-new-indesign-beta-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.whats-new.indesign-beta-overview.v1
  feature_name: Adobe InDesign (Beta) overview
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Adobe InDesign (Beta) overview
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-whats-new-indesign-server-overview-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.whats-new.indesign-server-overview.v1
  feature_name: Adobe InDesign Server overview
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Adobe InDesign Server overview
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-whats-new-release-notes-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.whats-new.release-notes.v1
  feature_name: Release notes
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / Release notes
- compatibility_record_id: compat.feature.indesign.osd-indesign-indesign-leaf-whats-new-whats-new-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: indesign
  source_family: adobe
  source_feature_row_id: osd.indesign.indesign.leaf.whats-new.whats-new.v1
  feature_name: What's new in Adobe InDesign
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: export
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioExportRecipe / What's new in Adobe InDesign
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-get-started-learn-the-basics-supported-file-formats-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-supported-file-formats-html.v1
  feature_name: Supported file formats
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Supported file formats
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-start-a-new-file-new-document-dialog-overview-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-new-document-dialog-overview-html.v1
  feature_name: New Document dialog overview
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / New Document dialog overview
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-start-a-new-file-create-documents-using-presets-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-documents-using-presets-html.v1
  feature_name: Create documents using presets
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Create documents using presets
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-start-a-new-file-create-and-save-custom-document-presets-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-and-save-custom-document-presets-html.v1
  feature_name: Create and save custom document presets
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Create and save custom document presets
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-start-a-new-file-create-documents-using-blank-templates-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-documents-using-blank-templates-html.v1
  feature_name: Create documents using blank templates
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Create documents using blank templates
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-start-a-new-file-create-files-on-large-canvases-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-files-on-large-canvases-html.v1
  feature_name: Create files with large canvases
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Create files with large canvases
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-start-a-new-file-create-documents-using-templates-from-adobe-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-documents-using-templates-from-adobe.v1
  feature_name: Create documents using templates from Adobe Stock
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Create documents using templates from Adobe Stock
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-start-a-new-file-rotate-canvas-view-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-rotate-canvas-view-html.v1
  feature_name: Rotate canvas view
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Rotate canvas view
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-start-a-new-file-organize-share-and-collaborate-using-project-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-organize-share-and-collaborate-using-project.v1
  feature_name: Organize, share, and collaborate using Projects
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: local_first_collaboration_primitive
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Organize, share, and collaborate using Projects
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-start-a-new-file-access-projects-in-the-illustrator-workspace-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-access-projects-in-the-illustrator-workspace.v1
  feature_name: Access projects in the Illustrator workspace and other apps
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Access projects in the Illustrator workspace and other apps
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-start-a-new-file-find-and-edit-adobe-express-templates-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-find-and-edit-adobe-express-templates-html.v1
  feature_name: Find and edit Adobe Express templates
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Find and edit Adobe Express templates
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-import-from-other-apps-place-linked-photoshop-documents-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-place-linked-photoshop-documents-html.v1
  feature_name: Place linked Photoshop documents
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Place linked Photoshop documents
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-import-from-other-apps-move-paths-from-photoshop-to-illustrat-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-move-paths-from-photoshop-to-illustrat.v1
  feature_name: Move paths from Photoshop to Illustrator
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Move paths from Photoshop to Illustrator
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-import-from-other-apps-move-part-of-an-image-from-photoshop-t-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-move-part-of-an-image-from-photoshop-t.v1
  feature_name: Move part of an image from Photoshop to Illustrator
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Move part of an image from Photoshop to Illustrator
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-import-from-other-apps-photoshop-import-options-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-photoshop-import-options-html.v1
  feature_name: Photoshop import options
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Photoshop import options
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-import-from-other-apps-place-and-edit-adobe-firefly-output-in-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-place-and-edit-adobe-firefly-output-in.v1
  feature_name: Place and edit images generated on the Adobe Firefly website
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: ai
  provider_posture: provider_adapter_or_local_model_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioModelToolContract / Place and edit images generated on the Adobe Firefly website
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-import-other-file-types-import-adobe-pdf-files-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-adobe-pdf-files-html.v1
  feature_name: Import Adobe PDF files
  support_kind: import
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Import Adobe PDF files
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-import-other-file-types-adobe-pdf-placement-options-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-adobe-pdf-placement-options-html.v1
  feature_name: Place Adobe PDF files
  support_kind: import
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Place Adobe PDF files
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-import-other-file-types-import-autocad-files-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-autocad-files-html.v1
  feature_name: Import AutoCAD files
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Import AutoCAD files
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-import-other-file-types-import-monotone-duotone-and-tritone-i-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-monotone-duotone-and-tritone-i.v1
  feature_name: Import monotone, duotone, and tritone images from Adobe PDF files
  support_kind: import
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Import monotone, duotone, and tritone images from Adobe PDF files
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-import-other-file-types-import-dcs-files-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-dcs-files-html.v1
  feature_name: Import DCS files
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Import DCS files
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-manage-project-files-upload-download-project-files-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-manage-project-files-upload-download-project-files-html.v1
  feature_name: Upload and download project files in Illustrator
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Upload and download project files in Illustrator
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-manage-linked-and-embedded-files-links-panel-overview-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-manage-linked-and-embedded-files-links-panel-overview-html.v1
  feature_name: Links panel overview
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Links panel overview
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-manage-linked-and-embedded-files-relink-replace-or-update-lin-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-manage-linked-and-embedded-files-relink-replace-or-update-lin.v1
  feature_name: Relink, replace, or update linked files
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Relink, replace, or update linked files
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-add-and-import-files-manage-linked-and-embedded-files-embed-images-and-files-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-manage-linked-and-embedded-files-embed-images-and-files-html.v1
  feature_name: Embed and unembed images and files
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Embed and unembed images and files
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-use-generative-ai-generate-print-bleed-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-generate-print-bleed-html.v1
  feature_name: Generate print bleed
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: ai
  provider_posture: provider_adapter_or_local_model_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioModelToolContract / Generate print bleed
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-manage-objects-select-objects-save-object-selections-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-select-objects-save-object-selections-html.v1
  feature_name: Save object selections
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Save object selections
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-manage-objects-traces-mockups-symbols-save-image-trace-presets-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-save-image-trace-presets-html.v1
  feature_name: Save custom tracing presets
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Save custom tracing presets
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-manage-objects-traces-mockups-symbols-save-mockups-as-templates-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-save-mockups-as-templates-html.v1
  feature_name: Save mockups as templates
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Save mockups as templates
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-manage-objects-traces-mockups-symbols-create-and-place-symbols-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-create-and-place-symbols-html.v1
  feature_name: Create and place symbols
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Create and place symbols
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-manage-objects-traces-mockups-symbols-create-or-import-symbol-libraries-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-create-or-import-symbol-libraries-html.v1
  feature_name: Create or import symbol libraries
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Create or import symbol libraries
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-measure-and-align-grids-and-guides-use-distance-guides-for-accurate-placement-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-use-distance-guides-for-accurate-placement-html.v1
  feature_name: Use Distance Guides for accurate placement
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Use Distance Guides for accurate placement
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-paint-and-fill-apply-and-edit-strokes-import-brushes-from-brush-libraries-or-other-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-import-brushes-from-brush-libraries-or-other.v1
  feature_name: Import brushes
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: ai
  provider_posture: provider_adapter_or_local_model_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioModelToolContract / Import brushes
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-manage-colors-use-swatches-replace-merge-or-delete-swatches-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-replace-merge-or-delete-swatches-html.v1
  feature_name: Replace, merge, or delete swatches
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: local_first_collaboration_primitive
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Replace, merge, or delete swatches
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-manage-colors-use-swatches-create-and-open-swatch-libraries-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-and-open-swatch-libraries-html.v1
  feature_name: Create and open swatch libraries
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Create and open swatch libraries
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-design-with-text-add-manage-text-add-or-remove-placeholder-text-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-or-remove-placeholder-text-html.v1
  feature_name: Add or remove placeholder text
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Add or remove placeholder text
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-design-with-text-fonts-and-scripts-supported-fonts-in-illustrator-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-supported-fonts-in-illustrator-html.v1
  feature_name: Supported font file types in Adobe Illustrator
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Supported font file types in Adobe Illustrator
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-design-with-text-fonts-and-scripts-find-and-replace-fonts-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-find-and-replace-fonts-html.v1
  feature_name: Find and replace fonts
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Find and replace fonts
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-design-with-text-fonts-and-scripts-preview-add-or-replace-missing-fonts-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-preview-add-or-replace-missing-fonts-html.v1
  feature_name: Preview, add, or replace missing fonts
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Preview, add, or replace missing fonts
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-design-with-text-fonts-and-scripts-format-japanese-text-mojikumi-kinsoku-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-format-japanese-text-mojikumi-kinsoku-html.v1
  feature_name: Format Japanese text in Illustrator
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Format Japanese text in Illustrator
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-design-with-text-fonts-and-scripts-mojikumi-kinsoku-overview-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-mojikumi-kinsoku-overview-html.v1
  feature_name: Mojikumi and Kinsoku overview for Japanese text formatting
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Mojikumi and Kinsoku overview for Japanese text formatting
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-design-with-text-special-characters-glyphs-replace-characters-with-alternate-glyph-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-replace-characters-with-alternate-glyph.v1
  feature_name: Replace characters with alternate glyphs
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Replace characters with alternate glyphs
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-create-manage-artboards-organize-manage-artboards-export-artboards-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-export-artboards-html.v1
  feature_name: Export selected artboards
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Export selected artboards
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-special-effects-styles-apply-filter-effects-apply-svg-filter-effects-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-apply-svg-filter-effects-html.v1
  feature_name: Apply SVG filter effects
  support_kind: fixture_required
  format_refs:
  - format_id: format.svg
    format_label: SVG
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Apply SVG filter effects
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-special-effects-styles-apply-filter-effects-work-with-svg-interactivity-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-work-with-svg-interactivity-html.v1
  feature_name: Work with SVG Interactivity
  support_kind: fixture_required
  format_refs:
  - format_id: format.svg
    format_label: SVG
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Work with SVG Interactivity
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-special-effects-styles-create-3d-graphics-export-3d-vector-artwork-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-export-3d-vector-artwork-html.v1
  feature_name: Export 3D vector artwork
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Export 3D vector artwork
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-automate-visualize-data-automate-actions-play-actions-on-a-batch-of-files-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-play-actions-on-a-batch-of-files-html.v1
  feature_name: Play actions on a batch of files
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Play actions on a batch of files
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-automate-visualize-data-automate-actions-set-up-data-source-files-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-set-up-data-source-files-html.v1
  feature_name: Set up data source files
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Set up data source files
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-automate-visualize-data-automate-actions-import-data-source-files-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-import-data-source-files-html.v1
  feature_name: Import data source files
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Import data source files
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-automate-visualize-data-visualize-data-format-columns-bars-and-lines-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-format-columns-bars-and-lines-html.v1
  feature_name: Format columns, bars, and lines
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Format columns, bars, and lines
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-automate-visualize-data-visualize-data-format-pie-graphs-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-format-pie-graphs-html.v1
  feature_name: Format pie graphs
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Format pie graphs
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-automate-visualize-data-visualize-data-format-the-text-in-graphs-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-format-the-text-in-graphs-html.v1
  feature_name: Format graph text
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: typography
  provider_posture: local_primitive
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioTextRunAndStory / Format graph text
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-save-and-export-export-files-to-different-formats-export-to-cloud-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-save-and-export-export-files-to-different-formats-export-to-cloud-html.v1
  feature_name: Export to Adobe cloud storage
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: provider_adapter_or_local_model_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Export to Adobe cloud storage
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-save-and-export-export-files-to-different-formats-export-for-screens-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-save-and-export-export-files-to-different-formats-export-for-screens-html.v1
  feature_name: Export for screens
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Export for screens
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-desktop-save-and-export-export-to-other-apps-export-assets-to-firefly-boards-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.desktop-save-and-export-export-to-other-apps-export-assets-to-firefly-boards-html.v1
  feature_name: Export assets to Firefly Boards
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: ai
  provider_posture: provider_adapter_or_local_model_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioModelToolContract / Export assets to Firefly Boards
- compatibility_record_id: compat.feature.illustrator.osd-illustrator-illustrator-desktop-leaf-kb-supported-file-formats-illustrator-html-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: illustrator
  source_family: adobe
  source_feature_row_id: osd.illustrator.illustrator.desktop.leaf.kb-supported-file-formats-illustrator-html.v1
  feature_name: Supported file formats
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Supported file formats
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-photo-desktop-leaf-appendix-fileformat-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-photo.desktop.leaf.appendix-fileformat.v1
  feature_name: Supported file formats
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Supported file formats
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-photo-desktop-leaf-exportpersona-exportoptionspanel-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-photo.desktop.leaf.exportpersona-exportoptionspanel.v1
  feature_name: Export Options panel
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Export Options panel
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-photo-desktop-leaf-exportpersona-exportpersona-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-photo.desktop.leaf.exportpersona-exportpersona.v1
  feature_name: Exporting using Export Persona
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Exporting using Export Persona
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-photo-desktop-leaf-exportpersona-exportsettings-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-photo.desktop.leaf.exportpersona-exportsettings.v1
  feature_name: Export Settings
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Export Settings
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-photo-desktop-leaf-getstarted-opendocument-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-opendocument.v1
  feature_name: Open documents and images
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Open documents and images
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-photo-desktop-leaf-getstarted-save-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-save.v1
  feature_name: Save
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Save
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-photo-desktop-leaf-sharing-export-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-photo.desktop.leaf.sharing-export.v1
  feature_name: Export
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Export
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-photo-desktop-leaf-sharing-print-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-photo.desktop.leaf.sharing-print.v1
  feature_name: Print
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Print
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-designer-desktop-leaf-appendix-fileformat-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-designer.desktop.leaf.appendix-fileformat.v1
  feature_name: Import and export file formats
  support_kind: round_trip
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Import and export file formats
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-designer-desktop-leaf-exportpersona-exportoptionspanel-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-designer.desktop.leaf.exportpersona-exportoptionspanel.v1
  feature_name: Export Options panel
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Export Options panel
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-designer-desktop-leaf-exportpersona-exportpersona-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-designer.desktop.leaf.exportpersona-exportpersona.v1
  feature_name: Exporting using Export Persona
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Exporting using Export Persona
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-designer-desktop-leaf-exportpersona-exportpersona-layerspanel-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-designer.desktop.leaf.exportpersona-exportpersona-layerspanel.v1
  feature_name: Layers panel (Export Persona)
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: layer
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioLayerGraph / Layers panel (Export Persona)
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-designer-desktop-leaf-exportpersona-exportsettings-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-designer.desktop.leaf.exportpersona-exportsettings.v1
  feature_name: Export Settings
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Export Settings
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-designer-desktop-leaf-getstarted-importpdf-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-importpdf.v1
  feature_name: Importing PDF documents
  support_kind: fixture_required
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Importing PDF documents
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-designer-desktop-leaf-getstarted-opendocument-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-opendocument.v1
  feature_name: Open documents and images
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Open documents and images
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-designer-desktop-leaf-getstarted-save-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-save.v1
  feature_name: Save
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Save
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-designer-desktop-leaf-sharesaveprint-createsvg-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-designer.desktop.leaf.sharesaveprint-createsvg.v1
  feature_name: Working with SVGs
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Working with SVGs
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-designer-desktop-leaf-sharesaveprint-export-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-designer.desktop.leaf.sharesaveprint-export.v1
  feature_name: Export
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Export
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-designer-desktop-leaf-sharesaveprint-print-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-designer.desktop.leaf.sharesaveprint-print.v1
  feature_name: Print
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: optional_integration
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Print
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-designer-desktop-leaf-tools-tools-placeimage-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-placeimage.v1
  feature_name: Place Tool
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Place Tool
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-publisher-desktop-leaf-advanced-pdfbookmarks-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-pdfbookmarks.v1
  feature_name: PDF bookmarks
  support_kind: fixture_required
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / PDF bookmarks
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-publisher-desktop-leaf-appendix-fileformat-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-publisher.desktop.leaf.appendix-fileformat.v1
  feature_name: Supported file formats
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Supported file formats
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-publisher-desktop-leaf-getstarted-importpdf-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-importpdf.v1
  feature_name: Importing PDF documents
  support_kind: fixture_required
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Importing PDF documents
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-publisher-desktop-leaf-getstarted-opendocument-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-opendocument.v1
  feature_name: Open documents
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: vector
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioVectorPathGraph / Open documents
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-publisher-desktop-leaf-getstarted-save-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-save.v1
  feature_name: Save
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Save
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-publisher-desktop-leaf-publishing-accessiblepdfs-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-accessiblepdfs.v1
  feature_name: Accessible PDFs
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Accessible PDFs
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-publisher-desktop-leaf-publishing-export-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-export.v1
  feature_name: Export as graphic
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Export as graphic
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-publisher-desktop-leaf-publishing-exportsettings-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-exportsettings.v1
  feature_name: Export Settings
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Export Settings
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-publisher-desktop-leaf-publishing-print-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-print.v1
  feature_name: Print
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Print
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-publisher-desktop-leaf-publishing-publishpdffiles-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-publishpdffiles.v1
  feature_name: Publishing PDF files
  support_kind: fixture_required
  format_refs:
  - format_id: format.pdf
    format_label: PDF
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Publishing PDF files
- compatibility_record_id: compat.feature.affinity.osd-affinity-affinity-publisher-desktop-leaf-tools-tools-placeimage-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: affinity
  source_family: affinity
  source_feature_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-placeimage.v1
  feature_name: Place Tool
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: page_layout
  provider_posture: local_primitive_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Place Tool
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-40826832449303-turn-webpages-into-editable-design-layers-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.40826832449303-turn-webpages-into-editable-design-layers.v1
  feature_name: Turn webpages into editable design layers
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Turn webpages into editable design layers
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-360025508373-publish-a-library-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.360025508373-publish-a-library.v1
  feature_name: Publish a library
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: provider_adapter_or_local_model_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Publish a library
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-360041486873-use-animated-gifs-in-prototypes-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.360041486873-use-animated-gifs-in-prototypes.v1
  feature_name: Use animated GIFs in prototypes
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Use animated GIFs in prototypes
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-41307983648407-export-animations-from-figma-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.41307983648407-export-animations-from-figma.v1
  feature_name: Export animations from Figma
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Export animations from Figma
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-360041003114-import-files-to-the-file-browser-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.360041003114-import-files-to-the-file-browser.v1
  feature_name: Import files to the file browser
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Import files to the file browser
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-360040514273-import-sketch-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.360040514273-import-sketch-files.v1
  feature_name: Import Sketch files
  support_kind: import
  format_refs:
  - format_id: format.sketch
    format_label: Sketch
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Import Sketch files
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-360040028114-export-static-designs-from-figma-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.360040028114-export-static-designs-from-figma.v1
  feature_name: Export static designs from Figma
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Export static designs from Figma
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-13402894554519-export-formats-and-settings-for-static-designs-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.13402894554519-export-formats-and-settings-for-static-designs.v1
  feature_name: Export formats and settings for static designs
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Export formats and settings for static designs
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-360040028114-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.360040028114.v1
  feature_name: Export assets
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Export assets
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-13402894554519-export-formats-and-settings-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.13402894554519-export-formats-and-settings.v1
  feature_name: Export settings
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Export settings
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-41307983648407-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.41307983648407.v1
  feature_name: Export animations
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Export animations
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-8403626871063-save-a-local-copy-of-files-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.8403626871063-save-a-local-copy-of-files.v1
  feature_name: Export a design file
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Export a design file
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-360041486873-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.360041486873.v1
  feature_name: Use animated GIFs in prototypes
  support_kind: fixture_required
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Use animated GIFs in prototypes
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-360041003114-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.360041003114.v1
  feature_name: Import files to the file browser
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Import files to the file browser
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-22012921621015-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.22012921621015.v1
  feature_name: export or download assets in Dev Mode
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / export or download assets in Dev Mode
- compatibility_record_id: compat.feature.figma.osd-figma-figma-platform-leaf-13402894554519-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.platform.leaf.13402894554519.v1
  feature_name: Figma's export formats and settings ?
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Figma's export formats and settings ?
- compatibility_record_id: compat.feature.figma.osd-figma-figma-figjam-leaf-import-export-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.figjam.leaf.import-export.v1
  feature_name: Import and export with FigJam
  support_kind: round_trip
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Import and export with FigJam
- compatibility_record_id: compat.feature.figma.osd-figma-figma-figjam-leaf-spreadsheet-data-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.figjam.leaf.spreadsheet-data.v1
  feature_name: Import spreadsheet data, images, and designs to FigJam
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Import spreadsheet data, images, and designs to FigJam
- compatibility_record_id: compat.feature.figma.osd-figma-figma-figjam-leaf-media-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.figjam.leaf.media.v1
  feature_name: Place images, video, and GIFs in FigJam
  support_kind: import
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: compatibility_shim
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioFileIO / Place images, video, and GIFs in FigJam
- compatibility_record_id: compat.feature.figma.osd-figma-figma-slides-leaf-category-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.slides.leaf.category.v1
  feature_name: Slide decks, templates, prototypes in slides, presenter notes, presentation, PowerPoint import, and export
  support_kind: round_trip
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: local_primitive
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Slide decks, templates, prototypes in slides, presenter notes, presentation, PowerPoint import, and export
- compatibility_record_id: compat.feature.figma.osd-figma-figma-sites-leaf-category-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.sites.leaf.category.v1
  feature_name: Responsive sites, breakpoints, blocks, embeds, CMS, interactions, preview, and publish
  support_kind: export
  format_refs:
  - format_id: format.site
    format_label: SITE local copy
  studio_primitive: file_io
  provider_posture: provider_adapter_or_local_model_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioPageSpread / Responsive sites, breakpoints, blocks, embeds, CMS, interactions, preview, and publish
- compatibility_record_id: compat.feature.figma.osd-figma-figma-community-leaf-category-v1.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  - COMPAT-S01
  source_app_key: figma
  source_family: figma
  source_feature_row_id: osd.figma.figma.community.leaf.category.v1
  feature_name: Community resources, templates, plugins, widgets, shaders, duplicate and publish flows
  support_kind: export
  format_refs:
  - format_id: format.unspecified
    format_label: Unspecified source-format workflow
  studio_primitive: file_io
  provider_posture: provider_adapter_or_local_model_candidate
  fixture_requirement: required_before_claiming_format_compatibility
  round_trip_rule: record preserved translated lossy and unsupported constructs in import/export receipts
  manual_topic_candidate: Studio / StudioCollaborationSession / Community resources, templates, plugins, widgets, shaders, duplicate and publish flows
```

</topic>

<topic id="sources" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Sources for the generated file-format compatibility registry.">

### [SFR-FILE-FORMAT-COMPATIBILITY-REGISTRY.sources] Sources

```yaml
sources:
- id: COMPAT-S01
  path: 33-online-source-distilled-feature-ledger.md
  note: Canonical file-format compatibility policy.
- id: COMPAT-PHOTOSHOP
  path: 34-photoshop-source-distilled-domain-ledger.md
  note: Photoshop format mentions.
- id: COMPAT-INDESIGN
  path: 35-indesign-source-distilled-domain-ledger.md
  note: InDesign format mentions.
- id: COMPAT-ILLUSTRATOR
  path: 36-illustrator-source-distilled-domain-ledger.md
  note: Illustrator format mentions.
- id: COMPAT-AFFINITY
  path: 37-affinity-source-distilled-domain-ledger.md
  note: Affinity format mentions.
- id: COMPAT-FIGMA
  path: 38-figma-source-distilled-domain-ledger.md
  note: Figma format mentions.
- id: COMPAT-ROWS
  path: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
  note: Generated feature rows with compatibility posture.
```

</topic>
