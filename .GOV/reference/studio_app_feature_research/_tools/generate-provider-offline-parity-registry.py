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

LOCAL_POSTURES = {"local_primitive", "local_primitive_candidate", "not_applicable", None}
ALWAYS_INCLUDE_DOMAINS = {
    "ai",
    "automation",
    "collaboration",
    "dev_mode",
    "interactive",
    "motion",
    "whiteboard",
}
PROVIDER_KEYWORDS = {
    "agent",
    "ai",
    "api",
    "branch",
    "cloud",
    "code connect",
    "collaboration",
    "comment",
    "community",
    "dev mode",
    "firefly",
    "generate",
    "generative",
    "library",
    "make",
    "mcp",
    "multiplayer",
    "plugin",
    "publish",
    "review",
    "share",
    "stock",
    "template",
    "webhook",
    "widget",
}


def yaml_blocks(text: str):
    return re.findall(r"```yaml\n(.*?)\n```", text, flags=re.S)


def load_feature_rows(file_name: str):
    for block in yaml_blocks((ROOT / file_name).read_text(encoding="utf-8")):
        data = yaml.safe_load(block)
        if isinstance(data, dict) and "source_distilled_feature_rows" in data:
            return data["source_distilled_feature_rows"]
    raise RuntimeError(f"No feature rows in {file_name}")


def slug(value: str):
    return re.sub(r"[^a-z0-9]+", "-", str(value).lower()).strip("-")


def selected(row: dict):
    posture = row.get("provider_posture")
    domain = row.get("primitive_domain")
    haystack = " ".join(
        str(row.get(key, ""))
        for key in [
            "feature_name",
            "source_category",
            "source_subcategory",
            "app_behavior",
            "user_goal",
            "manual_topic_candidate",
        ]
    ).lower()
    return (
        posture not in LOCAL_POSTURES
        or domain in ALWAYS_INCLUDE_DOMAINS
        or any(keyword in haystack for keyword in PROVIDER_KEYWORDS)
    )


def offline_parity_class(row: dict):
    posture = row.get("provider_posture")
    domain = row.get("primitive_domain")
    if posture == "compatibility_shim":
        return "compatibility_shim_with_local_receipts"
    if posture in {"provider_adapter", "optional_integration", "provider_adapter_or_local_model_candidate"}:
        return "optional_provider_adapter_with_offline_fallback"
    if posture == "local_first_collaboration_primitive" or domain in {"collaboration", "dev_mode", "motion", "whiteboard"}:
        return "local_first_collaboration_or_model_surface"
    if domain == "ai":
        return "core_local_model_or_optional_provider_adapter"
    if domain in {"interactive", "automation"}:
        return "local_first_runtime_or_automation_surface"
    return "core_local_primitive"


def local_first_requirement(row: dict, parity_class: str):
    domain = row.get("primitive_domain")
    if parity_class == "compatibility_shim_with_local_receipts":
        return "Implement local import/export or API compatibility with fixture-backed lossy-conversion and unsupported-feature receipts."
    if parity_class == "optional_provider_adapter_with_offline_fallback":
        return "Implement the Studio command and state transition locally first; provider access is an optional adapter and must have a deterministic offline fallback."
    if parity_class == "local_first_collaboration_or_model_surface":
        return "Implement local project state, receipts, history, attribution, and conflict recovery so collaboration/model workflows operate without a hosted service."
    if parity_class == "core_local_model_or_optional_provider_adapter":
        return "Prefer local model execution or deterministic local tool behavior; remote model providers are optional adapters with recorded prompts, seeds, provenance, and fallback outcomes."
    if parity_class == "local_first_runtime_or_automation_surface":
        return "Implement local runtime state and automation receipts so interactive output and scripted workflows remain reproducible offline."
    return f"Implement the {domain} behavior as a built-in Studio primitive without a required network dependency."


def adapter_requirement(row: dict, parity_class: str):
    posture = row.get("provider_posture")
    if parity_class == "optional_provider_adapter_with_offline_fallback":
        return "Adapter must be isolated behind a capability flag, mockable in tests, and never required for opening, editing, saving, or exporting local documents."
    if parity_class == "compatibility_shim_with_local_receipts":
        return "Compatibility adapter must emit per-feature preservation, downgrade, and unsupported-data receipts."
    if parity_class == "core_local_model_or_optional_provider_adapter":
        return "Remote provider adapter may exist, but local model/tool path remains the default parity path for the Studio module."
    if posture in {"local_primitive", "local_primitive_candidate"}:
        return "No provider adapter required for the source-distilled parity row."
    return "No mandatory provider adapter; add one only when it preserves source-app interoperability or optional operator workflow."


