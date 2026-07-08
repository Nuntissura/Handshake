import collections
import pathlib
import re

import yaml


ROOT = pathlib.Path(__file__).resolve().parents[1]
UPDATED_AT = "2026-07-05"

DOMAIN_FILES = {
    "photoshop": {
        "file": "34-photoshop-source-distilled-domain-ledger.md",
        "source_family": "adobe",
        "source_app_label": "Photoshop and Camera Raw",
    },
    "indesign": {
        "file": "35-indesign-source-distilled-domain-ledger.md",
        "source_family": "adobe",
        "source_app_label": "InDesign",
    },
    "illustrator": {
        "file": "36-illustrator-source-distilled-domain-ledger.md",
        "source_family": "adobe",
        "source_app_label": "Illustrator",
    },
    "affinity": {
        "file": "37-affinity-source-distilled-domain-ledger.md",
        "source_family": "affinity",
        "source_app_label": "Affinity Photo/Designer/Publisher",
    },
    "figma": {
        "file": "38-figma-source-distilled-domain-ledger.md",
        "source_family": "figma",
        "source_app_label": "Figma product family",
    },
}

STOP_ITEMS = {
    "",
    "and",
    "where documented",
    "where applicable",
    "related panels",
    "related tools",
    "related",
    "navigation-related",
    "navigation-related tools",
    "and related panels",
}

TRAILING_QUALIFIERS = [
    " where documented",
    " where applicable",
    " where available",
    " where supported",
    " workflows",
    " workflow",
    " tools",
    " tool",
    " panels",
    " panel",
]


def yaml_blocks(text: str):
    return re.findall(r"```yaml\n(.*?)\n```", text, flags=re.S)


def load_domains(file_name: str):
    for block in yaml_blocks((ROOT / file_name).read_text(encoding="utf-8")):
        data = yaml.safe_load(block)
        if isinstance(data, dict) and "domains" in data:
            return data["domains"]
    raise RuntimeError(f"No domains block found in {file_name}")


def slug(value: str):
    return re.sub(r"[^a-z0-9]+", "-", str(value).lower()).strip("-")


def normalize_label(value: str):
    value = re.sub(r"\s+", " ", str(value).strip())
    value = value.strip(" .;:")
    for qualifier in TRAILING_QUALIFIERS:
        if value.lower().endswith(qualifier):
            value = value[: -len(qualifier)].strip()
    return value


def flatten_scope(scope):
    if isinstance(scope, list):
        for item in scope:
            yield str(item)
    elif isinstance(scope, dict):
        for key, values in scope.items():
            if isinstance(values, list):
                for item in values:
                    yield f"{key}: {item}"
            else:
                yield f"{key}: {values}"


def split_scope_line(line: str):
    line = re.sub(r"^[a-z0-9_]+:\s*", "", line, flags=re.I)
    line = line.replace(" and ", ", ")
    line = line.replace("; ", ", ")
    for raw in line.split(","):
        item = normalize_label(raw)
        if not item or item.lower() in STOP_ITEMS:
            continue
        if len(item) < 3:
            continue
        yield item


def classify_kind(label: str):
    lowered = label.lower()
    if "panel" in lowered or "sidebar" in lowered or "toolbar" in lowered:
        return "panel_or_inspector"
    if "persona" in lowered:
        return "workspace_mode"
    if any(word in lowered for word in ["api", "script", "plugin", "webhook", "mcp", "code connect"]):
        return "automation_or_api_surface"
    if any(word in lowered for word in ["export", "save", "import", "open", "place", "package", "print", "publish"]):
        return "file_or_output_surface"
    if any(word in lowered for word in ["brush", "pen", "pencil", "lasso", "marquee", "wand", "crop", "zoom", "hand", "type", "shape", "selection", "node", "move"]):
        return "interactive_tool"
    return "command_or_workflow_surface"


def registry_row(app_key: str, config: dict, domain: dict, label: str):
    normalized = slug(label)
    primitive_domains = domain.get("studio_primitive_domains") or ["unknown"]
    return {
        "tool_registry_id": f"tool.{app_key}.{slug(domain.get('id'))}.{normalized}.v1",
        "source_ids": ["TOOLREG-S01", f"TOOLREG-{app_key.upper()}"],
        "source_family": config["source_family"],
        "source_app_key": app_key,
        "source_app_label": config["source_app_label"],
        "source_domain_id": domain.get("id"),
        "source_domain_name": domain.get("name"),
        "source_label": label,
        "normalized_label": normalized,
        "tool_kind": classify_kind(label),
        "source_distillation_status": "domain_scope_distilled_tool_or_surface_seed",
        "studio_primitive_domains": primitive_domains,
        "primary_studio_primitive": primitive_domains[0],
        "overlap_policy": "implement_once_in_handshake_native_studio_primitive_with_source_specific_variants",
        "affinity_overlap_guard": "preserve_affinity_provenance_do_not_relabel_as_adobe" if app_key == "affinity" else "not_applicable",
        "figma_local_first_guard": "local_first_replacement_for_cloud_collaboration_where_needed" if app_key == "figma" else "not_applicable",
        "manual_topic_candidate": domain.get("manual_topic_candidate"),
        "implementation_readiness": "needs_exact_source_page_or_behavior_promotion",
    }


