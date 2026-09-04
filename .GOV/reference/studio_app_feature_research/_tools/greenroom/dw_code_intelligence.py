"""dw_code_intelligence.py -- Task 4: document types, tag libraries, code hints,
code colouring, validators, linters, formatters.

Sources actually read (all offline):
  Configuration/DocumentTypes/MMDocumentTypes.xml            -- supported document types
  Configuration/DocumentTypes/MMDocumentTypeDeclarations.xml  -- shipped DOCTYPE strings
  Configuration/DocumentTypes/MMMimeTypes.xml                 -- extension -> mime map
  Configuration/DocumentTypes/NewDocuments/*                  -- the blank file each type starts from
  Configuration/TagLibraries/TagLibraries.vtm                 -- the tag library index
  Configuration/TagLibraries/**/*.vtm                         -- every tag: its attributes,
                                                                 attribute types and the
                                                                 enumerated values allowed
  Configuration/TagLibraries/CSS/properties.xml               -- CSS property/value vocabulary
  Configuration/TagLibraries/Validator/*.vtv                  -- validator rule sets
  Configuration/TagLibraries/CrossTagAttr/**                  -- cross-tag attribute sets
  Configuration/CodeHints/*.xml                               -- code hint menus and functions
  Configuration/CodeColoring/*.xml                            -- per-language colouring schemes
  Configuration/themes/*.xml                                  -- shipped code themes
  Configuration/LinterRuleSets/*                              -- shipped linter configs
  Configuration/Validators/*, Configuration/Formatters/*      -- validator/format menus
  Configuration/ESLintrc                                      -- shipped ESLint config
"""
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import dw_common as C                                       # noqa: E402
from dw_zstrings import load_all_strings, resolve           # noqa: E402


def node_dict(el, resolver):
    a = C.attrs_of(el)
    rec = {"node": el.tag.split("}")[-1], "attributes": a}
    for k in list(a):
        if k.startswith("mmstring:"):
            v, how = resolver(a[k])
            rec.setdefault("resolved", {})[k] = {"key": a[k], "value": v, "resolution": how}
    txt = (el.text or "").strip()
    if txt:
        rec["text"] = txt
    kids = [node_dict(c, resolver) for c in list(el)]
    if kids:
        rec["children"] = kids
    return rec