def receipt_requirement(row: dict, parity_class: str):
    base = "Receipt must include source feature id, Studio primitive, command id, input refs, output refs, warnings, offline/adapter mode, and undo/replay handles."
    if parity_class == "compatibility_shim_with_local_receipts":
        return base + " Include preserved fields, dropped fields, converted fields, fixture id, and round-trip status."
    if parity_class == "optional_provider_adapter_with_offline_fallback":
        return base + " Include provider name when used, fallback path, provider response hash, and local substitute result."
    if parity_class == "local_first_collaboration_or_model_surface":
        return base + " Include actor id, operation id, conflict status, merge result, and recovery action."
    if parity_class == "core_local_model_or_optional_provider_adapter":
        return base + " Include model/tool version, seed or deterministic input hash, prompt/policy refs where applicable, and provenance."
    return base


def implementation_readiness(row: dict, parity_class: str):
    if row.get("implementation_readiness") != "needs_command_contract_promotion":
        return row.get("implementation_readiness")
    if parity_class == "compatibility_shim_with_local_receipts":
        return "needs_command_contract_plus_format_fixture_promotion"
    if parity_class == "optional_provider_adapter_with_offline_fallback":
        return "needs_command_contract_plus_offline_fallback_and_adapter_mock"
    if parity_class == "local_first_collaboration_or_model_surface":
        return "needs_command_contract_plus_local_collaboration_receipt_contract"
    if parity_class == "core_local_model_or_optional_provider_adapter":
        return "needs_command_contract_plus_local_model_or_tool_fallback_contract"
    return "needs_command_contract_promotion"


def registry_row(app_key: str, row: dict, sequence: int):
    parity_class = offline_parity_class(row)
    feature_slug = slug(row.get("feature_name") or row.get("source_feature_id") or sequence)
    domain = row.get("primitive_domain", "unknown")
    return {
        "provider_offline_parity_id": f"provider-offline.{app_key}.{domain}.{feature_slug}.{sequence:04d}.v1",
        "source_ids": ["PROVIDER-OFFLINE-S01", f"PROVIDER-OFFLINE-{app_key.upper()}"],
        "source_app_key": app_key,
        "source_distilled_feature_id": row.get("source_distilled_feature_id"),
        "feature_use_card_id": row.get("feature_use_card_id"),
        "source_feature_id": row.get("source_feature_id"),
        "feature_name": row.get("feature_name"),
        "primitive_domain": domain,
        "studio_surface": row.get("studio_surface"),
        "provider_posture": row.get("provider_posture"),
        "file_format_compatibility": row.get("file_format_compatibility"),
        "offline_parity_class": parity_class,
        "local_first_requirement": local_first_requirement(row, parity_class),
        "adapter_requirement": adapter_requirement(row, parity_class),
        "receipt_requirement": receipt_requirement(row, parity_class),
        "manual_topic_candidate": row.get("manual_topic_candidate"),
        "manual_required_when": row.get("manual_required_when"),
        "implementation_readiness": implementation_readiness(row, parity_class),
        "verification_refs": row.get("verification_refs", []),
        "command_contract_refs": row.get("command_contract_refs", []),
        "source_refs": row.get("source_refs", []),
    }


def counter_dict(counter):
    return dict(sorted(counter.items(), key=lambda item: item[0]))


