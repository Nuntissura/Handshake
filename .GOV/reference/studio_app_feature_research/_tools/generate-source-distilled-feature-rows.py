import pathlib
import re

import yaml


ROOT = pathlib.Path(__file__).resolve().parents[1]
UPDATED_AT = "2026-07-05"

APP_CONFIGS = [
    {
        "app_key": "photoshop",
        "title": "Photoshop Source Distilled Feature Rows",
        "topic_id": "SFR-PHOTOSHOP-SOURCE-DISTILLED-FEATURE-ROWS",
        "source_cards": "15-photoshop-feature-use-cards.md",
        "domain_ledger": "34-photoshop-source-distilled-domain-ledger.md",
        "output": "39-photoshop-source-distilled-feature-rows.md",
        "source_inventory": "SFR-PHOTOSHOP-LEAF-INDEX",
    },
    {
        "app_key": "indesign",
        "title": "InDesign Source Distilled Feature Rows",
        "topic_id": "SFR-INDESIGN-SOURCE-DISTILLED-FEATURE-ROWS",
        "source_cards": "17-indesign-feature-use-cards.md",
        "domain_ledger": "35-indesign-source-distilled-domain-ledger.md",
        "output": "40-indesign-source-distilled-feature-rows.md",
        "source_inventory": "SFR-INDESIGN-LEAF-INDEX",
    },
    {
        "app_key": "illustrator",
        "title": "Illustrator Source Distilled Feature Rows",
        "topic_id": "SFR-ILLUSTRATOR-SOURCE-DISTILLED-FEATURE-ROWS",
        "source_cards": "24-illustrator-feature-use-cards.md",
        "domain_ledger": "36-illustrator-source-distilled-domain-ledger.md",
        "output": "41-illustrator-source-distilled-feature-rows.md",
        "source_inventory": "22-illustrator-leaf-index.md",
    },
    {
        "app_key": "affinity",
        "title": "Affinity Source Distilled Feature Rows",
        "topic_id": "SFR-AFFINITY-SOURCE-DISTILLED-FEATURE-ROWS",
        "source_cards": "16-affinity-feature-use-cards.md",
        "domain_ledger": "37-affinity-source-distilled-domain-ledger.md",
        "output": "42-affinity-source-distilled-feature-rows.md",
        "source_inventory": "04-affinity-leaf-index.md and 09-affinity-desktop-delta.md",
    },
    {
        "app_key": "figma",
        "title": "Figma Source Distilled Feature Rows",
        "topic_id": "SFR-FIGMA-SOURCE-DISTILLED-FEATURE-ROWS",
        "source_cards": "25-figma-feature-use-cards.md",
        "domain_ledger": "38-figma-source-distilled-domain-ledger.md",
        "output": "43-figma-source-distilled-feature-rows.md",
        "source_inventory": "23-figma-leaf-index.md",
    },
]


FIGMA_CATEGORY_PRIMITIVES = [
    ("figjam", "whiteboard"),
    ("motion", "motion"),
    ("slides", "presentation"),
    ("sites", "web"),
    ("buzz", "brand_assets"),
    ("make", "ai"),
    ("developer", "dev_mode"),
    ("api", "dev_mode"),
    ("dev", "dev_mode"),
    ("design", "design_systems"),
]

FORMAT_IMPORT_KEYWORDS = [
    "import",
    "open",
    "place",
    "load",
    "sketch import",
]

FORMAT_EXPORT_KEYWORDS = [
    "export",
    "save",
    "download",
    "publish",
    "package",
    "print",
]

FORMAT_GENERAL_KEYWORDS = [
    "supported file format",
    "file format",
    "local copy",
    "compatibility",
    "pdf",
    "svg",
    "eps",
    "psd",
    "png",
    "jpg",
    "jpeg",
    "gif",
    "webp",
    "tiff",
    "afphoto",
    "afdesign",
    "afpub",
    ".fig",
    ".jam",
    ".deck",
    ".buzz",
    ".site",
]

SOURCE_URL_PATH_OVERRIDES = {
    "https://help.figma.com/hc/en-us/articles/1500004362321-Guide-to-FigJam": "_source_snapshots/figma-figjam-guide-to-figjam-jina.md",
    "https://help.figma.com/hc/en-us/articles/1500007927941-Import-and-export-with-FigJam": "_source_snapshots/figma-figjam-import-export-jina.md",
    "https://help.figma.com/hc/en-us/articles/4407533721239-Import-spreadsheet-data-images-and-designs-to-FigJam": "_source_snapshots/figma-figjam-spreadsheet-data-jina.md",
    "https://help.figma.com/hc/en-us/articles/1500004290881-Place-images-video-and-GIFs-in-FigJam": "_source_snapshots/figma-figjam-media-jina.md",
    "https://help.figma.com/hc/en-us/categories/41274596092695-Figma-Motion": "_source_snapshots/figma-motion-category-jina.md",
    "https://help.figma.com/hc/en-us/categories/24146015318551-Figma-Slides": "_source_snapshots/figma-slides-category-jina.md",
    "https://help.figma.com/hc/en-us/categories/31823555275671-Figma-Sites": "_source_snapshots/figma-sites-category-jina.md",
    "https://help.figma.com/hc/en-us/categories/31194838351767-Figma-Buzz": "_source_snapshots/figma-buzz-category-jina.md",
    "https://help.figma.com/hc/en-us/categories/41306509921687-Build-with-Figma": "_source_snapshots/figma-build-category-jina.md",
    "https://help.figma.com/hc/en-us/sections/24369548041111": "_source_snapshots/figma-ai-section-jina.md",
    "https://help.figma.com/hc/en-us/sections/31830768959511": "_source_snapshots/figma-draw-section-jina.md",
    "https://help.figma.com/hc/en-us/categories/360002772634-Community": "_source_snapshots/figma-community-category-jina.md",
}


