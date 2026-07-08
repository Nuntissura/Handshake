import html
import re
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(".GOV/reference/studio_app_feature_research")
SNAP = ROOT / "_source_snapshots"
DATE = "2026-07-05"


def clean(value):
    value = html.unescape("" if value is None else str(value))
    replacements = {
        "\u2018": "'",
        "\u2019": "'",
        "\u201c": '"',
        "\u201d": '"',
        "\u2013": "-",
        "\u2014": "-",
        "\u2026": "...",
        "\xa0": " ",
    }
    for src, dst in replacements.items():
        value = value.replace(src, dst)
    value = re.sub(r"\s+", " ", value).strip()
    return "".join(ch if ord(ch) < 128 else "?" for ch in value)


def q(value):
    return '"' + clean(value).replace("\\", "\\\\").replace('"', '\\"') + '"'


def slug(value):
    value = re.sub(r"[^a-z0-9]+", "-", clean(value).lower()).strip("-")
    return value or "unknown"


def links(path):
    text = path.read_text(encoding="utf-8", errors="ignore")
    out = []
    for match in re.finditer(r"\[([^\]\n]{1,180})\]\((https?://[^)\s]+)\)", text):
        label = clean(re.sub(r"!\[[^\]]*\]\([^)]*\)", "", match.group(1)))
        url = match.group(2).split("#")[0]
        url = (
            url.replace("/hc/categories/", "/hc/en-us/categories/")
            .replace("/hc/sections/", "/hc/en-us/sections/")
            .replace("/hc/articles/", "/hc/en-us/articles/")
        )
        if label and not label.lower().startswith("image"):
            out.append((label, url))
    return out


def primitive(name, url):
    text = (name + " " + url).lower()
    if any(s in text for s in ["ai", "generative", "agent", "make", "weave", "model", "prompt", "firefly", "credits"]):
        return "ai"
    if any(s in text for s in ["export", "import", "file", "save", "place", "linked", "embed", "download", "upload", "pdf", "svg", "png", "jpg", "jpeg", "webp", "gif", "pptx", "sketch", "fig", "jam", "dwg", "dxf", "eps"]):
        return "file_io"
    if any(s in text for s in ["motion", "animation", "keyframe", "easing", "prototype", "interaction", "present", "slide", "timeline"]):
        return "interactive"
    if any(s in text for s in ["component", "variant", "slot", "library", "style", "variables", "design system", "token"]):
        return "style_system"
    if any(s in text for s in ["text", "font", "type", "typography", "glyph"]):
        return "typography"
    if any(s in text for s in ["vector", "path", "pen", "pencil", "shape", "anchor", "curve", "stroke", "fill", "draw", "illustration", "boolean", "simplify", "corner", "arc", "star", "spiral", "paintbrush", "brush", "blob"]):
        return "vector"
    if any(s in text for s in ["frame", "auto layout", "layout", "constraint", "section", "page", "canvas", "artboard", "grid", "site", "responsive"]):
        return "page_layout"
    if any(s in text for s in ["color", "gradient", "pattern", "blend", "effect", "shadow", "blur", "image", "recolor"]):
        return "color"
    if any(s in text for s in ["comment", "collaborat", "cursor", "spotlight", "meeting", "vote", "branch", "merge", "community", "publish", "team", "organization", "permission", "share"]):
        return "collaboration"
    if any(s in text for s in ["select", "selection", "mask"]):
        return "selection"
    if any(s in text for s in ["workspace", "toolbar", "preferences", "keyboard", "zoom", "view", "navigation", "sidebar", "properties", "panel"]):
        return "workspace"
    return "vector"


def surface(domain):
    return {
        "vector": "StudioVectorPathGraph",
        "typography": "StudioTextRunAndStory",
        "page_layout": "StudioPageSpread",
        "style_system": "StudioStyleRegistry",
        "file_io": "StudioFileIO",
        "interactive": "StudioInteractiveDocumentSurface",
        "ai": "StudioModelToolContract",
        "collaboration": "StudioCollaborationSession",
        "workspace": "StudioWorkspaceSurface",
        "color": "StudioColorPipeline",
        "selection": "StudioSelectionSet",
        "raster": "StudioRasterPipeline",
        "prepress": "StudioPreflightProfile",
        "automation": "StudioActionGraph",
    }.get(domain, "StudioGeneralToolSurface")


def category(url, fallback):
    path = urlparse(url).path.lower()
    for key in [
        "new-features",
        "get-started",
        "add-and-import-files",
        "use-generative-ai",
        "draw-shapes-and-paths",
        "manage-objects",
        "color-and-styling",
        "tool-techniques",
        "supported-file-formats",
        "import-and-export",
    ]:
        if key in path:
            return key.replace("-", "_")
    return fallback


def provider(domain, name, url):
    text = (name + " " + url).lower()
    if domain == "ai" or any(s in text for s in ["ai", "agent", "make", "weave", "firefly", "credit", "github", "mcp", "community", "publish", "cloud", "web search"]):
        return "provider_adapter_or_local_model_candidate"
    if any(s in text for s in ["share", "team", "organization", "collaborat", "comment", "meeting", "spotlight", "cursor", "branch", "merge"]):
        return "local_first_collaboration_primitive"
    if domain == "file_io":
        return "compatibility_shim"
    return "local_primitive"