def build(out_path):
    exact, lower, smeta = load_all_strings(C.INSTALL_ROOT)
    failures = []

    def R(key):
        return resolve(key, exact, lower)

    def load(path, stage):
        r, note = C.parse_xml_tolerant(path)
        if r is None:
            failures.append({"stage": stage, "path": C.rel(path), "error": note})
        return r, note

    # ---------------- document types ---------------------------------------
    dt_dir = os.path.join(C.CONFIG, "DocumentTypes")
    doc_types = []
    r, _ = load(os.path.join(dt_dir, "MMDocumentTypes.xml"), "MMDocumentTypes")
    if r is not None:
        for el in r.iter():
            if el.tag.split("}")[-1] != "documenttype":
                continue
            a = C.attrs_of(el)
            title = desc = None
            for c in el.iter():
                t = c.tag.split("}")[-1].lower()
                if t == "loadstring":
                    continue
                if t in ("title", "description"):
                    key = None
                    for gc in c.iter():
                        if gc.tag.split("}")[-1].lower() == "loadstring":
                            key = C.attrs_of(gc).get("id")
                    val = R(key)[0] if key else (c.text or "").strip() or None
                    if t == "title":
                        title = val
                    else:
                        desc = val
            starter = a.get("file")
            starter_path = os.path.join(dt_dir, "NewDocuments", starter) if starter else None
            doc_types.append({
                "id": a.get("id"),
                "title": title,
                "description": desc,
                "internal_type": a.get("internaltype"),
                "windows_file_extensions": [e for e in
                                            (a.get("winfileextension") or "").split(",") if e],
                "mac_file_extensions": [e for e in
                                        (a.get("macfileextension") or "").split(",") if e],
                "mime_type": a.get("mimetype"),
                "writes_byte_order_mark": a.get("writebyteordermark"),
                "dtd_context": next(((c.text or "").strip() for c in el.iter()
                                     if c.tag.split("}")[-1] == "dtdcontext"), None),
                "server_model": a.get("servermodel"),
                "previewfile": a.get("previewfile"),
                "starter_file": starter,
                "starter_file_exists": bool(starter_path and os.path.isfile(starter_path)),
                "starter_file_content": (C.read_text(starter_path)
                                         if starter_path and os.path.isfile(starter_path)
                                         and os.path.getsize(starter_path) < 20000 else None),
                "all_attributes": a,
                "provenance": "parsed",
            })

    doctype_decls = []
    r, _ = load(os.path.join(dt_dir, "MMDocumentTypeDeclarations.xml"), "MMDocumentTypeDeclarations")
    if r is not None:
        for el in r.iter():
            if el.tag.split("}")[-1] != "documenttypedeclaration":
                continue
            a = C.attrs_of(el)
            rec = {"id": a.get("id"), "attributes": a}
            for c in el.iter():
                t = c.tag.split("}")[-1].lower()
                if t in ("title", "doctype", "dtd", "xmlns", "description"):
                    rec[t] = (c.text or "").strip() or C.attrs_of(c)
            doctype_decls.append(rec)

    mime_types = []
    r, _ = load(os.path.join(dt_dir, "MMMimeTypes.xml"), "MMMimeTypes")
    if r is not None:
        for el in r.iter():
            if el is r:
                continue
            mime_types.append(C.attrs_of(el))

    # ---------------- tag libraries -----------------------------------------
    tl_dir = os.path.join(C.CONFIG, "TagLibraries")
    libraries = []
    r, _ = load(os.path.join(tl_dir, "TagLibraries.vtm"), "TagLibraries.vtm")
    tagrefs = {}
    if r is not None:
        for el in r.iter():
            if el.tag.split("}")[-1] != "taglibrary":
                continue
            a = C.attrs_of(el)
            name, how = R(a.get("mmstring:NAME") or a.get("mmstring:name") or "")
            refs = [C.attrs_of(c) for c in el.iter()
                    if c.tag.split("}")[-1] == "tagref"]
            libraries.append({
                "library_id": a.get("id"),
                "library_name": name,
                "library_name_resolution": how,
                "applies_to_document_types": [d for d in (a.get("doctypes") or "").split(",") if d],
                "tag_chooser": a.get("tagchooser"),
                "prefix": a.get("prefix"),
                "tag_count_declared": len(refs),
                "tag_refs": refs,
                "all_attributes": a,
                "provenance": "parsed",
            })
            for ref in refs:
                if ref.get("file"):
                    tagrefs[os.path.normpath(os.path.join(tl_dir,
                                                          ref["file"].replace("/", os.sep))).lower()] = \
                        (a.get("id"), ref.get("name"))

    tags = []
    attr_type_tally = {}
    enum_value_total = 0
    for p in sorted(C.walk(tl_dir, exts={".vtm"})):
        if os.path.basename(p).lower() == "taglibraries.vtm":
            continue
        # CrossTagAttr files invert the structure: they declare an attribute
        # group and then list the tags it applies to. Parsing them as tag
        # definitions would produce hundreds of bogus zero-attribute rows, so
        # they are handled separately below.
        if os.sep + "CrossTagAttr" + os.sep in p:
            continue
        root, note = C.parse_xml_tolerant(p)
        if root is None:
            failures.append({"stage": "vtm", "path": C.rel(p), "error": note})
            continue
        lib_id, declared_name = tagrefs.get(p.lower(), (None, None))
        for el in root.iter():
            if el.tag.split("}")[-1].upper() != "TAG":
                continue
            a = C.attrs_of(el)
            attribs = []
            for at in el.iter():
                if at.tag.split("}")[-1].lower() != "attrib":
                    continue
                aa = C.attrs_of(at)
                opts = [C.attrs_of(o) for o in at.iter()
                        if o.tag.split("}")[-1].lower() == "attriboption"]
                t = (aa.get("type") or "").lower() or "unspecified"
                attr_type_tally[t] = attr_type_tally.get(t, 0) + 1
                enum_value_total += len(opts)
                attribs.append({
                    "name": aa.get("name"),
                    "value_type": aa.get("type"),
                    "casesensitive": aa.get("casesensitive"),
                    "allowed_values": [{"value": o.get("value"), "caption": o.get("caption")}
                                       for o in opts] or None,
                    "all_attributes": aa,
                })
            fmt = next((C.attrs_of(f) for f in el.iter()
                        if f.tag.split("}")[-1].lower() == "tagformat"), None)
            dlg = next((C.attrs_of(f) for f in el.iter()
                        if f.tag.split("}")[-1].lower() == "tagdialog"), None)
            tags.append({
                "tag_name": a.get("name"),
                "library_id": lib_id,
                "declared_in_index_as": declared_name,
                "file": C.rel(p),
                "has_end_tag": a.get("endtag"),
                "bind_attribute": a.get("BIND") or a.get("bind"),
                "tag_type": a.get("tagtype"),
                "formatting_rules": fmt,
                "property_inspector_dialog": (dlg or {}).get("file"),
                "attribute_count": len(attribs),
                "attributes": attribs,
                "all_tag_attributes": a,
                "provenance": "parsed",
            })

    # CSS property vocabulary shipped with the tag libraries
    css_props = None
    cssp = os.path.join(tl_dir, "CSS", "properties.xml")
    if os.path.isfile(cssp):
        r, note = load(cssp, "css_properties")
        if r is not None:
            entries = []
            for el in r.iter():
                if el is r:
                    continue
                a = C.attrs_of(el)
                vals = [C.attrs_of(v) for v in list(el)]
                entries.append({"node": el.tag.split("}")[-1], "attributes": a,
                                "values": vals or None})
            css_props = {"file": C.rel(cssp), "entry_count": len(entries),
                         "entries": entries, "provenance": "parsed"}

    cross_tag = []
    cross_tag_groups = 0
    cross_tag_attrs = 0
    ctdir = os.path.join(tl_dir, "CrossTagAttr")
    for p in sorted(C.walk(ctdir)):
        if os.path.splitext(p)[1].lower() not in (".vtm", ".xml"):
            continue
        r, note = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "crosstagattr", "path": C.rel(p), "error": note})
            continue
        groups = []
        for g in r.iter():
            if g.tag.split("}")[-1] != "attribgroup":
                continue
            ga = C.attrs_of(g)
            attrs, applies = [], []
            for c in g.iter():
                t = c.tag.split("}")[-1].lower()
                ca = C.attrs_of(c)
                if t == "attrib":
                    opts = [C.attrs_of(o) for o in c.iter()
                            if o.tag.split("}")[-1].lower() == "attriboption"]
                    attrs.append({"name": ca.get("name"),
                                  "value_type": ca.get("type"),
                                  "allowed_values": [{"value": o.get("value"),
                                                      "caption": o.get("caption")}
                                                     for o in opts] or None,
                                  "all_attributes": ca})
                elif t == "tag":
                    applies.append(ca.get("name"))
            groups.append({"group_id": ga.get("id"), "group_name": ga.get("name"),
                           "attribute_count": len(attrs),
                           "attributes": attrs,
                           "applies_to_tag_count": len(applies),
                           "applies_to_tags": applies,
                           "all_attributes": ga})
            cross_tag_groups += 1
            cross_tag_attrs += len(attrs)
        cross_tag.append({"file": C.rel(p),
                          "attribute_group_count": len(groups),
                          "attribute_groups": groups,
                          "provenance": "parsed"})

    # ---------------- validators (.vtv) -------------------------------------
    validators = []
    vdir = os.path.join(tl_dir, "Validator")
    for p in sorted(C.walk(vdir)):
        r, note = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "vtv", "path": C.rel(p), "error": note})
            continue
        counts = {}
        for el in r.iter():
            t = el.tag.split("}")[-1]
            counts[t] = counts.get(t, 0) + 1
        validators.append({
            "file": C.rel(p),
            "rule_set": os.path.splitext(os.path.basename(p))[0],
            "node_counts": counts,
            "tree": node_dict(r, R),
            "provenance": "parsed",
        })

    # ---------------- code hints --------------------------------------------
    code_hints = []
    ch_dir = os.path.join(C.CONFIG, "CodeHints")
    for p in sorted(C.walk(ch_dir, exts={".xml"})):
        r, note = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "codehints", "path": C.rel(p), "error": note})
            continue
        groups = []
        for el in r.iter():
            if el.tag.split("}")[-1] != "menugroup":
                continue
            a = C.attrs_of(el)
            gname, how = R(a.get("mmstring:name") or "")
            items = []
            for c in el.iter():
                t = c.tag.split("}")[-1]
                if t in ("function", "menu", "object", "property", "method"):
                    ca = C.attrs_of(c)
                    entry = {"item_kind": t, "attributes": ca}
                    subs = [C.attrs_of(s) for s in list(c)
                            if s.tag.split("}")[-1] in ("menuitem", "item")]
                    if subs:
                        entry["items"] = subs
                    items.append(entry)
            groups.append({
                "group_id": a.get("id"),
                "group_label": gname,
                "group_label_resolution": how,
                "enabled_by_default": a.get("enabled"),
                "pattern": a.get("pattern"),
                "item_count": len(items),
                "items": items,
                "all_attributes": a,
            })
        code_hints.append({"file": C.rel(p), "menugroup_count": len(groups),
                           "menugroups": groups, "provenance": "parsed"})

    hint_descriptions = sorted(C.rel(p) for p in
                               C.walk(os.path.join(ch_dir, "Descriptions")))
    hint_builtin = sorted(C.rel(p) for p in
                          C.walk(os.path.join(ch_dir, "BuiltinCode")))
    hint_cms = sorted(C.rel(p) for p in C.walk(os.path.join(ch_dir, "CMS")))

    # ---------------- code colouring ----------------------------------------
    schemes = []
    cc_dir = os.path.join(C.CONFIG, "CodeColoring")
    for p in sorted(C.walk(cc_dir, exts={".xml"})):
        r, note = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "codecoloring", "path": C.rel(p), "error": note})
            continue
        if os.path.basename(p).lower() == "colors.xml":
            continue
        for el in r.iter():
            if el.tag.split("}")[-1] != "scheme":
                continue
            a = C.attrs_of(el)
            nm, how = R(a.get("mmstring:name") or "")
            kwlists, tagspecs, strings, blocks, sample = [], [], [], [], None
            for c in el.iter():
                t = c.tag.split("}")[-1]
                ca = C.attrs_of(c)
                ct = (c.text or "").strip()
                if t == "keywords":
                    kws = [(k.text or "").strip() for k in list(c)
                           if k.tag.split("}")[-1] == "keyword"]
                    lbl, _ = R(ca.get("mmstring:name") or "")
                    kwlists.append({"id": ca.get("id"), "label": lbl,
                                    "keyword_count": len(kws), "keywords": kws,
                                    "attributes": ca})
                elif t in ("tagGroup", "tagspec"):
                    tagspecs.append({"node": t, "attributes": ca, "text": ct or None})
                elif t in ("stringStart", "stringEnd", "stringEsc",
                           "charStart", "charEnd", "charEsc",
                           "commentStart", "commentEnd", "commentDelimiter"):
                    strings.append({"node": t, "delimiter": ct, "attributes": ca})
                elif t in ("blockStart", "blockEnd", "scriptStart", "scriptEnd",
                           "cssStart", "cssEnd"):
                    blocks.append({"node": t, "delimiter": ct, "attributes": ca})
                elif t == "sampleText":
                    sample = ct
            schemes.append({
                "scheme_id": a.get("id"),
                "scheme_name": nm,
                "scheme_name_resolution": how,
                "applies_to_document_types": [d for d in (a.get("doctypes") or "").split(",") if d],
                "priority": a.get("priority"),
                "ignore_case": next(((c.text or "").strip() for c in el.iter()
                                     if c.tag.split("}")[-1] == "ignoreCase"), None),
                "ignore_tags": next(((c.text or "").strip() for c in el.iter()
                                     if c.tag.split("}")[-1] == "ignoreTags"), None),
                "keyword_lists": kwlists,
                "keyword_total": sum(k["keyword_count"] for k in kwlists),
                "delimiters": strings,
                "embedded_block_delimiters": blocks,
                "tag_specs": tagspecs,
                "sample_text": sample,
                "file": C.rel(p),
                "all_attributes": a,
                "provenance": "parsed",
            })

    colors = []
    cpath = os.path.join(cc_dir, "Colors.xml")
    if os.path.isfile(cpath):
        r, note = load(cpath, "Colors.xml")
        if r is not None:
            for el in r.iter():
                if el.tag.split("}")[-1] == "syntaxColor":
                    colors.append(C.attrs_of(el))

    themes = []
    th_dir = os.path.join(C.CONFIG, "themes")
    for p in sorted(C.walk(th_dir, exts={".xml"})):
        r, note = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "theme", "path": C.rel(p), "error": note})
            continue
        entries = [C.attrs_of(el) for el in r.iter() if el is not r]
        stem = os.path.splitext(os.path.basename(p))[0]
        if stem.lower() == "themes":
            kind = "theme index"
        elif stem.lower().startswith("defaultcolors"):
            kind = "default colour palette for theme '%s'" % stem[13:]
        else:
            kind = "code theme definition"
        themes.append({"theme_name": stem, "theme_file_kind": kind,
                       "file": C.rel(p), "root_attributes": C.attrs_of(r),
                       "entry_count": len(entries), "entries": entries,
                       "provenance": "parsed"})

    # ---------------- linters, validators, formatters -----------------------
    linters = []
    for p in sorted(C.walk(os.path.join(C.CONFIG, "LinterRuleSets"))):
        txt = C.read_text(p)
        parsed = None
        try:
            parsed = json.loads(re.sub(r"/\*.*?\*/", "", txt, flags=re.S))
        except Exception:                                    # noqa: BLE001
            pass
        linters.append({
            "file": C.rel(p),
            "language": os.path.splitext(os.path.basename(p))[0],
            "config_format": os.path.splitext(p)[1].lstrip(".") or "unknown",
            "rules": parsed,
            "rule_count": len(parsed) if isinstance(parsed, dict) else None,
            "raw": None if parsed else txt,
            "provenance": "parsed" if parsed else "raw text kept; not valid JSON",
        })
    eslint = None
    ep = os.path.join(C.CONFIG, "ESLintrc")
    if os.path.isdir(ep):
        eslint = []
        for p in sorted(C.walk(ep)):
            txt = C.read_text(p)
            try:
                eslint.append({"file": C.rel(p), "config": json.loads(txt)})
            except Exception:                                # noqa: BLE001
                eslint.append({"file": C.rel(p), "raw": txt})
    elif os.path.isfile(ep):
        txt = C.read_text(ep)
        try:
            eslint = [{"file": C.rel(ep), "config": json.loads(txt)}]
        except Exception:                                    # noqa: BLE001
            eslint = [{"file": C.rel(ep), "raw": txt}]

    validator_menu = None
    vm = os.path.join(C.CONFIG, "Validators", "ValidatorMenu.xml")
    if os.path.isfile(vm):
        r, note = load(vm, "ValidatorMenu")
        if r is not None:
            validator_menu = node_dict(r, R)
    validator_scripts = sorted(C.rel(p) for p in C.walk(os.path.join(C.CONFIG, "Validators"))
                               if os.path.splitext(p)[1].lower() in (".as", ".htm", ".js"))

    format_menu = None
    fm = os.path.join(C.CONFIG, "Formatters", "FormatMenu.xml")
    if os.path.isfile(fm):
        r, note = load(fm, "FormatMenu")
        if r is not None:
            format_menu = node_dict(r, R)
    formatter_scripts = sorted(C.rel(p) for p in C.walk(os.path.join(C.CONFIG, "Formatters"))
                               if os.path.splitext(p)[1].lower() in (".as", ".js"))

    tag_highlight = []
    for p in sorted(C.walk(os.path.join(C.CONFIG, "TagHighlight"))):
        ext = os.path.splitext(p)[1].lower()
        if ext == ".xml":
            r, note = C.parse_xml_tolerant(p)
            if r is None:
                failures.append({"stage": "taghighlight", "path": C.rel(p), "error": note})
                continue
            tag_highlight.append({"file": C.rel(p), "kind": "xml",
                                  "tree": node_dict(r, R)})
        elif ext == ".txt":
            lines = [ln.strip() for ln in C.read_text(p).splitlines() if ln.strip()]
            tag_highlight.append({"file": C.rel(p), "kind": "line list",
                                  "entry_count": len(lines), "entries": lines})

    third_party_tags = []
    for p in sorted(C.walk(os.path.join(C.CONFIG, "ThirdPartyTags"))):
        if os.path.splitext(p)[1].lower() not in (".xml", ".vtm"):
            continue
        r, note = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "thirdpartytags", "path": C.rel(p), "error": note})
            continue
        third_party_tags.append({"file": C.rel(p), "tree": node_dict(r, R)})

    method = {
        "task": "4 - document types, tag libraries, code intelligence",
        "how": [
            "MMDocumentTypes.xml gives each supported document type its internal "
            "type, its file extensions per platform, its mime type, its BOM "
            "policy and the blank file it is created from; the blank file's "
            "content is included verbatim when it is small.",
            "Every *.vtm tag file is parsed into tag -> attributes -> allowed "
            "values. attriboption elements are the shipped enumerations, so the "
            "allowed_values lists are literal, not inferred.",
            "TagLibraries.vtm maps each tag file back to its library and to the "
            "document types that library applies to.",
            "CodeColoring schemes are parsed into keyword lists, comment/string "
            "delimiters and embedded-block delimiters, which is what a rebuild "
            "needs to reproduce the tokenizer, plus the shipped sample text.",
            "Colors.xml and Configuration/themes/*.xml give the colour values "
            "those token classes are painted with.",
            "LinterRuleSets are JSON-parsed where valid; the raw text is kept "
            "when it is not.",
        ],
        "not_done": [
            "Configuration/KnowledgeEngines ships JS_KnowledgeEngine.dll and "
            "PHP_KnowledgeEngine.dll. Those are compiled binaries, not config, so "
            "the semantic code-intelligence they provide is NOT recoverable from "
            "the Configuration tree and is reported here as a known gap rather "
            "than guessed at.",
        ],
        "string_tables": smeta,
    }

    doc = C.envelope("handshake.studio.dreamweaver.code_intelligence.v1", method, {
        "counts": {
            "document_types": len(doc_types),
            "document_type_extensions_windows": sum(len(d["windows_file_extensions"])
                                                    for d in doc_types),
            "doctype_declarations": len(doctype_decls),
            "mime_type_rows": len(mime_types),
            "tag_libraries": len(libraries),
            "tag_refs_declared_in_index": sum(l["tag_count_declared"] for l in libraries),
            "tag_definitions_parsed": len(tags),
            "tag_attribute_definitions": sum(t["attribute_count"] for t in tags),
            "tag_attribute_types": attr_type_tally,
            "enumerated_attribute_values": enum_value_total,
            "tags_with_a_property_inspector_dialog":
                sum(1 for t in tags if t["property_inspector_dialog"]),
            "css_property_vocabulary_entries": (css_props or {}).get("entry_count"),
            "cross_tag_attribute_files": len(cross_tag),
            "cross_tag_attribute_groups": cross_tag_groups,
            "cross_tag_attribute_definitions": cross_tag_attrs,
            "cross_tag_attribute_applications": sum(
                g["applies_to_tag_count"] for f in cross_tag
                for g in f["attribute_groups"]),
            "validator_rule_files": len(validators),
            "code_hint_files": len(code_hints),
            "code_hint_menugroups": sum(c["menugroup_count"] for c in code_hints),
            "code_hint_items": sum(g["item_count"] for c in code_hints
                                   for g in c["menugroups"]),
            "code_hint_description_files": len(hint_descriptions),
            "code_hint_builtin_files": len(hint_builtin),
            "code_hint_cms_files": len(hint_cms),
            "code_coloring_schemes": len(schemes),
            "code_coloring_keyword_lists": sum(len(s["keyword_lists"]) for s in schemes),
            "code_coloring_keywords_total": sum(s["keyword_total"] for s in schemes),
            "syntax_color_token_classes": len(colors),
            "code_theme_definitions": sum(1 for t in themes
                                          if t["theme_file_kind"] == "code theme definition"),
            "code_theme_default_palettes": sum(1 for t in themes
                                               if t["theme_file_kind"].startswith("default colour")),
            "theme_xml_files_total": len(themes),
            "linter_rule_sets": len(linters),
            "tag_highlight_files": len(tag_highlight),
            "tag_highlight_entries": sum(t.get("entry_count", 0) for t in tag_highlight),
            "third_party_tag_files": len(third_party_tags),
        },
        "document_types": doc_types,
        "doctype_declarations": doctype_decls,
        "mime_types": mime_types,
        "tag_libraries": libraries,
        "tags": tags,
        "css_property_vocabulary": css_props,
        "cross_tag_attributes": cross_tag,
        "validator_rule_files": validators,
        "code_hints": code_hints,
        "code_hint_support_files": {
            "descriptions": hint_descriptions,
            "builtin_code": hint_builtin,
            "cms": hint_cms,
        },
        "code_coloring_schemes": schemes,
        "syntax_color_token_classes": colors,
        "code_themes": themes,
        "linter_rule_sets": linters,
        "eslint_config": eslint,
        "validator_menu": validator_menu,
        "validator_scripts": validator_scripts,
        "format_menu": format_menu,
        "formatter_scripts": formatter_scripts,
        "tag_highlight": tag_highlight,
        "third_party_tags": third_party_tags,
        "excluded_ai": C.excluded_ai(
            "document types, tag libraries and code intelligence",
            candidates=[d["id"] for d in doc_types] + [d["title"] for d in doc_types]
                       + [l["library_id"] for l in libraries]
                       + [l["library_name"] for l in libraries]
                       + [s["scheme_id"] for s in schemes]
                       + [g["group_label"] for c in code_hints for g in c["menugroups"]],
            extra_note="Checked every document type, tag library, colouring "
                       "scheme and code-hint group. Configuration/KnowledgeEngines "
                       "ships JS_KnowledgeEngine.dll and PHP_KnowledgeEngine.dll; "
                       "these are static language analysers, not models, and are "
                       "reported as a binary gap rather than as an AI feature."),
        "failures": failures,
    })
    size = C.write_json(out_path, doc)
    return doc, size


if __name__ == "__main__":
    doc, size = build(sys.argv[1])
    print(json.dumps(doc["counts"], indent=1))
    print("failures:", len(doc["failures"]))
    for f in doc["failures"][:10]:
        print("  ", f)
    print("bytes:", size)
