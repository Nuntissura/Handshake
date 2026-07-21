import collections
import pathlib
import re

import yaml


ROOT = pathlib.Path(__file__).resolve().parents[1]
UPDATED_AT = "2026-07-21"

FEATURE_ROW_FILES = {
    "photoshop": "39-photoshop-source-distilled-feature-rows.md",
    "indesign": "40-indesign-source-distilled-feature-rows.md",
    "illustrator": "41-illustrator-source-distilled-feature-rows.md",
    "affinity": "42-affinity-source-distilled-feature-rows.md",
    "figma": "43-figma-source-distilled-feature-rows.md",
}

# Deep-delta files (51-55). These carry the sub-TOC inventory the leaf pipeline
# never saw. Before 2026-07-21 they were INVISIBLE to this matrix, so the matrix
# audited only ~half the corpus while reporting a green field-completeness signal.
# A1 (2026-07-21): ingest them so the dashboard reflects the whole corpus.
DEEP_DELTA_FILES = {
    "photoshop": "51-photoshop-deep-feature-delta.md",
    "illustrator": "52-illustrator-deep-feature-delta.md",
    "indesign": "53-indesign-deep-feature-delta.md",
    "affinity": "54-affinity-deep-feature-delta.md",
    "figma": "55-figma-deep-feature-delta.md",
}

REQUIRED_FIELDS = [
    "app_behavior",
    "user_goal",
    "source_refs",
    "manual_topic_candidate",
    "provider_posture",
    "file_format_compatibility",
    "command_contract_refs",
    "verification_refs",
]

# Deep-delta rows use a leaner, source-anchored schema (id/name/app_behavior/
# primitive_domain/source_url/verification_status/dedupe_status). They are NOT
# run through the leaf REQUIRED_FIELDS check (that would falsely flag every deep
# row); they get a schema-appropriate assessment instead.
DEEP_REQUIRED_FIELDS = [
    "id",
    "name",
    "app_behavior",
    "primitive_domain",
    "source_url",
    "verification_status",
]

LOCAL_PROVIDER_POSTURES = {"local_primitive", "local_primitive_candidate", "not_applicable", None}
PROVIDER_ADJACENT_DOMAINS = {
    "ai",
    "automation",
    "collaboration",
    "dev_mode",
    "interactive",
    "motion",
    "whiteboard",
}
FORMAT_RELEVANT_POSTURES = {"import", "export", "round_trip", "fixture_required"}


def yaml_blocks(text: str):
    return re.findall(r"```yaml\n(.*?)\n```", text, flags=re.S)


def load_list_from_yaml_file(file_name: str, key: str):
    for block in yaml_blocks((ROOT / file_name).read_text(encoding="utf-8")):
        data = yaml.safe_load(block)
        if isinstance(data, dict) and key in data:
            return data[key]
    raise RuntimeError(f"No {key} block in {file_name}")


def load_feature_rows(file_name: str):
    return load_list_from_yaml_file(file_name, "source_distilled_feature_rows")


def load_deep_records(file_name: str):
    """Collect `records:` across ALL yaml blocks in a deep-delta file (one block
    per modality subtopic), skipping non-record blocks (e.g. the EOF sources block)."""
    records = []
    for block in yaml_blocks((ROOT / file_name).read_text(encoding="utf-8")):
        data = yaml.safe_load(block)
        if isinstance(data, dict) and isinstance(data.get("records"), list):
            records.extend(data["records"])
    if not records:
        raise RuntimeError(f"No records blocks in {file_name}")
    return records


def load_provider_rows():
    return load_list_from_yaml_file("48-provider-offline-parity-registry.md", "provider_offline_parity_rows")


def load_tool_rows():
    return load_list_from_yaml_file("45-source-distilled-tool-registry.md", "tool_registry_rows")


def load_compat_records():
    return load_list_from_yaml_file("46-file-format-compatibility-registry.md", "compatibility_records")


def load_backlog_rows():
    return load_list_from_yaml_file("47-studio-rust-implementation-backlog.md", "primitive_backlog")