def file_compat(domain, name):
    text = (domain + " " + name).lower()
    if any(s in text for s in ["file", "import", "export", "save", "place", "download", "upload", "pdf", "svg", "png", "jpg", "sketch", "fig", "jam", "pptx", "dwg", "dxf", "eps", "format"]):
        return "must_preserve_existing_format_compatibility"
    return "not_applicable_runtime_state_command"


def unique_ids(records):
    seen = {}
    for record in records:
        base = record["id"]
        seen[base] = seen.get(base, 0) + 1
        if seen[base] > 1:
            record["id"] = f"{base}-{seen[base]}"


def illustrator_records():
    source_names = [
        "adobe-illustrator-desktop-jina.md",
        "illustrator-tools-jina.md",
        "illustrator-supported-file-formats-jina.md",
        "illustrator-release-notes-jina.md",
    ]
    rows = []
    seen = set()
    for source in source_names:
        for label, url in links(SNAP / source):
            if "helpx.adobe.com/illustrator" not in url:
                continue
            if not any(part in url for part in ["/desktop/", "/using/tool-techniques/", "/kb/supported-file-formats", "/desktop/new-features/", "/using/whats-new"]):
                continue
            key = url.lower()
            if key in seen:
                continue
            seen.add(key)
            name = clean(label.split('"')[0])
            if len(name) < 3:
                continue
            role = "support_context" if any(part in key for part in ["troubleshoot", "known-and-fixed", "technical-requirements", "system-requirements", "crash", "recover", "safe-mode", "repair", "preferences"]) else "feature_leaf"
            domain = primitive(name, url)
            rows.append(
                {
                    "id": "illustrator.desktop.leaf." + slug(urlparse(url).path.replace("/illustrator/", ""))[:90].strip("-"),
                    "app": "Illustrator desktop",
                    "name": name,
                    "source_category": category(url, "illustrator"),
                    "primitive_domain": domain,
                    "record_role": role,
                    "source_url": url,
                    "source_snapshot": source,
                }
            )
    unique_ids(rows)
    return rows


def figma_records():
    source_names = [
        "figma-design-category-jina.md",
        "figma-make-category-jina.md",
        "figma-import-export-jina.md",
        "figma-imports-jina.md",
        "figma-export-formats-jina.md",
        "figma-export-static-jina.md",
        "figma-api-docs-jina.md",
        "figma-release-notes-jina.md",
    ]
    rows = []
    seen = set()
    for source in source_names:
        for label, url in links(SNAP / source):
            if "help.figma.com/hc/" not in url and "figma.com/developers" not in url and "figma.com/release-notes" not in url:
                continue
            if not ("/articles/" in url or "developers" in url or "release-notes" in url):
                continue
            key = url.lower()
            if key in seen:
                continue
            seen.add(key)
            name = clean(label.split('"')[0])
            if not name or name.lower() in ["learn more", "accessibility"]:
                continue
            app = "Figma Make" if "make" in name.lower() or "figma-make" in url.lower() else "Figma Design"
            if "api" in name.lower() or "developers" in url.lower():
                app = "Figma Developer Platform"
            domain = primitive(name, url)
            rows.append(
                {
                    "id": "figma.platform.leaf." + slug(urlparse(url).path.replace("/hc/en-us/articles/", "").replace("/hc/articles/", "").replace("/", "-"))[:90].strip("-"),
                    "app": app,
                    "name": name,
                    "source_category": app.lower().replace(" ", "_"),
                    "primitive_domain": domain,
                    "record_role": "feature_leaf",
                    "source_url": url,
                    "source_snapshot": source,
                }
            )
    manual = [
        ("figma.figjam.leaf.guide-to-figjam", "FigJam", "Guide to FigJam", "figjam_canvas", "page_layout", "https://help.figma.com/hc/en-us/articles/1500004362321-Guide-to-FigJam", "figma-figjam-guide-to-figjam-jina.md"),
        ("figma.figjam.leaf.import-export", "FigJam", "Import and export with FigJam", "figjam_import_export", "file_io", "https://help.figma.com/hc/en-us/articles/1500007927941-Import-and-export-with-FigJam", "figma-figjam-import-export-jina.md"),
        ("figma.figjam.leaf.spreadsheet-data", "FigJam", "Import spreadsheet data, images, and designs to FigJam", "figjam_import_export", "file_io", "https://help.figma.com/hc/en-us/articles/4407533721239-Import-spreadsheet-data-images-and-designs-to-FigJam", "figma-figjam-spreadsheet-data-jina.md"),
        ("figma.figjam.leaf.media", "FigJam", "Place images, video, and GIFs in FigJam", "figjam_media", "file_io", "https://help.figma.com/hc/en-us/articles/1500004290881-Place-images-video-and-GIFs-in-FigJam", "figma-figjam-media-jina.md"),
        ("figma.motion.leaf.category", "Figma Motion", "Figma Motion timeline, keyframes, easing, anchors, and preset animations", "figma_motion", "interactive", "https://help.figma.com/hc/en-us/categories/41274596092695-Figma-Motion", "figma-motion-category-jina.md"),
        ("figma.slides.leaf.category", "Figma Slides", "Slide decks, templates, prototypes in slides, presenter notes, presentation, PowerPoint import, and export", "figma_slides", "interactive", "https://help.figma.com/hc/en-us/categories/24146015318551-Figma-Slides", "figma-slides-category-jina.md"),
        ("figma.sites.leaf.category", "Figma Sites", "Responsive sites, breakpoints, blocks, embeds, CMS, interactions, preview, and publish", "figma_sites", "page_layout", "https://help.figma.com/hc/en-us/categories/31823555275671-Figma-Sites", "figma-sites-category-jina.md"),
        ("figma.buzz.leaf.category", "Figma Buzz", "On-brand asset production workflows and templates", "figma_buzz", "automation", "https://help.figma.com/hc/en-us/categories/31194838351767-Figma-Buzz", "figma-buzz-category-jina.md"),
        ("figma.build.leaf.dev-mode", "Build with Figma", "Dev Mode inspect, measurements, annotations, code snippets, Code Connect, VS Code, and MCP", "figma_build", "automation", "https://help.figma.com/hc/en-us/categories/41306509921687-Build-with-Figma", "figma-build-category-jina.md"),
        ("figma.ai.leaf.category", "Figma AI", "AI workflows, agents, search, image text prototype assistance, custom skills, attachments, MCP connectors", "figma_ai", "ai", "https://help.figma.com/hc/en-us/sections/24369548041111", "figma-ai-section-jina.md"),
        ("figma.draw.leaf.category", "Figma Draw", "Illustration tools, brushes, transforms, textures, vectorize, recolor, shape builder, simplify vector", "figma_draw", "vector", "https://help.figma.com/hc/en-us/sections/31830768959511", "figma-draw-section-jina.md"),
        ("figma.community.leaf.category", "Figma Community", "Community resources, templates, plugins, widgets, shaders, duplicate and publish flows", "figma_community", "collaboration", "https://help.figma.com/hc/en-us/categories/360002772634-Community", "figma-community-category-jina.md"),
    ]
    for rid, app, name, cat, domain, url, snapshot in manual:
        if url.lower() not in seen:
            rows.append({"id": rid, "app": app, "name": name, "source_category": cat, "primitive_domain": domain, "record_role": "feature_leaf", "source_url": url, "source_snapshot": snapshot})
    unique_ids(rows)
    return rows


