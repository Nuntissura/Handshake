"""pp_lumetri.py -- the complete Lumetri colour model, offline.

Streams:
  L1  install/Lumetri/**/*.irtp   IRIDAS shader definitions: the declared
                                  control surface (name, kind, min, max,
                                  default, enum items, gang, wheel mode) AND
                                  the IRIDAScript body that states exactly what
                                  the shader computes. This is the engine.
  L2  install/LocalizedPresets/en_US/Effect Presets/Lumetri Presets.prfpset
                                  325 shipped Lumetri Color instances. The
                                  first instance yields the effect's full
                                  98-parameter declaration with stable
                                  ParameterID, hard bounds and soft UI bounds.
                                  Each ArbVideoComponentParam carries a base64
                                  <Lumetri> document: the shader graph the look
                                  actually runs.
  L3  install/Lumetri/Looks/**/*.look   shipped look files: shader stack with
                                  per-parameter values.
  L4  LUT catalogue               install/Lumetri/LUTs, install/Resources/LUTs,
                                  install/CanonLUTs. Counts by folder and
                                  format; .cube headers parsed for grid size
                                  and domain.
  L5  colour management           install/OpenColorIO-Configs/*.ocio parsed as
                                  YAML into roles, displays, views, colour
                                  spaces and their transforms; install/icc;
                                  the working-colour-space JSON that sequence
                                  presets carry.
  L6  install/ColorStyles/FactoryPresets.prcolorstyle  shipped colour styles.
  L7  install/AME Lumetri System Presets.xml  the Media Encoder side.
  L8  executable string table     $$$/Shaders/, $$$/HSL/, $$$/Lumetri/ and
                                  $$$/Premiere/ColorSettings namespaces.
"""
import base64
import collections
import json
import os
import re
import sys
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import pp_common as C
import pp_iridas as I
import pp_effects as E

SCRATCH = os.environ.get("PP_SCRATCH") or os.path.join(HERE, "_cache")

LUT_EXTS = (".cube", ".look", ".lut", ".ilut", ".itx", ".3dl", ".csp", ".bin",
            ".vlt", ".mga", ".m3d", ".rv3dlut")


def decode_lumetri_blob(b64):
    """base64 -> the <Lumetri> shader-graph document, summarised."""
    try:
        raw = base64.b64decode(b64)
    except Exception as exc:                          # noqa: BLE001
        return {"decode_error": repr(exc)}
    try:
        txt = raw.decode("utf-8")
    except UnicodeDecodeError:
        return {"decoded_bytes": len(raw), "text": False}
    if "<Lumetri" not in txt and "<look" not in txt:
        return {"decoded_bytes": len(raw), "recognised": False,
                "head": txt[:200]}
    shaders = []
    for m in re.finditer(r"<shader>(.*?)</shader>", txt, re.S):
        body = m.group(1)
        nm = re.search(r"<name>\"?([^\"<]*)\"?</name>", body)
        cn = re.search(r"<customname>\"?([^\"<]*)\"?</customname>", body)
        params = {}
        pm = re.search(r"<parameters>(.*?)</parameters>", body, re.S)
        if pm:
            for p in re.finditer(r"<([\w.]+)>\"?([^\"<]*)\"?</\1>", pm.group(1)):
                v, kind = I.decode_look_value(p.group(2))
                params[p.group(1)] = v
        shaders.append({"name": nm.group(1) if nm else None,
                        "custom_name": cn.group(1) if cn else None,
                        "parameter_count": len(params),
                        "parameters": params})
    guid = re.search(r"<guid>\"?([^\"<]*)\"?</guid>", txt)
    return {
        "decoded_bytes": len(raw),
        "document": "Lumetri shader graph",
        "guid": guid.group(1) if guid else None,
        "shader_count": len(shaders),
        "shaders": shaders,
    }


