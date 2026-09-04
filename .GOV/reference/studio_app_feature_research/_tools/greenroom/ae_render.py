"""After Effects 2026 -> aftereffects_render_output.json

Offline. Reads only. Never launches After Effects.

Sources
-------
1. Support Files/PluginSupport.dll carries a 351 KB NUL-terminated JSON media
   registry: {"AEVideoPlugins", "CacheVersion", "ExporterPlugins",
   "ImporterPlugins"} - every shipped exporter (name, ClassID, FileType,
   DefaultExtension, DoesVideo/DoesAudio) and every importer (description,
   extension mask, FileType, flags, subtypes).
2. Support Files/aelib.dll carries the After Effects importable-extension list
   ({"mExt": ".mov", "mOnlyClient": "AE,aecmd,PPro,AME"} ...).
3. $$$/MediaCore/Exporters/<Module>/<Param> ZStrings give the codec parameter
   labels and enumerated option labels per exporter module.
4. The Render Settings and Output Module dialogs ship as embedded Adobe Eve
   layout sources inside AfterFXLib.dll; every field and option is recovered
   from the Eve control tree.
5. Support Files/Plug-ins/Format/*.aex are the After Effects format plug-ins.

Negative finding
----------------
The factory Render Settings templates and Output Module templates are written
into the per-user preference files on first launch. This machine's
%APPDATA%/Adobe/After Effects/26.3 tree is EMPTY (the app has never been run,
and this teardown does not launch it), so the shipped template NAMES are
recovered from string evidence but their stored field values are not on disk.
"""

from __future__ import annotations

import collections
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ae_common as C  # noqa: E402
import ae_panels as PANELS  # noqa: E402


def media_registry():
    p = os.path.join(C.support_files(), "PluginSupport.dll")
    data = C.read_bytes(p)
    blobs = C.extract_json_nul_delimited(data, b'"ExporterPlugins"')
    if not blobs:
        return None, C.rel(p)
    return max(blobs, key=lambda b: len(str(b))), C.rel(p)


def ae_extensions():
    p = os.path.join(C.support_files(), "aelib.dll")
    data = C.read_bytes(p)
    out = []
    for blob in C.extract_json_nul_delimited(data, b'"mExt"'):
        for k, v in (blob.items() if isinstance(blob, dict) else []):
            if isinstance(v, list) and v and isinstance(v[0], dict) and "mExt" in v[0]:
                out.append({"group": k, "entries": v})
    return out, C.rel(p)


def flatten_exporters(reg):
    rows = []
    ep = (reg or {}).get("ExporterPlugins") or {}
    for mod in ep.get("ExporterModules", []) or []:
        name = mod.get("ModuleName")
        for e in mod.get("Exporter", []) or mod.get("Exporters", []) or []:
            rows.append(_exp_row(name, e))
        # some modules nest the list under a singular key holding a dict
        inner = mod.get("Exporter")
        if isinstance(inner, dict):
            rows.append(_exp_row(name, inner))
    return rows


def _exp_row(module, e):
    nm = e.get("Name", "")
    return {
        "module": module,
        "exporter_name": C.strip_zstring_key(nm),
        "exporter_name_key": C.zstring_key_of(nm),
        "default_extension": e.get("DefaultExtension"),
        "file_type_fourcc": _fourcc(e.get("FileType")),
        "file_type_raw": e.get("FileType"),
        "class_id": e.get("ClassID"),
        "api_version": e.get("APIVersion"),
        "does_video": bool(e.get("DoesVideo")),
        "does_audio": bool(e.get("DoesAudio")),
        "audio_only_unsupported": bool(e.get("DoesNotSupportAudioOnly")),
        "plugin_factory_guid": e.get("Plugin Factory Guid"),
        "sub_key": e.get("SubKeyName"),
        "index": e.get("Index"),
        "hidden_in_ui": bool(e.get("HideInUI")),
    }


