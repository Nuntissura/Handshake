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

FORMAT_PATTERNS = [
    ("format.psd", "PSD", r"\bpsd\b"),
    ("format.psb", "PSB", r"\bpsb\b"),
    ("format.ai", "AI", r"\.ai\b|\bai/ait\b|\bai/ait/pdf|ai/pdf-compatible"),
    ("format.ait", "AIT", r"\bait\b"),
    ("format.indd", "INDD", r"\bindd\b"),
    ("format.idml", "IDML", r"\bidml\b"),
    ("format.pdf", "PDF", r"\bpdf\b|pdf/x"),
    ("format.svg", "SVG", r"\bsvg\b"),
    ("format.eps", "EPS", r"\beps\b"),
    ("format.ps", "PostScript", r"\bps\b|postscript"),
    ("format.dwg", "DWG", r"\bdwg\b"),
    ("format.dxf", "DXF", r"\bdxf\b"),
    ("format.png", "PNG", r"\bpng\b"),
    ("format.jpeg", "JPEG/JPG", r"\bjpeg\b|\bjpg\b"),
    ("format.gif", "GIF", r"\bgif\b"),
    ("format.webp", "WebP", r"\bwebp\b"),
    ("format.tiff", "TIFF", r"\btiff\b|\btif\b"),
    ("format.raw", "RAW camera formats", r"\braw\b|camera/raw"),
    ("format.dng", "DNG", r"\bdng\b"),
    ("format.exr_hdr", "EXR/HDR", r"\bexr\b|\bhdr\b"),
    ("format.epub", "EPUB", r"\bepub\b"),
    ("format.html", "HTML", r"\bhtml\b"),
    ("format.css", "CSS", r"\bcss\b"),
    ("format.xml", "XML", r"\bxml\b"),
    ("format.csv", "CSV", r"\bcsv\b"),
    ("format.doc_word", "Word documents", r"\bdoc\b|\bdocx\b|\bword documents?\b|\bword files?\b"),
    ("format.xls_excel", "Excel spreadsheets", r"\bexcel\b|xlsx?"),
    ("format.pptx", "PPTX", r"\bpptx\b"),
    ("format.sketch", "Sketch", r"\bsketch\b"),
    ("format.fig", "FIG local copy", r"\bfig\b|\.fig\b"),
    ("format.jam", "JAM local copy", r"\bjam\b|\.jam\b"),
    ("format.deck", "DECK local copy", r"\bdeck\b|\.deck\b"),
    ("format.buzz", "BUZZ local copy", r"\bbuzz\b|\.buzz\b"),
    ("format.site", "SITE local copy", r"\bsite\b|\.site\b"),
    ("format.make", "MAKE local copy", r"\bmake files?\b|\.make\b"),
    ("format.afphoto", "AFPHOTO", r"\bafphoto\b|\.afphoto\b"),
    ("format.afdesign", "AFDESIGN", r"\bafdesign\b|\.afdesign\b"),
    ("format.afpub", "AFPUB", r"\bafpub\b|\.afpub\b"),
]

APP_NATIVE_FORMATS = {
    "photoshop": ["format.psd", "format.psb"],
    "indesign": ["format.indd", "format.idml"],
    "illustrator": ["format.ai", "format.ait"],
    "affinity": ["format.afphoto", "format.afdesign", "format.afpub"],
    "figma": ["format.fig", "format.jam", "format.deck", "format.buzz", "format.site", "format.make"],
}


def yaml_blocks(text: str):
    return re.findall(r"```yaml\n(.*?)\n```", text, flags=re.S)


def load_rows(file_name: str):
    for block in yaml_blocks((ROOT / file_name).read_text(encoding="utf-8")):
        data = yaml.safe_load(block)
        if isinstance(data, dict) and "source_distilled_feature_rows" in data:
            return data["source_distilled_feature_rows"]
    raise RuntimeError(f"No source_distilled_feature_rows in {file_name}")


def load_domains(file_name: str):
    for block in yaml_blocks((ROOT / file_name).read_text(encoding="utf-8")):
        data = yaml.safe_load(block)
        if isinstance(data, dict) and "domains" in data:
            return data["domains"]
    raise RuntimeError(f"No domains in {file_name}")


def flatten(value):
    if isinstance(value, list):
        for item in value:
            yield from flatten(item)
    elif isinstance(value, dict):
        for key, item in value.items():
            yield str(key)
            yield from flatten(item)
    elif value is not None:
        yield str(value)


def detect_formats(text: str):
    lowered = text.lower()
    matches = []
    for format_id, label, pattern in FORMAT_PATTERNS:
        if re.search(pattern, lowered, flags=re.I):
            matches.append({"format_id": format_id, "format_label": label})
    return matches


def domain_format_text(domain: dict):
    parts = [
        domain.get("name", ""),
        domain.get("app_behavior", ""),
        domain.get("tool_and_feature_scope", ""),
        " ".join(domain.get("implementation_notes", [])),
    ]
    return " ".join(flatten(parts))


