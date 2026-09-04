#!/usr/bin/env python3
"""Build the microtask coverage plan: every extracted capability -> an existing MT or a new MT.

Consumes the cross-app capability registry plus every per-app deep-teardown artifact that
exists under installed_exports/*/offline/, and emits the authoring plan for WP-KERNEL-STUDIO:

  studio-mt-coverage-plan.json
    domains[]            per Studio domain: capability count, existing MT count, required MT count
    assignments[]        capability -> existing MT (COVERED/PARTIAL) with score
    new_mt_specs[]       clustered uncovered capabilities -> one proposed MT each, with the
                         behavioural payload (parameters/options/defaults) the MT must implement
    parameter_index      capability -> parameters/ranges/defaults harvested from teardown files

Reference material. Writes nothing into .GOV/task_packets.
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import re
from pathlib import Path

STOP = {"the", "a", "an", "to", "of", "and", "or", "for", "with", "in", "on", "by", "new", "current", "this", "all", "from", "as", "at", "is", "are", "be", "set", "get"}

BAND = {
    "foundation": (1, 20), "raster": (21, 80), "vector": (81, 140), "layout": (141, 190),
    "typography": (191, 230), "color": (231, 260), "effects": (261, 320), "design_system": (321, 370),
    "prototype": (371, 410), "raw": (411, 440), "interop": (441, 480), "automation": (481, 520),
    "whiteboard": (521, 540), "cross_cutting": (541, 560),
}
# Domains the packet has no band for yet; the app scope expansion added them.
NEW_BANDS = {"motion": (600, 699), "video": (700, 799), "catalog": (800, 849), "web": (850, 899), "document_model": (900, 949)}

# Files that carry behavioural depth, keyed by the field holding parameter-bearing records.
TEARDOWN_SOURCES = [
    ("photoshop", "photoshop_parameter_surface.json"), ("photoshop", "photoshop_enums.json"),
    ("photoshop", "photoshop_preset_contents.json"), ("photoshop", "photoshop_dialogs.json"),
    ("illustrator", "illustrator_parameter_surface.json"), ("illustrator", "illustrator_enums.json"),
    ("illustrator", "illustrator_effects.json"), ("illustrator", "illustrator_library_contents.json"),
    ("affinity", "affinity_brush_parameters.json"), ("affinity", "affinity_adjustment_parameters.json"),
    ("affinity", "affinity_preset_contents.json"), ("affinity", "affinity_scripting_api_detail.json"),
    ("affinity", "affinity_tool_panel_registry.json"),
    ("indesign", "indesign_dom_full.json"), ("indesign", "indesign_dialogs.json"),
    ("indesign", "indesign_text_model.json"), ("indesign", "indesign_preset_contents.json"),
    ("lightroom_classic", "lightroom_develop_parameters.json"), ("lightroom_classic", "lightroom_export_pipeline.json"),
    ("lightroom_classic", "lightroom_sdk_api.json"), ("lightroom_classic", "lightroom_templates.json"),
    ("figma", "figma_object_model.json"),
    ("aftereffects", "aftereffects_effects_catalogue.json"), ("aftereffects", "aftereffects_presets.json"),
    ("aftereffects", "aftereffects_layer_property_model.json"), ("aftereffects", "aftereffects_scripting_expressions.json"),
    ("aftereffects", "aftereffects_panels_dialogs.json"), ("aftereffects", "aftereffects_render_output.json"),
    ("aftereffects", "aftereffects_commands_shortcuts.json"), ("aftereffects", "aftereffects_text_shape_mask.json"),
    ("premiere", "premiere_effects_catalogue.json"), ("premiere", "premiere_lumetri_color.json"),
    ("premiere", "premiere_export_pipeline.json"), ("premiere", "premiere_sequence_project_model.json"),
    ("premiere", "premiere_panels_dialogs.json"), ("premiere", "premiere_commands_shortcuts.json"),
    ("premiere", "premiere_graphics_text.json"), ("premiere", "premiere_media_io.json"),
    ("dreamweaver", "dreamweaver_command_surface.json"), ("dreamweaver", "dreamweaver_panels_dialogs.json"),
    ("dreamweaver", "dreamweaver_objects_behaviors.json"), ("dreamweaver", "dreamweaver_code_intelligence.json"),
    ("dreamweaver", "dreamweaver_site_server_model.json"), ("dreamweaver", "dreamweaver_templates_css.json"),
]


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def norm(s: str) -> str:
    s = re.sub(r"\.\.\.|…", "", str(s))
    s = re.sub(r"[^a-z0-9 ]+", " ", s.lower())
    return re.sub(r"\s+", " ", s).strip()


def toks(s: str) -> frozenset:
    return frozenset(t for t in norm(s).split() if t not in STOP and len(t) > 2)


def walk_params(obj, path="", out=None, depth=0):
    """Harvest any {name: value} leaves that look like parameters, with their location."""
    if out is None:
        out = []
    if depth > 8:
        return out
    if isinstance(obj, dict):
        keys = set(obj.keys())
        if {"name"} & keys and (keys & {"type", "default", "range", "values", "min", "max", "enum", "unit"}):
            out.append({"path": path, "param": obj})
        else:
            for k, v in obj.items():
                walk_params(v, f"{path}.{k}" if path else str(k), out, depth + 1)
    elif isinstance(obj, list):
        for i, v in enumerate(obj[:2000]):
            walk_params(v, f"{path}[]", out, depth + 1)
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--greenroom", type=Path, required=True)
    ap.add_argument("--packet", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    reg_path = args.greenroom / "fold_back" / "studio-capability-registry.json"
    reg = json.loads(reg_path.read_text(encoding="utf-8"))
    caps = reg["capabilities"]

    # existing MTs
    mts = []
    for f in sorted(args.packet.glob("MT-*.json")):
        m = json.loads(f.read_text(encoding="utf-8"))
        mts.append({
            "mt_id": m["mt_id"],
            "clause": m.get("clause", ""),
            "summary": m.get("scope", {}).get("summary", ""),
            "tok": toks(m.get("clause", "") + " " + m.get("scope", {}).get("summary", "")),
            "acceptance_rows": len(m.get("scope", {}).get("acceptance_criteria", []) or []),
        })
    band_of = {}
    for m in mts:
        n = int(m["mt_id"].split("-")[1])
        for d, (a, b) in BAND.items():
            if a <= n <= b:
                band_of[m["mt_id"]] = d
                break

    # behavioural payload harvested from whatever teardown files exist
    payload: dict[str, list] = collections.defaultdict(list)
    sources_present, sources_missing = [], []
    for app, fname in TEARDOWN_SOURCES:
        p = args.greenroom / "installed_exports" / app / "offline" / fname
        if not p.exists():
            sources_missing.append(f"{app}/{fname}")
            continue
        sources_present.append(f"{app}/{fname}")
        try:
            data = json.loads(p.read_text(encoding="utf-8"))
        except Exception as e:  # noqa: BLE001
            sources_missing.append(f"{app}/{fname} (unreadable: {e})")
            continue
        for row in walk_params(data):
            nm = row["param"].get("name")
            if not nm:
                continue
            payload[norm(nm)].append({"app": app, "file": fname, "path": row["path"], "param": row["param"]})

    # assign capabilities
    assignments, uncovered = [], []
    for c in caps:
        t = toks(c["name"])
        if not t:
            continue
        best, best_mt = 0.0, None
        for m in mts:
            if not m["tok"]:
                continue
            inter = len(t & m["tok"])
            if not inter:
                continue
            sc = inter / len(t)
            if sc > best:
                best, best_mt = sc, m["mt_id"]
        params = payload.get(norm(c["name"]), [])
        rec = {
            "capability_id": c["id"], "name": c["name"], "kind": c["kind"], "domain": c["domain"],
            "source_apps": c["source_apps"], "app_count": c["app_count"],
            "mt": best_mt, "score": round(best, 2),
            "state": "COVERED" if best >= 0.6 else ("PARTIAL" if best >= 0.34 else "UNCOVERED"),
            "behaviour_records": len(params),
        }
        assignments.append(rec)
        if rec["state"] == "UNCOVERED":
            uncovered.append({**rec, "behaviour": params[:6]})

    # cluster uncovered by domain + leading token, one proposed MT per cluster
    clusters: dict[tuple, list] = collections.defaultdict(list)
    for u in uncovered:
        key_tok = sorted(toks(u["name"]))[:1]
        clusters[(u["domain"], key_tok[0] if key_tok else "misc")].append(u)

    # New microtasks are allocated in a dedicated 1000+ space, one contiguous block per domain,
    # sized to the actual cluster count. The original 1..560 bands are far too small: allocating
    # inside them made clusters collide and silently overwrite each other's contracts.
    AUTHOR_BASE = 10000
    BLOCK = 2000  # widened: cross_cutting alone exceeded 500 clusters after the GRD-001 unmerge
    DOMAIN_ORDER = ["foundation", "document_model", "raster", "vector", "layout", "typography", "color",
                    "effects", "design_system", "prototype", "raw", "interop", "automation", "whiteboard",
                    "motion", "video", "catalog", "web", "cross_cutting"]
    DOMAIN_BLOCK = {d: (AUTHOR_BASE + i * BLOCK, AUTHOR_BASE + (i + 1) * BLOCK - 1) for i, d in enumerate(DOMAIN_ORDER)}

    def next_id(domain: str, used: dict) -> str:
        lo, hi = DOMAIN_BLOCK.get(domain, (AUTHOR_BASE + len(DOMAIN_ORDER) * BLOCK, 99999))
        n = used.get(domain, lo)
        if n > hi:
            raise SystemExit(f"domain block exhausted for {domain}: {lo}..{hi}; widen BLOCK")
        used[domain] = n + 1
        return f"MT-{n:05d}"

    used_counter: dict[str, int] = {}
    existing_ids = {m["mt_id"] for m in mts}
    new_specs = []
    for (domain, key), rows in sorted(clusters.items(), key=lambda kv: (kv[0][0], -len(kv[1]))):
        if len(rows) < 2:
            continue
        while True:
            mid = next_id(domain, used_counter)
            if mid not in existing_ids:
                break
        beh = [b for r in rows for b in r.get("behaviour", [])]
        new_specs.append({
            "proposed_mt_id": mid,
            "domain": domain,
            "cluster_key": key,
            "capability_count": len(rows),
            "capabilities": [{"name": r["name"], "kind": r["kind"], "apps": r["source_apps"]} for r in rows[:60]],
            "behaviour_records": len(beh),
            "behaviour_sample": beh[:10],
            "acceptance_rows_required": max(4, min(12, 2 + len(rows) // 4)),
            "note": "Behavioural payload must be attached before this MT is authored; capability names alone are not an implementable contract.",
        })

    by_dom = collections.Counter(a["domain"] for a in assignments)
    by_state = collections.Counter(a["state"] for a in assignments)
    dom_rows = []
    for d in sorted(set(list(BAND) + list(NEW_BANDS) + list(by_dom))):
        existing = sum(1 for m in mts if band_of.get(m["mt_id"]) == d)
        unc = sum(1 for a in assignments if a["domain"] == d and a["state"] == "UNCOVERED")
        proposed = sum(1 for s in new_specs if s["domain"] == d)
        dom_rows.append({"domain": d, "capabilities": by_dom.get(d, 0), "existing_mts": existing, "uncovered_capabilities": unc, "proposed_new_mts": proposed, "legacy_band": BAND.get(d) or NEW_BANDS.get(d), "authoring_block": DOMAIN_BLOCK.get(d)})

    doc = {
        "schema_id": "handshake.reference.studio_mt_coverage_plan@1",
        "generated_at": now(),
        "purpose": "Authoring plan for WP-KERNEL-STUDIO: map every extracted capability to an existing microtask or a proposed new one, and attach the behavioural payload each microtask must implement.",
        "inputs": {"capability_registry": str(reg_path.name), "existing_mts": len(mts), "teardown_sources_present": sources_present, "teardown_sources_missing": sources_missing},
        "totals": {"capabilities": len(assignments), "by_state": dict(by_state), "by_domain": dict(by_dom), "behaviour_records_indexed": sum(len(v) for v in payload.values()), "proposed_new_mts": len(new_specs)},
        "domains": dom_rows,
        "new_mt_specs": new_specs,
        "assignments": assignments,
    }
    (args.out / "studio-mt-coverage-plan.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    print(f"[plan] capabilities={len(assignments)} state={dict(by_state)}")
    print(f"[plan] behaviour records indexed={sum(len(v) for v in payload.values())} from {len(sources_present)} teardown files ({len(sources_missing)} not yet produced)")
    print(f"[plan] proposed new MTs={len(new_specs)}")
    for r in dom_rows:
        print(f"   {r['domain']:14s} caps={r['capabilities']:6d} existing_mts={r['existing_mts']:4d} uncovered={r['uncovered_capabilities']:6d} new_mts={r['proposed_new_mts']:4d}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