def parse_ocio(path):
    import yaml

    class L(yaml.SafeLoader):
        pass

    def keep(loader, tag_suffix, node):
        if isinstance(node, yaml.MappingNode):
            d = loader.construct_mapping(node, deep=True)
        elif isinstance(node, yaml.SequenceNode):
            d = {"items": loader.construct_sequence(node, deep=True)}
        else:
            d = {"value": loader.construct_scalar(node)}
        d["$ocio_type"] = tag_suffix.lstrip("!<").rstrip(">")
        return d

    # OCIO writes verbatim tags, "!<ColorSpace>", which PyYAML resolves to the
    # bare tag "ColorSpace" -- so the catch-all must be registered on "".
    L.add_multi_constructor("", keep)
    L.add_multi_constructor("!", keep)
    L.add_multi_constructor("tag:yaml.org,2002:", keep)
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        cfg = yaml.load(fh, Loader=L)

    def tf_summary(t, depth=0):
        if depth > 6:
            return {"$truncated": True}
        if isinstance(t, dict):
            out = {"type": t.get("$ocio_type")}
            for k, v in t.items():
                if k == "$ocio_type":
                    continue
                if isinstance(v, (dict, list)):
                    out[k] = tf_summary(v, depth + 1)
                else:
                    out[k] = v
            return out
        if isinstance(t, list):
            return [tf_summary(x, depth + 1) for x in t]
        return t

    spaces = []
    for cs in (cfg.get("colorspaces") or []):
        spaces.append({
            "name": cs.get("name"),
            "family": cs.get("family"),
            "equality_group": cs.get("equalitygroup"),
            "bit_depth": cs.get("bitdepth"),
            "is_data": cs.get("isdata"),
            "allocation": cs.get("allocation"),
            "allocation_vars": cs.get("allocationvars"),
            "encoding": cs.get("encoding"),
            "categories": cs.get("categories"),
            "description": (cs.get("description") or "").strip() or None,
            "to_reference": tf_summary(cs.get("to_reference") or cs.get("to_scene_reference")),
            "from_reference": tf_summary(cs.get("from_reference") or cs.get("from_scene_reference")),
        })
    displays = {}
    for dname, views in (cfg.get("displays") or {}).items():
        rows = []
        for v in (views or []):
            if isinstance(v, dict):
                rows.append({"name": v.get("name"),
                             "colorspace": v.get("colorspace"),
                             "view_transform": v.get("view_transform"),
                             "display_colorspace": v.get("display_colorspace"),
                             "looks": v.get("looks")})
        displays[dname] = rows
    looks = []
    for lk in (cfg.get("looks") or []):
        looks.append({"name": lk.get("name"), "process_space": lk.get("process_space"),
                      "transform": tf_summary(lk.get("transform"))})
    vts = []
    for vt in (cfg.get("view_transforms") or []):
        vts.append({"name": vt.get("name"),
                    "from_scene_reference": tf_summary(vt.get("from_scene_reference")),
                    "to_scene_reference": tf_summary(vt.get("to_scene_reference"))})
    return {
        "file": C.rel(path),
        "ocio_profile_version": cfg.get("ocio_profile_version"),
        "name": cfg.get("name"),
        "description": (cfg.get("description") or "").strip() or None,
        "search_path": cfg.get("search_path"),
        "luma_coefficients": cfg.get("luma"),
        "roles": cfg.get("roles") or {},
        "active_displays": cfg.get("active_displays"),
        "active_views": cfg.get("active_views"),
        "displays": displays,
        "display_count": len(displays),
        "view_count": sum(len(v) for v in displays.values()),
        "view_transforms": vts,
        "looks": looks,
        "colorspace_count": len(spaces),
        "colorspaces": spaces,
        "display_colorspaces": [
            {"name": cs.get("name"), "family": cs.get("family"),
             "from_display_reference": tf_summary(cs.get("from_display_reference")),
             "to_display_reference": tf_summary(cs.get("to_display_reference"))}
            for cs in (cfg.get("display_colorspaces") or [])],
    }