def support_kind_from_compat(value: str):
    if value in {"import", "export", "round_trip", "fixture_required"}:
        return value
    return "fixture_required"


def feature_compatibility_records(rows_by_app):
    records = []
    for app_key, rows in rows_by_app.items():
        for row in rows:
            compat = row.get("file_format_compatibility")
            if compat == "not_applicable":
                continue
            text = " ".join(
                str(row.get(key, ""))
                for key in ["feature_name", "source_category", "source_subcategory", "app_behavior", "user_goal"]
            )
            formats = detect_formats(text)
            if not formats:
                formats = [{"format_id": "format.unspecified", "format_label": "Unspecified source-format workflow"}]
            records.append(
                {
                    "compatibility_record_id": f"compat.feature.{app_key}.{row['source_distilled_feature_id'].replace('.', '-')}.v1",
                    "source_ids": row.get("source_ids", []) + ["COMPAT-S01"],
                    "source_app_key": app_key,
                    "source_family": "adobe" if app_key in {"photoshop", "indesign", "illustrator"} else app_key,
                    "source_feature_row_id": row.get("source_distilled_feature_id"),
                    "feature_name": row.get("feature_name"),
                    "support_kind": support_kind_from_compat(compat),
                    "format_refs": formats,
                    "studio_primitive": row.get("primitive_domain"),
                    "provider_posture": row.get("provider_posture"),
                    "fixture_requirement": "required_before_claiming_format_compatibility",
                    "round_trip_rule": "record preserved translated lossy and unsupported constructs in import/export receipts",
                    "manual_topic_candidate": row.get("manual_topic_candidate"),
                }
            )
    return records


def domain_format_records(domains_by_app):
    records = []
    for app_key, domains in domains_by_app.items():
        for domain in domains:
            text = domain_format_text(domain)
            formats = detect_formats(text)
            if not formats:
                continue
            records.append(
                {
                    "compatibility_record_id": f"compat.domain.{app_key}.{domain['id'].replace('.', '-')}.v1",
                    "source_ids": ["COMPAT-S01", f"COMPAT-{app_key.upper()}"],
                    "source_app_key": app_key,
                    "source_domain_id": domain.get("id"),
                    "source_domain_name": domain.get("name"),
                    "support_kind": "fixture_required",
                    "format_refs": formats,
                    "studio_primitive_domains": domain.get("studio_primitive_domains", []),
                    "fixture_requirement": "create representative source fixture set before implementation claim",
                    "round_trip_rule": "domain-level format mention must be refined into import export or round-trip command contracts",
                    "manual_topic_candidate": domain.get("manual_topic_candidate"),
                }
            )
    return records


def native_format_records():
    records = []
    for app_key, format_ids in APP_NATIVE_FORMATS.items():
        for format_id in format_ids:
            label = next(label for fid, label, _ in FORMAT_PATTERNS if fid == format_id)
            records.append(
                {
                    "compatibility_record_id": f"compat.native.{app_key}.{format_id.replace('.', '-')}.v1",
                    "source_ids": ["COMPAT-S01", f"COMPAT-{app_key.upper()}"],
                    "source_app_key": app_key,
                    "source_family": "adobe" if app_key in {"photoshop", "indesign", "illustrator"} else app_key,
                    "support_kind": "round_trip",
                    "format_refs": [{"format_id": format_id, "format_label": label}],
                    "fixture_requirement": "golden native document fixtures covering layers text color links effects masks and export settings",
                    "round_trip_rule": "native format compatibility requires import export diagnostics and explicit unsupported-feature receipts",
                    "manual_topic_candidate": f"studio.manual.file-compatibility.{app_key}.native",
                }
            )
    return records


def format_family_matrix(records):
    by_format = collections.defaultdict(list)
    for record in records:
        for fmt in record.get("format_refs", []):
            by_format[fmt["format_id"]].append(record)
    matrix = []
    for format_id, grouped in sorted(by_format.items()):
        labels = sorted({fmt["format_label"] for record in grouped for fmt in record.get("format_refs", []) if fmt["format_id"] == format_id})
        apps = sorted({record["source_app_key"] for record in grouped})
        support = {app: sorted({record["support_kind"] for record in grouped if record["source_app_key"] == app}) for app in apps}
        matrix.append(
            {
                "format_id": format_id,
                "format_labels": labels,
                "source_apps_present": apps,
                "support_by_app": support,
                "record_count": len(grouped),
                "fixture_policy": "fixture_required_for_every_supported_app_and_direction",
                "compatibility_posture": "native_round_trip_target" if any(record["compatibility_record_id"].startswith("compat.native") for record in grouped) else "source_observable_import_export_target",
            }
        )
    return matrix


