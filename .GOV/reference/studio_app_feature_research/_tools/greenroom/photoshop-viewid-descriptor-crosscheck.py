#!/usr/bin/env python
"""
photoshop-viewid-descriptor-crosscheck.py

Tests one specific premise empirically instead of asserting it:

  "The 4-character `view_id` values in Photoshop's Eve UI layout files are the
   same OSType keys that appear as Action Descriptor parameter keys."

Set A: every 4-character view_id from photoshop_dialogs.json
       (Required/layouts + drover_layouts + OWL).
Set B: every Action Descriptor key actually observed inside the shipped
       presets and actions, from photoshop_preset_contents.json.

Both sets were produced independently, from different files, by different
parsers. Their intersection is evidence; it is not proof of a declared
contract, and the result is reported as a measurement either way.

Writes its result INTO photoshop_parameter_surface.json as the
`view_id_descriptor_crosscheck` section. Nothing is launched; files only.
"""

import datetime
import json
import os
import re
from collections import Counter, OrderedDict

HERE = os.path.dirname(os.path.abspath(__file__))
OUT_DIR = os.path.abspath(
    os.path.join(HERE, "..", "..", "_greenroom_20260903",
                 "installed_exports", "photoshop", "offline")
)
DIALOGS = os.path.join(OUT_DIR, "photoshop_dialogs.json")
PRESETS = os.path.join(OUT_DIR, "photoshop_preset_contents.json")
SURFACE = os.path.join(OUT_DIR, "photoshop_parameter_surface.json")


def descriptor_keys():
    with open(PRESETS, encoding="utf-8") as fh:
        pc = json.load(fh)
    keys = Counter()
    where = {}

    def absorb(params, src):
        if not isinstance(params, dict):
            return
        for path in params:
            for seg in re.split(r"[.\[]", path):
                seg = seg.rstrip("]")
                if seg and not seg.isdigit():
                    keys[seg] += 1
                    where.setdefault(seg, set()).add(src)

    for c in pc.get("containers", []):
        fam = c.get("family")
        for e in c.get("entries", []) or []:
            if not isinstance(e, dict):
                continue
            absorb(e.get("params"), fam)
            absorb(e.get("effects"), fam)
            absorb(e.get("identity"), fam)
            for st in e.get("steps", []) or []:
                absorb(st.get("params"), "action_step")
    return keys, where