def write_feature_map(path, topic_id, title, summary, records, sources):
    lines = [
        "---",
        f"file_id: {q(path.stem)}",
        f"topic_id: {topic_id}",
        f"title: {q(title)}",
        "status: draft",
        f"summary: {q(summary)}",
        f"sources: {len(sources)}",
        f"updated_at: {q(DATE)}",
        "---",
        "",
        f"## [{topic_id}] {title}",
        "",
        f"### [{topic_id}.inventory] Feature Families",
        "",
        "```yaml",
        "feature_families:",
    ]
    for rid, name, behavior, domain in records:
        lines += [
            f"  - id: {q(rid)}",
            f"    name: {q(name)}",
            f"    app_behavior: {q(behavior)}",
            f"    primitive_domain: {q(domain)}",
            f"    studio_surface: {q(surface(domain))}",
            '    naming_posture: "handshake_native_name_with_vendor_source_refs"',
            '    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"',
        ]
    lines += [
        "```",
        "",
        f"### [{topic_id}.implementation-notes] Implementation Notes",
        "",
        "```yaml",
        "implementation_notes:",
        '  local_first: "Studio is built-in, local-first, no-cloud-required, and Rust-forward."',
        '  parity_rule: "Vendor names define source behavior and compatibility only; Studio product surfaces use Handshake-native names."',
        '  file_format_rule: "Do not invent a replacement interchange format; implement compatibility adapters, fixtures, diagnostics, and explicit unsupported-feature receipts."',
        "```",
        "",
        f"### [{topic_id}.sources] Sources",
        "",
        "```yaml",
        "sources:",
    ]
    for sid, url, note in sources:
        lines.append(f"  - {{ id: {sid}, url: {q(url)}, note: {q(note)} }}")
    lines.append("```")
    path.write_text("\n".join(lines) + "\n", encoding="ascii")


def write_leaf_index(path, topic_id, title, app, records, source_note):
    lines = [
        "---",
        f"file_id: {q(path.stem)}",
        f"topic_id: {topic_id}",
        f"title: {q(title)}",
        "status: draft",
        'summary: "Generated leaf inventory from current official source snapshots and verified web-open/source-agent evidence."',
        "sources: 1",
        f"updated_at: {q(DATE)}",
        "---",
        "",
        f"## [{topic_id}] {title}",
        "",
        f"### [{topic_id}.inventory] Leaf Inventory",
        "",
        "```yaml",
        f"as_of: {q(DATE)}",
        f"app_family: {q(app)}",
        f"leaf_count: {len(records)}",
        f"feature_leaf_count: {sum(1 for r in records if r['record_role'] == 'feature_leaf')}",
        f"support_context_count: {sum(1 for r in records if r['record_role'] != 'feature_leaf')}",
        f"coverage_basis: {q(source_note)}",
        "records:",
    ]
    for record in records:
        lines += [
            f"  - id: {q(record['id'])}",
            f"    app: {q(record['app'])}",
            f"    name: {q(record['name'])}",
            f"    record_role: {q(record['record_role'])}",
            f"    source_category: {q(record['source_category'])}",
            f"    primitive_domain: {q(record['primitive_domain'])}",
            f"    studio_surface: {q(surface(record['primitive_domain']))}",
            '    naming_posture: "handshake_native_name_with_vendor_source_refs"',
            '    local_first_posture: "local_rust_core_unless_provider_or_compatibility"',
            f"    source_snapshot: {q(record['source_snapshot'])}",
            f"    source_url: {q(record['source_url'])}",
        ]
    lines += [
        "```",
        "",
        f"### [{topic_id}.sources] Sources",
        "",
        "```yaml",
        "sources:",
        '  - { id: SRC-SNAPSHOTS, path: "_source_snapshots/", note: "Local current-source snapshots and web-open/source-agent evidence used to generate this leaf index." }',
        "```",
    ]
    path.write_text("\n".join(lines) + "\n", encoding="ascii")


