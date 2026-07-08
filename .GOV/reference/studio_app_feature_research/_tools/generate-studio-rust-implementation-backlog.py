import collections
import pathlib
import re

import yaml


ROOT = pathlib.Path(__file__).resolve().parents[1]
UPDATED_AT = "2026-07-05"

FEATURE_ROW_FILES = {
    "photoshop": "39-photoshop-source-distilled-feature-rows.md",
    "indesign": "40-indesign-source-distilled-feature-rows.md",
    "illustrator": "41-illustrator-source-distilled-feature-rows.md",
    "affinity": "42-affinity-source-distilled-feature-rows.md",
    "figma": "43-figma-source-distilled-feature-rows.md",
}


def yaml_blocks(text: str):
    return re.findall(r"```yaml\n(.*?)\n```", text, flags=re.S)


def load_feature_rows(file_name: str):
    for block in yaml_blocks((ROOT / file_name).read_text(encoding="utf-8")):
        data = yaml.safe_load(block)
        if isinstance(data, dict) and "source_distilled_feature_rows" in data:
            return data["source_distilled_feature_rows"]
    raise RuntimeError(f"No feature rows in {file_name}")


def load_tool_rows():
    for block in yaml_blocks((ROOT / "45-source-distilled-tool-registry.md").read_text(encoding="utf-8")):
        data = yaml.safe_load(block)
        if isinstance(data, dict) and "tool_registry_rows" in data:
            return data["tool_registry_rows"]
    raise RuntimeError("No tool_registry_rows in 45-source-distilled-tool-registry.md")


def load_compat_records():
    for block in yaml_blocks((ROOT / "46-file-format-compatibility-registry.md").read_text(encoding="utf-8")):
        data = yaml.safe_load(block)
        if isinstance(data, dict) and "compatibility_records" in data:
            return data["compatibility_records"], data["format_family_matrix"]
    raise RuntimeError("No compatibility records in 46-file-format-compatibility-registry.md")


def load_primitive_map():
    for block in yaml_blocks((ROOT / "05-studio-primitive-map.md").read_text(encoding="utf-8")):
        data = yaml.safe_load(block)
        if isinstance(data, dict) and "primitive_mappings" in data:
            return {row["primitive_domain"]: row for row in data["primitive_mappings"]}
    raise RuntimeError("No primitive_mappings in 05-studio-primitive-map.md")


def pascal(value: str):
    return "".join(part.capitalize() for part in re.split(r"[^a-zA-Z0-9]+", value) if part)


def derived_mapping(domain: str):
    return {
        "primitive_domain": domain,
        "studio_primitive": f"Studio{pascal(domain)}",
        "engine_module": f"studio_{domain}",
        "state_authority": "derived candidate from source-distilled research; promote into 05-studio-primitive-map.md before implementation",
        "model_tool_surface": f"studio.{domain}.mutate_or_execute",
        "diagnostics": ["needs_diagnostic_contract"],
        "app_references": [],
    }


def compat_primitive(record: dict):
    if record.get("studio_primitive"):
        return record["studio_primitive"]
    domains = record.get("studio_primitive_domains") or []
    if domains:
        return domains[0]
    return "file_io"


def high_roi_additions(domain: str):
    common = [
        "typed Rust command contract",
        "model-visible receipt",
        "undo/replay proof",
        "internal Studio UserManual topic",
    ]
    if domain in {"file_io", "export", "prepress"}:
        return common + ["format fixtures", "unsupported-feature diagnostics", "round-trip report"]
    if domain in {"ai", "collaboration"}:
        return common + ["offline fallback", "optional provider adapter", "attribution and recovery receipts"]
    if domain in {"layer", "raster", "vector", "typography", "page_layout"}:
        return common + ["visual regression fixture", "state snapshot diagnostics", "performance guard"]
    return common + ["source-app behavior comparison fixture"]


def verification_needs(domain: str):
    needs = [
        "exact source-page or app-behavior inspection before implementation",
        "command-contract acceptance criteria",
        "receipt schema validation",
        "same-change Studio UserManual update",
    ]
    if domain in {"file_io", "export", "prepress"}:
        needs += ["import/export fixture set", "round-trip unsupported-feature report"]
    if domain in {"raster", "vector", "typography", "page_layout", "layer"}:
        needs += ["golden render or state fixture", "undo/redo replay test"]
    if domain in {"ai", "collaboration"}:
        needs += ["offline behavior test", "provider-adapter mock test"]
    return needs


