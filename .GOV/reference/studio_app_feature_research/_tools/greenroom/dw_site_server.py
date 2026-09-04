"""dw_site_server.py -- Task 5: the site, server and publishing model.

Sources actually read (all offline):
  Configuration/ServerModels/*.htm         -- the server model contract: display
                                              name, folder, server info, the
                                              language/version each supports
  Configuration/ServerBehaviors/**/*.edml  -- the server-behaviour catalogue:
                                              group definitions, participants,
                                              insert/search/update code blocks
  Configuration/ServerBehaviors/**/*.htm   -- their parameter dialogs
  Configuration/ServerFormats/**           -- the dynamic-data format catalogue
  Configuration/Connections/**             -- connection string builders per model
  Configuration/DataSources/**             -- binding sources per model
  Configuration/Components/**              -- the Components panel node types
  Configuration/WebServices/**             -- WSDL introspectors and proxy generators
  Configuration/SFTP/config                -- shipped SFTP/ssh algorithm policy
  Configuration/SourceControl/*            -- source-control plug-in catalogue
  Configuration/Queries/                   -- shipped saved queries
  Configuration/Dialogs/Eve/Site*.eve etc  -- the site/transfer/sync dialogs
  Configuration/BrowserProfiles/*          -- target-browser capability profiles
  Configuration/*.txt (FTPExtensionMap,
      ViewableExtTypes, ActiveXNames,
      Extensions, IceIdList)               -- transfer/preview extension policy
  en_US/Resources/*.zbin                   -- the site-definition field vocabulary
"""
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import dw_common as C                                       # noqa: E402
import dw_eve                                               # noqa: E402
from dw_zstrings import load_all_strings, resolve           # noqa: E402

SERVER_MODEL_HOOKS = ("getServerModelDisplayName", "getServerModelFolderName",
                      "getServerInfo", "getServerModelExtension",
                      "getServerLanguage", "getServerVersion",
                      "getServerSupportsCharset", "canRecognizeDocument",
                      "getFileExtensions", "getServerModelDelimiters")
SITE_DIALOG_HINT = re.compile(r"^(site|ftp|server|remote|sync|synch|testing|git|"
                              r"checkin|checkout|transfer|clone|repos)", re.I)


