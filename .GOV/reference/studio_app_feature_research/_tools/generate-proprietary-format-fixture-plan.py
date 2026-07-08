import collections
import pathlib
import re

import yaml


ROOT = pathlib.Path(__file__).resolve().parents[1]
UPDATED_AT = "2026-07-05"

PRIVATE_SCHEMA_POSTURE = {
    "format.afdesign": "vendor_private_native_document",
    "format.afphoto": "vendor_private_native_document",
    "format.afpub": "vendor_private_native_document",
    "format.ai": "vendor_private_or_pdf_compatible_native_document",
    "format.ait": "vendor_private_template_document",
    "format.buzz": "vendor_private_local_copy_document",
    "format.deck": "vendor_private_local_copy_document",
    "format.fig": "vendor_private_local_copy_document",
    "format.idml": "documented_interchange_xml_with_source_behavior_fixtures",
    "format.indd": "vendor_private_native_document",
    "format.jam": "vendor_private_local_copy_document",
    "format.make": "vendor_private_local_copy_document",
    "format.psb": "partly_documented_large_native_document",
    "format.psd": "partly_documented_native_document",
    "format.site": "vendor_private_local_copy_document",
}

FORMAT_DOMAIN_HINTS = {
    "format.afdesign": ["vector", "layer", "color", "typography", "export"],
    "format.afphoto": ["raster", "layer", "mask", "color", "raw", "export"],
    "format.afpub": ["page_layout", "typography", "tables", "prepress", "export"],
    "format.ai": ["vector", "typography", "color", "layer", "export"],
    "format.ait": ["vector", "typography", "style_system", "export"],
    "format.buzz": ["brand_assets", "design_systems", "export"],
    "format.deck": ["presentation", "typography", "interactive", "export"],
    "format.fig": ["design_systems", "vector", "page_layout", "prototype", "export"],
    "format.idml": ["page_layout", "typography", "tables", "prepress", "export"],
    "format.indd": ["page_layout", "typography", "tables", "prepress", "export"],
    "format.jam": ["whiteboard", "collaboration", "file_io", "export"],
    "format.make": ["ai", "web", "dev_mode", "export"],
    "format.psb": ["raster", "layer", "mask", "color", "export"],
    "format.psd": ["raster", "layer", "mask", "typography", "color", "export"],
    "format.site": ["web", "interactive", "design_systems", "export"],
}


def yaml_blocks(text: str):
    return re.findall(r"```yaml\n(.*?)\n```", text, flags=re.S)


def load_format_registry():
    text = (ROOT / "46-file-format-compatibility-registry.md").read_text(encoding="utf-8")
    for block in yaml_blocks(text):
        data = yaml.safe_load(block)
        if isinstance(data, dict) and "format_family_matrix" in data:
            return data["format_family_matrix"], data["compatibility_records"]
    raise RuntimeError("No format registry records found")


def slug(value: str):
    return re.sub(r"[^a-z0-9]+", "-", str(value).lower()).strip("-")


def fixture_families(format_id: str, source_apps: list[str]):
    domains = FORMAT_DOMAIN_HINTS.get(format_id, ["file_io", "export"])
    families = [
        "empty_minimal_document",
        "metadata_and_color_profile",
        "linked_and_embedded_assets",
        "text_font_and_missing_font_cases",
        "unsupported_feature_probe",
    ]
    if any(domain in domains for domain in ["raster", "mask", "raw"]):
        families += ["layers_masks_adjustments", "smart_or_live_filters", "bit_depth_hdr_and_transparency"]
    if any(domain in domains for domain in ["vector", "design_systems"]):
        families += ["paths_shapes_booleans_symbols", "gradients_patterns_appearances", "components_instances_variables"]
    if any(domain in domains for domain in ["page_layout", "prepress", "tables"]):
        families += ["pages_spreads_masters", "threaded_text_tables_footnotes", "preflight_bleed_package_pdf"]
    if any(domain in domains for domain in ["whiteboard", "presentation", "web", "interactive"]):
        families += ["interactive_nodes_or_frames", "comments_history_and_collaboration_artifacts", "export_publish_state"]
    if any(domain in domains for domain in ["ai", "dev_mode"]):
        families += ["generated_or_provider_backed_nodes", "code_api_or_dev_handoff_artifacts"]
    if "figma" in source_apps:
        families += ["local_copy_version_skew", "library_component_detachment", "multiplayer_history_loss_probe"]
    if "affinity" in source_apps:
        families += ["studiolink_persona_state", "affinity_live_adjustment_stack"]
    if "illustrator" in source_apps:
        families += ["pdf_compatible_ai_toggle", "legacy_ai_version_fixture", "effects_appearance_expansion"]
    if "indesign" in source_apps:
        families += ["idml_vs_indd_comparison", "book_and_package_dependency_fixture"]
    if "photoshop" in source_apps:
        families += ["psd_psb_size_boundary", "smart_object_round_trip_fixture"]
    return sorted(dict.fromkeys(families))