def main():
    feature_rows_by_app = {app: load_feature_rows(file_name) for app, file_name in FEATURE_ROW_FILES.items()}
    all_feature_rows = []
    for app_key, rows in feature_rows_by_app.items():
        for row in rows:
            enriched = dict(row)
            enriched["source_app_key"] = app_key
            all_feature_rows.append(enriched)
    tool_rows = load_tool_rows()
    compat_records, format_matrix = load_compat_records()
    primitive_map = load_primitive_map()

    domains = sorted(
        set(row.get("primitive_domain") for row in all_feature_rows)
        | set(row.get("primary_studio_primitive") for row in tool_rows)
        | set(compat_primitive(row) for row in compat_records)
    )

    feature_by_domain = collections.defaultdict(list)
    for row in all_feature_rows:
        feature_by_domain[row.get("primitive_domain")].append(row)

    tool_by_domain = collections.defaultdict(list)
    for row in tool_rows:
        tool_by_domain[row.get("primary_studio_primitive")].append(row)

    compat_by_domain = collections.defaultdict(list)
    for row in compat_records:
        compat_by_domain[compat_primitive(row)].append(row)

    backlog = []
    for domain in domains:
        mapping = primitive_map.get(domain) or derived_mapping(domain)
        features = feature_by_domain[domain]
        tools = tool_by_domain[domain]
        compat = compat_by_domain[domain]
        source_apps = sorted(
            set(row.get("source_app_key") for row in features)
            | set(row.get("source_app_key") for row in tools)
            | set(row.get("source_app_key") for row in compat)
        )
        provider_counts = collections.Counter(row.get("provider_posture", "not_applicable") for row in features)
        compatibility_counts = collections.Counter(row.get("file_format_compatibility", "not_applicable") for row in features)
        format_refs = sorted(
            {
                fmt["format_id"]
                for record in compat
                for fmt in record.get("format_refs", [])
            }
        )
        backlog.append(
            {
                "backlog_id": f"studio.backlog.{domain}.v1",
                "primitive_domain": domain,
                "mapping_status": "existing_primitive_map" if domain in primitive_map else "derived_candidate_needs_primitive_map_promotion",
                "studio_primitive": mapping["studio_primitive"],
                "engine_module": mapping["engine_module"],
                "model_tool_surface": mapping["model_tool_surface"],
                "source_apps_present": source_apps,
                "source_counts": {
                    "feature_rows": len(features),
                    "tool_registry_rows": len(tools),
                    "compatibility_records": len(compat),
                    "format_refs": len(format_refs),
                },
                "provider_posture_counts": dict(sorted(provider_counts.items())),
                "file_format_compatibility_counts": dict(sorted(compatibility_counts.items())),
                "format_refs": format_refs[:40],
                "base_scope": f"Implement {mapping['studio_primitive']} as a local-first Rust-backed Studio primitive with source-specific behavior variants.",
                "high_roi_additions": high_roi_additions(domain),
                "reuse": {
                    "primitive_map": "05-studio-primitive-map.md",
                    "command_contract_seed": "10-studio-command-contracts.md",
                    "feature_rows": "39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md",
                    "tool_registry": "45-source-distilled-tool-registry.md",
                    "format_registry": "46-file-format-compatibility-registry.md",
                },
                "gaps_closed_against_rebuild": [
                    "groups source-app feature/tool records into one Studio implementation lane",
                    "keeps vendor provenance separate from shipped Handshake-native naming",
                    "preserves Affinity rows as source variants rather than Adobe overlap",
                    "carries manual and fixture promotion obligations forward",
                ],
                "risks": [
                    "overclaiming parity before exact source-page behavior inspection",
                    "implementing duplicate primitives instead of shared Studio primitive",
                    "format compatibility loss without representative fixtures",
                    "provider/cloud behavior accidentally becoming a local-first dependency",
                ],
                "failure_scenarios": [
                    "source-app option variant has no Studio state-model equivalent",
                    "round-trip import/export silently drops unsupported data",
                    "manual topic is skipped when a command ships",
                    "model agent lacks enough receipt fields to diagnose failure",
                ],
                "remediations": [
                    "promote selected rows through typed command contracts before product code",
                    "require fixtures and unsupported-feature receipts for compatibility features",
                    "add same-change Studio UserManual entries for implemented commands",
                    "run local/offline tests for provider-adjacent behavior",
                ],
                "verification_needs": verification_needs(domain),
            }
        )

    build_slices = [
        {
            "slice_id": "studio.slice.layered-raster-core.v1",
            "primitive_domains": ["layer", "raster", "mask", "selection", "color", "raw", "camera_raw"],
            "purpose": "Unblock Photoshop and Affinity Photo class raster editing, non-destructive layers, masks, selections, and raw development.",
        },
        {
            "slice_id": "studio.slice.vector-typography-design.v1",
            "primitive_domains": ["vector", "typography", "style_system", "brush_engine", "geometry", "design_systems"],
            "purpose": "Unblock Illustrator, Affinity Designer, Photoshop vector/type, and Figma Draw/design-system behavior.",
        },
        {
            "slice_id": "studio.slice.layout-prepress-publishing.v1",
            "primitive_domains": ["page_layout", "tables", "prepress", "export", "presentation", "web", "brand_assets"],
            "purpose": "Unblock InDesign, Affinity Publisher, Figma Slides/Sites/Buzz, and publication export/preflight behavior.",
        },
        {
            "slice_id": "studio.slice.file-compatibility.v1",
            "primitive_domains": ["file_io", "export", "prepress", "asset_pipeline"],
            "purpose": "Preserve existing creative file-format compatibility through fixtures, adapters, and unsupported-feature diagnostics.",
        },
        {
            "slice_id": "studio.slice.automation-collaboration-ai.v1",
            "primitive_domains": ["automation", "dev_mode", "ai", "collaboration", "interactive", "motion", "whiteboard"],
            "purpose": "Unblock model/operator workflow, local-first collaboration, Figma-like interaction/motion, and optional provider adapters.",
        },
    ]

    coverage = {
        "coverage": {
            "distillation_status": "source_distilled_studio_rust_implementation_backlog",
            "backlog_item_count": len(backlog),
            "build_slice_count": len(build_slices),
            "feature_row_count": len(all_feature_rows),
            "tool_registry_row_count": len(tool_rows),
            "compatibility_record_count": len(compat_records),
            "format_family_count": len(format_matrix),
            "policy": {
                "not_product_authority": "This is implementation planning input only until promoted into a work packet or spec authority.",
                "local_first_rule": "Every backlog item targets built-in local-first Studio behavior; provider behavior remains optional adapter scope.",
                "naming_rule": "Shipped commands and modules use Handshake-native names; vendor names stay in source refs and compatibility fixtures.",
                "manual_rule": "Every implemented command must update the internal Studio UserManual in the same change.",
            },
        }
    }

    records = {
        "build_slices": build_slices,
        "primitive_backlog": backlog,
    }

    sources = {
        "sources": [
            {"id": "BACKLOG-S01", "path": "05-studio-primitive-map.md", "note": "Existing Studio primitive and engine module map."},
            {"id": "BACKLOG-S02", "path": "10-studio-command-contracts.md", "note": "Command-contract schema and seed contracts."},
            {"id": "BACKLOG-S03", "path": "18-feature-use-card-manual-handoff-index.md", "note": "Manual handoff obligations."},
            {"id": "BACKLOG-S04", "path": "39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md", "note": "Source-distilled feature rows."},
            {"id": "BACKLOG-S05", "path": "44-cross-app-overlap-and-affinity-dedupe-map.md", "note": "Overlap and Affinity dedupe policy."},
            {"id": "BACKLOG-S06", "path": "45-source-distilled-tool-registry.md", "note": "Tool and surface registry."},
            {"id": "BACKLOG-S07", "path": "46-file-format-compatibility-registry.md", "note": "File-format compatibility registry."},
        ]
    }

    frontmatter = {
        "file_id": "studio-rust-implementation-backlog",
        "file_kind": "source_distilled_implementation_backlog",
        "topic_id": "SFR-STUDIO-RUST-IMPLEMENTATION-BACKLOG",
        "title": "Studio Rust Implementation Backlog",
        "status": "draft",
        "updated_at": UPDATED_AT,
        "backlog_item_count": len(backlog),
        "build_slice_count": len(build_slices),
        "feature_row_count": len(all_feature_rows),
        "tool_registry_row_count": len(tool_rows),
        "compatibility_record_count": len(compat_records),
    }

    text = "\n".join(
        [
            "---",
            yaml.safe_dump(frontmatter, sort_keys=False, allow_unicode=False).strip(),
            "---",
            "",
            "## [SFR-STUDIO-RUST-IMPLEMENTATION-BACKLOG] Studio Rust Implementation Backlog",
            "",
            '<topic id="backlog-coverage" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Coverage and policy for the source-distilled Studio Rust implementation backlog.">',
            "",
            "### [SFR-STUDIO-RUST-IMPLEMENTATION-BACKLOG.coverage] Coverage",
            "",
            "```yaml",
            yaml.safe_dump(coverage, sort_keys=False, allow_unicode=False, width=160).strip(),
            "```",
            "",
            "</topic>",
            "",
            '<topic id="primitive-backlog" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Build slices and primitive backlog records for future Studio implementation work.">',
            "",
            "### [SFR-STUDIO-RUST-IMPLEMENTATION-BACKLOG.records] Primitive Backlog",
            "",
            "```yaml",
            yaml.safe_dump(records, sort_keys=False, allow_unicode=False, width=180).strip(),
            "```",
            "",
            "</topic>",
            "",
            '<topic id="sources" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Sources for the generated Studio Rust implementation backlog.">',
            "",
            "### [SFR-STUDIO-RUST-IMPLEMENTATION-BACKLOG.sources] Sources",
            "",
            "```yaml",
            yaml.safe_dump(sources, sort_keys=False, allow_unicode=False, width=140).strip(),
            "```",
            "",
            "</topic>",
            "",
        ]
    )
    (ROOT / "47-studio-rust-implementation-backlog.md").write_text(text, encoding="utf-8")
    print("47-studio-rust-implementation-backlog.md")
    print(f"backlog_items: {len(backlog)}")
    print(f"build_slices: {len(build_slices)}")
    print(f"feature_rows: {len(all_feature_rows)}")
    print(f"tool_rows: {len(tool_rows)}")
    print(f"compatibility_records: {len(compat_records)}")


if __name__ == "__main__":
    main()