def main():
    rows = []
    all_feature_counts = {}
    sequence = 1
    for app_key, file_name in FEATURE_ROW_FILES.items():
        feature_rows = load_feature_rows(file_name)
        all_feature_counts[app_key] = len(feature_rows)
        for source_row in feature_rows:
            if selected(source_row):
                rows.append(registry_row(app_key, source_row, sequence))
                sequence += 1

    by_app = collections.Counter(row["source_app_key"] for row in rows)
    by_domain = collections.Counter(row["primitive_domain"] for row in rows)
    by_posture = collections.Counter(row["provider_posture"] for row in rows)
    by_parity = collections.Counter(row["offline_parity_class"] for row in rows)
    app_domain_matrix = collections.defaultdict(collections.Counter)
    app_parity_matrix = collections.defaultdict(collections.Counter)
    for row in rows:
        app_domain_matrix[row["source_app_key"]][row["primitive_domain"]] += 1
        app_parity_matrix[row["source_app_key"]][row["offline_parity_class"]] += 1

    coverage = {
        "source_feature_row_files": FEATURE_ROW_FILES,
        "total_source_feature_rows": sum(all_feature_counts.values()),
        "selected_provider_offline_rows": len(rows),
        "source_feature_rows_by_app": counter_dict(collections.Counter(all_feature_counts)),
        "provider_offline_rows_by_app": counter_dict(by_app),
        "provider_offline_rows_by_primitive_domain": counter_dict(by_domain),
        "provider_offline_rows_by_provider_posture": counter_dict(by_posture),
        "provider_offline_rows_by_offline_parity_class": counter_dict(by_parity),
        "app_domain_matrix": {app: counter_dict(counter) for app, counter in sorted(app_domain_matrix.items())},
        "app_offline_parity_matrix": {app: counter_dict(counter) for app, counter in sorted(app_parity_matrix.items())},
        "local_first_rule": "Provider, cloud, AI, collaboration, community, and compatibility behavior remains in the source inventory, but built-in Studio behavior must be local-first and network-optional.",
        "naming_rule": "Vendor app names remain source/provenance references. Studio command, panel, and manual names remain Handshake-native.",
        "source_private_schema_rule": "Vendor-private hosted behavior and undocumented proprietary schemas are compatibility targets, fixture targets, adapter surfaces, or local primitive requirements; they are not proof that Studio must depend on the vendor service.",
    }

    doc = {"provider_offline_parity_rows": rows}

    frontmatter = {
        "file_id": "48-provider-offline-parity-registry",
        "file_kind": "provider_offline_parity_registry",
        "topic_id": "SFR-PROVIDER-OFFLINE-PARITY-REGISTRY",
        "title": "Provider Offline Parity Registry",
        "status": "draft",
        "summary": "Generated registry that classifies source-distilled provider, cloud, AI, collaboration, runtime, automation, and compatibility-adjacent rows into local-first Studio parity requirements.",
        "updated_at": UPDATED_AT,
        "provider_offline_parity_row_count": len(rows),
        "source_feature_row_count": sum(all_feature_counts.values()),
    }

    source_block = {
        "sources": [
            {"id": "PROVIDER-OFFLINE-S01", "path": "33-online-source-distilled-feature-ledger.md", "note": "Unified source-distilled ledger policy."},
            {"id": "PROVIDER-OFFLINE-S02", "path": "11-provider-posture-map.md", "note": "Provider posture classifications for the first app set."},
            {"id": "PROVIDER-OFFLINE-S03", "path": "26-illustrator-figma-provider-posture-map.md", "note": "Provider posture classifications for Illustrator and Figma."},
            {"id": "PROVIDER-OFFLINE-S04", "path": "39-photoshop-source-distilled-feature-rows.md", "note": "Photoshop source-distilled feature rows."},
            {"id": "PROVIDER-OFFLINE-S05", "path": "40-indesign-source-distilled-feature-rows.md", "note": "InDesign source-distilled feature rows."},
            {"id": "PROVIDER-OFFLINE-S06", "path": "41-illustrator-source-distilled-feature-rows.md", "note": "Illustrator source-distilled feature rows."},
            {"id": "PROVIDER-OFFLINE-S07", "path": "42-affinity-source-distilled-feature-rows.md", "note": "Affinity source-distilled feature rows."},
            {"id": "PROVIDER-OFFLINE-S08", "path": "43-figma-source-distilled-feature-rows.md", "note": "Figma source-distilled feature rows."},
            {"id": "PROVIDER-OFFLINE-S09", "path": "44-cross-app-overlap-and-affinity-dedupe-map.md", "note": "Cross-app overlap and Affinity dedupe policy."},
            {"id": "PROVIDER-OFFLINE-S10", "path": "47-studio-rust-implementation-backlog.md", "note": "Implementation-facing primitive backlog."},
            {"id": "PROVIDER-OFFLINE-S11", "path": "_tools/generate-provider-offline-parity-registry.py", "note": "Provider/offline parity registry generator."},
        ]
    }

    text = "---\n"
    text += yaml.safe_dump(frontmatter, sort_keys=False, allow_unicode=False)
    text += "---\n\n"
    text += "## [SFR-PROVIDER-OFFLINE-PARITY-REGISTRY] Provider Offline Parity Registry\n\n"
    text += '<topic id="provider-offline-coverage" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Coverage and local-first policy for generated provider/offline parity rows.">\n\n'
    text += "### [SFR-PROVIDER-OFFLINE-PARITY-REGISTRY.coverage] Provider Offline Coverage\n\n"
    text += "```yaml\n"
    text += yaml.safe_dump({"provider_offline_parity_coverage": coverage}, sort_keys=False, allow_unicode=False, width=1400)
    text += "```\n\n</topic>\n\n"
    text += '<topic id="provider-offline-parity-rows" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Machine-readable provider/offline parity rows.">\n\n'
    text += "### [SFR-PROVIDER-OFFLINE-PARITY-REGISTRY.rows] Provider Offline Parity Rows\n\n"
    text += "```yaml\n"
    text += yaml.safe_dump(doc, sort_keys=False, allow_unicode=False, width=1400)
    text += "```\n\n</topic>\n\n"
    text += '<topic id="provider-offline-sources" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Sources for provider/offline parity registry.">\n\n'
    text += "### [SFR-PROVIDER-OFFLINE-PARITY-REGISTRY.sources] Sources\n\n"
    text += "```yaml\n"
    text += yaml.safe_dump(source_block, sort_keys=False, allow_unicode=False, width=1400)
    text += "```\n\n</topic>\n"

    (ROOT / "48-provider-offline-parity-registry.md").write_text(text, encoding="utf-8")
    print(f"wrote 48-provider-offline-parity-registry.md with {len(rows)} provider/offline parity rows")


if __name__ == "__main__":
    main()