def support_directions(family: dict):
    directions = sorted({direction for values in family.get("support_by_app", {}).values() for direction in values})
    if "round_trip" in directions:
        return ["import", "edit_preserve", "export", "round_trip"]
    return directions


def row_for_family(family: dict, records_by_format: dict):
    format_id = family["format_id"]
    source_apps = family.get("source_apps_present", [])
    support = support_directions(family)
    domains = FORMAT_DOMAIN_HINTS.get(format_id, ["file_io", "export"])
    labels = family.get("format_labels", [])
    schema_posture = PRIVATE_SCHEMA_POSTURE.get(format_id, "source_observable_format")
    return {
        "fixture_plan_id": f"format-fixture.{slug(format_id)}.v1",
        "format_id": format_id,
        "format_labels": labels,
        "source_apps_present": source_apps,
        "schema_public_status": schema_posture,
        "compatibility_posture": family.get("compatibility_posture"),
        "support_by_app": family.get("support_by_app", {}),
        "required_support_directions": support,
        "studio_primitive_domains": domains,
        "rust_implementation_lanes": [f"studio_{domain}" for domain in domains],
        "fixture_families": fixture_families(format_id, source_apps),
        "minimum_fixture_count_rule": "at_least_one_fixture_per_fixture_family_per_supported_app_and_direction",
        "round_trip_assertions": [
            "open_without_crash",
            "preserve_document_graph_or_emit_explicit_unsupported_feature_receipt",
            "preserve_visible_render_for_supported_features",
            "preserve linked or embedded asset references where supported",
            "preserve color profile and units where supported",
            "export_or_save_with_deterministic_receipt",
            "reopen_exported_output_and_compare_supported_state",
        ],
        "unsupported_feature_policy": [
            "do_not_silently_drop_source_data",
            "emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice",
            "keep original source blob or substructure when preservation is possible",
            "mark lossy conversion in the command receipt and internal Studio UserManual topic",
        ],
        "receipt_required_fields": [
            "format_id",
            "source_app_key",
            "fixture_id",
            "operation_direction",
            "parser_version",
            "writer_version",
            "preserved_features",
            "converted_features",
            "unsupported_features",
            "dropped_features",
            "render_comparison_result",
            "round_trip_result",
            "recovery_steps",
        ],
        "manual_topic_candidate": f"Studio / File Compatibility / {' '.join(labels) if labels else format_id}",
        "implementation_readiness": "needs_fixture_corpus_before_product_parity_claim",
        "compatibility_record_refs": sorted(records_by_format.get(format_id, [])),
    }


