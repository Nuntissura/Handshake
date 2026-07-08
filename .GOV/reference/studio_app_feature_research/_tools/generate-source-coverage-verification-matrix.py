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


def coverage_status(missing_fields: list[str], counts: dict):
    if missing_fields:
        return "missing_required_source_distilled_fields"
    if not counts["source_ref_url_count"]:
        return "missing_source_url"
    if not counts["source_ref_path_count"]:
        return "source_distilled_complete_without_local_snapshot_path"
    return "source_distilled_complete_with_local_snapshot_path"


def row_obligations(feature_row: dict, provider_refs: list[str], tool_domain_count: int, compat_app_domain_count: int):
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


def main():
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

    matrix_rows = []
    summary_by_app = {}
    summary_by_domain = collections.defaultdict(collections.Counter)
    status_counts = collections.Counter()
    evidence_counts = collections.Counter()
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
        status_counts[status] += 1
        evidence_counts[strength] += 1
        for field in missing_fields:
            missing_field_counts[field] += 1
        summary_by_domain[domain][status] += 1
        matrix_rows.append(
            {
                "coverage_row_id": f"coverage.{source_row.get('source_distilled_feature_id')}",
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

    rows_by_app = collections.defaultdict(list)
    for row in matrix_rows:
        rows_by_app[row["source_app_key"]].append(row)

    for app_key, rows in sorted(rows_by_app.items()):
        summary_by_app[app_key] = {
            "feature_rows": len(rows),
            "coverage_status_counts": dict(sorted(collections.Counter(row["coverage_status"] for row in rows).items())),
            "evidence_strength_counts": dict(sorted(collections.Counter(row["evidence_strength"] for row in rows).items())),
            "provider_posture_or_runtime_adjacent_rows": sum(1 for row in rows if row["provider_posture_or_runtime_adjacent"]),
            "provider_offline_registry_selected_rows": sum(1 for row in rows if row["provider_offline_registry_selected"]),
            "rows_with_format_registry_app_domain_records": sum(1 for row in rows if row["format_registry_app_domain_record_count"]),
            "rows_with_tool_registry_app_domain_rows": sum(1 for row in rows if row["tool_registry_app_domain_row_count"]),
            "local_snapshot_path_gap_count": sum(1 for row in rows if row["source_ref_url_count"] and not row["source_ref_path_count"]),
        }

    coverage = {
        "feature_row_count": len(matrix_rows),
        "source_feature_row_files": FEATURE_ROW_FILES,
        "required_fields": REQUIRED_FIELDS,
        "coverage_status_counts": dict(sorted(status_counts.items())),
        "evidence_strength_counts": dict(sorted(evidence_counts.items())),
        "missing_required_field_counts": dict(sorted(missing_field_counts.items())),
        "provider_posture_or_runtime_adjacent_rows": provider_posture_or_runtime_adjacent_count,
        "provider_posture_or_runtime_adjacent_rows_without_provider_offline_registry": provider_posture_or_runtime_adjacent_without_registry,
        "provider_offline_registry_selected_rows": provider_offline_registry_selected_count,
        "summary_by_app": summary_by_app,
        "coverage_status_by_primitive_domain": {
            domain: dict(sorted(counter.items())) for domain, counter in sorted(summary_by_domain.items())
        },
        "interpretation": {
            "source_distilled_complete_with_local_snapshot_path": "The feature row has required source-distilled fields, a source URL, and a local path reference.",
            "source_distilled_complete_without_local_snapshot_path": "The feature row has required source-distilled fields and a source URL, but no local source snapshot path in the row.",
            "missing_required_source_distilled_fields": "The generated row is missing one or more required source-distilled planning fields.",
            "not_product_authority": "This matrix verifies source-distilled planning coverage only; product implementation still requires exact behavior inspection, command contract promotion, fixtures/tests, receipts, and Studio UserManual entry.",
        },
    }

    frontmatter = {
        "file_id": "49-source-coverage-verification-matrix",
        "file_kind": "source_coverage_verification_matrix",
        "topic_id": "SFR-SOURCE-COVERAGE-VERIFICATION-MATRIX",
        "title": "Source Coverage Verification Matrix",
        "status": "draft",
        "summary": "Generated matrix that audits every source-distilled feature row for required planning fields, source reference strength, provider/offline registry linkage, format registry linkage, tool registry linkage, and implementation obligations.",
        "updated_at": UPDATED_AT,
        "coverage_row_count": len(matrix_rows),
        "source_feature_row_count": len(matrix_rows),
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
            {"id": "COVERAGE-S10", "path": "_tools/generate-source-coverage-verification-matrix.py", "note": "Source coverage verification matrix generator."},
        ]
    }

    text = "---\n"
    text += yaml.safe_dump(frontmatter, sort_keys=False, allow_unicode=False)
    text += "---\n\n"
    text += "## [SFR-SOURCE-COVERAGE-VERIFICATION-MATRIX] Source Coverage Verification Matrix\n\n"
    text += '<topic id="source-coverage-summary" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Coverage summary for every source-distilled feature row.">\n\n'
    text += "### [SFR-SOURCE-COVERAGE-VERIFICATION-MATRIX.summary] Coverage Summary\n\n"
    text += "```yaml\n"
    text += yaml.safe_dump({"source_coverage_summary": coverage}, sort_keys=False, allow_unicode=False, width=1400)
    text += "```\n\n</topic>\n\n"
    text += '<topic id="source-coverage-rows" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Machine-readable coverage rows for every source-distilled feature row.">\n\n'
    text += "### [SFR-SOURCE-COVERAGE-VERIFICATION-MATRIX.rows] Coverage Rows\n\n"
    text += "```yaml\n"
    text += yaml.safe_dump({"source_coverage_rows": matrix_rows}, sort_keys=False, allow_unicode=False, width=1400)
    text += "```\n\n</topic>\n\n"
    text += '<topic id="source-coverage-sources" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Sources for source coverage verification matrix.">\n\n'
    text += "### [SFR-SOURCE-COVERAGE-VERIFICATION-MATRIX.sources] Sources\n\n"
    text += "```yaml\n"
    text += yaml.safe_dump(source_block, sort_keys=False, allow_unicode=False, width=1400)
    text += "```\n\n</topic>\n"

    (ROOT / "49-source-coverage-verification-matrix.md").write_text(text, encoding="utf-8")
    print(f"wrote 49-source-coverage-verification-matrix.md with {len(matrix_rows)} coverage rows")


if __name__ == "__main__":
    main()
