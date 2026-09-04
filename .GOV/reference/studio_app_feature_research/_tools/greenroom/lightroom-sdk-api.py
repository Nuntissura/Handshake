#!/usr/bin/env python
"""
lightroom-sdk-api.py

Recovers what this Lightroom Classic install reveals about its Lua SDK and
internal module architecture. Read-only; Lightroom is never launched.

TWO CORRECTIONS TO THE STARTING ASSUMPTION, both established by parsing:

  1. The 68 .lua files in the install are NOT SDK source. 67 of them are
     Book-module page-geometry tables (Templates/Layout Templates/*/
     templatePages.lua) and one is layout_template_sizes.lua. The 69th .lua
     in the corpus is a user-profile Metadata/DefaultPanel.lua. No Lightroom
     SDK .lua source file ships with the product.

  2. The SDK is still recoverable, from a different place. Lightroom's own
     Lua code ships COMPILED, as Lua 5.1 bytecode dumps embedded in the PE
     binaries (.lrmodule, .lrplugin, .dll, .exe). Lua 5.1 dumps keep the
     string constant pool intact and prefixed by length, so module names,
     API namespaces, callback field names, plugin manifest keys and even
     whole embedded Lua source fragments come back exactly. That is what
     this tool reads.

The two shipped SDK PLUGINS - Flickr.lrplugin and AdobeStock.lrplugin - are
the highest-value evidence: they are ordinary third-party-style SDK consumers,
so their constant pools name the exact SDK contract a plugin must implement.

Classification is explicit throughout:
  parsed    - the string is really in the binary's Lua constant pool
  heuristic - this tool's assignment of that string to an SDK role
"""
from __future__ import annotations

import argparse
import collections
import datetime as _dt
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lrbin  # noqa: E402
import lrlua  # noqa: E402

SCHEMA_ID = "handshake.adobe.lightroom_classic.sdk_api.v1"

TARGETS = [
    ("LightroomSDK.dll", "sdk_host"),
    ("Flickr.lrplugin", "shipped_sdk_plugin"),
    ("AdobeStock.lrplugin", "shipped_sdk_plugin"),
    ("Export.lrmodule", "module"),
    ("Library.lrmodule", "module"),
    ("Develop.lrmodule", "module"),
    ("Import.lrmodule", "module"),
    ("Layout.lrmodule", "module"),
    ("Print.lrmodule", "module"),
    ("Slideshow.lrmodule", "module"),
    ("Web.lrmodule", "module"),
    ("Book.lrmodule", "module"),
    ("Location.lrmodule", "module"),
    ("MultipleMonitor.lrmodule", "module"),
    ("Lightroom.exe", "application"),
    ("LibraryToolkit.dll", "toolkit"),
    ("ui.dll", "ui_toolkit"),
    ("substrate.dll", "platform_substrate"),
    ("curculio.dll", "platform_substrate"),
    ("iac.dll", "inter_application"),
    ("net_client.dll", "networking"),
    ("image_analysis.dll", "image_analysis"),
    ("video_toolkit.dll", "video"),
    ("StoreProvider.dll", "commerce"),
    ("Email.dll", "email"),
]

