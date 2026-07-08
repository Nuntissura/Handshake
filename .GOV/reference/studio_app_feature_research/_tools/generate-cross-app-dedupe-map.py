import collections
import pathlib
import re

import yaml


ROOT = pathlib.Path(__file__).resolve().parents[1]
UPDATED_AT = "2026-07-05"

ROW_FILES = {
    "photoshop": "39-photoshop-source-distilled-feature-rows.md",
    "indesign": "40-indesign-source-distilled-feature-rows.md",
    "illustrator": "41-illustrator-source-distilled-feature-rows.md",
    "affinity": "42-affinity-source-distilled-feature-rows.md",
    "figma": "43-figma-source-distilled-feature-rows.md",
}

DOMAIN_FILES = {
    "photoshop": "34-photoshop-source-distilled-domain-ledger.md",
    "indesign": "35-indesign-source-distilled-domain-ledger.md",
    "illustrator": "36-illustrator-source-distilled-domain-ledger.md",
    "affinity": "37-affinity-source-distilled-domain-ledger.md",
    "figma": "38-figma-source-distilled-domain-ledger.md",
}

ADOBE_APPS = {"photoshop", "indesign", "illustrator"}


def yaml_blocks(text: str):
    return re.findall(r"```yaml\n(.*?)\n```", text, flags=re.S)


def load_rows(file_name: str):
    blocks = yaml_blocks((ROOT / file_name).read_text(encoding="utf-8"))
    for block in blocks:
        data = yaml.safe_load(block)
        if isinstance(data, dict) and "source_distilled_feature_rows" in data:
            return data["source_distilled_feature_rows"]
    raise RuntimeError(f"No source_distilled_feature_rows in {file_name}")


def load_domains(file_name: str):
    blocks = yaml_blocks((ROOT / file_name).read_text(encoding="utf-8"))
    for block in blocks:
        data = yaml.safe_load(block)
        if isinstance(data, dict) and "domains" in data:
            return data["domains"]
    raise RuntimeError(f"No domains in {file_name}")


def stable_primitive_id(domain: str):
    normalized = re.sub(r"[^a-z0-9]+", "-", str(domain).lower()).strip("-")
    return f"studio.primitive.{normalized}.v1"


def normalize_feature_name(name: str):
    return re.sub(r"[^a-z0-9]+", " ", str(name).lower()).strip()


def distinctive_markers(row: dict):
    text = " ".join(
        str(row.get(key, ""))
        for key in ["feature_name", "source_category", "source_subcategory", "app_behavior"]
    ).lower()
    markers = []
    for marker in ["persona", "studiolink", "export persona", "afphoto", "afdesign", "afpub", "live filter", "tone mapping"]:
        if marker in text:
            markers.append(marker)
    return markers