def main():
    rows_by_app = {app: load_rows(file_name) for app, file_name in ROW_FILES.items()}
    domains_by_app = {app: load_domains(file_name) for app, file_name in DOMAIN_FILES.items()}
    feature_records = feature_compatibility_records(rows_by_app)
    domain_records = domain_format_records(domains_by_app)
    native_records = native_format_records()
    all_records = native_records + domain_records + feature_records
    matrix = format_family_matrix(all_records)

    coverage = {
        "coverage": {
            "distillation_status": "source_distilled_file_format_compatibility_registry",
            "compatibility_record_count": len(all_records),
            "native_format_record_count": len(native_records),
            "domain_format_record_count": len(domain_records),
            "feature_format_record_count": len(feature_records),
            "format_family_count": len(matrix),
            "policy": {
                "format_compatibility_rule": "Do not invent a replacement interchange format for Studio parity scope.",
                "fixture_rule": "Every import/export/round-trip claim needs representative fixtures and receipts.",
                "native_rule": "Native source formats are compatibility targets with explicit unsupported-feature diagnostics.",
                "local_first_rule": "Provider/cloud publishing is optional adapter behavior; local import/export fixtures remain primary.",
            },
            "source_files": {
                "feature_rows": ROW_FILES,
                "domain_ledgers": DOMAIN_FILES,
            },
        }
    }

    records = {
        "format_family_matrix": matrix,
        "compatibility_records": all_records,
    }

    sources = {
        "sources": [
            {"id": "COMPAT-S01", "path": "33-online-source-distilled-feature-ledger.md", "note": "Canonical file-format compatibility policy."},
            {"id": "COMPAT-PHOTOSHOP", "path": "34-photoshop-source-distilled-domain-ledger.md", "note": "Photoshop format mentions."},
            {"id": "COMPAT-INDESIGN", "path": "35-indesign-source-distilled-domain-ledger.md", "note": "InDesign format mentions."},
            {"id": "COMPAT-ILLUSTRATOR", "path": "36-illustrator-source-distilled-domain-ledger.md", "note": "Illustrator format mentions."},
            {"id": "COMPAT-AFFINITY", "path": "37-affinity-source-distilled-domain-ledger.md", "note": "Affinity format mentions."},
            {"id": "COMPAT-FIGMA", "path": "38-figma-source-distilled-domain-ledger.md", "note": "Figma format mentions."},
            {"id": "COMPAT-ROWS", "path": "39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md", "note": "Generated feature rows with compatibility posture."},
        ]
    }

    frontmatter = {
        "file_id": "file-format-compatibility-registry",
        "file_kind": "source_distilled_file_format_compatibility_registry",
        "topic_id": "SFR-FILE-FORMAT-COMPATIBILITY-REGISTRY",
        "title": "File Format Compatibility Registry",
        "status": "draft",
        "updated_at": UPDATED_AT,
        "compatibility_record_count": len(all_records),
        "format_family_count": len(matrix),
        "native_format_record_count": len(native_records),
        "domain_format_record_count": len(domain_records),
        "feature_format_record_count": len(feature_records),
    }

    text = "\n".join(
        [
            "---",
            yaml.safe_dump(frontmatter, sort_keys=False, allow_unicode=False).strip(),
            "---",
            "",
            "## [SFR-FILE-FORMAT-COMPATIBILITY-REGISTRY] File Format Compatibility Registry",
            "",
            '<topic id="compatibility-coverage" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Coverage and policy for source-distilled file-format compatibility records.">',
            "",
            "### [SFR-FILE-FORMAT-COMPATIBILITY-REGISTRY.coverage] Coverage",
            "",
            "```yaml",
            yaml.safe_dump(coverage, sort_keys=False, allow_unicode=False, width=160).strip(),
            "```",
            "",
            "</topic>",
            "",
            '<topic id="compatibility-records" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Machine-readable format-family matrix and compatibility records.">',
            "",
            "### [SFR-FILE-FORMAT-COMPATIBILITY-REGISTRY.records] Compatibility Records",
            "",
            "```yaml",
            yaml.safe_dump(records, sort_keys=False, allow_unicode=False, width=180).strip(),
            "```",
            "",
            "</topic>",
            "",
            '<topic id="sources" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Sources for the generated file-format compatibility registry.">',
            "",
            "### [SFR-FILE-FORMAT-COMPATIBILITY-REGISTRY.sources] Sources",
            "",
            "```yaml",
            yaml.safe_dump(sources, sort_keys=False, allow_unicode=False, width=140).strip(),
            "```",
            "",
            "</topic>",
            "",
        ]
    )
    (ROOT / "46-file-format-compatibility-registry.md").write_text(text, encoding="utf-8")
    print("46-file-format-compatibility-registry.md")
    print(f"compatibility_records: {len(all_records)}")
    print(f"format_families: {len(matrix)}")
    print(f"native_records: {len(native_records)}")
    print(f"domain_records: {len(domain_records)}")
    print(f"feature_records: {len(feature_records)}")


if __name__ == "__main__":
    main()