def source_ref_counts(row: dict):
    refs = row.get("source_refs") or []
    return {
        "source_ref_count": len(refs),
        "source_ref_url_count": sum(1 for ref in refs if ref.get("url")),
        "source_ref_path_count": sum(1 for ref in refs if ref.get("path")),
    }


def is_provider_adjacent(row: dict):
    return row.get("provider_posture") not in LOCAL_PROVIDER_POSTURES or row.get("primitive_domain") in PROVIDER_ADJACENT_DOMAINS


def evidence_strength(counts: dict):
    if counts["source_ref_url_count"] and counts["source_ref_path_count"]:
        return "url_and_local_snapshot"
    if counts["source_ref_url_count"]:
        return "url_only"
    if counts["source_ref_path_count"]:
        return "local_snapshot_only"
    return "missing_source_ref"


def coverage_status(missing_fields, counts: dict):
    if missing_fields:
        return "missing_required_source_distilled_fields"
    if not counts["source_ref_url_count"]:
        return "missing_source_url"
    if not counts["source_ref_path_count"]:
        return "source_distilled_complete_without_local_snapshot_path"
    return "source_distilled_complete_with_local_snapshot_path"


def row_obligations(feature_row: dict, provider_refs, tool_domain_count: int, compat_app_domain_count: int):
    obligations = ["exact_source_page_or_behavior_inspection_before_product_implementation"]
    if not feature_row.get("command_contract_refs"):
        obligations.append("command_contract_refs_missing")
    else:
        obligations.append("promote_command_contract_before_product_code")
    if not feature_row.get("verification_refs"):
        obligations.append("verification_refs_missing")
    else:
        obligations.append("replace_placeholder_verification_ref_with_fixture_or_test")
    if is_provider_adjacent(feature_row):
        obligations.append("apply_provider_offline_parity_contract")
    if feature_row.get("file_format_compatibility") in FORMAT_RELEVANT_POSTURES:
        obligations.append("apply_format_fixture_and_round_trip_receipt")
    if tool_domain_count:
        obligations.append("cross_check_tool_registry_domain_rows")
    if compat_app_domain_count:
        obligations.append("cross_check_format_registry_app_domain_records")
    if not provider_refs and is_provider_adjacent(feature_row):
        obligations.append("provider_offline_registry_match_missing")
    return obligations


# ------------------------------------------------------------------ deep-delta layer
def deep_coverage_status(missing_fields, verification):
    if missing_fields:
        return "deep_delta_missing_required_fields"
    if verification == "UNVERIFIED":
        return "deep_delta_unverified_source"
    if verification == "PARTIAL":
        return "deep_delta_partial_source"
    return "deep_delta_verified_with_source"


def deep_evidence_strength(has_url, verification):
    if not has_url:
        return "deep_missing_source_url"
    if verification == "VERIFIED":
        return "deep_verified_source_url"
    if verification == "PARTIAL":
        return "deep_partial_source_url"
    return "deep_unverified_source_url"


def build_deep_matrix_rows():
    rows = []
    for app_key, file_name in DEEP_DELTA_FILES.items():
        for rec in load_deep_records(file_name):
            missing = [f for f in DEEP_REQUIRED_FIELDS if not rec.get(f)]
            verification = rec.get("verification_status")
            has_url = bool(rec.get("source_url"))
            domain = rec.get("primitive_domain")
            status = deep_coverage_status(missing, verification)
            strength = deep_evidence_strength(has_url, verification)
            rows.append(
                {
                    "coverage_row_id": f"coverage.{rec.get('id')}",
                    "row_layer": "deep_delta",
                    "source_app_key": app_key,
                    "source_distilled_feature_id": rec.get("id"),
                    "feature_name": rec.get("name"),
                    "primitive_domain": domain,
                    "dedupe_status": rec.get("dedupe_status"),
                    "deepens_leaf_id": rec.get("deepens_leaf_id"),
                    "verification_status": verification,
                    "coverage_status": status,
                    "evidence_strength": strength,
                    "missing_required_fields": missing,
                    "has_app_behavior": bool(rec.get("app_behavior")),
                    "has_source_url": has_url,
                    "source_ids": rec.get("source_ids") or [],
                    "implementation_obligations": [
                        "exact_source_page_or_behavior_inspection_before_product_implementation",
                        "promote_deep_row_to_feature_row_and_command_contract_before_product_code",
                    ],
                }
            )
    return rows