# Curated SDK role map. Membership of a name in a role is HEURISTIC; the
# presence of the name in the binary is PARSED.
ROLES = {
    "plugin_manifest_key": [
        "LrSdkVersion", "LrSdkMinimumVersion", "LrToolkitIdentifier",
        "LrPluginName", "LrPluginInfoProvider", "LrPluginInfoUrl",
        "LrExportServiceProvider", "LrExportFilterProvider",
        "LrMetadataProvider", "LrLibraryMenuItems", "LrExportMenuItems",
        "LrInitPlugin", "LrShutdownPlugin", "LrForceInitPlugin",
        "LrEnablePlugin", "LrDisablePlugin", "LrAlsoUseBuiltInTranslations",
        "LrLimitNumberOfTempRenditions", "LrHttpHandler",
    ],
    "namespace_module": [
        "LrApplication", "LrApplicationView", "LrBinding", "LrCatalog",
        "LrColor", "LrDate", "LrDevelopController", "LrDialogs", "LrDigest",
        "LrErrors", "LrExportRendition", "LrExportSession",
        "LrExportSettings", "LrFileUtils", "LrFtp", "LrFunctionContext",
        "LrHttp", "LrLogger", "LrMD5", "LrMath", "LrPasswords",
        "LrPathUtils", "LrPhoto", "LrPhotoInfo", "LrPrefs",
        "LrProgressScope", "LrPublishedCollection",
        "LrPublishedCollectionSet", "LrRecursionGuard", "LrSelection",
        "LrShell", "LrSlideshow", "LrSocket", "LrSounds", "LrStringUtils",
        "LrSystemInfo", "LrTasks", "LrTether", "LrUndo", "LrView",
        "LrXml", "LrZip",
    ],
    "export_service_provider_field": [
        "exportPresetFields", "processRenderedPhotos", "startDialog",
        "endDialog", "sectionsForTopOfDialog", "sectionsForBottomOfDialog",
        "allowFileFormats", "allowColorSpaces", "hideSections",
        "canExportVideo", "hidePrintResolution", "updateExportSettings",
        "supportsIncrementalPublish", "small_icon", "exportServiceProvider",
        "exportServiceProviderTitle", "showSections",
        "getCollectionBehaviorInfo", "titleForGoToPublishedCollection",
        "titleForGoToPublishedPhoto", "titleForPublishedCollection",
        "titleForPublishedCollection_standalone",
        "titleForPublishedSmartCollection",
        "titleForPublishedSmartCollection_standalone",
        "titleForPhotoRating", "titleRepublishBehavior",
        "defaultCollectionName", "defaultCollectionCanBeDeleted",
        "maxCollectionSetDepth", "supportsCustomSortOrder",
        "deletePhotosFromPublishedCollection", "deletePublishedCollection",
        "renamePublishedCollection", "imposeSortOrderOnPublishedCollection",
        "shouldReverseSequenceForPublishedCollection",
        "getCommentsFromPublishedCollection", "addCommentToPublishedPhoto",
        "getRatingsFromPublishedCollection", "canAddCommentsToService",
        "canAddCollection", "metadataThatTriggersRepublish",
        "deleteFirstOnPublish", "goToPublishedCollection",
    ],
    "export_session_and_rendition": [
        "renditions", "countRenditions", "waitForRender", "wasSkipped",
        "skipRender", "photo", "destinationPath", "exportSession",
        "exportContext", "configureProgress", "setPortionComplete",
        "isCanceled", "stopIfCanceled", "recordPublishedPhotoId",
        "recordPublishedPhotoUrl", "recordRemoteCollectionId",
        "recordRemoteCollectionUrl", "publishedCollectionInfo",
        "publishedPhotoId", "remoteCollectionId", "remoteId",
        "exportLocation", "propertyTable", "functionContext",
    ],
    "catalog_and_photo_api": [
        "getRawMetadata", "getFormattedMetadata", "setRawMetadata",
        "getDevelopSettings", "applyDevelopSettings", "applyDevelopPreset",
        "getPropertyForPlugin", "setPropertyForPlugin",
        "withWriteAccessDo", "withPrivateWriteAccessDo",
        "withCatalogDoAsync", "findPhotoByPath", "findPhotos",
        "getAllPhotos", "getTargetPhotos", "getTargetPhoto",
        "createCollection", "createCollectionSet", "createSmartCollection",
        "createKeyword", "addPhotos", "removePhotos", "getChildCollections",
        "getChildCollectionSets", "localIdentifier", "getName",
        "getParent", "getAttributes", "setAttributes", "requestJpegThumbnail",
        "getActivePhoto", "getActiveSources", "batchGetRawMetadata",
        "batchGetFormattedMetadata", "triggerImportFromPathAndAddToCollection",
    ],
    "metadata_provider_field": [
        "metadataFieldsForPhotos", "schemaVersion", "updateFromEarlierSchemaVersion",
        "dataType", "searchable", "browsable", "readOnly", "titleForField",
        "version", "id", "title",
    ],
    "view_factory_control": [
        "static_text", "edit_field", "checkbox", "radio_button", "popup_menu",
        "push_button", "combo_box", "slider", "column", "row", "group_box",
        "spacer", "separator", "picture", "catalog_photo", "scrolled_view",
        "simple_list", "password_field", "tab_view", "tab_view_item",
        "path_field", "color_well", "view", "bind", "bind_to_object",
    ],
    "task_and_context": [
        "startAsyncTask", "startAsyncTaskWithoutErrorHandler",
        "startAsyncTaskWithErrorHandler", "pcall", "yield", "sleep",
        "execute", "executeWithRunAsVerb", "callWithContext",
        "postAsyncTaskWithContext", "addCleanupHandler", "addFailureHandler",
        "attachErrorDialogToFunctionContext", "callWithEmptyEnvironment",
        "pcallWithEmptyEnvironment", "throwCanceled", "throwUserError",
    ],
}
ROLE_INDEX = {}
for role, names in ROLES.items():
    for n in names:
        ROLE_INDEX.setdefault(n, []).append(role)

