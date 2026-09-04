"""Run the whole offline After Effects 2026 teardown.

Reads only. NEVER launches After Effects or any other application.

    python ae_run_all.py

Override roots with the AE_INSTALL_ROOT and AE_OUT_ROOT environment variables.
"""

from __future__ import annotations

import importlib
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ae_common as C  # noqa: E402

TOOLS = [
    ("ae_effects", "aftereffects_effects_catalogue.json"),
    ("ae_presets", "aftereffects_presets.json"),
    ("ae_layer_model", "aftereffects_layer_property_model.json"),
    ("ae_scripting", "aftereffects_scripting_expressions.json"),
    ("ae_panels", "aftereffects_panels_dialogs.json"),
    ("ae_render", "aftereffects_render_output.json"),
    ("ae_commands", "aftereffects_commands_shortcuts.json"),
    ("ae_text_shape_mask", "aftereffects_text_shape_mask.json"),
]


def main():
    print("install root : %s" % C.install_root())
    print("output root  : %s" % C.out_root())
    print("app launched : False")
    failures = []
    for mod_name, out_name in TOOLS:
        t0 = time.time()
        try:
            mod = importlib.import_module(mod_name)
            mod.main()
            print("OK   %-22s -> %-46s %.1fs"
                  % (mod_name, out_name, time.time() - t0))
        except Exception as exc:  # noqa: BLE001
            failures.append((mod_name, "%s: %s" % (type(exc).__name__, exc)))
            print("FAIL %-22s %s" % (mod_name, exc))
    if failures:
        print("\n%d tool(s) failed:" % len(failures))
        for m, e in failures:
            print("  %s: %s" % (m, e))
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