def yaml_blocks(text: str):
    return re.findall(r"```yaml\n(.*?)\n```", text, flags=re.S)


def load_feature_cards(path: pathlib.Path):
    for block in yaml_blocks(path.read_text(encoding="utf-8")):
        data = yaml.safe_load(block)
        if isinstance(data, dict) and isinstance(data.get("feature_use_cards"), list):
            return data["feature_use_cards"]
    raise RuntimeError(f"No feature_use_cards YAML block found in {path}")


def source_distilled_id(app_key: str, card: dict) -> str:
    raw = card.get("source_feature_id") or card["feature_use_card_id"]
    raw = str(raw).replace(" ", "-").replace("_", "-")
    raw = re.sub(r"[^a-zA-Z0-9.-]+", "-", raw).strip("-").lower()
    return f"osd.{app_key}.{raw}.v1"


def normalized_text(card: dict) -> str:
    parts = [
        card.get("feature_name", ""),
        card.get("source_category", ""),
        card.get("source_subcategory", ""),
    ]
    return " ".join(str(part).lower() for part in parts if part)


def normalize_file_format_compatibility(app_key: str, card: dict) -> str:
    text = normalized_text(card)
    padded = f" {text} "

    def has_keyword(keyword: str) -> bool:
        if re.fullmatch(r"[a-z0-9]+", keyword):
            return re.search(rf"\b{re.escape(keyword)}\b", text) is not None
        return keyword in text

    has_import = any(has_keyword(keyword) for keyword in FORMAT_IMPORT_KEYWORDS)
    has_export = any(has_keyword(keyword) for keyword in FORMAT_EXPORT_KEYWORDS)
    if has_import and has_export:
        return "round_trip"
    if has_export:
        return "export"
    if has_import:
        return "import"
    if any(keyword in text for keyword in FORMAT_GENERAL_KEYWORDS):
        return "fixture_required"
    if any(token in padded for token in [" .fig ", " .jam ", " .deck ", " .buzz ", " .site "]):
        return "fixture_required"

    raw = str(card.get("file_format_compatibility", "not_applicable")).lower()
    if app_key != "figma" and raw == "must_preserve_existing_format_compatibility":
        return "fixture_required"
    return "not_applicable"


def normalize_primitive_domain(app_key: str, card: dict) -> str:
    current = card.get("primitive_domain") or "unknown"
    if app_key != "figma":
        return current

    text = normalized_text(card)
    if normalize_file_format_compatibility(app_key, card) != "not_applicable":
        return "file_io"
    for marker, primitive in FIGMA_CATEGORY_PRIMITIVES:
        if marker in text:
            return primitive
    return current


def compact_row(app_key: str, card: dict, domain_ledger: str):
    handoff = card.get("user_manual_handoff") or {}
    source_refs = []
    for ref in card.get("source_refs") or []:
        local_snapshot_path = ref.get("path")
        label = ref.get("label", "")
        if not local_snapshot_path and label:
            candidate = pathlib.Path("_source_snapshots") / label
            if (ROOT / candidate).exists():
                local_snapshot_path = str(candidate).replace("\\", "/")
        if not local_snapshot_path and ref.get("url") in SOURCE_URL_PATH_OVERRIDES:
            candidate = SOURCE_URL_PATH_OVERRIDES[ref["url"]]
            if (ROOT / candidate).exists():
                local_snapshot_path = candidate
        source_refs.append(
            {
                "label": label,
                **({"path": local_snapshot_path} if local_snapshot_path else {}),
                **({"url": ref["url"]} if ref.get("url") else {}),
            }
        )

    return {
        "source_distilled_feature_id": source_distilled_id(app_key, card),
        "source_ids": ["ROWS-S01", "ROWS-S02"],
        "feature_use_card_id": card.get("feature_use_card_id"),
        "source_feature_id": card.get("source_feature_id"),
        "feature_name": card.get("feature_name"),
        "source_apps": card.get("source_apps", []),
        "source_inventory": card.get("source_inventory"),
        "source_category": card.get("source_category"),
        **({"source_subcategory": card["source_subcategory"]} if card.get("source_subcategory") else {}),
        "source_domain_ledger": domain_ledger,
        "feature_kind": "source_help_leaf_or_category_feature",
        "studio_surface": card.get("studio_surface"),
        "primitive_domain": normalize_primitive_domain(app_key, card),
        "provider_posture": card.get("provider_posture"),
        "file_format_compatibility": normalize_file_format_compatibility(app_key, card),
        "naming_posture": card.get("naming_posture"),
        "app_behavior": card.get("purpose"),
        "user_goal": card.get("user_goal"),
        "implementation_readiness": card.get("implementation_readiness"),
        "manual_topic_candidate": handoff.get("topic_candidate"),
        "manual_required_when": handoff.get("required_when"),
        "command_contract_refs": card.get("command_contract_refs", []),
        "verification_refs": card.get("verification_refs", []),
        "source_confidence": "online_source_distilled_from_feature_use_card",
        "source_refs": source_refs,
    }