def main():
    families, records = load_format_registry()
    records_by_format = collections.defaultdict(list)
    for record in records:
        for fmt in record.get("format_refs", []):
            records_by_format[fmt.get("format_id")].append(record.get("compatibility_record_id"))

    target_families = [
        family for family in families if family.get("compatibility_posture") == "native_round_trip_target"
    ]
    rows = [row_for_family(family, records_by_format) for family in target_families]
    by_app = collections.Counter(app for row in rows for app in row["source_apps_present"])
    by_schema = collections.Counter(row["schema_public_status"] for row in rows)
    by_direction = collections.Counter(direction for row in rows for direction in row["required_support_directions"])

    coverage = {
        "format_fixture_plan_count": len(rows),
        "source_format_family_count": len(families),
        "source_compatibility_record_count": len(records),
        "target_selection_rule": "format families with compatibility_posture native_round_trip_target",
        "format_fixture_targets_by_app": dict(sorted(by_app.items())),
        "schema_public_status_counts": dict(sorted(by_schema.items())),
        "support_direction_counts": dict(sorted(by_direction.items())),
        "policy": {
            "compatibility_rule": "Preserve compatibility with source creative formats through fixture-driven import, export, edit-preserve, and round-trip contracts.",
            "no_new_interchange_rule": "Do not invent a replacement interchange format for Studio parity scope.",
            "private_schema_rule": "Undocumented vendor-private structures are handled through fixtures, preservation blobs, explicit unsupported-feature receipts, and lossy-conversion diagnostics.",
            "claim_rule": "A format is not parity-complete until representative fixtures pass and unsupported features are documented in receipts and the Studio UserManual.",
        },
    }

    frontmatter = {
        "file_id": "50-proprietary-format-fixture-plan",
        "file_kind": "proprietary_format_fixture_plan",
        "topic_id": "SFR-PROPRIETARY-FORMAT-FIXTURE-PLAN",
        "title": "Proprietary Format Fixture Plan",
        "status": "draft",
        "summary": "Generated fixture and receipt plan for native, proprietary, local-copy, and round-trip source creative formats that Studio must preserve without inventing a replacement interchange format.",
        "updated_at": UPDATED_AT,
        "format_fixture_plan_count": len(rows),
        "source_format_family_count": len(families),
    }

    source_block = {
        "sources": [
            {"id": "FORMAT-FIXTURE-S01", "path": "46-file-format-compatibility-registry.md", "note": "Source-distilled file-format compatibility registry."},
            {"id": "FORMAT-FIXTURE-S02", "path": "49-source-coverage-verification-matrix.md", "note": "Coverage matrix proving source URL and local snapshot evidence for feature rows."},
            {"id": "FORMAT-FIXTURE-S03", "path": "47-studio-rust-implementation-backlog.md", "note": "Implementation-facing primitive backlog."},
            {"id": "FORMAT-FIXTURE-S04", "path": "_tools/generate-proprietary-format-fixture-plan.py", "note": "Generator for this fixture plan."},
        ]
    }

    text = "---\n"
    text += yaml.safe_dump(frontmatter, sort_keys=False, allow_unicode=False)
    text += "---\n\n"
    text += "## [SFR-PROPRIETARY-FORMAT-FIXTURE-PLAN] Proprietary Format Fixture Plan\n\n"
    text += '<topic id="format-fixture-summary" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Coverage summary for proprietary/native format fixture planning.">\n\n'
    text += "### [SFR-PROPRIETARY-FORMAT-FIXTURE-PLAN.summary] Fixture Plan Summary\n\n"
    text += "```yaml\n"
    text += yaml.safe_dump({"format_fixture_plan_summary": coverage}, sort_keys=False, allow_unicode=False, width=1400)
    text += "```\n\n</topic>\n\n"
    text += '<topic id="format-fixture-rows" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Machine-readable fixture plan rows for native and proprietary format targets.">\n\n'
    text += "### [SFR-PROPRIETARY-FORMAT-FIXTURE-PLAN.rows] Fixture Plan Rows\n\n"
    text += "```yaml\n"
    text += yaml.safe_dump({"format_fixture_plan_rows": rows}, sort_keys=False, allow_unicode=False, width=1400)
    text += "```\n\n</topic>\n\n"
    text += '<topic id="format-fixture-sources" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Sources for proprietary format fixture plan.">\n\n'
    text += "### [SFR-PROPRIETARY-FORMAT-FIXTURE-PLAN.sources] Sources\n\n"
    text += "```yaml\n"
    text += yaml.safe_dump(source_block, sort_keys=False, allow_unicode=False, width=1400)
    text += "```\n\n</topic>\n"

    (ROOT / "50-proprietary-format-fixture-plan.md").write_text(text, encoding="utf-8")
    print(f"wrote 50-proprietary-format-fixture-plan.md with {len(rows)} fixture plan rows")


if __name__ == "__main__":
    main()