def build(out_path):
    exact, lower, smeta = load_all_strings(C.INSTALL_ROOT)
    failures = []

    def R(key):
        return resolve(key, exact, lower)

    def tree(el):
        rec = {"node": el.tag.split("}")[-1], "attributes": C.attrs_of(el)}
        t = (el.text or "").strip()
        if t:
            rec["text"] = t
        kids = [tree(c) for c in list(el)]
        if kids:
            rec["children"] = kids
        return rec

    # ---------------- server models ----------------------------------------
    sm_dir = os.path.join(C.CONFIG, "ServerModels")
    server_models = []
    for p in sorted(C.walk(sm_dir, exts={".htm", ".html"})):
        if os.path.dirname(p) != sm_dir:
            continue
        txt = C.read_text(p)
        js, includes = C.extract_js(txt, os.path.dirname(p))
        rec = {"file": C.rel(p),
               "model_key": os.path.splitext(os.path.basename(p))[0],
               "js_includes": includes,
               "declared": {},
               "provenance": "parsed"}
        for hook in SERVER_MODEL_HOOKS:
            lit = C.literal_returns(js, hook)
            body = C.js_block(js, hook)
            if lit:
                rec["declared"][hook] = lit[0] if len(lit) == 1 else lit
            elif body is not None:
                rec["declared"][hook] = {"computed_at_runtime": True,
                                         "source": body.strip()[:4000]}
        # getServerInfo() typically builds a literal object -- keep the source
        si = C.js_block(js, "getServerInfo")
        if si:
            rec["server_info_source"] = si.strip()
            rec["server_info_literals"] = dict(
                re.findall(r"(\w+)\s*[:=]\s*['\"]([^'\"]*)['\"]", si))
        rec["api_hooks_implemented"] = sorted(
            set(f["name"] for f in C.js_functions(js)))
        server_models.append(rec)
    server_model_dirs = sorted(d for d in os.listdir(sm_dir)
                               if os.path.isdir(os.path.join(sm_dir, d)))

    # ---------------- server behaviours ------------------------------------
    sb_dir = os.path.join(C.CONFIG, "ServerBehaviors")
    server_behaviors, sb_participants = [], []
    for p in sorted(C.walk(sb_dir, exts={".edml"})):
        r, note = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "edml", "path": C.rel(p), "error": note})
            continue
        model = os.path.relpath(os.path.dirname(p), sb_dir).split(os.sep)[0]
        a = C.attrs_of(r)
        kind = r.tag.split("}")[-1]
        if kind == "group":
            parts = [C.attrs_of(g) for g in r.iter()
                     if g.tag.split("}")[-1] == "groupParticipant"]
            sel = next((C.attrs_of(g).get("selectParticipant") for g in r.iter()
                        if g.tag.split("}")[-1] == "groupParticipants"), None)
            server_behaviors.append({
                "behavior_name": a.get("name"),
                "server_model": model,
                "file": C.rel(p),
                "edml_version": a.get("version"),
                "parameter_dialog_file": a.get("serverBehavior"),
                "hidden_from_server_behavior_builder": a.get("hideFromBuilder"),
                "select_participant": sel,
                "participant_count": len(parts),
                "participants": parts,
                "all_attributes": a,
                "provenance": "parsed",
            })
        else:
            # participant files: they carry the actual code blocks
            blocks = []
            for el in r.iter():
                t = el.tag.split("}")[-1]
                if t in ("insertText", "searchPatterns", "updatePatterns",
                         "translations", "quickSearch", "searchPattern",
                         "openTag", "closeTag", "delimiters"):
                    body = (el.text or "").strip()
                    blocks.append({"block": t, "attributes": C.attrs_of(el),
                                   "content": body or None,
                                   "children": [tree(c) for c in list(el)] or None})
            sb_participants.append({
                "participant_name": os.path.splitext(os.path.basename(p))[0],
                "server_model": model,
                "file": C.rel(p),
                "root_node": kind,
                "root_attributes": a,
                "code_blocks": blocks,
                "provenance": "parsed",
            })
    sb_dialogs = []
    for p in sorted(C.walk(sb_dir, exts={".htm", ".html"})):
        try:
            s = C.read_surface(p, R)
        except Exception as exc:                            # noqa: BLE001
            failures.append({"stage": "sb_dialog", "path": C.rel(p), "error": repr(exc)})
            continue
        sb_dialogs.append({
            "file": C.rel(p),
            "server_model": os.path.relpath(os.path.dirname(p), sb_dir).split(os.sep)[0],
            "title": s["title"],
            "control_count": len(s["controls"]),
            "controls": s["controls"],
            "dialog_buttons": s["command_buttons"],
            "js_functions": s["js_functions"],
            "provenance": "parsed",
        })

    # ---------------- server formats ---------------------------------------
    server_formats = []
    sf_dir = os.path.join(C.CONFIG, "ServerFormats")
    for p in sorted(C.walk(sf_dir, exts={".xml"})):
        r, note = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "serverformats", "path": C.rel(p), "error": note})
            continue
        entries = []
        for el in r.iter():
            if el is r:
                continue
            a = C.attrs_of(el)
            lbl = None
            for k in a:
                if k.startswith("mmstring:"):
                    lbl = R(a[k])[0]
            entries.append({"node": el.tag.split("}")[-1], "label": lbl,
                            "attributes": a, "text": (el.text or "").strip() or None})
        server_formats.append({
            "file": C.rel(p),
            "server_model": os.path.relpath(os.path.dirname(p), sf_dir).split(os.sep)[0],
            "entry_count": len(entries),
            "entries": entries,
            "provenance": "parsed",
        })
    server_format_scripts = sorted(C.rel(p) for p in C.walk(sf_dir, exts={".htm", ".html"}))

    # ---------------- connections ------------------------------------------
    connections = []
    cn_dir = os.path.join(C.CONFIG, "Connections")
    for p in sorted(C.walk(cn_dir, exts={".htm", ".html"})):
        try:
            s = C.read_surface(p, R)
        except Exception as exc:                            # noqa: BLE001
            failures.append({"stage": "connection", "path": C.rel(p), "error": repr(exc)})
            continue
        connections.append({
            "file": C.rel(p),
            "server_model": os.path.relpath(os.path.dirname(p), cn_dir).split(os.sep)[0],
            "title": s["title"],
            "settings_controls": s["controls"],
            "settings_control_count": len(s["controls"]),
            "dialog_buttons": s["command_buttons"],
            "js_functions": s["js_functions"],
            "provenance": "parsed",
        })
    connection_edml = []
    for p in sorted(C.walk(cn_dir, exts={".edml"})):
        r, note = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "connection_edml", "path": C.rel(p), "error": note})
            continue
        connection_edml.append({"file": C.rel(p), "tree": tree(r), "provenance": "parsed"})
    jdbc = sorted(C.rel(p) for p in C.walk(os.path.join(C.CONFIG, "JDBCDrivers")))

    # ---------------- data sources / bindings -------------------------------
    data_sources = []
    ds_dir = os.path.join(C.CONFIG, "DataSources")
    for p in sorted(C.walk(ds_dir, exts={".xml"})):
        r, note = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "datasources_xml", "path": C.rel(p), "error": note})
            continue
        rows = []
        for el in r.iter():
            if el is r:
                continue
            a = C.attrs_of(el)
            lbl = None
            for k in a:
                if k.startswith("mmstring:"):
                    lbl = R(a[k])[0]
            rows.append({"node": el.tag.split("}")[-1], "label": lbl, "attributes": a})
        data_sources.append({
            "file": C.rel(p),
            "server_model": os.path.relpath(os.path.dirname(p), ds_dir).split(os.sep)[0],
            "entry_count": len(rows),
            "entries": rows,
            "provenance": "parsed",
        })
    data_source_dialogs = []
    for p in sorted(C.walk(ds_dir, exts={".htm", ".html"})):
        try:
            s = C.read_surface(p, R)
        except Exception as exc:                            # noqa: BLE001
            failures.append({"stage": "datasource_dialog", "path": C.rel(p),
                             "error": repr(exc)})
            continue
        data_source_dialogs.append({
            "file": C.rel(p), "title": s["title"],
            "control_count": len(s["controls"]), "controls": s["controls"],
            "js_functions": s["js_functions"], "provenance": "parsed"})

    # ---------------- components panel node types ---------------------------
    components = []
    cp_dir = os.path.join(C.CONFIG, "Components")
    for p in sorted(C.walk(cp_dir, exts={".xml", ".edml"})):
        r, note = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "components", "path": C.rel(p), "error": note})
            continue
        components.append({"file": C.rel(p),
                           "server_model": os.path.relpath(os.path.dirname(p),
                                                           cp_dir).split(os.sep)[0],
                           "tree": tree(r), "provenance": "parsed"})

    # ---------------- web services ------------------------------------------
    web_services = []
    ws_dir = os.path.join(C.CONFIG, "WebServices")
    for p in sorted(C.walk(ws_dir)):
        ext = os.path.splitext(p)[1].lower()
        if ext == ".xml":
            r, note = C.parse_xml_tolerant(p)
            if r is None:
                failures.append({"stage": "webservices", "path": C.rel(p), "error": note})
                continue
            web_services.append({"file": C.rel(p), "role": "proxy generator config",
                                 "tree": tree(r), "provenance": "parsed"})
        elif ext in (".htm", ".html"):
            try:
                s = C.read_surface(p, R)
            except Exception as exc:                        # noqa: BLE001
                failures.append({"stage": "webservices_htm", "path": C.rel(p),
                                 "error": repr(exc)})
                continue
            web_services.append({"file": C.rel(p),
                                 "role": "introspector or proxy generator surface",
                                 "title": s["title"], "controls": s["controls"],
                                 "js_functions": s["js_functions"],
                                 "provenance": "parsed"})

    # ---------------- transports and transfer policy ------------------------
    sftp_cfg = None
    sp = os.path.join(C.CONFIG, "SFTP", "config")
    if os.path.isfile(sp):
        raw = C.read_text(sp)
        sftp_cfg = {
            "file": C.rel(sp),
            "raw": raw,
            "directives": [{"keyword": ln.split(None, 1)[0],
                            "value": (ln.split(None, 1)[1] if len(ln.split(None, 1)) > 1 else "")}
                           for ln in raw.splitlines() if ln.strip()
                           and not ln.strip().startswith("#")],
            "provenance": "parsed",
        }
    transport_dlls = sorted(
        f for f in os.listdir(C.INSTALL_ROOT)
        if f.lower().startswith("netio") and f.lower().endswith(".dll"))

    source_control = []
    scp = os.path.join(C.CONFIG, "SourceControl")
    for p in sorted(C.walk(scp)):
        rec = {"file": C.rel(p), "bytes": os.path.getsize(p)}
        if os.path.splitext(p)[1].lower() in (".cdf", ".txt", ".xml", ".ini"):
            rec["content"] = C.read_text(p)
        source_control.append(rec)
    source_control_localized = sorted(
        C.rel(p) for p in C.walk(os.path.join(C.INSTALL_ROOT, "en_US",
                                              "Configuration", "SourceControl")))

    queries_dir = os.path.join(C.CONFIG, "Queries")
    queries = sorted(C.rel(p) for p in C.walk(queries_dir))

    text_policies = {}
    for fn in ("FTPExtensionMap.txt", "FTPExtensionMapMac.txt", "ViewableExtTypes.txt",
               "ViewableExtTypesMac.txt", "CustomBrowserViewableExtTypes.txt",
               "ActiveXNames.txt", "Extensions.txt", "IceIdList.txt"):
        p = os.path.join(C.CONFIG, fn)
        if not os.path.isfile(p):
            continue
        raw = C.read_text(p)
        lines = [ln for ln in (l.strip() for l in raw.splitlines()) if ln]
        text_policies[fn] = {"file": C.rel(p), "line_count": len(lines),
                             "lines": lines, "provenance": "parsed"}

    # ---------------- site / transfer dialogs -------------------------------
    site_dialogs = []
    eve_dir = os.path.join(C.CONFIG, "Dialogs", "Eve")
    for p in sorted(C.walk(eve_dir, exts={".eve"})):
        stem = os.path.splitext(os.path.basename(p))[0]
        if not SITE_DIALOG_HINT.match(stem):
            continue
        try:
            layouts = dw_eve.parse_eve(C.read_text(p))
        except Exception as exc:                            # noqa: BLE001
            failures.append({"stage": "site_dialog", "path": C.rel(p), "error": repr(exc)})
            continue
        for lay in layouts:
            flat = dw_eve.flatten_controls(lay["nodes"])
            top = lay["nodes"][0]["args"] if lay["nodes"] else {}
            _k, ttl = dw_eve.split_localized(top.get("name"))
            site_dialogs.append({
                "layout_name": lay["layout_name"],
                "file": C.rel(p),
                "window_title": ttl if isinstance(ttl, str) else None,
                "control_count": len(flat),
                "controls": flat,
                "provenance": "parsed",
            })

    # ---------------- site definition vocabulary ----------------------------
    site_vocab = {}
    for prefix in ("sitesetup/", "site/", "sitemanager/", "RemoteFileBrowser/",
                   "Connections/", "sync/", "git/", "drf/"):
        rows = {k: v for k, v in exact.items() if k.startswith(prefix)}
        if rows:
            site_vocab[prefix.rstrip("/")] = {
                "key_count": len(rows),
                "strings": rows,
                "provenance": "resolved: read from the shipped en_US ZString table; "
                              "these are the field labels, tooltips and messages of "
                              "the site-definition and file-transfer surfaces. They "
                              "name the parameters but do not by themselves give the "
                              "value ranges.",
            }

    # ---------------- browser targets ---------------------------------------
    browser_profiles = []
    bp_dir = os.path.join(C.CONFIG, "BrowserProfiles")
    for p in sorted(C.walk(bp_dir)):
        ext = os.path.splitext(p)[1].lower()
        if ext == ".txt":
            raw = C.read_text(p)
            head = [ln for ln in raw.splitlines()[:8]]
            browser_profiles.append({
                "file": C.rel(p),
                "profile": os.path.splitext(os.path.basename(p))[0],
                "line_count": len(raw.splitlines()),
                "header_lines": head,
                "bytes": os.path.getsize(p),
                "provenance": "parsed (header only; the body is a large tag support table)",
            })
        elif ext == ".xml":
            r, note = C.parse_xml_tolerant(p)
            if r is None:
                failures.append({"stage": "browserprofiles", "path": C.rel(p),
                                 "error": note})
                continue
            browser_profiles.append({"file": C.rel(p), "profile": "exceptions",
                                     "tree": tree(r), "provenance": "parsed"})

    method = {
        "task": "5 - site, server and publishing model",
        "how": [
            "Server models are JS contract files; each declared hook is read from "
            "its literal return where it has one, and carried as source where it "
            "computes at runtime.",
            "Server behaviours are Adobe EDML. A 'group' file declares the "
            "behaviour and names its participants; each participant file holds "
            "the actual insertText / searchPatterns / updatePatterns code blocks. "
            "Both are exported so the rebuild sees the behaviour AND the code it "
            "writes.",
            "Connection and data-source dialogs are parsed for their form controls, "
            "which is the literal parameter set of a connection definition.",
            "The site-definition parameter vocabulary is taken from the shipped "
            "en_US string table under the sitesetup/, site/, sync/ and git/ "
            "prefixes, plus the Eve dialogs whose layout name is site/ftp/sync "
            "related. This is labelled explicitly because the Site Setup dialog "
            "itself is a native surface with no declarative control file.",
            "Transport support is evidenced by the shipped NetIO*.dll set and by "
            "the SFTP ssh policy file; both are reported as found, not inferred.",
        ],
        "known_gap": "Configuration/Queries ships empty on a clean install (it is "
                     "the user's saved-query store), so there is no shipped query "
                     "catalogue to export. Reported as empty rather than omitted.",
        "string_tables": smeta,
    }

    doc = C.envelope("handshake.studio.dreamweaver.site_server_model.v1", method, {
        "counts": {
            "server_models": len(server_models),
            "server_model_resource_folders": len(server_model_dirs),
            "server_behavior_groups": len(server_behaviors),
            "server_behavior_participant_files": len(sb_participants),
            "server_behavior_code_blocks": sum(len(p["code_blocks"]) for p in sb_participants),
            "server_behavior_dialogs": len(sb_dialogs),
            "server_behavior_dialog_controls": sum(d["control_count"] for d in sb_dialogs),
            "server_format_definition_files": len(server_formats),
            "server_format_entries": sum(f["entry_count"] for f in server_formats),
            "server_format_scripts": len(server_format_scripts),
            "connection_surfaces": len(connections),
            "connection_setting_controls": sum(c["settings_control_count"] for c in connections),
            "connection_edml_files": len(connection_edml),
            "jdbc_driver_files": len(jdbc),
            "data_source_definition_files": len(data_sources),
            "data_source_entries": sum(d["entry_count"] for d in data_sources),
            "data_source_dialogs": len(data_source_dialogs),
            "component_definition_files": len(components),
            "web_service_files": len(web_services),
            "site_and_transfer_dialogs": len(site_dialogs),
            "site_and_transfer_dialog_controls": sum(d["control_count"] for d in site_dialogs),
            "site_vocabulary_prefixes": len(site_vocab),
            "site_vocabulary_strings": sum(v["key_count"] for v in site_vocab.values()),
            "browser_capability_profiles": sum(1 for b in browser_profiles
                                               if b["profile"] != "exceptions"),
            "browser_profile_files_including_exceptions_xml": len(browser_profiles),
            "transport_dlls_shipped": len(transport_dlls),
            "source_control_files": len(source_control),
            "shipped_saved_queries": len(queries),
            "extension_policy_files": len(text_policies),
        },
        "server_models": server_models,
        "server_model_resource_folders": server_model_dirs,
        "server_behaviors": server_behaviors,
        "server_behavior_participants": sb_participants,
        "server_behavior_dialogs": sb_dialogs,
        "server_formats": server_formats,
        "server_format_scripts": server_format_scripts,
        "connections": connections,
        "connection_edml": connection_edml,
        "jdbc_driver_files": jdbc,
        "data_sources": data_sources,
        "data_source_dialogs": data_source_dialogs,
        "components": components,
        "web_services": web_services,
        "publishing_transports": {
            "netio_dlls_shipped": transport_dlls,
            "netio_dll_provenance": "parsed: file names present in the install root. "
                                    "NetIO.dll (HTTP/WebDAV core), NetIODav.dll "
                                    "(WebDAV), NetIOFTP.dll (FTP/FTPS), "
                                    "NetIOSFTP.dll (SFTP). The mapping of each DLL "
                                    "to its protocol is heuristic, taken from the "
                                    "file names; the file list itself is parsed.",
            "sftp_ssh_policy": sftp_cfg,
            "ftp_extension_policy": text_policies.get("FTPExtensionMap.txt"),
        },
        "source_control": source_control,
        "source_control_localized_files": source_control_localized,
        "shipped_saved_queries": queries,
        "extension_policy_files": text_policies,
        "site_and_transfer_dialogs": site_dialogs,
        "site_definition_vocabulary": site_vocab,
        "browser_profiles": browser_profiles,
        "excluded_ai": C.excluded_ai(
            "site, server and publishing model",
            candidates=[m["model_key"] for m in server_models]
                       + [b["behavior_name"] for b in server_behaviors]
                       + [c["file"] for c in connections]
                       + [w["file"] for w in web_services]
                       + [d["layout_name"] for d in site_dialogs],
            extra_note="Checked every server model, server behaviour, connection "
                       "surface, web-service file and site/transfer dialog."),
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