def markdown_for_config(config: dict, rows: list):
    source_count = sum(len(row.get("source_refs", [])) for row in rows)
    frontmatter = {
        "file_id": config["output"].removesuffix(".md"),
        "file_kind": "source_distilled_feature_rows",
        "topic_id": config["topic_id"],
        "title": config["title"],
        "status": "draft",
        "updated_at": UPDATED_AT,
        "app_key": config["app_key"],
        "source_cards": config["source_cards"],
        "source_domain_ledger": config["domain_ledger"],
        "feature_row_count": len(rows),
        "source_ref_count": source_count,
    }
    coverage = {
        "coverage": {
            "app_key": config["app_key"],
            "source_cards": config["source_cards"],
            "source_inventory": config["source_inventory"],
            "source_domain_ledger": config["domain_ledger"],
            "feature_row_count": len(rows),
            "distillation_status": "online_source_distilled_feature_rows",
            "installed_exports_role": "optional_enrichment_only",
            "naming_rule": "Vendor product names remain source/provenance and compatibility references only.",
            "manual_handoff_rule": "Promote manual_topic_candidate into the internal Studio UserManual in the same change that implements the feature behavior.",
        }
    }
    records = {"source_distilled_feature_rows": rows}
    sources = {
        "sources": [
            {"id": "ROWS-S01", "path": config["source_cards"], "note": "Generated Feature Use Cards used as row source."},
            {"id": "ROWS-S02", "path": config["domain_ledger"], "note": "Online-source-distilled domain ledger used as row context."},
            {"id": "ROWS-S03", "path": "33-online-source-distilled-feature-ledger.md", "note": "Canonical source-distilled merge record."},
        ]
    }

    return "\n".join(
        [
            "---",
            yaml.safe_dump(frontmatter, sort_keys=False, allow_unicode=False).strip(),
            "---",
            "",
            f"## [{config['topic_id']}] {config['title']}",
            "",
            f"<topic id=\"feature-row-coverage\" status=\"current\" version=\"0.1\" updated_at=\"{UPDATED_AT}\" ingestable=\"true\" summary=\"Coverage and source policy for generated source-distilled feature rows.\">",
            "",
            f"### [{config['topic_id']}.coverage] Feature Row Coverage",
            "",
            "```yaml",
            yaml.safe_dump(coverage, sort_keys=False, allow_unicode=False, width=120).strip(),
            "```",
            "",
            "</topic>",
            "",
            f"<topic id=\"source-distilled-feature-rows\" status=\"current\" version=\"0.1\" updated_at=\"{UPDATED_AT}\" ingestable=\"true\" summary=\"Machine-readable source-distilled feature rows.\">",
            "",
            f"### [{config['topic_id']}.rows] Source Distilled Feature Rows",
            "",
            "```yaml",
            yaml.safe_dump(records, sort_keys=False, allow_unicode=False, width=160).strip(),
            "```",
            "",
            "</topic>",
            "",
            f"<topic id=\"sources\" status=\"current\" version=\"0.1\" updated_at=\"{UPDATED_AT}\" ingestable=\"true\" summary=\"Sources for this generated row ledger.\">",
            "",
            f"### [{config['topic_id']}.sources] Sources",
            "",
            "```yaml",
            yaml.safe_dump(sources, sort_keys=False, allow_unicode=False, width=120).strip(),
            "```",
            "",
            "</topic>",
            "",
        ]
    )


def main():
    total = 0
    for config in APP_CONFIGS:
        cards = load_feature_cards(ROOT / config["source_cards"])
        rows = [compact_row(config["app_key"], card, config["domain_ledger"]) for card in cards]
        output = ROOT / config["output"]
        output.write_text(markdown_for_config(config, rows), encoding="utf-8")
        total += len(rows)
        print(f"{config['output']}: {len(rows)} rows")
    print(f"total_rows: {total}")


if __name__ == "__main__":
    main()