def main():
    rows_by_app = {app: load_rows(file_name) for app, file_name in ROW_FILES.items()}
    domains_by_app = {app: load_domains(file_name) for app, file_name in DOMAIN_FILES.items()}

    primitive_matrix = []
    all_primitives = sorted(
        {
            row.get("primitive_domain") or "unknown"
            for rows in rows_by_app.values()
            for row in rows
        }
    )
    for primitive in all_primitives:
        app_counts = {
            app: sum(1 for row in rows if (row.get("primitive_domain") or "unknown") == primitive)
            for app, rows in rows_by_app.items()
        }
        source_apps_present = [app for app, count in app_counts.items() if count]
        has_adobe = any(app in ADOBE_APPS for app in source_apps_present)
        has_affinity = app_counts.get("affinity", 0) > 0
        has_figma = app_counts.get("figma", 0) > 0
        if has_affinity and has_adobe:
            overlap_class = "affinity_shared_with_adobe_via_studio_primitive"
        elif has_affinity and not has_adobe:
            overlap_class = "affinity_unique_or_non_adobe_shared_candidate"
        elif has_adobe and has_figma:
            overlap_class = "adobe_figma_shared_via_studio_primitive"
        elif has_adobe:
            overlap_class = "adobe_source_only_current_rows"
        elif has_figma:
            overlap_class = "figma_source_only_current_rows"
        else:
            overlap_class = "other"
        primitive_matrix.append(
            {
                "studio_primitive_id": stable_primitive_id(primitive),
                "primitive_domain": primitive,
                "overlap_class": overlap_class,
                "source_apps_present": source_apps_present,
                "row_counts": app_counts,
                "implementation_rule": "implement_once_in_studio_primitive_with_source_specific_behavior_variants",
            }
        )

    affinity_domains = []
    adobe_domain_words = {
        "raster",
        "raw",
        "photo",
        "vector",
        "layout",
        "page",
        "typography",
        "text",
        "color",
        "prepress",
        "export",
        "file",
        "automation",
        "workspace",
        "selection",
        "mask",
        "layer",
    }
    for domain in domains_by_app["affinity"]:
        text = " ".join(
            str(domain.get(key, ""))
            for key in ["id", "name", "app_behavior", "manual_topic_candidate"]
        ).lower()
        has_obvious_shared_words = any(word in text for word in adobe_domain_words)
        if any(token in text for token in ["persona", "studiolink", "export persona", "affinity native"]):
            affinity_status = "affinity_distinct_workflow_candidate"
        elif has_obvious_shared_words:
            affinity_status = "shared_studio_primitive_with_affinity_variant"
        else:
            affinity_status = "affinity_unique_candidate_needs_source_page_confirmation"
        affinity_domains.append(
            {
                "affinity_domain_id": domain.get("id"),
                "name": domain.get("name"),
                "dedupe_status": affinity_status,
                "source_behavior_preservation_rule": "retain Affinity source row and exact behavior notes; do not collapse it into an Adobe source label",
                "studio_primitive_domains": domain.get("studio_primitive_domains", []),
                "manual_topic_candidate": domain.get("manual_topic_candidate"),
            }
        )

    source_app_counts = {
        app: dict(collections.Counter((row.get("source_apps") or ["unknown"])[0] for row in rows))
        for app, rows in rows_by_app.items()
    }

    adobe_rows = [
        row
        for app in ADOBE_APPS
        for row in rows_by_app[app]
    ]
    adobe_by_name = collections.defaultdict(list)
    for row in adobe_rows:
        adobe_by_name[normalize_feature_name(row.get("feature_name", ""))].append(row)

    adobe_apps_by_primitive = collections.defaultdict(set)
    for app in ADOBE_APPS:
        for row in rows_by_app[app]:
            adobe_apps_by_primitive[row.get("primitive_domain") or "unknown"].add(app)

    exact_name_overlap_records = []
    affinity_relation_overlay = []
    for row in rows_by_app["affinity"]:
        normalized_name = normalize_feature_name(row.get("feature_name", ""))
        exact_matches = adobe_by_name.get(normalized_name, [])
        shared_adobe_apps = sorted(adobe_apps_by_primitive.get(row.get("primitive_domain") or "unknown", set()))
        markers = distinctive_markers(row)
        relation_class = ["affinity_source_row"]
        if exact_matches:
            relation_class.append("affinity_exact_name_overlap_with_adobe")
        if shared_adobe_apps:
            relation_class.append("affinity_shared_primitive_overlap_with_adobe")
        if markers:
            relation_class.append("affinity_distinctive_candidate")
        if not exact_matches:
            relation_class.append("affinity_current_corpus_name_absent_from_adobe")

        uniqueness_status = "not_claimed"
        if markers:
            uniqueness_status = "distinctive_candidate_needs_source_page_confirmation"
        elif not exact_matches:
            uniqueness_status = "current_corpus_name_absent_from_adobe"

        affinity_relation_overlay.append(
            {
                "affinity_row_id": row.get("source_distilled_feature_id"),
                "source_ids": row.get("source_ids", []),
                "affinity_source_app": (row.get("source_apps") or ["unknown"])[0],
                "feature_name": row.get("feature_name"),
                "normalized_feature_name": normalized_name,
                "studio_surface": row.get("studio_surface"),
                "primitive_domain": row.get("primitive_domain"),
                "relation_class": relation_class,
                "adobe_overlap": {
                    "exact_normalized_name_matches": [
                        match.get("source_distilled_feature_id") for match in exact_matches
                    ],
                    "shared_primitive_adobe_apps": shared_adobe_apps,
                    "equivalence_claim": "exact_name_only_not_behavioral_equivalence" if exact_matches else "none",
                },
                "affinity_distinctive_markers": markers,
                "uniqueness_claim_status": uniqueness_status,
                "verification_needed": [
                    "direct_source_page_comparison_before_claiming_unique_behavior",
                    "command_contract_mapping_before_implementation",
                ],
            }
        )

    for normalized_name, matches in sorted(adobe_by_name.items()):
        affinity_matches = [
            row
            for row in rows_by_app["affinity"]
            if normalize_feature_name(row.get("feature_name", "")) == normalized_name
        ]
        if not affinity_matches:
            continue
        exact_name_overlap_records.append(
            {
                "normalized_feature_name": normalized_name,
                "relation_class": "affinity_exact_name_overlap_with_adobe",
                "equivalence_claim": "exact_name_only_not_behavioral_equivalence",
                "affinity_row_refs": [row.get("source_distilled_feature_id") for row in affinity_matches],
                "adobe_row_refs": [row.get("source_distilled_feature_id") for row in matches],
            }
        )

    coverage = {
        "policy": {
            "goal": "Prevent confusing overlap while preserving every source-observable feature/tool record for Studio rebuild planning.",
            "core_rule": "Shared capability across source apps maps to one Handshake-native Studio primitive, not duplicate Adobe/Affinity/Figma implementations.",
            "source_variant_rule": "Each source app retains its source_distilled_feature_id, source refs, provider posture, compatibility posture, and manual topic candidate.",
            "affinity_rule": "Affinity rows are never renamed as Adobe rows. Shared behavior is grouped by Studio primitive; Affinity-specific workflow variants remain explicit.",
            "vendor_name_rule": "Vendor product names appear only in source/provenance/compatibility references.",
            "file_format_rule": "Compatibility targets remain explicit import/export fixtures; Studio does not invent a replacement interchange format for parity scope.",
        },
        "taxonomy_enums": {
            "source_family": ["studio", "adobe", "affinity", "figma"],
            "relation_class": [
                "shared_studio_primitive",
                "adobe_source_row",
                "affinity_source_row",
                "affinity_exact_name_overlap_with_adobe",
                "affinity_shared_primitive_overlap_with_adobe",
                "affinity_distinctive_candidate",
                "affinity_current_corpus_name_absent_from_adobe",
                "uniqueness_not_proven",
            ],
            "evidence_basis": [
                "explicit_row_provenance",
                "explicit_parity_matrix",
                "manual_surface_grouping",
                "exact_normalized_name_match",
                "domain_ledger_statement",
                "inferred_semantic_overlap",
                "not_proven",
            ],
        },
        "row_coverage": {
            app: {
                "source_row_file": ROW_FILES[app],
                "source_domain_file": DOMAIN_FILES[app],
                "feature_row_count": len(rows_by_app[app]),
                "domain_count": len(domains_by_app[app]),
                "source_app_counts": source_app_counts[app],
            }
            for app in ROW_FILES
        },
        "total_feature_row_count": sum(len(rows) for rows in rows_by_app.values()),
        "affinity_exact_name_overlap_count": len(exact_name_overlap_records),
    }

    body = {
        "primitive_overlap_matrix": primitive_matrix,
        "affinity_dedupe_domains": affinity_domains,
        "affinity_exact_name_overlap_records": exact_name_overlap_records,
        "affinity_relation_overlay": affinity_relation_overlay,
    }

    sources = {
        "sources": [
            {"id": "DEDUPE-S01", "path": "33-online-source-distilled-feature-ledger.md", "note": "Source-distilled merge contract."},
            *[
                {"id": f"DEDUPE-R{i:02d}", "path": file_name, "note": f"{app} source-distilled feature rows."}
                for i, (app, file_name) in enumerate(ROW_FILES.items(), start=1)
            ],
            *[
                {"id": f"DEDUPE-D{i:02d}", "path": file_name, "note": f"{app} source-distilled domain ledger."}
                for i, (app, file_name) in enumerate(DOMAIN_FILES.items(), start=1)
            ],
        ]
    }

    frontmatter = {
        "file_id": "cross-app-overlap-and-affinity-dedupe-map",
        "file_kind": "source_distilled_overlap_map",
        "topic_id": "SFR-CROSS-APP-OVERLAP-AFFINITY-DEDUPE",
        "title": "Cross-App Overlap and Affinity Dedupe Map",
        "status": "draft",
        "updated_at": UPDATED_AT,
        "feature_row_count": coverage["total_feature_row_count"],
        "primitive_domain_count": len(primitive_matrix),
        "affinity_domain_count": len(affinity_domains),
        "affinity_relation_overlay_count": len(affinity_relation_overlay),
        "affinity_exact_name_overlap_count": len(exact_name_overlap_records),
    }

    text = "\n".join(
        [
            "---",
            yaml.safe_dump(frontmatter, sort_keys=False, allow_unicode=False).strip(),
            "---",
            "",
            "## [SFR-CROSS-APP-OVERLAP-AFFINITY-DEDUPE] Cross-App Overlap and Affinity Dedupe Map",
            "",
            f"<topic id=\"overlap-policy\" status=\"current\" version=\"0.1\" updated_at=\"{UPDATED_AT}\" ingestable=\"true\" summary=\"Policy for source-app overlap, Affinity dedupe, and Studio implementation grouping.\">",
            "",
            "### [SFR-CROSS-APP-OVERLAP-AFFINITY-DEDUPE.policy] Overlap Policy",
            "",
            "```yaml",
            yaml.safe_dump(coverage, sort_keys=False, allow_unicode=False, width=140).strip(),
            "```",
            "",
            "</topic>",
            "",
            f"<topic id=\"primitive-overlap-matrix\" status=\"current\" version=\"0.1\" updated_at=\"{UPDATED_AT}\" ingestable=\"true\" summary=\"Generated primitive-domain overlap matrix across source app families.\">",
            "",
            "### [SFR-CROSS-APP-OVERLAP-AFFINITY-DEDUPE.matrix] Primitive Overlap Matrix",
            "",
            "```yaml",
            yaml.safe_dump(body, sort_keys=False, allow_unicode=False, width=160).strip(),
            "```",
            "",
            "</topic>",
            "",
            f"<topic id=\"sources\" status=\"current\" version=\"0.1\" updated_at=\"{UPDATED_AT}\" ingestable=\"true\" summary=\"Sources for the generated overlap and Affinity dedupe map.\">",
            "",
            "### [SFR-CROSS-APP-OVERLAP-AFFINITY-DEDUPE.sources] Sources",
            "",
            "```yaml",
            yaml.safe_dump(sources, sort_keys=False, allow_unicode=False, width=140).strip(),
            "```",
            "",
            "</topic>",
            "",
        ]
    )
    (ROOT / "44-cross-app-overlap-and-affinity-dedupe-map.md").write_text(text, encoding="utf-8")
    print("44-cross-app-overlap-and-affinity-dedupe-map.md")
    print(f"feature_rows: {coverage['total_feature_row_count']}")
    print(f"primitive_domains: {len(primitive_matrix)}")
    print(f"affinity_domains: {len(affinity_domains)}")


if __name__ == "__main__":
    main()