def flatten_importers(reg):
    rows = []
    ip = (reg or {}).get("ImporterPlugins") or {}
    for mod in ip.get("ImporterModules", []) or []:
        name = mod.get("ModuleName")
        imp = mod.get("Importer") or {}
        for d in imp.get("ImpDesc", []) or []:
            desc = d.get("Description", "")
            rows.append({
                "module": name,
                "description": C.strip_zstring_key(desc),
                "description_key": C.zstring_key_of(desc),
                "extensions": d.get("Extensions"),
                "file_type_fourcc": _fourcc(d.get("FileType")),
                "file_type_raw": d.get("FileType"),
                "flags_raw": d.get("Flags"),
                "sub_types": d.get("SubTypes"),
                "keep_loaded": imp.get("ImpKeepLoaded"),
                "priority": imp.get("ImpPriority"),
                "plugin_factory_guid": imp.get("Plugin Factory Guid"),
            })
    return rows


def _fourcc(v):
    if not isinstance(v, int) or v < 0 or v > 0xFFFFFFFF:
        return None
    b = v.to_bytes(4, "big")
    return b.decode("latin-1") if all(32 <= c < 127 for c in b) else None


def exporter_parameter_labels(idx):
    """$$$/MediaCore/Exporters/<Module>/<Param>=<label>, grouped per module."""
    groups = collections.defaultdict(list)
    for k, v in C.keys_under("MediaCore/Exporters/", idx).items():
        parts = k.split("/")
        if len(parts) < 4:
            continue
        mod = parts[2]
        leaf = "/".join(parts[3:])
        text = v["text"]
        rec = {"param_key": leaf, "label": text}
        if "|" in text and text.count("|") >= 1 and len(text) < 600:
            rec["options"] = text.split("|")
            rec["option_count"] = len(rec["options"])
        groups[mod].append(rec)
    return {k: sorted(v, key=lambda r: r["param_key"]) for k, v in groups.items()}


RENDER_EVE_HINTS = ("RenderSettings", "OutputModule", "OMDialog", "Template",
                    "RenderQueue", "OutputTo", "Render", "Output")


def render_dialog_surfaces():
    """Eve layouts whose name or strings identify the render/output dialogs."""
    out = []
    sf = C.support_files()
    for p in C.iter_files(sf, (".dll", ".aex", ".exe"),
                          skip_dirs=("node_modules", "CEPHtmlEngine")):
        data = C.read_bytes(p)
        if b"@place_column" not in data and b"@place_row" not in data:
            continue
        for off, blob in PANELS.eve_blobs(data):
            try:
                text = blob.decode("utf-8")
            except UnicodeDecodeError:
                text = blob.decode("latin-1")
            if not any(h in text for h in RENDER_EVE_HINTS):
                continue
            for s in PANELS.eve_surface(text, C.rel(p), off):
                out.append(s)
    return out