def purpose(name, domain):
    name = clean(name)
    return {
        "vector": f"Use {name} to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.",
        "page_layout": f"Use {name} to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.",
        "style_system": f"Use {name} to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.",
        "file_io": f"Use {name} to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.",
        "interactive": f"Use {name} to define prototype, presentation, motion, animation, or runtime interaction behavior in Studio.",
        "ai": f"Use {name} as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.",
        "collaboration": f"Use {name} to reproduce collaborative workflow behavior through local-first CRDT/EventLedger state, attribution, and recoverable receipts.",
        "typography": f"Use {name} to author, style, shape, inspect, or export text behavior with explicit font dependencies.",
        "color": f"Use {name} to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.",
        "workspace": f"Use {name} to make workspace, preference, navigation, and diagnostic behavior predictable for operators and models.",
        "automation": f"Use {name} to automate, inspect, hand off, or integrate Studio documents through typed local commands and extension surfaces.",
    }.get(domain, f"Use {name} as a source-backed Studio feature candidate with local-first Rust behavior.")


def write_cards(path, title, app_family, records, source_file):
    features = [r for r in records if r["record_role"] == "feature_leaf"]
    lines = [
        "---",
        f"file_id: {q(path.stem)}",
        'file_kind: "studio_app_feature_use_cards"',
        f"updated_at: {q(DATE)}",
        f"app_family: {q(app_family)}",
        f"source_inventory: {q(source_file)}",
        f"card_count: {len(features)}",
        'status: "generated_full_coverage_from_leaf_inventory"',
        "---",
        "",
        f'<topic id="feature-use-card-coverage" status="current" version="0.1" updated_at="{DATE}" ingestable="true" summary="Generated Feature Use Cards for stable source-backed leaves.">',
        "",
        f"# {title}",
        "",
        "```yaml",
        f"feature_use_card_count: {len(features)}",
        'explanation_status: "toc_inferred_from_verified_help_inventory"',
        'manual_entry_status: "planning_only_until_implemented"',
        'local_first_posture: "built_in_no_cloud_required_rust_forward"',
        "```",
        "",
        "</topic>",
        "",
        f'<topic id="generated-feature-use-cards" status="current" version="0.1" updated_at="{DATE}" ingestable="true" summary="Machine-readable generated Feature Use Card records.">',
        "",
        "```yaml",
        "feature_use_cards:",
    ]
    for record in features:
        domain = record["primitive_domain"]
        surf = surface(domain)
        post = provider(domain, record["name"], record["source_url"])
        lines += [
            f"  - feature_use_card_id: {q('fuc.' + record['id'] + '.v0')}",
            f"    source_feature_id: {q(record['id'])}",
            f"    feature_name: {q(record['name'])}",
            f"    source_apps: [{q(record['app'])}]",
            f"    source_inventory: {q(source_file)}",
            f"    source_category: {q(record['source_category'])}",
            f"    studio_surface: {q(surf)}",
            f"    primitive_domain: {q(domain)}",
            '    naming_posture: "handshake_native_name_with_vendor_source_refs"',
            f"    file_format_compatibility: {q(file_compat(domain, record['name']))}",
            '    local_first_posture: "built_in_no_cloud_required_rust_forward"',
            f"    provider_posture: {q(post)}",
            '    explanation_status: "toc_inferred"',
            '    manual_entry_status: "planning_only"',
            '    implementation_readiness: "needs_command_contract_promotion"',
            f"    purpose: {q(purpose(record['name'], domain))}",
            f"    user_goal: {q('A Studio operator can perform the source workflow named ' + clean(record['name']) + ' with Handshake-native commands, local state, receipts, and recovery.')}",
            "    when_to_use:",
            '      - "When equivalent source-app capability is required in Studio."',
            '      - "When the operation needs a typed Rust command contract and no-context UserManual topic."',
            '      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."',
            "    typical_workflow:",
            '      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."',
            '      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."',
            '      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."',
            '      - "Preview or validate when supported, then apply the operation."',
            '      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."',
            "    key_options:",
            f"      source_category: {q(record['source_category'])}",
            '      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"',
            '      compatibility_mode: "required_for_import_export_or_migration_paths"',
            '      preview_mode: "required_when_output_or_destructive_state_changes"',
            f"      provider_posture: {q(post)}",
            f"    expected_result: {q('Studio can represent or execute ' + clean(record['name']) + ' with visible state changes, undo/replay behavior, diagnostics, and a command receipt.')}",
            "    common_mistakes:",
            '      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."',
            '      - "Skipping source-page or app-behavior inspection before implementation."',
            '      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."',
            "    edge_cases:",
            '      - "Source app option variants that are not expressible in the initial Studio primitive."',
            '      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."',
            '      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."',
            "    recovery_steps:",
            '      - "Use Studio undo or restore the previous document snapshot."',
            '      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."',
            '      - "Rerun with corrected options or fall back to a documented compatibility shim."',
            "    handshake_tool_design_notes:",
            '      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."',
            '      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."',
            '      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."',
            "    equivalent_features:",
            f"      - app: {q(record['app'])}",
            f"        feature_id: {q(record['id'])}",
            f"        feature_name: {q(record['name'])}",
            f"    command_contract_refs: [{q('needs_contract.' + slug(domain) + '.v0')}]",
            f"    verification_refs: [{q('needs_fixture.' + slug(domain) + '.v0')}]",
            "    user_manual_handoff:",
            f"      topic_candidate: {q('Studio / ' + surf + ' / ' + clean(record['name']))}",
            '      required_when: "same_change_as_product_behavior_implementation"',
            '      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]',
            f"      no_context_operator_note: {q('Explain how to use and recover ' + clean(record['name']) + ' inside local-first Studio without relying on vendor product naming or cloud state.')}",
            "    source_refs:",
            f"      - label: {q(record['source_snapshot'])}",
            f"        url: {q(record['source_url'])}",
        ]
    if "illustrator" in source_file:
        source_topic = "SFR-ILLUSTRATOR-USE-CARDS.sources"
        sources = [
            '  - { id: ILL-FUC-S01, path: "22-illustrator-leaf-index.md", note: "Source inventory used to generate Illustrator Feature Use Cards." }',
            '  - { id: ILL-FUC-S02, path: "_source_snapshots/adobe-illustrator-desktop-jina.md", note: "Official Illustrator desktop help snapshot." }',
            '  - { id: ILL-FUC-S03, path: "_source_snapshots/illustrator-tools-jina.md", note: "Official Illustrator tools snapshot." }',
            '  - { id: ILL-FUC-S04, path: "_source_snapshots/illustrator-supported-file-formats-jina.md", note: "Official Illustrator supported file formats snapshot." }',
            '  - { id: ILL-FUC-S05, path: "_source_snapshots/illustrator-release-notes-jina.md", note: "Official Illustrator release notes snapshot." }',
        ]
    else:
        source_topic = "SFR-FIGMA-USE-CARDS.sources"
        sources = [
            '  - { id: FIG-FUC-S01, path: "23-figma-leaf-index.md", note: "Source inventory used to generate Figma Feature Use Cards." }',
            '  - { id: FIG-FUC-S02, path: "_source_snapshots/figma-design-category-jina.md", note: "Official Figma Design category snapshot." }',
            '  - { id: FIG-FUC-S03, path: "_source_snapshots/figma-make-category-jina.md", note: "Official Figma Make category snapshot." }',
            '  - { id: FIG-FUC-S04, path: "_source_snapshots/figma-import-export-jina.md", note: "Official Figma import/export snapshot." }',
            '  - { id: FIG-FUC-S05, path: "_source_snapshots/figma-export-formats-jina.md", note: "Official Figma export format snapshot." }',
            '  - { id: FIG-FUC-S06, path: "_source_snapshots/figma-api-docs-jina.md", note: "Official Figma developer docs snapshot." }',
        ]
    lines += [
        "```",
        "",
        "</topic>",
        "",
        f"### [{source_topic}] Sources",
        "",
        "```yaml",
        "sources:",
        *sources,
        "```",
    ]
    path.write_text("\n".join(lines) + "\n", encoding="ascii")