def main():
    rows = []
    source_row_sets = []
    for app_key, config in DOMAIN_FILES.items():
        domains = load_domains(config["file"])
        seen = set()
        for domain in domains:
            for line in flatten_scope(domain.get("tool_and_feature_scope", [])):
                for label in split_scope_line(line):
                    key = (domain.get("id"), slug(label))
                    if key in seen:
                        continue
                    seen.add(key)
                    rows.append(registry_row(app_key, config, domain, label))
        source_row_sets.append(
            {
                "source_app_key": app_key,
                "source_family": config["source_family"],
                "source_domain_file": config["file"],
                "source_domain_count": len(domains),
                "tool_registry_row_count": sum(1 for row in rows if row["source_app_key"] == app_key),
            }
        )

    normalized_counts = collections.Counter(row["normalized_label"] for row in rows)
    cross_app_overlap = []
    rows_by_normalized = collections.defaultdict(list)
    for row in rows:
        rows_by_normalized[row["normalized_label"]].append(row)
    for normalized, grouped in sorted(rows_by_normalized.items()):
        apps = sorted({row["source_app_key"] for row in grouped})
        if len(apps) < 2:
            continue
        cross_app_overlap.append(
            {
                "normalized_label": normalized,
                "source_labels": sorted({row["source_label"] for row in grouped}),
                "source_apps_present": apps,
                "row_refs": [row["tool_registry_id"] for row in grouped],
                "equivalence_claim": "name_overlap_only_not_behavioral_equivalence",
                "implementation_rule": "group_under_studio_primitive_only_after_source_behavior_comparison",
            }
        )

    coverage = {
        "coverage": {
            "distillation_status": "source_distilled_tool_registry_seed",
            "row_count": len(rows),
            "unique_normalized_label_count": len(normalized_counts),
            "cross_app_name_overlap_count": len(cross_app_overlap),
            "source_row_sets": source_row_sets,
            "policy": {
                "provenance_rule": "Each row keeps source app provenance and source domain.",
                "overlap_rule": "Name overlap is not behavioral equivalence; Studio implementation reuse happens through primitive promotion.",
                "affinity_rule": "Affinity rows remain Affinity-provenance rows even when names or primitives overlap with Adobe.",
                "local_first_rule": "Cloud/provider tools are retained as source behavior but implemented as local-first primitives or optional adapters.",
            },
        }
    }

    records = {
        "tool_registry_rows": rows,
        "cross_app_name_overlap": cross_app_overlap,
    }

    sources = {
        "sources": [
            {"id": "TOOLREG-S01", "path": "33-online-source-distilled-feature-ledger.md", "note": "Canonical source-distilled merge policy."},
            {"id": "TOOLREG-PHOTOSHOP", "path": "34-photoshop-source-distilled-domain-ledger.md", "note": "Photoshop source-distilled tool scopes."},
            {"id": "TOOLREG-INDESIGN", "path": "35-indesign-source-distilled-domain-ledger.md", "note": "InDesign source-distilled tool scopes."},
            {"id": "TOOLREG-ILLUSTRATOR", "path": "36-illustrator-source-distilled-domain-ledger.md", "note": "Illustrator source-distilled tool scopes."},
            {"id": "TOOLREG-AFFINITY", "path": "37-affinity-source-distilled-domain-ledger.md", "note": "Affinity source-distilled tool scopes."},
            {"id": "TOOLREG-FIGMA", "path": "38-figma-source-distilled-domain-ledger.md", "note": "Figma source-distilled tool scopes."},
            {"id": "TOOLREG-DEDUPE", "path": "44-cross-app-overlap-and-affinity-dedupe-map.md", "note": "Cross-app overlap and Affinity dedupe policy."},
        ]
    }

    frontmatter = {
        "file_id": "source-distilled-tool-registry",
        "file_kind": "source_distilled_tool_registry",
        "topic_id": "SFR-SOURCE-DISTILLED-TOOL-REGISTRY",
        "title": "Source Distilled Tool Registry",
        "status": "draft",
        "updated_at": UPDATED_AT,
        "tool_registry_row_count": len(rows),
        "unique_normalized_label_count": len(normalized_counts),
        "cross_app_name_overlap_count": len(cross_app_overlap),
    }

    text = "\n".join(
        [
            "---",
            yaml.safe_dump(frontmatter, sort_keys=False, allow_unicode=False).strip(),
            "---",
            "",
            "## [SFR-SOURCE-DISTILLED-TOOL-REGISTRY] Source Distilled Tool Registry",
            "",
            '<topic id="tool-registry-coverage" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Coverage and policy for source-distilled tool registry rows.">',
            "",
            "### [SFR-SOURCE-DISTILLED-TOOL-REGISTRY.coverage] Coverage",
            "",
            "```yaml",
            yaml.safe_dump(coverage, sort_keys=False, allow_unicode=False, width=160).strip(),
            "```",
            "",
            "</topic>",
            "",
            '<topic id="tool-registry-rows" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Machine-readable source-distilled tool and surface registry rows.">',
            "",
            "### [SFR-SOURCE-DISTILLED-TOOL-REGISTRY.rows] Tool Registry Rows",
            "",
            "```yaml",
            yaml.safe_dump(records, sort_keys=False, allow_unicode=False, width=180).strip(),
            "```",
            "",
            "</topic>",
            "",
            '<topic id="sources" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Sources for the generated source-distilled tool registry.">',
            "",
            "### [SFR-SOURCE-DISTILLED-TOOL-REGISTRY.sources] Sources",
            "",
            "```yaml",
            yaml.safe_dump(sources, sort_keys=False, allow_unicode=False, width=140).strip(),
            "```",
            "",
            "</topic>",
            "",
        ]
    )
    (ROOT / "45-source-distilled-tool-registry.md").write_text(text, encoding="utf-8")
    print("45-source-distilled-tool-registry.md")
    print(f"tool_registry_rows: {len(rows)}")
    print(f"unique_normalized_labels: {len(normalized_counts)}")
    print(f"cross_app_name_overlaps: {len(cross_app_overlap)}")


if __name__ == "__main__":
    main()