def main(out_dir):
    R = C.PREMIERE_ROOT
    sources = []
    failures = []
    table = C.premiere_strings(SCRATCH)

    # ---- L1 shaders
    shaders = []
    for p in sorted(C.walk_files(os.path.join(R, "Lumetri"), exts=(".irtp",))):
        try:
            s = I.parse_irtp(p)
        except Exception as exc:                      # noqa: BLE001
            failures.append({"stage": "L1_irtp", "path": C.rel(p),
                             "error": repr(exc),
                             "traceback": traceback.format_exc()})
            continue
        s["file"] = C.rel(p)
        s["group"] = C.rel(p).split("/")[1] if "/" in C.rel(p) else None
        shaders.append(s)
    sources.append({
        "id": "L1_irtp", "path": "Lumetri/**/*.irtp",
        "how": ("IRIDAS shader grammar parsed with a purpose-written tokenizer "
                "(pp_iridas.py); the IRIDAScript body is captured verbatim and "
                "never executed"),
        "shaders_parsed": len(shaders),
        "controls_parsed": sum(s["control_count"] for s in shaders),
        "iridascript_lines": sum(s["iridascript_lines"] for s in shaders)})

    # ---- L2 the Lumetri Color effect's own parameter declaration
    lum_prfp = os.path.join(R, "LocalizedPresets", "en_US", "Effect Presets",
                            "Lumetri Presets.prfpset")
    lumetri_effect = None
    look_presets = []
    graph_samples = []
    try:
        presets, st = E.parse_prfpset(lum_prfp)
        objects, _root = C.parse_premiere_data(lum_prfp)
        # first component declares the full parameter surface
        for pr in presets:
            for comp in pr["components"]:
                if lumetri_effect is None and comp["parameters"]:
                    lumetri_effect = {
                        "match_name": comp["match_name"],
                        "display_name": comp["display_name"],
                        "declared_parameter_count": len(comp["parameters"]),
                        "parameters": comp["parameters"],
                    }
                break
            look_presets.append({
                "preset_name": pr["preset_name"],
                "bin_path": pr["bin_path"],
                "description": pr["description"],
                "non_default_parameters": {
                    p["name"]: p.get("effective_value")
                    for comp in pr["components"] for p in comp["parameters"]
                    if p.get("name") and p.get("effective_value") not in (None, 0, "0")
                    and not p.get("arbitrary_data")},
            })
        # decode a bounded sample of the shader-graph blobs
        for oid, el in list(objects.items()):
            if C._strip_ns(el.tag) != "ArbVideoComponentParam":
                continue
            blob = el.find("StartKeyframeValue")
            if blob is None or not (blob.text or "").strip():
                continue
            dec = decode_lumetri_blob(blob.text.strip())
            if dec.get("shader_count"):
                graph_samples.append({"object_id": oid, **dec})
            if len(graph_samples) >= 6:
                break
        sources.append({"id": "L2_lumetri_prfpset", "path": C.rel(lum_prfp),
                        "how": ("PremiereData object graph; ArbVideoComponentParam "
                                "StartKeyframeValue base64 decoded to the "
                                "<Lumetri> shader-graph document"),
                        "shipped_look_presets": len(look_presets),
                        "objects_indexed": st["objects_indexed"],
                        "shader_graphs_decoded": len(graph_samples)})
    except Exception as exc:                          # noqa: BLE001
        failures.append({"stage": "L2_lumetri_prfpset", "path": C.rel(lum_prfp),
                         "error": repr(exc), "traceback": traceback.format_exc()})

    # group the 98 parameters into their UI sections using the group headers
    sections = []
    if lumetri_effect:
        cur = None
        for p in sorted(lumetri_effect["parameters"],
                        key=lambda x: (x["declared_index"] is None,
                                       x["declared_index"] or 0)):
            code = p.get("control_type_code")
            if code == "11":
                cur = {"section": p.get("name"),
                       "parameter_id": p.get("parameter_id"),
                       "declared_index": p.get("declared_index"),
                       "parameters": []}
                sections.append(cur)
                continue
            if code == "12":
                cur = None
                continue
            row = {k: p[k] for k in
                   ("name", "declared_index", "parameter_id",
                    "control_type_code", "control_type_label", "param_kind",
                    "min", "max", "ui_min", "ui_max", "units",
                    "arbitrary_data") if k in p}
            if cur is None:
                cur = {"section": "(ungrouped)", "parameters": []}
                sections.append(cur)
            cur["parameters"].append(row)
        lumetri_effect["sections"] = sections
        lumetri_effect["section_count"] = len(sections)

    # ---- L3 looks
    looks = []
    for p in sorted(C.walk_files(os.path.join(R, "Lumetri", "Looks"), exts=(".look",))):
        try:
            lk = I.parse_look(p)
        except Exception as exc:                      # noqa: BLE001
            failures.append({"stage": "L3_look", "path": C.rel(p),
                             "error": repr(exc)})
            continue
        lk["file"] = C.rel(p)
        looks.append(lk)
    sources.append({"id": "L3_look", "path": "Lumetri/Looks/**/*.look",
                    "how": "XML shader stack; values decoded with the D/N prefix rule",
                    "looks_parsed": len(looks)})

    # ---- L4 LUT catalogue
    lut_roots = [("Lumetri/LUTs", os.path.join(R, "Lumetri", "LUTs")),
                 ("Lumetri/Film", os.path.join(R, "Lumetri", "Film")),
                 ("Resources/LUTs", os.path.join(R, "Resources", "LUTs")),
                 ("CanonLUTs", os.path.join(R, "CanonLUTs"))]
    lut_catalogue = []
    lut_totals = collections.Counter()
    for label, root in lut_roots:
        if not os.path.isdir(root):
            continue
        for p in sorted(C.walk_files(root, exts=LUT_EXTS)):
            ext = os.path.splitext(p)[1].lower()
            rec = {"name": os.path.splitext(os.path.basename(p))[0],
                   "root": label,
                   "folder": os.path.dirname(C.rel(p)),
                   "format": ext.lstrip("."),
                   "bytes": os.path.getsize(p)}
            if ext == ".cube":
                rec.update(I.parse_cube_header(p))
            lut_catalogue.append(rec)
            lut_totals["%s|%s" % (label, ext.lstrip("."))] += 1
    by_folder = collections.Counter(r["folder"] for r in lut_catalogue)
    by_format = collections.Counter(r["format"] for r in lut_catalogue)
    cube_sizes = collections.Counter(
        str(r.get("size")) for r in lut_catalogue if r.get("size"))
    sources.append({"id": "L4_luts",
                    "roots": [r[0] for r in lut_roots],
                    "how": ("directory walk over LUT extensions; .cube headers "
                            "parsed for TITLE / LUT_3D_SIZE / DOMAIN_MIN / "
                            "DOMAIN_MAX"),
                    "lut_files": len(lut_catalogue)})

    # ---- L5 colour management
    ocio = []
    for p in sorted(C.walk_files(os.path.join(R, "OpenColorIO-Configs"),
                                 exts=(".ocio",))):
        try:
            ocio.append(parse_ocio(p))
        except Exception as exc:                      # noqa: BLE001
            failures.append({"stage": "L5_ocio", "path": C.rel(p),
                             "error": repr(exc),
                             "traceback": traceback.format_exc()})
    icc = [{"file": C.rel(p), "bytes": os.path.getsize(p)}
           for p in sorted(C.walk_files(os.path.join(R, "icc")))]
    sources.append({"id": "L5_colour_management",
                    "how": "OCIO configs loaded as YAML with !<Tag> nodes preserved",
                    "ocio_configs": len(ocio),
                    "icc_profiles": len(icc)})

    # working colour spaces actually referenced by shipped sequence presets
    working_spaces = collections.Counter()
    tonemap = collections.Counter()
    for p in C.walk_files(os.path.join(R, "Settings", "SequencePresets"),
                          exts=(".sqpreset",)):
        try:
            with open(p, "r", encoding="utf-8", errors="replace") as fh:
                txt = fh.read()
        except OSError:
            continue
        for m in re.finditer(r"<SequenceWorkingColorSpace>(.*?)</SequenceWorkingColorSpace>", txt, re.S):
            try:
                j = json.loads(m.group(1))
                working_spaces[j.get("workingSpaceID")] += 1
            except Exception:                          # noqa: BLE001
                pass
        for m in re.finditer(r"<AutoToneMapEnabled>(\w+)</AutoToneMapEnabled>", txt):
            tonemap[m.group(1)] += 1

    # ---- L6 colour styles
    color_styles = None
    cs_path = os.path.join(R, "ColorStyles", "FactoryPresets.prcolorstyle")
    if os.path.isfile(cs_path):
        try:
            objs, _r = C.parse_premiere_data(cs_path)
            names = []
            for oid, el in objs.items():
                base = el.find("TreeItemBase")
                if base is None:
                    continue
                n = base.find("Name")
                if n is None or not (n.text or "").strip():
                    continue
                key, txt = C.split_localized((n.text or "").strip().strip('"'))
                names.append({"name": txt, "string_key": key,
                              "object_kind": C._strip_ns(el.tag)})
            color_styles = {"file": C.rel(cs_path),
                            "objects_indexed": len(objs),
                            "entries": names,
                            "entry_count": len(names)}
        except Exception as exc:                      # noqa: BLE001
            failures.append({"stage": "L6_color_styles", "path": C.rel(cs_path),
                             "error": repr(exc)})
    sources.append({"id": "L6_color_styles", "path": C.rel(cs_path),
                    "how": "PremiereData tree walk",
                    "entries": color_styles["entry_count"] if color_styles else 0})

    # ---- L7 AME Lumetri system presets
    ame_lumetri = None
    ame_path = os.path.join(R, "AME Lumetri System Presets.xml")
    if os.path.isfile(ame_path):
        try:
            presets, st = E.parse_prfpset(ame_path)
            ame_lumetri = {
                "file": C.rel(ame_path),
                "objects_indexed": st["objects_indexed"],
                "preset_count": len(presets),
                "presets": [{"preset_name": p["preset_name"],
                             "bin_path": p["bin_path"],
                             "match_name": p["match_name"],
                             "description": p["description"]}
                            for p in presets],
            }
        except Exception as exc:                      # noqa: BLE001
            failures.append({"stage": "L7_ame_lumetri", "path": C.rel(ame_path),
                             "error": repr(exc),
                             "traceback": traceback.format_exc()})
    sources.append({"id": "L7_ame_lumetri", "path": C.rel(ame_path),
                    "how": "PremiereData object graph, same reader as the effect presets",
                    "presets": ame_lumetri["preset_count"] if ame_lumetri else 0})

    # ---- L8 string namespaces
    ns_dump = {}
    for prefix, label in (("$$$/Shaders/", "shader control labels"),
                          ("$$$/HSL/", "HSL Secondary panel"),
                          ("$$$/Lumetri/", "Lumetri panel"),
                          ("$$$/MediaCore/ColorManagement/", "colour management"),
                          ("$$$/Premiere/ColorSettings", "colour settings"),
                          ("$$$/dvamediatypes/Color", "colour media types")):
        rows = {k: v for k, v in table.items() if k.startswith(prefix)}
        if rows:
            ns_dump[prefix] = {"purpose": label, "count": len(rows),
                               "strings": dict(sorted(rows.items()))}
    sources.append({"id": "L8_exe_strings",
                    "how": "namespace slice of the executable's $$$ literals",
                    "namespaces": len(ns_dump),
                    "strings": sum(v["count"] for v in ns_dump.values())})

    payload = C.envelope(
        "handshake.studio.premiere.lumetri_color.v1",
        {
            "summary": ("The Lumetri colour grading model: the effect's declared "
                        "98-parameter surface with hard and soft bounds, the "
                        "IRIDAS shader engine behind it including the per-pixel "
                        "maths, the shipped look and LUT catalogue, and the "
                        "colour-management configuration."),
            "streams": {
                "L1_irtp": "IRIDAS shader definitions and IRIDAScript source",
                "L2_lumetri_prfpset": "Lumetri Color declared parameters and 325 shipped looks",
                "L3_look": "shipped .look shader stacks with values",
                "L4_luts": "LUT and look file catalogue with .cube headers",
                "L5_colour_management": "OCIO configs, ICC profiles, working spaces",
                "L6_color_styles": "shipped colour styles",
                "L7_ame_lumetri": "Media Encoder Lumetri system presets",
                "L8_exe_strings": "shader / HSL / colour-management string namespaces",
            },
            "reading_the_parameter_surface": (
                "lumetri_color_effect.sections reconstructs the panel layout: a "
                "parameter with control_type_code 11 opens a section and code 12 "
                "closes it; that grouping is derived from the serialization order "
                "and is marked heuristic. Every min/max/ui_min/ui_max/parameter_id "
                "value is read verbatim from the shipped preset."),
            "known_gaps": [
                ("Parameters whose ParameterID is -1 are present in the "
                 "serialization but carry no stable identifier; they are HDR and "
                 "HSL Secondary controls that Premiere addresses positionally."),
                ("Shader-graph blobs are decoded for a bounded sample of six "
                 "objects rather than all 2600, to keep the artifact usable; the "
                 "decoder is deterministic and can be re-run over the rest."),
            ],
        },
        sources,
        {
            "extraction_summary": {
                "shaders_parsed": len(shaders),
                "shader_controls": sum(s["control_count"] for s in shaders),
                "iridascript_lines_captured": sum(s["iridascript_lines"] for s in shaders),
                "lumetri_declared_parameters": (lumetri_effect or {}).get("declared_parameter_count", 0),
                "lumetri_ui_sections": len(sections),
                "shipped_look_presets": len(look_presets),
                "look_files_parsed": len(looks),
                "lut_files": len(lut_catalogue),
                "ocio_configs": len(ocio),
                "ocio_colorspaces_total": sum(o["colorspace_count"] for o in ocio),
                "ocio_views_total": sum(o["view_count"] for o in ocio),
                "colour_styles": color_styles["entry_count"] if color_styles else 0,
                "ame_lumetri_presets": ame_lumetri["preset_count"] if ame_lumetri else 0,
                "count_semantics": ("lut_files counts LUT files because a LUT IS a "
                                    "file; every other number counts entities, not files"),
            },
            "lumetri_color_effect": lumetri_effect,
            "shader_engine": {
                "note": ("Each entry is one IRIDAS shader. controls[] is the "
                         "declared parameter surface; iridascript is the shipped "
                         "source of the transform, captured verbatim."),
                "shaders": shaders,
            },
            "shader_graph_samples": graph_samples,
            "shipped_look_presets": look_presets,
            "look_files": looks,
            "lut_catalogue": {
                "totals_by_format": dict(by_format),
                "totals_by_folder": dict(by_folder),
                "cube_grid_sizes": dict(cube_sizes),
                "files": lut_catalogue,
            },
            "colour_management": {
                "ocio_configs": ocio,
                "icc_profiles": icc,
                "working_colour_spaces_used_by_shipped_sequence_presets":
                    dict(working_spaces),
                "auto_tone_map_default_in_sequence_presets": dict(tonemap),
                "canon_lut_binaries": sum(
                    1 for r in lut_catalogue if r["root"] == "CanonLUTs"),
            },
            "colour_styles": color_styles,
            "ame_lumetri_system_presets": ame_lumetri,
            "string_namespaces": ns_dump,
            "failures": failures,
        })

    path, size = C.write_json(out_dir, "premiere_lumetri_color.json", payload)
    print("wrote", path, size, "bytes")
    print("shaders", len(shaders), "lumetri params",
          (lumetri_effect or {}).get("declared_parameter_count"),
          "looks", len(look_presets), "luts", len(lut_catalogue),
          "ocio", len(ocio), "failures", len(failures))
    return payload


if __name__ == "__main__":
    main(sys.argv[1])