def write_local_first_doc():
    (ROOT / "19-studio-local-first-rust-posture.md").write_text(
        """---
file_id: "studio-local-first-rust-parity-posture"
topic_id: SFR-STUDIO-LOCAL-FIRST-RUST
status: draft
summary: "Local-first, no-cloud, Rust-forward posture for expanding Studio into Photoshop, Affinity, InDesign, Illustrator, and Figma parity without vendor product naming."
sources: 6
updated_at: "2026-07-05"
---

## [SFR-STUDIO-LOCAL-FIRST-RUST] Studio Local-First Rust Parity Posture

### [SFR-STUDIO-LOCAL-FIRST-RUST.policy] Policy

```yaml
studio_identity:
  module: "Studio"
  product_home: "Handshake"
  built_in: true
  local_first: true
  no_cloud_required: true
  rust_forward: true
  vendor_names_in_product_surface: false
  vendor_names_allowed_for: [source_refs, compatibility_notes, fixtures, migration_docs]
core_rules:
  - "Studio is a built-in local creative module for Handshake, not an external cloud clone."
  - "Core creative behavior must run locally in Rust-native engines wherever technically practical."
  - "Cloud, account, credit, community, and AI-provider behaviors are optional adapters or local-model/provider-neutral abstractions, never core requirements."
  - "File compatibility must target existing creative formats; do not invent a replacement interchange format for this rebuild scope."
  - "Every promoted feature needs a typed command contract, fixtures, receipts, diagnostics, undo/replay, and an internal Studio UserManual topic."
```

### [SFR-STUDIO-LOCAL-FIRST-RUST.engine-map] Rust-Forward Engine Map

```yaml
engine_targets:
  - { engine_module: studio_vector, owns: [illustrator_paths, figma_vector_networks, draw_tools, shape_builder, boolean_geometry, svg_pdf_vector_io] }
  - { engine_module: studio_layout, owns: [figma_frames, auto_layout, artboards, boards, slides, sites, responsive_constraints, page_spreads] }
  - { engine_module: studio_layer_graph, owns: [layers, groups, masks, placed_assets, components, object_order, visibility_locking] }
  - { engine_module: studio_typography, owns: [text_runs, font_resolution, glyphs, type_on_path, text_styles, accessibility_text] }
  - { engine_module: studio_style_registry, owns: [styles, variables, tokens, components, variants, libraries, symbols] }
  - { engine_module: studio_import_export, owns: [ai_ait, pdf, svg_svgz, eps_ps, psd, dwg_dxf, fig_jam_sketch, png_jpg_webp_tiff_gif, pptx, csv] }
  - { engine_module: studio_interaction, owns: [prototype_flows, smart_animate, overlays, motion_timeline, keyframes, slide_presentations] }
  - { engine_module: studio_collaboration, owns: [local_crdt, comments, branches, merge, history, meetings, cursor_chat, voting, attribution] }
  - { engine_module: studio_model_tools, owns: [provider_neutral_ai, local_model_tools, prompt_receipts, generation_provenance, optional_provider_adapters] }
  - { engine_module: studio_extensibility, owns: [plugins, widgets, mcp_server, rest_facade, scripting, local_package_registry] }
```

### [SFR-STUDIO-LOCAL-FIRST-RUST.compatibility] Compatibility Posture

```yaml
compatibility_targets:
  illustrator: [ai, ait, pdf, svg, svgz, eps, ps, dwg, dxf, psd, png, jpg, jpeg, gif, tiff, webp, css, txt]
  figma: [fig, jam, deck, buzz, site, make, sketch, png, jpg, svg, pdf_1_7, gif, tiff, webp, mp4, mov, webm, pptx, csv]
compatibility_rules:
  - "Prefer faithful import/export with explicit unsupported-feature diagnostics over silent lossy conversion."
  - "Round-trip tests must declare what is preserved, transformed, rasterized, ignored, or represented by compatibility shims."
  - "Proprietary local-copy formats with undocumented schemas are compatibility targets, not internal Studio storage authority."
```

### [SFR-STUDIO-LOCAL-FIRST-RUST.sources] Sources

```yaml
sources:
  - { id: LFR-S01, path: "00-preamble.md", note: "Existing Studio app feature research preamble and naming/file compatibility policy." }
  - { id: LFR-S02, path: "05-studio-primitive-map.md", note: "Existing Studio primitive and Rust module map." }
  - { id: LFR-S03, url: "https://helpx.adobe.com/illustrator/desktop.html", note: "Official Illustrator desktop help." }
  - { id: LFR-S04, url: "https://helpx.adobe.com/illustrator/kb/supported-file-formats-illustrator.html", note: "Official Illustrator supported file formats." }
  - { id: LFR-S05, url: "https://help.figma.com/hc/en-us", note: "Official Figma Help Center." }
  - { id: LFR-S06, url: "https://developers.figma.com/", note: "Official Figma developer documentation." }
```
""",
        encoding="ascii",
    )


