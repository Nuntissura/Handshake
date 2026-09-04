#!/usr/bin/env python3
"""Handshake Studio green room: mechanical classification of Affinity UI strings + API classes
into a feature inventory, and a name-level delta against the existing research corpus rows.

Affinity .NET string keys have the shape "English text [UI context]"; the bracketed context is
the app's own classifier (e.g. "[Tool summary]", "[Tool Group]", "[Command description]",
"[Layer type]", "[Preferences Category Title]", "[Shortcut Categories]", "[Menu ...]").

Outputs (JSON only):
  <out>/affinity-offline-inventory.json  context histogram + inventory rows by kind
  <out>/affinity-offline-delta.json      rows whose name has no fuzzy match in corpus feature names
No LLM is used. Reference material only.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import re
from pathlib import Path

KEY_RE = re.compile(r"^(.*?)\s*\[([^\]]+)\]\s*$")
NAME_LINE_RE = re.compile(r"^\s*-?\s*(?:name|feature_name|display_name|tool|command|panel)\s*:\s*[\"']?(.+?)[\"']?\s*$", re.M)
ID_LINE_RE = re.compile(r"^\s*(?:-\s*)?id\s*:\s*[\"']?([A-Za-z0-9._:-]+)[\"']?", re.M)

# context -> (kind, domain_hint)
CONTEXT_MAP = [
    (re.compile(r"tool summary|tool tooltip|tool label|tool name|tool description|\btool\b", re.I), "tool"),
    (re.compile(r"tool group|tools category", re.I), "tool_group"),
    (re.compile(r"command description|command name|command title|\bcommand\b|customise keys|shortcut categories", re.I), "command"),
    (re.compile(r"\bmenu\b|menu item|context menu|studio menu", re.I), "menu"),
    (re.compile(r"panel title|panel\b|page string title|page title|studio\b", re.I), "panel"),
    (re.compile(r"dialog|message box|alert|prompt", re.I), "dialog"),
    (re.compile(r"preferences|preference category", re.I), "preference"),
    (re.compile(r"layer type|layer panel|blend mode", re.I), "layer_model"),
    (re.compile(r"adjustment|filter|live filter|effect|fx", re.I), "adjustment_or_effect"),
    (re.compile(r"export|import|file format|quick export|slice", re.I), "interop"),
    (re.compile(r"property collection|preset|presets panel|preset manager|asset", re.I), "preset_system"),
    (re.compile(r"text style|typography|font|glyph|paragraph|character|opentype|story|text frame", re.I), "typography"),
    (re.compile(r"colour|color|swatch|palette|gradient|ocio|icc|profile|soft proof", re.I), "color"),
    (re.compile(r"page|spread|master|book|index|toc|cross-ref|data merge|preflight|footnote|endnote|table", re.I), "layout"),
    (re.compile(r"develop|raw|lens|tone map|hdr|astro|stack|liquify|clone|inpaint|retouch|selection|mask|channel|pixel", re.I), "raster"),
    (re.compile(r"node|curve|pen|shape|boolean|contour|knife|vector|brush|stroke|appearance|symbol|constraint|artboard|grid|guide|snap", re.I), "vector"),
]
DOMAIN_MAP = [
    (re.compile(r"develop|raw|lens|tone map|hdr|astro|stack|liquify|clone|inpaint|retouch|heal|dodge|burn|sponge|pixel|selection brush|flood|marquee|mask|channel|frequency", re.I), "raster"),
    (re.compile(r"node|curve|pen|shape|boolean|contour|knife|vector|stroke|appearance|symbol|constraint|artboard|corner|pencil|point transform|transform", re.I), "vector"),
    (re.compile(r"page|spread|master|book|index|toc|cross-ref|data merge|preflight|footnote|endnote|table|column|frame text|text wrap|baseline grid|package|imposition", re.I), "layout"),
    (re.compile(r"text style|typography|font|glyph|paragraph|character|opentype|kerning|tracking|leading|hyphen|justif|story|artistic text|text frame|variable font", re.I), "typography"),
    (re.compile(r"colour|color|swatch|palette|gradient|ocio|icc|profile|soft proof|pantone|lut|32-bit|hdr", re.I), "color"),
    (re.compile(r"blend mode|layer effect|outer shadow|inner shadow|bevel|emboss|glow|gaussian|blur|noise|distort|sharpen|filter|live filter|adjustment", re.I), "effects"),
    (re.compile(r"style|asset|preset|macro|library|template", re.I), "design_system"),
    (re.compile(r"slice|export|import|pdf|svg|eps|psd|idml|dwg|dxf|jpeg|png|tiff|webp|jxl|heic|placed|linked|resource manager", re.I), "interop"),
    (re.compile(r"\bmacro|\bbatch\b|\bscripts?\b|\bautomation\b|data merge", re.I), "automation"),
    (re.compile(r"canva|ai |generative|remove background|magic", re.I), "excluded_ai"),
]
AI_RE = re.compile(r"canva ai|generative|\bai\b|firefly|magic|credits", re.I)
NOISE_CONTEXTS = re.compile(r"object detection class|message|error|warning|tooltip hint|status|progress|analytics|telemetry|licen|account|sign in|update", re.I)


def now_iso() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def norm(s: str) -> str:
    s = s.replace("…", "").replace("&", " and ")
    s = re.sub(r"%[@\d]|\|[A-Z_+]+\||\.\.\.|:$", " ", s)
    s = re.sub(r"[^a-z0-9 ]+", " ", s.lower())
    return re.sub(r"\s+", " ", s).strip()


def tokens(s: str) -> set:
    stop = {"the", "a", "an", "to", "of", "and", "or", "for", "with", "in", "on", "by", "tool", "panel", "dialog", "command", "new", "current", "selected", "this", "all"}
    return {t for t in norm(s).split() if len(t) > 2 and t not in stop}


def classify_context(ctx: str) -> str:
    for rx, kind in CONTEXT_MAP:
        if rx.search(ctx):
            return kind
    return "other"


def classify_domain(text: str, ctx: str) -> str:
    # Generic contexts ("Command description", "Tool summary") carry no domain signal; classify on text alone.
    generic = re.search(r"command description|tool summary|tool description|page string title|hintline|reflect", ctx, re.I)
    blob = text if generic else f"{text} {ctx}"
    for rx, dom in DOMAIN_MAP:
        if rx.search(blob):
            return dom
    return "cross_cutting"


def load_persona_strings(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8-sig"))
    merged = {}
    for asm in data.get("assemblies", []):
        for rs in asm.get("resource_sets", []):
            for k, v in rs.get("entries", {}).items():
                merged[k] = v
    return merged


def load_corpus_names(paths: list[Path]) -> dict:
    names = {}
    for p in paths:
        if not p.exists():
            continue
        text = p.read_text(encoding="utf-8", errors="replace")
        for m in NAME_LINE_RE.finditer(text):
            n = m.group(1).strip().strip("\"'")
            if 2 < len(n) < 120:
                names.setdefault(norm(n), {"name": n, "sources": set()})["sources"].add(p.name)
        for m in ID_LINE_RE.finditer(text):
            i = m.group(1)
            key = norm(i.replace(".", " ").replace("_", " ").replace("-", " "))
            names.setdefault(key, {"name": i, "sources": set()})["sources"].add(p.name)
    return names


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--offline", type=Path, required=True, help="installed_exports/affinity/offline")
    ap.add_argument("--corpus", type=Path, required=True, help="studio_app_feature_research root")
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    strings = load_persona_strings(args.offline / "ui_strings_dotnet_en-US.json")
    lproj = json.loads((args.offline / "ui_strings_lproj_en-US.json").read_text(encoding="utf-8"))
    api = json.loads((args.offline / "scripting_api_surface.json").read_text(encoding="utf-8"))
    presets = json.loads((args.offline / "presets_names_scan.json").read_text(encoding="utf-8")) if (args.offline / "presets_names_scan.json").exists() else {"files": []}

    ctx_hist: dict[str, int] = {}
    rows: dict[str, dict] = {}
    excluded_ai = set()
    messages_skipped = 0
    for key, value in strings.items():
        m = KEY_RE.match(key)
        text, ctx = (m.group(1), m.group(2)) if m else (key, "")
        text = text.strip()
        if not text or len(text) > 90 or text.count(" ") > 9:
            messages_skipped += 1
            continue
        ctx_hist[ctx] = ctx_hist.get(ctx, 0) + 1
        if NOISE_CONTEXTS.search(ctx) and not re.search(r"tool|command|menu|panel|preferences|layer|shortcut", ctx, re.I):
            messages_skipped += 1
            continue
        if AI_RE.search(text) or AI_RE.search(ctx):
            excluded_ai.add(text)
            continue
        kind = classify_context(ctx)
        if kind == "other":
            continue
        n = norm(text)
        if len(n) < 3:
            continue
        rid = f"af.{kind}.{re.sub(r' ', '_', n)[:60]}"
        row = rows.setdefault(rid, {"id": rid, "kind": kind, "name": text, "domain": classify_domain(text, ctx), "evidence": []})
        if len(row["evidence"]) < 6:
            row["evidence"].append(key)

    # lproj tables (non-autocorrect) add option/enum values
    for tname, tbl in lproj.get("tables", {}).items():
        if tname in ("autocorrect", "abbreviations", "title_exceptions"):
            continue
        for k, v in tbl.get("entries", {}).items():
            if 2 < len(v) <= 60 and v.count(" ") <= 6:
                n = norm(v)
                rid = f"af.option.{tname}.{re.sub(r' ', '_', n)[:50]}"
                rows.setdefault(rid, {"id": rid, "kind": "option", "name": v, "domain": classify_domain(v, tname), "evidence": [f"lproj:{tname}:{k}"]})

    # API classes as document-model primitives
    api_rows = 0
    for mod, mdata in api.get("modules", {}).items():
        for cls in mdata.get("classes", []):
            rid = f"af.primitive.{cls['name']}"
            rows[rid] = {"id": rid, "kind": "primitive", "name": cls["name"], "domain": classify_domain(cls["name"] + " " + mod, mod), "evidence": [f"jslib:{mod}.js class {cls['name']} ({cls['member_count']} members)"], "extends": cls.get("extends"), "member_count": cls["member_count"]}
            api_rows += 1
    for nm, syms in api.get("native_modules", {}).items():
        for s in syms:
            rid = f"af.api_symbol.{nm.split(':')[1]}.{s}"
            rows.setdefault(rid, {"id": rid, "kind": "api_symbol", "name": s, "domain": classify_domain(s + " " + nm, nm), "evidence": [f"native:{nm}"]})

    # preset names
    preset_rows = 0
    for f in presets.get("files", []):
        fam = Path(f["file"]).stem
        for nm in f.get("candidate_names", [])[:400]:
            if 2 < len(nm) <= 60:
                rid = f"af.preset.{fam}.{re.sub(r' ', '_', norm(nm))[:50]}"
                if rid not in rows:
                    rows[rid] = {"id": rid, "kind": "preset", "name": nm, "domain": classify_domain(nm + " " + fam, fam), "evidence": [f"propcol:{fam}"]}
                    preset_rows += 1

    # corpus delta by fuzzy token overlap
    corpus_paths = [args.corpus / "42-affinity-source-distilled-feature-rows.md", args.corpus / "54-affinity-deep-feature-delta.md", args.corpus / "37-affinity-source-distilled-domain-ledger.md", args.corpus / "16-affinity-feature-use-cards.md", args.corpus / "09-affinity-desktop-delta.md", args.corpus / "02-affinity-suite-feature-map.md", args.corpus / "04-affinity-leaf-index.md", args.corpus / "58-parity-feature-gap-register.md", args.corpus / "63-parity-repass-delta.md", args.corpus / "66-parity-gap-closeout-delta.md"]
    corpus = load_corpus_names(corpus_paths)
    corpus_tok = {k: tokens(v["name"]) for k, v in corpus.items()}
    delta, confirms = [], 0
    for row in rows.values():
        if row["kind"] in ("api_symbol", "option"):
            continue
        rt = tokens(row["name"])
        if not rt:
            continue
        best, best_key = 0.0, None
        for ck, ct in corpus_tok.items():
            if not ct:
                continue
            inter = len(rt & ct)
            if inter == 0:
                continue
            score = inter / max(1, min(len(rt), len(ct)))
            if score > best:
                best, best_key = score, ck
        if best >= 0.67:
            confirms += 1
            row["closest_corpus_row"] = corpus[best_key]["name"]
            row["corpus_match_score"] = round(best, 2)
        else:
            delta.append({"id": row["id"], "kind": row["kind"], "name": row["name"], "domain": row["domain"], "closest_corpus_row": corpus[best_key]["name"] if best_key else "NONE", "match_score": round(best, 2), "evidence": row["evidence"][:3]})

    kinds: dict[str, int] = {}
    for r in rows.values():
        kinds[r["kind"]] = kinds.get(r["kind"], 0) + 1
    inventory = {
        "schema_id": "handshake.reference.studio_greenroom_app_inventory@1",
        "app": "affinity", "app_version": "3.2.3.4646", "generated_at": now_iso(),
        "method": "Mechanical: .NET string keys split into text + [UI context]; context regex -> kind; text/context regex -> Studio domain; lproj tables -> options; JSLib classes -> primitives; propcol scan -> presets. Messages, errors, object-detection classes and Canva AI strings excluded. No LLM judgement.",
        "counts": {"persona_strings": len(strings), "messages_skipped": messages_skipped, "rows": len(rows), "by_kind": kinds, "api_primitives": api_rows, "preset_rows": preset_rows, "excluded_ai": len(excluded_ai)},
        "context_histogram_top": sorted(ctx_hist.items(), key=lambda kv: -kv[1])[:150],
        "excluded_ai": sorted(excluded_ai)[:300],
        "inventory": sorted(rows.values(), key=lambda r: (r["kind"], r["domain"], r["name"].lower())),
    }
    (args.out / "affinity-offline-inventory.json").write_text(json.dumps(inventory, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    by_dom: dict[str, int] = {}
    for d in delta:
        by_dom[d["domain"]] = by_dom.get(d["domain"], 0) + 1
    delta_doc = {
        "schema_id": "handshake.reference.studio_greenroom_app_delta@1",
        "app": "affinity", "app_version": "3.2.3.4646", "generated_at": now_iso(),
        "method": "Token-overlap name matching (threshold 0.67 on the smaller token set) of inventory rows against feature/id names scraped from the Affinity corpus files. This is a candidate list for human/LLM triage, not a verdict; false positives are expected for renamed features.",
        "corpus_files": [p.name for p in corpus_paths if p.exists()], "corpus_name_count": len(corpus),
        "delta_confirms_corpus": confirms, "delta_candidate_count": len(delta), "delta_by_domain": by_dom,
        "delta_new_vs_corpus": sorted(delta, key=lambda d: (d["domain"], d["kind"], d["name"].lower())),
    }
    (args.out / "affinity-offline-delta.json").write_text(json.dumps(delta_doc, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    print(f"[classify] rows={len(rows)} by_kind={kinds}")
    print(f"[classify] corpus_names={len(corpus)} confirms={confirms} delta_candidates={len(delta)} by_domain={by_dom}")
    print(f"[classify] excluded_ai={len(excluded_ai)} messages_skipped={messages_skipped}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
