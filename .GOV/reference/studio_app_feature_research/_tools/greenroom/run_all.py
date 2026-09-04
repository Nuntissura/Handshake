"""run_all.py -- build every Dreamweaver 2021 offline export in one pass.

Usage: python run_all.py <output_dir>
Reads the installed Configuration tree read-only. Never launches Dreamweaver.
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import dw_common as C                                       # noqa: E402
import dw_command_surface                                   # noqa: E402
import dw_panels_dialogs                                    # noqa: E402
import dw_objects_behaviors                                 # noqa: E402
import dw_code_intelligence                                 # noqa: E402
import dw_site_server                                       # noqa: E402
import dw_templates_css                                     # noqa: E402
from dw_zstrings import load_all_strings                    # noqa: E402

BUILDS = [
    ("dreamweaver_command_surface.json", dw_command_surface.build),
    ("dreamweaver_panels_dialogs.json", dw_panels_dialogs.build),
    ("dreamweaver_objects_behaviors.json", dw_objects_behaviors.build),
    ("dreamweaver_code_intelligence.json", dw_code_intelligence.build),
    ("dreamweaver_site_server_model.json", dw_site_server.build),
    ("dreamweaver_templates_css.json", dw_templates_css.build),
]


def main(outdir):
    os.makedirs(outdir, exist_ok=True)
    summary = {}
    for name, fn in BUILDS:
        path = os.path.join(outdir, name)
        doc, size = fn(path)
        summary[name] = {"bytes": size,
                         "failures": len(doc["failures"]),
                         "counts": doc["counts"]}
        print("== %s  %d bytes  %d failures" % (name, size, len(doc["failures"])))

    # supporting artifact: the decoded English string table, so every label key
    # cited anywhere in the six exports can be resolved by a downstream tool.
    exact, lower, meta = load_all_strings(C.INSTALL_ROOT)
    sdoc = C.envelope(
        "handshake.studio.dreamweaver.strings_en_US.v1",
        {"task": "7 - English string tables",
         "how": ["en_US/Resources/strings.zbin and NonLocalisedStrings.zbin are "
                 "Adobe 'ZString Binary Format' files. No Adobe library was used; "
                 "the container was reverse-engineered from its bytes and is "
                 "documented in dw_zstrings.py. Every entry of both files decodes "
                 "without error.",
                 "Keys are UTF-8, values UTF-16LE. Merged here with strings.zbin "
                 "winning on the few duplicate keys."],
         "string_tables": meta},
        {"counts": {"unique_keys": len(exact),
                    "strings_zbin_entries": meta.get("strings.zbin", {}).get("entry_count"),
                    "nonlocalised_zbin_entries":
                        meta.get("NonLocalisedStrings.zbin", {}).get("entry_count")},
         "strings": exact,
         "excluded_ai": C.excluded_ai(
             "English string tables",
             candidates=list(exact.keys()) + list(exact.values()),
             extra_note="Swept all %d decoded keys and all %d decoded values."
                        % (len(exact), len(exact))),
         "failures": []})
    sp = os.path.join(outdir, "dreamweaver_strings_en_US.json")
    ssize = C.write_json(sp, sdoc)
    summary["dreamweaver_strings_en_US.json"] = {"bytes": ssize, "failures": 0,
                                                 "counts": sdoc["counts"]}
    print("== dreamweaver_strings_en_US.json  %d bytes" % ssize)
    return summary


if __name__ == "__main__":
    s = main(sys.argv[1])
    print(json.dumps({k: {"bytes": v["bytes"], "failures": v["failures"]}
                      for k, v in s.items()}, indent=1))