def main():
    # ---------- leaf layer (unchanged logic) ----------
    feature_rows = []
    for app_key, file_name in FEATURE_ROW_FILES.items():
        for row in load_feature_rows(file_name):
            enriched = dict(row)
            enriched["source_app_key"] = app_key
            feature_rows.append(enriched)

    provider_by_feature_id = collections.defaultdict(list)
    for row in load_provider_rows():
        provider_by_feature_id[row.get("source_distilled_feature_id")].append(row.get("provider_offline_parity_id"))

    tool_domain_counts = collections.Counter(
        (row.get("source_app_key"), row.get("primary_studio_primitive")) for row in load_tool_rows()
    )
    compat_domain_counts = collections.Counter(
        (row.get("source_app_key"), row.get("studio_primitive") or (row.get("studio_primitive_domains") or [None])[0])
        for row in load_compat_records()
    )
    backlog_by_domain = {row.get("primitive_domain"): row.get("backlog_id") for row in load_backlog_rows()}

    leaf_rows = []
    leaf_status_counts = collections.Counter()
    leaf_evidence_counts = collections.Counter()
    missing_field_counts = collections.Counter()
    provider_posture_or_runtime_adjacent_count = 0
    provider_posture_or_runtime_adjacent_without_registry = 0
    provider_offline_registry_selected_count = 0

    for source_row in feature_rows:
        counts = source_ref_counts(source_row)
        missing_fields = [field for field in REQUIRED_FIELDS if not source_row.get(field)]
        app_key = source_row["source_app_key"]
        domain = source_row.get("primitive_domain")
        provider_refs = provider_by_feature_id.get(source_row.get("source_distilled_feature_id"), [])
        tool_count = tool_domain_counts[(app_key, domain)]
        compat_count = compat_domain_counts[(app_key, domain)]
        status = coverage_status(missing_fields, counts)
        strength = evidence_strength(counts)
        provider_posture_or_runtime_adjacent = is_provider_adjacent(source_row)
        provider_offline_registry_selected = bool(provider_refs)
        if provider_posture_or_runtime_adjacent:
            provider_posture_or_runtime_adjacent_count += 1
            if not provider_refs:
                provider_posture_or_runtime_adjacent_without_registry += 1
        if provider_offline_registry_selected:
            provider_offline_registry_selected_count += 1
        leaf_status_counts[status] += 1
        leaf_evidence_counts[strength] += 1
        for field in missing_fields:
            missing_field_counts[field] += 1
        leaf_rows.append(
            {
                "coverage_row_id": f"coverage.{source_row.get('source_distilled_feature_id')}",
                "row_layer": "leaf",
                "source_app_key": app_key,
                "source_distilled_feature_id": source_row.get("source_distilled_feature_id"),
                "source_feature_id": source_row.get("source_feature_id"),
                "feature_name": source_row.get("feature_name"),
                "primitive_domain": domain,
                "backlog_ref": backlog_by_domain.get(domain),
                "provider_posture": source_row.get("provider_posture"),
                "file_format_compatibility": source_row.get("file_format_compatibility"),
                "coverage_status": status,
                "evidence_strength": strength,
                "missing_required_fields": missing_fields,
                "has_app_behavior": bool(source_row.get("app_behavior")),
                "has_user_goal": bool(source_row.get("user_goal")),
                "has_manual_topic_candidate": bool(source_row.get("manual_topic_candidate")),
                "has_command_contract_refs": bool(source_row.get("command_contract_refs")),
                "has_verification_refs": bool(source_row.get("verification_refs")),
                "source_ref_count": counts["source_ref_count"],
                "source_ref_url_count": counts["source_ref_url_count"],
                "source_ref_path_count": counts["source_ref_path_count"],
                "provider_posture_or_runtime_adjacent": provider_posture_or_runtime_adjacent,
                "provider_offline_registry_selected": provider_offline_registry_selected,
                "provider_offline_parity_refs": provider_refs,
                "tool_registry_app_domain_row_count": tool_count,
                "format_registry_app_domain_record_count": compat_count,
                "implementation_obligations": row_obligations(source_row, provider_refs, tool_count, compat_count),
            }
        )

    # ---------- deep-delta layer (new) ----------
    deep_rows = build_deep_matrix_rows()
    deep_status_counts = collections.Counter(r["coverage_status"] for r in deep_rows)
    deep_evidence_counts = collections.Counter(r["evidence_strength"] for r in deep_rows)
    deep_verification_counts = collections.Counter(r["verification_status"] for r in deep_rows)

    matrix_rows = leaf_rows + deep_rows

    # ---------- per-app summaries ----------
    leaf_by_app = collections.defaultdict(list)
    for row in leaf_rows:
        leaf_by_app[row["source_app_key"]].append(row)
    leaf_summary_by_app = {}
    for app_key, rows in sorted(leaf_by_app.items()):
        leaf_summary_by_app[app_key] = {
            "feature_rows": len(rows),
            "coverage_status_counts": dict(sorted(collections.Counter(r["coverage_status"] for r in rows).items())),
            "evidence_strength_counts": dict(sorted(collections.Counter(r["evidence_strength"] for r in rows).items())),
            "provider_posture_or_runtime_adjacent_rows": sum(1 for r in rows if r["provider_posture_or_runtime_adjacent"]),
            "provider_offline_registry_selected_rows": sum(1 for r in rows if r["provider_offline_registry_selected"]),
            "rows_with_format_registry_app_domain_records": sum(1 for r in rows if r["format_registry_app_domain_record_count"]),
            "rows_with_tool_registry_app_domain_rows": sum(1 for r in rows if r["tool_registry_app_domain_row_count"]),
            "local_snapshot_path_gap_count": sum(1 for r in rows if r["source_ref_url_count"] and not r["source_ref_path_count"]),
        }

    deep_by_app = collections.defaultdict(list)
    for row in deep_rows:
        deep_by_app[row["source_app_key"]].append(row)
    deep_summary_by_app = {}
    for app_key, rows in sorted(deep_by_app.items()):
        deep_summary_by_app[app_key] = {
            "deep_rows": len(rows),
            "coverage_status_counts": dict(sorted(collections.Counter(r["coverage_status"] for r in rows).items())),
            "verification_status_counts": dict(sorted(collections.Counter(r["verification_status"] for r in rows).items())),
            "new_surface_rows": sum(1 for r in rows if r.get("dedupe_status") == "new_surface"),
            "deepens_existing_rows": sum(1 for r in rows if r.get("dedupe_status") == "deepens_existing"),
        }

    combined_status_counts = collections.Counter(r["coverage_status"] for r in matrix_rows)

    coverage = {
        "scope_and_honesty_note": (
            "This matrix now covers ALL corpus feature rows across BOTH layers: the leaf pipeline "
            "(39-43) AND the deep-delta inventory (51-55). Before 2026-07-21 (A1) it saw only the leaf "
            "layer while reporting a green field-completeness signal, which over-signalled readiness. "
            "IMPORTANT: this audits FIELD/SOURCE completeness of rows that EXIST; a matrix over existing "
            "rows CANNOT detect a MISSING feature (semantic omission). For known semantic parity gaps see "
            "58-parity-feature-gap-register.md; for team/production workflow gaps see 59; the deep layer's "
            "own verification (VERIFIED/PARTIAL/UNVERIFIED) is carried per row."
        ),
        "corpus_layers": {
            "leaf_row_count": len(leaf_rows),
            "deep_delta_row_count": len(deep_rows),
            "total_row_count": len(matrix_rows),
        },
        "combined_coverage_status_counts": dict(sorted(combined_status_counts.items())),
        "leaf_layer": {
            "feature_row_count": len(leaf_rows),
            "source_feature_row_files": FEATURE_ROW_FILES,
            "required_fields": REQUIRED_FIELDS,
            "coverage_status_counts": dict(sorted(leaf_status_counts.items())),
            "evidence_strength_counts": dict(sorted(leaf_evidence_counts.items())),
            "missing_required_field_counts": dict(sorted(missing_field_counts.items())),
            "provider_posture_or_runtime_adjacent_rows": provider_posture_or_runtime_adjacent_count,
            "provider_posture_or_runtime_adjacent_rows_without_provider_offline_registry": provider_posture_or_runtime_adjacent_without_registry,
            "provider_offline_registry_selected_rows": provider_offline_registry_selected_count,
            "summary_by_app": leaf_summary_by_app,
        },
        "deep_delta_layer": {
            "deep_row_count": len(deep_rows),
            "deep_delta_files": DEEP_DELTA_FILES,
            "deep_required_fields": DEEP_REQUIRED_FIELDS,
            "coverage_status_counts": dict(sorted(deep_status_counts.items())),
            "evidence_strength_counts": dict(sorted(deep_evidence_counts.items())),
            "verification_status_counts": dict(sorted(deep_verification_counts.items())),
            "summary_by_app": deep_summary_by_app,
        },
        "interpretation": {
            "source_distilled_complete_with_local_snapshot_path": "Leaf row has required source-distilled fields, a source URL, and a local path reference.",
            "source_distilled_complete_without_local_snapshot_path": "Leaf row has required fields and a source URL, but no local snapshot path in the row.",
            "missing_required_source_distilled_fields": "Leaf row is missing one or more required source-distilled planning fields.",
            "deep_delta_verified_with_source": "Deep row has all required deep fields and a source that was fetched/inspected (VERIFIED).",
            "deep_delta_partial_source": "Deep row is named from an inspected overview/TOC but the specific leaf body was not re-fetched (PARTIAL).",
            "deep_delta_unverified_source": "Deep row rests on a search-snippet or uninspected source (UNVERIFIED).",
            "deep_delta_missing_required_fields": "Deep row is missing one or more required deep fields.",
            "not_product_authority": "This matrix verifies source-distilled + deep-delta planning coverage only; product implementation still requires exact behavior inspection, command contract promotion, fixtures/tests, receipts, and Studio UserManual entry.",
            "cannot_detect_missing_features": "A coverage matrix over existing rows cannot detect a feature that has NO row. Semantic-omission gaps are tracked separately in 58/59.",
        },
    }

    frontmatter = {
        "file_id": "49-source-coverage-verification-matrix",
        "file_kind": "source_coverage_verification_matrix",
        "topic_id": "SFR-SOURCE-COVERAGE-VERIFICATION-MATRIX",
        "title": "Source Coverage Verification Matrix",
        "status": "draft",
        "summary": "Generated matrix that audits every corpus feature row across BOTH the leaf pipeline (39-43) and the deep-delta inventory (51-55) for field/source completeness, evidence strength, and implementation obligations. Field/source completeness of existing rows only; does not detect missing features (see 58/59).",
        "updated_at": UPDATED_AT,
        "coverage_row_count": len(matrix_rows),
        "leaf_row_count": len(leaf_rows),
        "deep_delta_row_count": len(deep_rows),
    }

    source_block = {
        "sources": [
            {"id": "COVERAGE-S01", "path": "39-photoshop-source-distilled-feature-rows.md", "note": "Photoshop source-distilled feature rows."},
            {"id": "COVERAGE-S02", "path": "40-indesign-source-distilled-feature-rows.md", "note": "InDesign source-distilled feature rows."},
            {"id": "COVERAGE-S03", "path": "41-illustrator-source-distilled-feature-rows.md", "note": "Illustrator source-distilled feature rows."},
            {"id": "COVERAGE-S04", "path": "42-affinity-source-distilled-feature-rows.md", "note": "Affinity source-distilled feature rows."},
            {"id": "COVERAGE-S05", "path": "43-figma-source-distilled-feature-rows.md", "note": "Figma source-distilled feature rows."},
            {"id": "COVERAGE-S06", "path": "45-source-distilled-tool-registry.md", "note": "Source-distilled tool registry."},
            {"id": "COVERAGE-S07", "path": "46-file-format-compatibility-registry.md", "note": "File-format compatibility registry."},
            {"id": "COVERAGE-S08", "path": "47-studio-rust-implementation-backlog.md", "note": "Studio Rust implementation backlog."},
            {"id": "COVERAGE-S09", "path": "48-provider-offline-parity-registry.md", "note": "Provider/offline parity registry."},
            {"id": "COVERAGE-S10", "path": "51-photoshop-deep-feature-delta.md", "note": "Photoshop deep-delta records (A1 ingest)."},
            {"id": "COVERAGE-S11", "path": "52-illustrator-deep-feature-delta.md", "note": "Illustrator deep-delta records (A1 ingest)."},
            {"id": "COVERAGE-S12", "path": "53-indesign-deep-feature-delta.md", "note": "InDesign deep-delta records (A1 ingest)."},
            {"id": "COVERAGE-S13", "path": "54-affinity-deep-feature-delta.md", "note": "Affinity deep-delta records (A1 ingest)."},
            {"id": "COVERAGE-S14", "path": "55-figma-deep-feature-delta.md", "note": "Figma deep-delta records (A1 ingest)."},
            {"id": "COVERAGE-S15", "path": "_tools/generate-source-coverage-verification-matrix.py", "note": "Source coverage verification matrix generator."},
        ]
    }

    text = "---\n"
    text += yaml.safe_dump(frontmatter, sort_keys=False, allow_unicode=False)
    text += "---\n\n"
    text += "## [SFR-SOURCE-COVERAGE-VERIFICATION-MATRIX] Source Coverage Verification Matrix\n\n"
    text += f'<topic id="source-coverage-summary" status="current" version="0.2" updated_at="{UPDATED_AT}" ingestable="true" summary="Coverage summary across leaf + deep-delta corpus layers.">\n\n'
    text += "### [SFR-SOURCE-COVERAGE-VERIFICATION-MATRIX.summary] Coverage Summary\n\n"
    text += "```yaml\n"
    text += yaml.safe_dump({"source_coverage_summary": coverage}, sort_keys=False, allow_unicode=False, width=1400)
    text += "```\n\n</topic>\n\n"
    text += f'<topic id="source-coverage-rows" status="current" version="0.2" updated_at="{UPDATED_AT}" ingestable="true" summary="Machine-readable coverage rows for every corpus feature row (leaf + deep-delta).">\n\n'
    text += "### [SFR-SOURCE-COVERAGE-VERIFICATION-MATRIX.rows] Coverage Rows\n\n"
    text += "```yaml\n"
    text += yaml.safe_dump({"source_coverage_rows": matrix_rows}, sort_keys=False, allow_unicode=False, width=1400)
    text += "```\n\n</topic>\n\n"
    text += f'<topic id="source-coverage-sources" status="current" version="0.2" updated_at="{UPDATED_AT}" ingestable="true" summary="Sources for source coverage verification matrix.">\n\n'
    text += "### [SFR-SOURCE-COVERAGE-VERIFICATION-MATRIX.sources] Sources\n\n"
    text += "```yaml\n"
    text += yaml.safe_dump(source_block, sort_keys=False, allow_unicode=False, width=1400)
    text += "```\n\n</topic>\n"

    (ROOT / "49-source-coverage-verification-matrix.md").write_text(text, encoding="utf-8")
    print(
        f"wrote 49-source-coverage-verification-matrix.md: {len(matrix_rows)} total rows "
        f"({len(leaf_rows)} leaf + {len(deep_rows)} deep-delta)"
    )


if __name__ == "__main__":
    main()