def main():
    illustrator = illustrator_records()
    figma = figma_records()
    write_local_first_doc()
    write_feature_map(
        ROOT / "20-illustrator-feature-map.md",
        "SFR-ILLUSTRATOR",
        "Illustrator Feature Map",
        "Illustrator vector, typography, object, color, AI, import/export, print, and automation feature families for Handshake-native Studio parity.",
        [
            ("illustrator.vector_paths", "Vector paths and drawing tools", "Pen, pencil, curvature, anchor point, smooth, erase, cut, simplify, live shapes, and path editing.", "vector"),
            ("illustrator.live_shapes", "Live shapes and shape construction", "Lines, arcs, stars, spirals, polygons, pie shapes, shape builder, shaper, combine shapes.", "vector"),
            ("illustrator.object_arrangement", "Object selection and arrangement", "Selection methods, magic wand, grouping, isolation, move, align, distribute, expand, stack order, transforms.", "vector"),
            ("illustrator.artboards_canvas", "Artboards, canvas, workspace, and UI", "Large canvas, artboards, workspaces, properties/control/context panels, toolbars, preferences, shortcuts.", "page_layout"),
            ("illustrator.color_appearance", "Color, fills, strokes, gradients, mesh, patterns, and appearance", "Fill/stroke models, swatches, gradients, mesh, recolor, blend modes, appearances, graphic styles.", "color"),
            ("illustrator.typography", "Typography and glyph-aware editing", "Text objects, type on path, fonts, glyph snapping/guides, proofreading/translation/rewrite where AI-backed.", "typography"),
            ("illustrator.layers_symbols_assets", "Layers, symbols, links, embedded assets", "Layer organization, symbols, linked/embedded files, relink all instances, placed files.", "layer"),
            ("illustrator.import_export_formats", "Import/export/save/place formats", "AI/AIT, PDF, SVG/SVGZ, EPS/PS, DWG/DXF, PSD, raster formats, CSS, save for web/screens.", "file_io"),
            ("illustrator.generative_ai", "Generative and AI-assisted vector workflows", "Text to vector graphic, generative recolor/patterns/shape fills, vectorize raster, edit generated artwork, partner models.", "ai"),
            ("illustrator.print_prepress", "Print, PDF, package, and prepress output", "PDF output, separations, color management, linked asset/package concerns, print-ready vector output.", "prepress"),
            ("illustrator.automation_extensions", "Actions, scripts, variables, plugins, and extensibility", "Automation and extension surfaces needed for parity with production Illustrator workflows.", "automation"),
            ("illustrator.recovery_diagnostics", "Recovery, performance, troubleshooting, and damaged files", "Crash recovery, safe mode, damaged documents, missing plugins/fonts/printers, performance diagnostics.", "workspace"),
        ],
        [
            ("ILL-S01", "https://helpx.adobe.com/illustrator/desktop.html", "Official Illustrator desktop help."),
            ("ILL-S02", "https://helpx.adobe.com/illustrator/using/tools-in-illustrator.html", "Official Illustrator tools overview."),
            ("ILL-S03", "https://helpx.adobe.com/illustrator/kb/supported-file-formats-illustrator.html", "Official Illustrator supported file formats."),
            ("ILL-S04", "https://helpx.adobe.com/illustrator/desktop/new-features/release-notes.html", "Official Illustrator release notes."),
        ],
    )
    write_feature_map(
        ROOT / "21-figma-feature-map.md",
        "SFR-FIGMA",
        "Figma Feature Map",
        "Figma Design, Draw, FigJam, Motion, Slides, Sites, Buzz, Make, Dev Mode, API, AI, and collaboration feature families for local-first Rust Studio parity.",
        [
            ("figma.canvas_layers", "Canvas, files, pages, layers, frames, groups, sections", "Core editable design graph and navigation model.", "page_layout"),
            ("figma.vector_draw", "Vector networks, pen, pencil, brush, shape builder, simplify, vectorize", "Illustration and vector authoring parity including Figma Draw.", "vector"),
            ("figma.typography", "Text, fonts, text properties, text styles", "Font loading, typography, text styles, text-to-path conversion.", "typography"),
            ("figma.visual_styling", "Fills, gradients, patterns, images, effects, blend modes, color profiles", "Visual style stack and color pipeline.", "color"),
            ("figma.auto_layout", "Auto layout, constraints, responsive sizing, grids", "Responsive layout engine and constraints.", "page_layout"),
            ("figma.components_systems", "Components, instances, variants, slots, styles, variables, libraries", "Design-system graph and reusable token/component registry.", "style_system"),
            ("figma.prototyping_motion", "Prototypes, interactions, smart animate, variables in prototypes, Motion timeline", "Runtime interaction/timeline model.", "interactive"),
            ("figma.import_export_formats", "Import/export, local copies, Sketch import, .fig, SVG/PDF/PNG/JPG/video/animation export", "Compatibility and asset IO surface.", "file_io"),
            ("figma.collaboration", "Comments, multiplayer, branches, history, sharing, meetings, FigJam sessions", "Local-first CRDT collaboration replacement for cloud collaboration.", "collaboration"),
            ("figma.figjam", "FigJam whiteboard, sticky notes, tables, mind maps, meetings, imports/exports", "Whiteboard and workshop parity.", "page_layout"),
            ("figma.dev_mode_api", "Dev Mode, inspect, Code Connect, MCP, REST, plugin/widget APIs", "Developer handoff and extension surfaces.", "automation"),
            ("figma.make_ai", "Make, AI agent, Weave, generative plugins, web/code workflows", "Provider/local model adapter and local code-generation sandbox.", "ai"),
            ("figma.slides_sites_buzz", "Slides, Sites, Buzz and adjacent canvas products", "Presentation, responsive site, and brand asset production surfaces.", "interactive"),
        ],
        [
            ("FIG-S01", "https://help.figma.com/hc/en-us/categories/360002042553-Figma-Design", "Official Figma Design category."),
            ("FIG-S02", "https://help.figma.com/hc/en-us/categories/360002051633-FigJam", "Official FigJam category."),
            ("FIG-S03", "https://help.figma.com/hc/en-us/categories/31304285531543-Figma-Make", "Official Figma Make category."),
            ("FIG-S04", "https://help.figma.com/hc/en-us/categories/41274596092695-Figma-Motion", "Official Figma Motion category."),
            ("FIG-S05", "https://developers.figma.com/", "Official Figma developer docs."),
            ("FIG-S06", "https://www.figma.com/release-notes/", "Official Figma release notes."),
        ],
    )
    write_leaf_index(ROOT / "22-illustrator-leaf-index.md", "SFR-ILLUSTRATOR-LEAF", "Illustrator Help Leaf Index", "Illustrator", illustrator, "Official Illustrator desktop/tools/file-format/release-note snapshots parsed from Adobe Help via local Jina Reader snapshots.")
    write_leaf_index(ROOT / "23-figma-leaf-index.md", "SFR-FIGMA-LEAF", "Figma Help Leaf Index", "Figma", figma, "Official Figma Design and Make article snapshots plus verified category/source-agent evidence for FigJam, Motion, Slides, Sites, Buzz, AI, Build, Community, and file compatibility surfaces.")
    write_cards(ROOT / "24-illustrator-feature-use-cards.md", "Illustrator Feature Use Cards", "Illustrator", illustrator, "22-illustrator-leaf-index.md")
    write_cards(ROOT / "25-figma-feature-use-cards.md", "Figma Feature Use Cards", "Figma", figma, "23-figma-leaf-index.md")

    provider_rows = []
    for record in illustrator + figma:
        post = provider(record["primitive_domain"], record["name"], record["source_url"])
        if post != "local_primitive":
            provider_rows.append((record, post))
    lines = [
        "---",
        'file_id: "illustrator-figma-provider-posture-map"',
        "topic_id: SFR-ILLUSTRATOR-FIGMA-PROVIDER-POSTURE",
        "status: draft",
        'summary: "Provider, local-first collaboration, and compatibility posture rows for Illustrator and Figma parity expansion."',
        "sources: 4",
        f'updated_at: "{DATE}"',
        "---",
        "",
        "## [SFR-ILLUSTRATOR-FIGMA-PROVIDER-POSTURE] Illustrator/Figma Provider Posture Map",
        "",
        "### [SFR-ILLUSTRATOR-FIGMA-PROVIDER-POSTURE.inventory] Inventory",
        "",
        "```yaml",
        "provider_posture_records:",
    ]
    for record, post in provider_rows:
        lines += [
            f"  - id: {q(record['id'])}",
            f"    name: {q(record['name'])}",
            f"    app: {q(record['app'])}",
            f"    primitive_domain: {q(record['primitive_domain'])}",
            f"    provider_posture: {q(post)}",
            '    local_first_rule: "Core Studio must operate without cloud; this feature is adapter, local-model, local-collaboration, or compatibility-gated as noted."',
            f"    source_url: {q(record['source_url'])}",
        ]
    lines += [
        "```",
        "",
        "### [SFR-ILLUSTRATOR-FIGMA-PROVIDER-POSTURE.sources] Sources",
        "",
        "```yaml",
        "sources:",
        '  - { id: IFP-S01, path: "22-illustrator-leaf-index.md", note: "Illustrator leaf records." }',
        '  - { id: IFP-S02, path: "23-figma-leaf-index.md", note: "Figma leaf records." }',
        '  - { id: IFP-S03, path: "19-studio-local-first-rust-posture.md", note: "Local-first Rust posture." }',
        '  - { id: IFP-S04, path: "_source_snapshots/", note: "Official source snapshots." }',
        "```",
    ]
    (ROOT / "26-illustrator-figma-provider-posture-map.md").write_text("\n".join(lines) + "\n", encoding="ascii")

    lanes = [
        ("vector_authoring", "StudioVectorPathGraph", "Illustrator paths/live shapes/shape builder; Figma vector networks/Draw/shape builder/vectorize."),
        ("canvas_layout", "StudioPageSpread", "Illustrator artboards/large canvas; Figma pages/frames/sections/boards/slides/sites/auto layout."),
        ("design_systems", "StudioStyleRegistry", "Illustrator symbols/graphic styles; Figma components/variants/slots/styles/variables/libraries."),
        ("typography", "StudioTextRunAndStory", "Illustrator type/glyph tools; Figma text/fonts/text styles/text-to-path."),
        ("appearance_color", "StudioColorPipeline", "Illustrator fills/strokes/gradients/mesh/recolor; Figma fills/effects/patterns/blends/color profiles."),
        ("interaction_motion", "StudioInteractiveDocumentSurface", "Figma prototypes/Motion/Slides and Illustrator web/export animation-adjacent output."),
        ("file_compatibility", "StudioFileIO", "AI/AIT/PDF/SVG/EPS/DWG/DXF/PSD plus FIG/JAM/SKETCH/PPTX/media/static/animation exports."),
        ("collaboration_local", "StudioCollaborationSession", "Figma multiplayer/FigJam meetings/comments/history and Illustrator projects converted to local CRDT/EventLedger workflows."),
        ("ai_provider_local", "StudioModelToolContract", "Illustrator Firefly/partner models and Figma AI/Make/Weave reinterpreted as provider-neutral/local model commands."),
        ("extensibility_dev", "StudioActionGraph", "Figma plugin/widget/API/MCP and Illustrator automation/plugins/scripts as local extension host targets."),
    ]
    lines = [
        "---",
        'file_id: "illustrator-figma-parity-matrix"',
        "topic_id: SFR-ILLUSTRATOR-FIGMA-PARITY",
        "status: draft",
        'summary: "Primitive-centered parity lanes for adding Illustrator and Figma clone coverage to local-first Rust Studio."',
        "sources: 5",
        f'updated_at: "{DATE}"',
        "---",
        "",
        "## [SFR-ILLUSTRATOR-FIGMA-PARITY] Illustrator/Figma Studio Parity Matrix",
        "",
        "### [SFR-ILLUSTRATOR-FIGMA-PARITY.matrix] Matrix",
        "",
        "```yaml",
        "parity_lanes:",
    ]
    for lane, surf, desc in lanes:
        lines += [
            f"  - id: {q('parity.' + lane)}",
            f"    studio_surface: {q(surf)}",
            f"    parity_scope: {q(desc)}",
            '    local_first_requirement: "No cloud dependency for core use; optional providers and compatibility shims must expose diagnostics."',
            '    rust_forward_requirement: "Promote through typed Rust command contracts, fixtures, receipts, undo/replay, and UserManual topic."',
        ]
    lines += [
        "```",
        "",
        "### [SFR-ILLUSTRATOR-FIGMA-PARITY.sources] Sources",
        "",
        "```yaml",
        "sources:",
        '  - { id: IFP-M01, path: "19-studio-local-first-rust-posture.md", note: "Local-first Rust posture." }',
        '  - { id: IFP-M02, path: "20-illustrator-feature-map.md", note: "Illustrator feature map." }',
        '  - { id: IFP-M03, path: "21-figma-feature-map.md", note: "Figma feature map." }',
        '  - { id: IFP-M04, path: "22-illustrator-leaf-index.md", note: "Illustrator leaves." }',
        '  - { id: IFP-M05, path: "23-figma-leaf-index.md", note: "Figma leaves." }',
        "```",
    ]
    (ROOT / "27-illustrator-figma-parity-matrix.md").write_text("\n".join(lines) + "\n", encoding="ascii")

    print(f"illustrator_total={len(illustrator)}")
    print(f"illustrator_feature={sum(1 for r in illustrator if r['record_role'] == 'feature_leaf')}")
    print(f"figma_total={len(figma)}")
    print(f"figma_feature={sum(1 for r in figma if r['record_role'] == 'feature_leaf')}")
    print(f"provider_rows={len(provider_rows)}")


if __name__ == "__main__":
    main()
