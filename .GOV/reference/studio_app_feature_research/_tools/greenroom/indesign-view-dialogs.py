#!/usr/bin/env python3
"""Handshake Studio green room: extract InDesign dialog and panel layouts (offline).

idrc_VIEW holds InDesign's compiled widget hierarchies - the binary form of the Fred (.fr)
view resources that define every dialog and panel. idrc_PMST holds the localized strings.
This tool walks the VIEW node stream, pulls the per-dialog control inventory, and joins the
English (locale_id 1/2) PMST tables of the owning plug-in.

The application is never launched.

VIEW GRAMMAR (reversed 2026-09-04, little-endian)
    u32 resource_owner_id
    repeated nodes:
        u8[2] magic 0x33 0x33
        u32   class_id          # boss / implementation class id
        ...   payload           # class-specific, length not self-describing

The payload is NOT length-prefixed, so node boundaries are found by scanning for the next
0x3333 magic followed by a class id that the corpus-wide census has already confirmed. Node
payloads are reported as raw length plus the strings and integers found inside them.

WHAT IS PARSED vs WHAT IS NOT
    parsed    : node sequence, class ids, payload byte spans, embedded length-prefixed
                strings, embedded u32 fields, the D::RESS source path (dialog name + locale)
    heuristic : the role guessed for each class id, derived from payload shape and how often
                that class carries a label. Reported under class_id_census[].role_hint and
                never merged into the parsed fields.
    absent    : the install ships no class-id -> widget-name table. All 414 plug-in and DLL
                binaries were scanned for widget class-name strings matching
                k<Name>WidgetBoss / k<Name>PanelWidgetBoss / k<Name>WidgetID: zero hits in
                zero files. A looser pattern that also accepts any k<Name>Boss does match
                80 names across 11 binaries, but those are command and text-attribute boss
                names (kKernBoss, kPositioningBoss, kDataMergeFrameAdornmentBoss, ...), not
                a widget class registry. Control kinds are therefore reported as numeric
                class ids, and no widget names are invented.

Output: indesign_dialogs.json
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import re
import struct
from pathlib import Path

MAGIC = b"\x33\x33"
ENGLISH_LOCALES = {1, 2}
RESS = re.compile(rb"D::RESS:ID:[\x20-\x7e]{5,240}")
ENGLISH_TAG = re.compile(r"_(enUS|enGB|enIN)\.fr$", re.I)
LOCALE_TAG = re.compile(r"_([a-z]{2}[A-Z]{2})\.fr$")


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def owner_of(p: Path) -> str:
    for part in p.parts:
        if part.startswith("(") and part.endswith("Resources)"):
            return part.strip("()").replace(" Resources", "")
    return "APP_ROOT"


def parse_pmst(data: bytes):
    if len(data) < 12:
        return None
    loc, _u, cnt = struct.unpack_from("<III", data, 0)
    o, out = 12, {}
    for _ in range(cnt):
        if o + 2 > len(data):
            break
        kl = struct.unpack_from("<H", data, o)[0]; o += 2
        k = data[o:o + kl]; o += kl
        if o + 2 > len(data):
            break
        vl = struct.unpack_from("<H", data, o)[0]; o += 2
        v = data[o:o + vl]; o += vl
        if len(k) != kl or len(v) != vl:
            break
        try:
            out[k.decode("ascii")] = v.decode("utf-8")
        except UnicodeDecodeError:
            continue
    return {"locale_id": loc, "strings": out}


def node_offsets(data: bytes, allowed: set | None):
    """Offsets of every 0x3333 node whose class id is plausible."""
    out = []
    i = 0
    n = len(data)
    while True:
        i = data.find(MAGIC, i)
        if i < 0 or i + 6 > n:
            break
        cid = struct.unpack_from("<I", data, i + 2)[0]
        ok = cid != 0 and cid < 0x0200_0000
        if ok and allowed is not None:
            ok = cid in allowed
        if ok:
            out.append((i, cid))
            i += 6
        else:
            i += 1
    return out


def strings_in(chunk: bytes):
    """Length-prefixed ASCII strings inside a node payload."""
    out = []
    i = 0
    while i + 2 <= len(chunk):
        ln = struct.unpack_from("<H", chunk, i)[0]
        if 2 <= ln <= 300 and i + 2 + ln <= len(chunk):
            s = chunk[i + 2:i + 2 + ln]
            if all(32 <= b < 127 for b in s) and sum(1 for b in s if 65 <= b < 123) >= 2:
                out.append(s.decode("ascii"))
                i += 2 + ln
                continue
        i += 1
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--max-nodes-per-file", type=int, default=1200)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    files = sorted(p for p in args.root.rglob("*.idrc") if p.parent.name == "idrc_VIEW")

    # ---- pass 1: class-id census (unfiltered), to build the plausible-id set -------------
    raw_census = collections.Counter()
    blobs = {}
    for p in files:
        d = p.read_bytes()
        blobs[p] = d
        for _, cid in node_offsets(d, None):
            raw_census[cid] += 1
    # a class id used at least 3 times corpus-wide is a real class, not a data coincidence
    allowed = {cid for cid, n in raw_census.items() if n >= 3}

    # ---- English PMST pool per plug-in ---------------------------------------------------
    pmst_by_plugin: dict[str, dict] = collections.defaultdict(dict)
    global_strings: dict[str, str] = {}
    pmst_files = [p for p in args.root.rglob("*.idrc") if p.parent.name == "idrc_PMST"]
    pmst_en = 0
    for p in pmst_files:
        try:
            with p.open("rb") as fh:
                head = fh.read(4)
                if len(head) < 4 or struct.unpack("<I", head)[0] not in ENGLISH_LOCALES:
                    continue
                rec = parse_pmst(head + fh.read())
        except Exception:  # noqa: BLE001
            continue
        if not rec:
            continue
        pmst_en += 1
        pmst_by_plugin[owner_of(p)].update(rec["strings"])
        for _k, _v in rec["strings"].items():
            global_strings.setdefault(_k, _v)

    # ---- pass 2: per-resource control inventory -------------------------------------------
    resources = []
    class_stat = collections.defaultdict(lambda: {"nodes": 0, "with_label": 0,
                                                  "payload_total": 0, "labels": []})
    locale_hist = collections.Counter()
    for p in files:
        d = blobs[p]
        plugin = owner_of(p)
        rel = str(p.relative_to(args.root)).replace("\\", "/")
        m = RESS.search(d)
        src = m.group().decode("latin-1").rstrip("\x00") if m else None
        dialog = None
        locale = None
        if src:
            leaf = src.split(":")[-1]
            lm = LOCALE_TAG.search(leaf)
            locale = lm.group(1) if lm else None
            dialog = LOCALE_TAG.sub("", leaf) or leaf
        locale_hist[locale or "<none>"] += 1
        offs = node_offsets(d, allowed)
        controls = []
        for idx, (off, cid) in enumerate(offs[:args.max_nodes_per_file]):
            end = offs[idx + 1][0] if idx + 1 < len(offs) else len(d)
            payload = d[off + 6:end]
            labels = strings_in(payload)
            if src and labels and labels[0].startswith("D::RESS"):
                labels = labels[1:]
            fields = list(struct.unpack_from("<%dI" % min(8, len(payload) // 4), payload)) \
                if len(payload) >= 4 else []
            resolved = []
            for lb in labels[:6]:
                txt = pmst_by_plugin.get(plugin, {}).get(lb) or global_strings.get(lb)
                resolved.append({"raw": lb, "english": txt,
                                 "source": ("idrc_PMST" if txt else "literal_in_view")})
            st = class_stat[cid]
            st["nodes"] += 1
            st["payload_total"] += len(payload)
            if labels:
                st["with_label"] += 1
                if len(st["labels"]) < 12:
                    st["labels"].append(labels[0][:60])
            controls.append({"index": idx, "offset": off, "class_id": cid,
                             "class_id_hex": f"{cid:#x}", "payload_bytes": len(payload),
                             "labels": labels[:6], "resolved_labels": resolved,
                             "leading_u32": fields})
        resources.append({
            "file": rel, "plugin": plugin,
            "resource_id": p.stem,
            "bytes": len(d),
            "source_path": src,
            "dialog_or_panel": dialog,
            "locale": locale,
            "is_english": bool(src and ENGLISH_TAG.search(src)) or (src is None),
            "node_count": len(offs),
            "nodes_emitted": len(controls),
            "distinct_class_ids": len({c["class_id"] for c in controls}),
            "label_count": sum(len(c["labels"]) for c in controls),
            "controls": controls,
        })

    # ---- heuristic role hints -------------------------------------------------------------
    census = []
    for cid, st in sorted(class_stat.items(), key=lambda kv: -kv[1]["nodes"]):
        avg = st["payload_total"] / st["nodes"] if st["nodes"] else 0
        frac = st["with_label"] / st["nodes"] if st["nodes"] else 0
        if frac > 0.8:
            hint = "carries a visible label almost always (static text / button / group title)"
        elif frac > 0.25:
            hint = "sometimes carries a label"
        elif avg > 60:
            hint = "large fixed-size record, no label (geometry / view state)"
        else:
            hint = "small record, no label (structural or reference node)"
        census.append({"class_id": cid, "class_id_hex": f"{cid:#x}", "nodes": st["nodes"],
                       "nodes_with_label": st["with_label"],
                       "mean_payload_bytes": round(avg, 1),
                       "role_hint": hint, "role_hint_basis": "heuristic",
                       "label_samples": st["labels"]})

    english = [r for r in resources if r["is_english"]]
    # per-dialog rollup across the English resources
    by_dialog = collections.defaultdict(lambda: {"resources": [], "plugins": set(),
                                                 "controls": 0, "labels": [],
                                                 "label_rows": []})
    for r in english:
        key = r["dialog_or_panel"] or f"{r['plugin']}#{r['resource_id']}"
        g = by_dialog[key]
        g["resources"].append(r["file"])
        g["plugins"].add(r["plugin"])
        g["controls"] += r["nodes_emitted"]
        for c in r["controls"]:
            for rl in c["resolved_labels"]:
                shown = rl["english"] or rl["raw"]
                if shown not in g["labels"] and len(g["labels"]) < 400:
                    g["labels"].append(shown)
                    g["label_rows"].append(rl)
    dialogs = [{"dialog_or_panel": k, "plugins": sorted(v["plugins"]),
                "resource_files": v["resources"], "control_nodes": v["controls"],
                "labels": v["labels"], "label_count": len(v["labels"]),
                "labels_resolved_from_pmst": sum(1 for r in v["label_rows"] if r["english"]),
                "label_detail": v["label_rows"]}
               for k, v in sorted(by_dialog.items(), key=lambda kv: -kv[1]["controls"])]

    doc = {
        "schema_id": "handshake.reference.indesign_dialogs@1",
        "generated_at": now(),
        "source_root": str(args.root),
        "resource_codes": ["VIEW", "PMST"],
        "method": (
            "Offline walk of idrc_VIEW compiled widget hierarchies plus a join onto the English "
            "(locale_id 1 or 2) idrc_PMST string tables. The application was never launched. "
            "PARSED: node sequence, boss/implementation class ids, payload spans, embedded "
            "length-prefixed labels, embedded u32 fields, and the D::RESS source path that names "
            "each dialog and its locale. HEURISTIC and labelled as such: node boundaries (VIEW "
            "payloads are not length-prefixed, so boundaries come from scanning for the next "
            "0x3333 magic whose class id occurs at least 3 times corpus-wide) and "
            "class_id_census[].role_hint. NOT AVAILABLE: the install ships no class-id to "
            "widget-name table, so control kinds are numeric class ids. All 414 plug-in/DLL "
            "binaries were scanned for k<Name>WidgetBoss / k<Name>PanelWidgetBoss / "
            "k<Name>WidgetID strings and produced zero hits in zero files; a looser k<Name>Boss "
            "pattern matches 80 names in 11 binaries but those are command and text-attribute "
            "boss names, not a widget registry. No widget names are invented here."
        ),
        "format": {
            "view": "u32 root/resource id at offset 0, then repeated "
                    "{u8[2] 0x33 0x33, u32 class_id, payload}. The first node magic is at "
                    "offset 4; this parser locates nodes by scanning for the magic, so the "
                    "leading u32 is skipped naturally.",
            "payload": "class-specific, not length-prefixed",
            "labels": "u16 length + ASCII, embedded in the payload. A label is either the "
                      "literal English string or a PMST key; resolved_labels[].english holds "
                      "the English text when the key resolves against the plug-in's "
                      "locale_id 1/2 idrc_PMST table (or, failing that, the corpus-wide "
                      "English key pool), and source says which.",
            "source_path": "ASCII 'D::RESS:ID:...:<DialogName>_<locale>.fr' identifies the "
                           "dialog and its locale",
        },
        "totals": {
            "view_resource_files": len(files),
            "view_bytes": sum(len(b) for b in blobs.values()),
            "nodes_found": sum(r["node_count"] for r in resources),
            "nodes_emitted": sum(r["nodes_emitted"] for r in resources),
            "distinct_class_ids_raw": len(raw_census),
            "distinct_class_ids_confirmed": len(allowed),
            "resources_with_source_path": sum(1 for r in resources if r["source_path"]),
            "english_or_locale_neutral_resources": len(english),
            "distinct_dialogs_or_panels": len(dialogs),
            "labels_extracted": sum(r["label_count"] for r in resources),
            "dialog_labels_resolved_from_pmst": sum(d_["labels_resolved_from_pmst"]
                                                    for d_ in dialogs),
            "dialog_labels_total": sum(d_["label_count"] for d_ in dialogs),
            "pmst_files_scanned": len(pmst_files),
            "pmst_english_files_used": pmst_en,
            "plugins_with_english_strings": len(pmst_by_plugin),
            "english_string_keys": sum(len(v) for v in pmst_by_plugin.values()),
        },
        "locale_histogram": dict(locale_hist.most_common()),
        "class_id_census": census,
        "dialogs_and_panels": dialogs,
        "resources": resources,
        "plugin_english_strings": {k: v for k, v in sorted(pmst_by_plugin.items())},
    }
    outp = args.out / "indesign_dialogs.json"
    outp.write_text(json.dumps(doc, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    t = doc["totals"]
    print(f"[view] files={t['view_resource_files']} nodes={t['nodes_found']} "
          f"class_ids raw={t['distinct_class_ids_raw']} confirmed={t['distinct_class_ids_confirmed']}")
    print(f"[view] dialogs={t['distinct_dialogs_or_panels']} english_resources="
          f"{t['english_or_locale_neutral_resources']} labels={t['labels_extracted']}")
    print(f"[view] pmst english files={t['pmst_english_files_used']}/{t['pmst_files_scanned']} "
          f"keys={t['english_string_keys']} plugins={t['plugins_with_english_strings']}")
    print(f"[view] -> {outp} ({outp.stat().st_size/1048576:.1f} MB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