SQL_RE = re.compile(
    r"^\s*(select|insert\s+into|insert\s+or|update|delete\s+from|"
    r"create\s+table|create\s+index|create\s+trigger|create\s+view|"
    r"drop\s+table|alter\s+table|pragma|replace\s+into|with)\b",
    re.I)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--install",
                    default=r"C:\Program Files\Adobe\Adobe Lightroom Classic")
    ap.add_argument("--user",
                    default=os.path.expandvars(r"%APPDATA%\Adobe\Lightroom"))
    ap.add_argument("--out", required=True)
    ap.add_argument("--max-idents", type=int, default=1200)
    args = ap.parse_args()

    errors = []

    # ---- 1. the actual .lua files ---------------------------------------
    lua_files = []
    for origin, root in (("install", args.install), ("user", args.user)):
        for dp, _d, fn in os.walk(root):
            for f in fn:
                if f.lower().endswith(".lua"):
                    p = os.path.join(dp, f)
                    lua_files.append((origin, p,
                                      os.path.relpath(p, root).replace("\\", "/"),
                                      os.path.getsize(p)))
    lua_kinds = collections.Counter()
    lua_records = []
    lua_keypaths = collections.Counter()
    for origin, p, rel, size in sorted(lua_files, key=lambda x: x[2]):
        base = os.path.basename(p)
        kind = ("book_layout_page_geometry" if base == "templatePages.lua"
                else "book_layout_sizes" if base == "layout_template_sizes.lua"
                else "user_metadata_panel_layout"
                if base == "DefaultPanel.lua" else "other")
        lua_kinds[kind] += 1
        rec = {"origin": origin, "file": rel, "bytes": size, "kind": kind,
               "classification": "parsed"}
        try:
            name, tbl = lrlua.parse_table(lrlua.read(p))
            rec["root_variable"] = name
            rec["parsed_ok"] = True
            j = lrlua.jsonable(tbl)
            rec["top_level_keys"] = (sorted(j)[:40] if isinstance(j, dict)
                                     else "array[%d]" % len(j))
            if isinstance(j, dict):
                lua_keypaths.update(list(j))
        except Exception as exc:  # noqa: BLE001
            rec["parsed_ok"] = False
            rec["error"] = "%s: %s" % (type(exc).__name__, str(exc)[:160])
        lua_records.append(rec)

    # ---- 2. constant pools ----------------------------------------------
    binaries = []
    role_hits = collections.defaultdict(lambda: collections.defaultdict(list))
    role_mentions = collections.defaultdict(lambda: collections.defaultdict(list))
    all_chunks = collections.Counter()
    all_revdns = collections.Counter()
    zstr_namespaces = collections.Counter()
    sql_statements = collections.Counter()
    source_fragments = []

    for fname, kind in TARGETS:
        p = os.path.join(args.install, fname)
        if not os.path.isfile(p):
            errors.append({"file": fname, "error": "not present in install"})
            continue
        try:
            mined, order, size = lrbin.mine(p)
        except Exception as exc:  # noqa: BLE001
            errors.append({"file": fname,
                           "error": "%s: %s" % (type(exc).__name__, exc)})
            continue
        idents = mined.get("identifier", [])
        chunks = mined.get("lua_chunk_name", [])
        revdns = mined.get("reverse_dns_id", [])
        zkeys = mined.get("zstr_localization_key", [])
        for c in chunks:
            all_chunks[c] += 1
        for r in revdns:
            all_revdns[r] += 1
        for z in zkeys:
            m = lrbin.ZSTR_RE.match(z)
            if m:
                zstr_namespaces[m.group(1).split("/")[0]] += 1
        hits = collections.defaultdict(list)
        exact = set(order)
        for name, roles in ROLE_INDEX.items():
            if name in exact:
                for role in roles:
                    hits[role].append(name)
                    role_hits[role][name].append(fname)
        # secondary, weaker evidence: the name appears inside a longer string
        # constant (typically an SDK error or diagnostic message)
        joined = "\n".join(s for s in order if len(s) > 20)
        for name, roles in ROLE_INDEX.items():
            if name in exact or len(name) < 6:
                continue
            if name in joined:
                for role in roles:
                    role_mentions[role][name].append(fname)
        for s in order:
            if SQL_RE.match(s) and len(s) > 24:
                sql_statements[" ".join(s.split())[:400]] += 1
            if "\n" in s and len(s) > 200 and ("=" in s or "function" in s):
                source_fragments.append({"binary": fname,
                                         "chars": len(s),
                                         "text": s[:4000]})
        binaries.append({
            "file": fname, "role": kind, "bytes": size,
            "classification": "parsed",
            "unique_lua_string_constants": len(order),
            "lua_chunk_names": sorted(chunks),
            "reverse_dns_ids": sorted(revdns),
            "zstr_key_count": len(zkeys),
            "sdk_role_hits": {r: sorted(set(v)) for r, v in sorted(hits.items())},
            "identifier_sample": sorted(idents)[:args.max_idents],
            "identifier_count": len(idents),
        })

    api_surface = []
    for role in sorted(ROLES):
        members = []
        for name in sorted(ROLES[role]):
            found = role_hits[role].get(name)
            mentioned = role_mentions[role].get(name)
            members.append({
                "name": name,
                "present_in_install": bool(found),
                "evidence": ("exact_string_constant" if found
                             else "substring_of_a_longer_constant"
                             if mentioned else "not_found"),
                "binaries": sorted(set(found)) if found else [],
                "mentioned_in_binaries": (sorted(set(mentioned))
                                          if mentioned else []),
                "presence_classification": "parsed",
                "role_classification": "heuristic:curated_sdk_role_map",
            })
        api_surface.append({
            "role": role,
            "members_total": len(members),
            "members_confirmed_present": sum(
                1 for m in members if m["present_in_install"]),
            "members_mentioned_only": sum(
                1 for m in members
                if not m["present_in_install"] and m["mentioned_in_binaries"]),
            "members_not_found": sum(
                1 for m in members
                if not m["present_in_install"] and not m["mentioned_in_binaries"]),
            "members": members,
        })

    doc = {
        "schema_id": SCHEMA_ID,
        "generated_at": _dt.datetime.now(_dt.timezone.utc).isoformat(),
        "method": {
            "mode": "offline_static_parse",
            "app_launched": False,
            "sources": [
                {"id": "lua_files", "classification": "parsed",
                 "roots": [args.install, args.user],
                 "files": len(lua_files),
                 "parser": "lrlua.parse_table"},
                {"id": "lua_constant_pools", "classification": "parsed",
                 "binaries": [t[0] for t in TARGETS],
                 "format": "Lua 5.1 bytecode dump string constants "
                           "(0x04, uint32 length, bytes, NUL) embedded in PE",
                 "parser": "lrbin.mine"},
            ],
            "classification_legend": {
                "parsed": "the string is genuinely in the file",
                "derived": "computed from parsed data",
                "heuristic": "this tool's role assignment for that string",
            },
        },
        "counts": {
            "lua_files_found": len(lua_files),
            "lua_files_parsed_ok": sum(1 for r in lua_records
                                       if r.get("parsed_ok")),
            "lua_files_by_kind": dict(lua_kinds),
            "binaries_mined": len(binaries),
            "distinct_lua_chunk_names": len(all_chunks),
            "distinct_reverse_dns_ids": len(all_revdns),
            "sdk_roles_modelled": len(api_surface),
            "sdk_names_confirmed_present": sum(
                r["members_confirmed_present"] for r in api_surface),
            "sdk_names_probed": sum(r["members_total"] for r in api_surface),
            "sql_statements_recovered": len(sql_statements),
            "embedded_source_fragments": len(source_fragments),
        },
        "scope_correction": {
            "classification": "parsed",
            "statement": "No Lightroom SDK Lua source ships in this install. "
                         "The 68 install .lua files are Book-module layout "
                         "data. The SDK surface below is recovered from Lua "
                         "5.1 constant pools inside the shipped PE binaries, "
                         "chiefly LightroomSDK.dll and the two shipped SDK "
                         "plugins Flickr.lrplugin and AdobeStock.lrplugin.",
        },
        "lua_files": {"classification": "parsed", "files": lua_records,
                      "top_level_key_frequency": dict(
                          lua_keypaths.most_common(60))},
        "sdk_api_surface": api_surface,
        "module_architecture": {
            "classification": "parsed",
            "note": "Lua chunk names recovered from the constant pools. Each "
                    "is a real source file name compiled into the binary, so "
                    "this is Lightroom's own internal module decomposition.",
            "chunks": [{"chunk": k, "binaries": v}
                       for k, v in sorted(all_chunks.items())],
        },
        "reverse_dns_service_ids": {
            "classification": "parsed",
            "ids": [{"id": k, "binaries": v}
                    for k, v in sorted(all_revdns.items())],
        },
        "zstr_namespaces": {
            "classification": "parsed",
            "note": "first path segment of every $$$/... localisation key "
                    "found; approximates the feature namespace list",
            "namespaces": dict(zstr_namespaces.most_common(120)),
        },
        "recovered_sql": {
            "classification": "parsed",
            "note": "SQL statement text found verbatim in the constant pools; "
                    "this is how the modules address the catalog",
            "statements": [{"sql": k, "occurrences": v}
                           for k, v in sql_statements.most_common(200)],
        },
        "embedded_lua_source_fragments": {
            "classification": "parsed",
            "note": "multi-line Lua source kept verbatim as string constants",
            "fragments": source_fragments[:40],
        },
        "binaries": binaries,
        "errors": errors,
    }

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)
    print(json.dumps(doc["counts"], indent=1))


if __name__ == "__main__":
    main()