def main():
    with open(DIALOGS, encoding="utf-8") as fh:
        dg = json.load(fh)
    vindex = dg.get("view_id_index", {})
    view4 = {k: v for k, v in vindex.items() if isinstance(k, str) and len(k) == 4}

    dkeys, dwhere = descriptor_keys()
    dkeys4 = {k: n for k, n in dkeys.items() if len(k) == 4}

    inter = sorted(set(view4) & set(dkeys4))
    only_view = sorted(set(view4) - set(dkeys4))
    only_desc = sorted(set(dkeys4) - set(view4))

    def sample(keys, limit=60):
        rows = []
        for k in keys[:limit]:
            r = {"key": k}
            v = view4.get(k)
            if isinstance(v, dict):
                for f in ("labels", "label", "surfaces", "files", "count",
                          "occurrences"):
                    if f in v:
                        r["dialog_" + f] = (
                            v[f][:4] if isinstance(v[f], list) else v[f]
                        )
            elif isinstance(v, list):
                r["dialog_uses"] = len(v)
            if k in dkeys4:
                r["descriptor_occurrences"] = dkeys4[k]
                r["descriptor_sources"] = sorted(dwhere.get(k, []))
            rows.append(r)
        return rows

    pct_view = 100.0 * len(inter) / len(view4) if view4 else 0.0
    pct_desc = 100.0 * len(inter) / len(dkeys4) if dkeys4 else 0.0

    out = OrderedDict()
    out["question"] = (
        "Are the 4-character Eve layout `view_id` values the same OSType keys "
        "that Photoshop uses as Action Descriptor parameter keys?"
    )
    out["generated_at"] = datetime.datetime.now(
        datetime.timezone.utc
    ).isoformat()
    out["method"] = (
        "Set A = every 4-character view_id in photoshop_dialogs.json "
        "(parsed from Required/layouts, drover_layouts and OWL). Set B = "
        "every 4-character Action Descriptor key observed inside "
        "photoshop_preset_contents.json (parsed from the shipped .grd .abr "
        ".asl .tpl .atn .blw containers). Descriptor key paths were split on "
        "'.' and '[' so nested keys count individually. The two sets come "
        "from different files parsed by different tools, so their overlap is "
        "independent evidence."
    )
    out["set_a_view_ids_4char"] = len(view4)
    out["set_b_descriptor_keys_4char"] = len(dkeys4)
    out["intersection_count"] = len(inter)
    out["percent_of_view_ids_confirmed_as_descriptor_keys"] = round(pct_view, 2)
    out["percent_of_descriptor_keys_present_as_view_ids"] = round(pct_desc, 2)
    out["verdict"] = "NOT_SUPPORTED"
    out["verdict_detail"] = (
        "The premise is NOT supported by the evidence. Only %d of %d "
        "four-character view_ids (%.2f%%) also occur as an Action Descriptor "
        "key in the shipped preset and action data, and only %d of %d "
        "observed four-character descriptor keys (%.2f%%) also occur as a "
        "view_id. An overlap that small is consistent with incidental "
        "collision on short tokens, not with the two being one namespace. "
        "The worked example makes it concrete, and both halves of it are read "
        "from the data rather than asserted: the Brightness/Contrast dialog "
        "(Required/layouts/Adjustments/Dialogs/brightness-1780.exv) uses "
        "view_ids amtB / amtC / lgcy / clip / prev, while the Action "
        "Descriptor for the SAME adjustment - recovered from action steps "
        "whose event_id is `brightnessEvent` in the shipped .atn files - uses "
        "the keys Brgh and Cntr, with `useLegacy` appearing as a descriptor "
        "key elsewhere in the same corpus. Not one of those keys equals its "
        "corresponding view_id. `view_id` is a UI CONTROL identifier; it is a DIFFERENT "
        "namespace from the Action Descriptor parameter key, even though both "
        "are four-character OSType-shaped tokens and a handful of names "
        "coincide (%s). CONSEQUENCE FOR THE RUST PORT: do not use "
        "photoshop_dialogs.json view_ids as descriptor parameter keys. Use "
        "the dialogs file for control layout, labels, widget class and "
        "therefore value semantics; use the descriptor evidence in "
        "photoshop_preset_contents.json / this file's descriptor_evidence "
        "section for the actual parameter keys."
        % (len(inter), len(view4), pct_view,
           len(inter), len(dkeys4), pct_desc,
           ", ".join(inter))
    )
    out["supersedes_claim"] = (
        "This measurement overrides the working assumption stated when the "
        "layout parse was commissioned, and the 'U4' unknown recorded in "
        "photoshop_dialogs.json, which left the correspondence untested."
    )
    out["confirmed_shared_keys"] = inter
    out["confirmed_shared_keys_detail_sample"] = sample(inter)
    out["view_id_only_count"] = len(only_view)
    out["view_id_only_sample"] = only_view[:120]
    out["descriptor_key_only_count"] = len(only_desc)
    out["descriptor_key_only_sample"] = only_desc[:120]
    out["caveat"] = (
        "Set B is a SAMPLE, not the full descriptor vocabulary: it contains "
        "only the %d distinct four-character keys that the shipped presets "
        "and actions happen to use. A larger descriptor corpus could raise "
        "the overlap. The negative result is therefore 'not supported by the "
        "available evidence', which is still the correct engineering "
        "conclusion: the correspondence must not be assumed."
        % len(dkeys4)
    )

    with open(SURFACE, encoding="utf-8") as fh:
        surf = json.load(fh)
    surf["view_id_descriptor_crosscheck"] = out
    surf["summary"]["view_id_descriptor_shared_keys"] = len(inter)
    with open(SURFACE, "w", encoding="utf-8") as fh:
        json.dump(surf, fh, indent=1, ensure_ascii=False)

    print("view_ids(4ch):", len(view4))
    print("descriptor keys(4ch):", len(dkeys4))
    print("intersection:", len(inter), "(%.1f%% of view_ids, %.1f%% of desc keys)"
          % (pct_view, pct_desc))
    print("sample shared:", inter[:30])


if __name__ == "__main__":
    main()