def main():
    idx = C.build_english_index()
    reg, reg_path = media_registry()
    exporters = flatten_exporters(reg)
    importers = flatten_importers(reg)
    exts, ext_path = ae_extensions()
    params = exporter_parameter_labels(idx)
    dialogs = render_dialog_surfaces()

    fmt_dir = os.path.join(C.support_files(), "Plug-ins", "Format")
    format_plugins = []
    if os.path.isdir(fmt_dir):
        for fn in sorted(os.listdir(fmt_dir)):
            if not fn.lower().endswith(".aex"):
                continue
            full = os.path.join(fmt_dir, fn)
            data = C.read_bytes(full)
            ns = collections.Counter()
            for k, _v in C.zstrings(data):
                ns[k.split("/LStr/")[0]] += 1
            format_plugins.append({
                "file": C.rel(full),
                "bytes": os.path.getsize(full),
                "zstring_namespaces": dict(ns.most_common(5)),
            })

    # template names that exist as strings
    tmpl = {}
    for pref in ("ae/OutputModuleTemplates/", "AE/Edit/Templates/",
                 "AE/DialogPrefsOM/", "AE/DialogPrefsRS/"):
        for k, v in C.keys_under(pref, idx).items():
            tmpl[k] = v["text"]
    # render-settings vocabulary that ships in the EGG string block
    egg_terms = {}
    WANT = ("Best Settings", "Draft Settings", "Current Settings", "Lossless",
            "Lossless with Alpha", "Alpha Only", "Multi-Machine Sequence",
            "Multi-Machine Settings", "Custom", "Full", "Half", "Third",
            "Quarter", "Use comp's frame rate", "Use this frame rate")
    for k, v in C.keys_under("AE/EGG/", idx).items():
        if v["text"] in WANT:
            egg_terms.setdefault(v["text"], []).append(k)

    gpu_modules = []
    if reg and isinstance(reg.get("AEVideoPlugins"), dict):
        for m in reg["AEVideoPlugins"].get("AEModules", []) or []:
            for e in m.get("Effects", []) or []:
                gpu_modules.append({
                    "gpu_video_filter_match_name": e.get("GPUVideoFilter.MatchName"),
                    "module": m.get("ModuleName") or m.get("mFullPath"),
                })

    user_root = os.path.join(C.user_data_root(), "26.3")
    user_state = {}
    if os.path.isdir(user_root):
        for d in sorted(os.listdir(user_root)):
            full = os.path.join(user_root, d)
            if os.path.isdir(full):
                user_state[d] = len(os.listdir(full))

    method = {
        "app_launched": False,
        "tool": "_tools/greenroom/ae_render.py",
        "evidence": [
            {"label": "parsed", "path": reg_path,
             "what": "NUL-terminated JSON media registry with ExporterPlugins, "
                     "ImporterPlugins and AEVideoPlugins",
             "extraction": "widen from the anchor to the surrounding NUL "
                           "boundaries, then json.loads"},
            {"label": "parsed", "path": ext_path,
             "what": "After Effects importable file-extension table (mExt)"},
            {"label": "parsed", "path": "Support Files/**/*.dll|*.aex",
             "what": "$$$/MediaCore/Exporters/<Module>/<Param> codec parameter "
                     "labels and pipe-delimited option lists"},
            {"label": "parsed", "path": "Support Files/AfterFXLib.dll",
             "what": "embedded Adobe Eve layout sources for the Render "
                     "Settings / Output Module / Output To dialogs",
             "extraction": "brace-matched 'layout X { ... }' then dw_eve"},
            {"label": "inventory", "path": "Support Files/Plug-ins/Format/*.aex",
             "what": "shipped format plug-in binaries"},
        ],
        "failures_and_limits": [
            "Factory Render Settings templates and Output Module templates are "
            "materialised into the per-user preference files on first launch. "
            "%%APPDATA%%/Adobe/After Effects/26.3 subfolders are all empty "
            "(%s), because After Effects has never been run on this machine and "
            "this teardown does not launch it. Template NAMES are recovered "
            "from string evidence; their stored field values are not on disk."
            % user_state,
            "Exporter parameter labels are recovered as label + option strings. "
            "Numeric ranges and defaults for codec parameters are held by the "
            "exporter plug-ins themselves and are NOT declared in any on-disk "
            "manifest, so no ranges are asserted here.",
            "FileType integers are reported raw plus a four-character-code "
            "rendering where all four bytes are printable ASCII.",
        ],
        "counts": {
            "exporters": len(exporters),
            "importers": len(importers),
            "exporter_modules_with_parameter_strings": len(params),
            "exporter_parameter_strings": sum(len(v) for v in params.values()),
            "render_output_dialog_surfaces": len(dialogs),
            "render_output_dialog_controls": sum(d.get("control_count", 0)
                                                 for d in dialogs),
            "format_plugins": len(format_plugins),
            "gpu_video_filter_registrations": len(gpu_modules),
        },
    }

    payload = {
        "summary": {
            "exporters": len(exporters),
            "importers": len(importers),
            "distinct_output_extensions": sorted(
                {e["default_extension"] for e in exporters if e["default_extension"]}),
            "exporter_modules": sorted({e["module"] for e in exporters if e["module"]}),
            "render_output_dialog_surfaces": len(dialogs),
            "format_plugins": len(format_plugins),
        },
        "exporters": exporters,
        "importers": importers,
        "after_effects_importable_extensions": exts,
        "exporter_parameter_strings": params,
        "render_and_output_dialogs": dialogs,
        "template_name_strings": tmpl,
        "render_settings_vocabulary": egg_terms,
        "format_plugins": format_plugins,
        "gpu_video_filter_registrations": gpu_modules,
        "user_side_state": user_state,
    }
    C.write_json("aftereffects_render_output.json",
                 "handshake.studio.teardown.aftereffects.render_output",
                 method, payload)
    print("exporters=%d importers=%d param_modules=%d dialogs=%d fmt=%d"
          % (len(exporters), len(importers), len(params), len(dialogs),
             len(format_plugins)), file=sys.stderr)


if __name__ == "__main__":
    main()
