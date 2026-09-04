#!/usr/bin/env python
"""
lightroom-catalog-model.py

Recovers Lightroom Classic's catalog model offline, read-only.

WHAT A "CATALOG" ACTUALLY IS - established by inspecting a real one:
it is not one file. It is a five-part bundle:

  <name>.lrcat                 SQLite 3 relational core
  <name>.lrcat-data/           RocksDB key-value store for large payloads
  <name> Previews.lrdata/      SQLite index + sharded JPEG preview pyramid
  <name> Smart Previews.lrdata/ lossy-DNG proxy renditions
  <name> Helper.lrdata/        SQLite FTS5 search indexes + metadata worklist

Every SQLite file here is opened with the URI flags mode=ro&immutable=1, which
forbids writes and suppresses journal/WAL creation, so no shipped or user file
is modified. Lightroom is never launched.

The install ships NO template or default catalog: the only .lrcat files on this
machine are the operator's own, discovered from the Lightroom preferences file.

HANDSHAKE CONTEXT: the Handshake product forbids SQLite. This document is a
model of the CONCEPT - what entities a professional photo catalog must carry
and how they relate - not an instruction to use SQLite.
"""
from __future__ import annotations

import argparse
import collections
import datetime as _dt
import json
import os
import re
import sqlite3
import sys
import urllib.parse

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lrbin  # noqa: E402

SCHEMA_ID = "handshake.adobe.lightroom_classic.catalog_model.v1"

# Table -> subsystem. HEURISTIC grouping by this tool.
GROUPS = [
    ("core_image_identity", [r"^Adobe_images$", r"^Adobe_imageProperties$",
                             r"^Adobe_AdditionalMetadata$",
                             r"^AgLibraryFile$", r"^AgLibraryFolder",
                             r"^AgLibraryRootFolder$", r"^AgFolderContent$",
                             r"^AgLibraryImageAttributes$",
                             r"^AgLibraryFileAssetMetadata$"]),
    ("develop", [r"^Adobe_imageDevelop", r"^Adobe_libraryImageDevelop",
                 r"^AgDevelopAdditionalMetadata$",
                 r"^Adobe_imageProofSettings$"]),
    ("metadata_harvest", [r"^AgHarvested", r"^AgInterned", r"^AgLibraryIPTC$",
                          r"^AgMetadataSearchIndex$",
                          r"^AgLibraryImageSearchData$",
                          r"^AgLibraryImageSaveXMP$",
                          r"^AgLibraryImageXMPUpdater$"]),
    ("keywords", [r"^AgLibraryKeyword"]),
    ("faces_people", [r"^Adobe_faceProperties$", r"^AgLibraryFace",
                      r"^Adobe_libraryImageFaceProcessHistory$"]),
    ("collections", [r"^AgLibraryCollection", r"^Migrated"]),
    ("stacks", [r"Stack"]),
    ("import", [r"^AgLibraryImport", r"^AgParsedImportHash$",
                r"^AgTempImages$"]),
    ("publish", [r"^AgLibraryPublished", r"^AgPublish", r"^AgRemotePhoto$",
                 r"^AgPhotoComment$"]),
    ("cloud_sync", [r"Oz", r"^LrMobileSyncChangeCounter$",
                    r"^AgSpecialSourceContent$", r"SyncedAsset", r"SyncedAlbum"]),
    ("output_modules", [r"^AgOutputImageAsset$", r"^AgLastCatalogExport$"]),
    ("video", [r"^AgVideoInfo$"]),
    ("proxies_previews", [r"^AgDNGProxyInfo", r"^AgSourceColorProfileConstants$"]),
    ("custom_properties", [r"^AgPhotoProperty", r"^AgSearchablePhotoProperty"]),
    ("app_state", [r"^Adobe_variables", r"^AgMRULists$",
                   r"^AgLibraryBackups$", r"^Adobe_namedIdentityPlate$",
                   r"^MigrationSchemaVersion$", r"^MigratedInfo$",
                   r"^AgLibraryUpdatedImages$",
                   r"^AgLibraryImageChangeCounter$",
                   r"^AgLibraryCollectionChangeCounter$",
                   r"^AgLibraryCollectionImageChangeCounter$"]),
    ("sqlite_internal", [r"^sqlite_"]),
]
_G = [(g, [re.compile(p) for p in ps]) for g, ps in GROUPS]


def group_of(name):
    for g, pats in _G:
        for p in pats:
            if p.search(name):
                return g
    return "unclassified"


# Windows cloud-placeholder attributes. Opening a dehydrated OneDrive file
# would force a network download of the whole file, so such files are
# reported and skipped rather than silently pulled down.
FILE_ATTRIBUTE_OFFLINE = 0x1000
FILE_ATTRIBUTE_RECALL_ON_OPEN = 0x00040000
FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS = 0x00400000
CLOUD_MASK = (FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_RECALL_ON_OPEN
              | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)


def is_cloud_placeholder(path):
    try:
        attrs = os.stat(path).st_file_attributes
    except (AttributeError, OSError):
        return False
    return bool(attrs & CLOUD_MASK)


def ro_uri(path):
    return ("file:" + urllib.parse.quote(path.replace("\\", "/"))
            + "?mode=ro&immutable=1")


def dump_sqlite(path, row_counts=True):
    con = sqlite3.connect(ro_uri(path), uri=True)
    cur = con.cursor()
    out = {"file": os.path.basename(path),
           "path": path,
           "bytes": os.path.getsize(path),
           "classification": "parsed",
           "open_mode": "mode=ro&immutable=1 (no write, no journal)",
           "tables": [], "views": [], "triggers": [], "index_count": 0}
    try:
        out["sqlite_user_version"] = cur.execute(
            "pragma user_version").fetchone()[0]
        out["sqlite_page_size"] = cur.execute("pragma page_size").fetchone()[0]
        out["sqlite_encoding"] = cur.execute("pragma encoding").fetchone()[0]
    except sqlite3.Error:
        pass
    master = cur.execute(
        "select type, name, tbl_name, sql from sqlite_master").fetchall()
    idx = collections.Counter()
    for typ, name, tbl, sql in master:
        if typ == "index":
            idx[tbl] += 1
    out["index_count"] = sum(idx.values())
    for typ, name, tbl, sql in master:
        if typ == "view":
            out["views"].append({"name": name, "sql": sql})
        elif typ == "trigger":
            out["triggers"].append({"name": name, "table": tbl, "sql": sql})
    for typ, name, tbl, sql in master:
        if typ != "table":
            continue
        cols = []
        try:
            for cid, cname, ctype, notnull, dflt, pk in cur.execute(
                    'pragma table_info("%s")' % name.replace('"', '""')):
                cols.append({"cid": cid, "name": cname,
                             "declared_type": ctype or None,
                             "not_null": bool(notnull),
                             "default": dflt, "primary_key": pk})
        except sqlite3.Error as exc:
            cols = [{"error": str(exc)}]
        fks = []
        try:
            for r in cur.execute('pragma foreign_key_list("%s")'
                                 % name.replace('"', '""')):
                fks.append({"table": r[2], "from": r[3], "to": r[4],
                            "on_update": r[5], "on_delete": r[6]})
        except sqlite3.Error:
            pass
        rows = None
        if row_counts:
            try:
                rows = cur.execute('select count(*) from "%s"'
                                   % name.replace('"', '""')).fetchone()[0]
            except sqlite3.Error:
                rows = None
        out["tables"].append({
            "name": name,
            "subsystem": group_of(name),
            "subsystem_classification": "heuristic:curated_table_group_map",
            "columns": cols,
            "column_count": len(cols),
            "foreign_keys": fks,
            "index_count": idx.get(name, 0),
            "row_count": rows,
            "create_sql": sql,
        })
    con.close()
    return out


def rocksdb_facts(dirpath):
    files = []
    kinds = collections.Counter()
    total = 0
    for f in sorted(os.listdir(dirpath)):
        p = os.path.join(dirpath, f)
        if not os.path.isfile(p):
            continue
        size = os.path.getsize(p)
        total += size
        ext = os.path.splitext(f)[1].lower() or f
        kinds[ext] += 1
        files.append({"name": f, "bytes": size})
    opt = None
    for f in sorted(os.listdir(dirpath), reverse=True):
        if f.startswith("OPTIONS-"):
            with open(os.path.join(dirpath, f), encoding="utf-8",
                      errors="replace") as fh:
                txt = fh.read()
            sections = re.findall(r"^\[(.+?)\]", txt, re.M)
            ver = re.search(r"rocksdb_version=(\S+)", txt)
            cfs = re.findall(r'^\[CFOptions "(.+?)"\]', txt, re.M)
            opt = {"options_file": f,
                   "rocksdb_version": ver.group(1) if ver else None,
                   "sections": sections,
                   "column_families": cfs}
            break
    return {"classification": "parsed", "path": dirpath,
            "engine": "RocksDB (identified by CURRENT/MANIFEST/OPTIONS/"
                      "IDENTITY/*.sst/*.blob layout and the OPTIONS header)",
            "total_bytes": total, "file_kinds": dict(kinds),
            "files": files, "options": opt}


def previews_facts(dirpath, cap=6000):
    shards = 0
    preview_files = 0
    scanned = 0
    truncated = False
    sizes = collections.Counter()
    sample = []
    pat = re.compile(r"^(.+?)-([0-9a-f]{32})_(\d+)$")
    for dp, dn, fn in os.walk(dirpath):
        if dp != dirpath:
            shards += 1
        for f in fn:
            scanned += 1
            m = pat.match(f)
            if m:
                preview_files += 1
                sizes[m.group(3)] += 1
                if len(sample) < 5:
                    sample.append(f)
        if scanned >= cap:
            truncated = True
            break
    return {"classification": "parsed", "path": dirpath,
            "shard_directories_scanned": shards,
            "directory_entries_scanned": scanned,
            "scan_truncated_at_cap": truncated,
            "scan_cap": cap,
            "preview_pyramid_files_seen": preview_files,
            "filename_grammar": "<image-uuid>-<32 hex digest>_<long edge px>",
            "filename_grammar_classification":
                "derived:regex matched every counted file",
            "long_edge_sizes_observed": dict(sizes.most_common(20)),
            "sample_filenames": sample,
            "count_caveat": "directory-entry counts are a scan sample capped "
                            "to avoid hydrating cloud-backed storage; they are "
                            "not a total file count"}


def find_catalogs(prefs_path):
    found = []
    if not os.path.isfile(prefs_path):
        return found, "preferences file not found: %s" % prefs_path
    with open(prefs_path, encoding="utf-8", errors="replace") as fh:
        txt = fh.read()
    for m in re.finditer(r'([A-Za-z]:\\+(?:[^"\\]|\\+)+?\.lrcat)', txt):
        # the preferences file is Lua source inside a quoted Lua string, so
        # every path separator arrives multiply escaped; collapse it.
        p = os.path.normpath(re.sub(r"\\{2,}", "\\\\", m.group(1)))
        if p not in found:
            found.append(p)
    return found, None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--install",
                    default=r"C:\Program Files\Adobe\Adobe Lightroom Classic")
    ap.add_argument("--user",
                    default=os.path.expandvars(r"%APPDATA%\Adobe\Lightroom"))
    ap.add_argument("--catalog", default=None,
                    help="explicit .lrcat; otherwise discovered from prefs")
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    errors = []
    notes = []

    def stage(msg):
        print("[stage] %s" % msg, file=sys.stderr, flush=True)

    # ---- does the install ship a template catalog? ------------------------
    stage("scanning install for a shipped catalog")
    shipped = []
    for dp, _d, fn in os.walk(args.install):
        for f in fn:
            if f.lower().endswith((".lrcat", ".lrcat-data")):
                shipped.append(os.path.join(dp, f))

    # ---- locate a real catalog -------------------------------------------
    prefs = os.path.join(args.user, "Preferences",
                         "Lightroom Classic CC 7 Preferences.agprefs")
    discovered, perr = find_catalogs(prefs)
    if perr:
        errors.append({"stage": "catalog_discovery", "error": perr})
    catalog = args.catalog
    if not catalog:
        existing = [c for c in discovered if os.path.isfile(c)]
        if existing:
            # newest catalog is the live one; older ones are upgrade sources
            catalog = max(existing, key=os.path.getmtime)
            notes.append({
                "note": "catalog selected by newest mtime among the paths "
                        "named in the preferences file",
                "candidates": [{"path": c,
                                "mtime": _dt.datetime.fromtimestamp(
                                    os.path.getmtime(c),
                                    _dt.timezone.utc).isoformat(),
                                "bytes": os.path.getsize(c)}
                               for c in existing],
                "selected": catalog,
            })
    if catalog and not os.path.isfile(catalog):
        errors.append({"stage": "catalog_open",
                       "error": "path from preferences does not exist: %s"
                                % catalog})
        catalog = None

    core = None
    bundle = {}
    if catalog and is_cloud_placeholder(catalog):
        errors.append({"stage": "catalog_open",
                       "error": "catalog is a dehydrated cloud placeholder; "
                                "not opened to avoid forcing a download: %s"
                                % catalog})
        catalog = None
    if catalog:
        stage("reading core schema: %s" % catalog)
        try:
            core = dump_sqlite(catalog)
        except Exception as exc:  # noqa: BLE001
            errors.append({"stage": "catalog_schema",
                           "error": "%s: %s" % (type(exc).__name__, exc)})
        base = catalog[:-len(".lrcat")]
        data_dir = catalog + "-data"
        stage("big data store: %s" % data_dir)
        if os.path.isdir(data_dir):
            try:
                bundle["big_data_store"] = rocksdb_facts(data_dir)
            except Exception as exc:  # noqa: BLE001
                errors.append({"stage": "lrcat-data",
                               "error": "%s: %s" % (type(exc).__name__, exc)})
        for suffix, key in ((" Previews.lrdata", "previews"),
                            (" Smart Previews.lrdata", "smart_previews"),
                            (" Helper.lrdata", "helper")):
            d = base + suffix
            if not os.path.isdir(d):
                continue
            stage("bundle part: %s" % key)
            rec = {"classification": "parsed", "path": d, "databases": [],
                   "skipped_cloud_placeholders": []}
            for f in sorted(os.listdir(d)):
                if not f.lower().endswith(".db"):
                    continue
                fp = os.path.join(d, f)
                if is_cloud_placeholder(fp):
                    rec["skipped_cloud_placeholders"].append({
                        "file": f, "bytes": os.path.getsize(fp),
                        "reason": "dehydrated cloud placeholder; opening it "
                                  "would force a full download, so it was not "
                                  "opened"})
                    continue
                try:
                    rec["databases"].append(dump_sqlite(fp, row_counts=False))
                except Exception as exc:  # noqa: BLE001
                    errors.append({"stage": key + ":" + f,
                                   "error": "%s: %s" % (
                                       type(exc).__name__, exc)})
            if key == "previews":
                rec.update(previews_facts(d))
            elif key == "smart_previews":
                n = 0
                scanned = 0
                for dp, _dn, fn in os.walk(d):
                    for f in fn:
                        scanned += 1
                        if f.lower().endswith(".dng"):
                            n += 1
                    if scanned >= 6000:
                        break
                rec["lossy_dng_proxies_seen"] = n
                rec["scan_capped"] = scanned >= 6000
                rec["format_note"] = ("Smart Previews are lossy-compressed DNG "
                                      "renditions used for offline develop")
            bundle[key] = rec

    # ---- cloud/XMP asset schema shipped in the install --------------------
    xmp_schema_path = os.path.join(args.install, "Resources",
                                   "xmp_schema.json")
    xmp_schema = None
    if os.path.isfile(xmp_schema_path):
        try:
            with open(xmp_schema_path, encoding="utf-8") as fh:
                raw = json.load(fh)
            ns = {}
            for k, v in raw.items():
                if k.startswith("_") or not isinstance(v, dict):
                    continue
                ns[k] = {
                    "namespace": v.get("_namespace"),
                    "fields": sorted(f for f in v if not f.startswith("_")),
                }
            xmp_schema = {
                "classification": "parsed",
                "file": "Resources/xmp_schema.json",
                "purpose": raw.get("__implementation_notes__", [])[:1],
                "datatypes": sorted(k for k in raw.get("_datatypes", {})
                                    if not k.startswith("_")),
                "namespaces": ns,
                "raw": raw,
            }
        except Exception as exc:  # noqa: BLE001
            errors.append({"stage": "xmp_schema",
                           "error": "%s: %s" % (type(exc).__name__, exc)})

    # ---- SQL recovered from Library.lrmodule ------------------------------
    stage("mining Library.lrmodule for SQL")
    sql = collections.Counter()
    libmod = os.path.join(args.install, "Library.lrmodule")
    SQLRE = re.compile(r"^\s*(select|insert|update|delete|create|pragma|"
                       r"replace|with|drop|alter)\b", re.I)
    if os.path.isfile(libmod):
        try:
            _m, order, _s = lrbin.mine(libmod)
            for s in order:
                if SQLRE.match(s) and len(s) > 30:
                    sql[" ".join(s.split())[:400]] += 1
        except Exception as exc:  # noqa: BLE001
            errors.append({"stage": "library_sql",
                           "error": "%s: %s" % (type(exc).__name__, exc)})

    by_sub = collections.Counter()
    total_cols = 0
    if core:
        for t in core["tables"]:
            by_sub[t["subsystem"]] += 1
            total_cols += t["column_count"]

    doc = {
        "schema_id": SCHEMA_ID,
        "generated_at": _dt.datetime.now(_dt.timezone.utc).isoformat(),
        "method": {
            "mode": "offline_static_parse",
            "app_launched": False,
            "writes_to_source": "none; every SQLite file opened with "
                                "mode=ro&immutable=1",
            "sources": [
                {"id": "catalog_core", "classification": "parsed",
                 "path": catalog,
                 "how_located": "parsed out of the Lightroom preferences file "
                                "at %s" % prefs},
                {"id": "catalog_bundle_siblings", "classification": "parsed",
                 "members": sorted(bundle)},
                {"id": "install_xmp_schema", "classification": "parsed",
                 "path": xmp_schema_path},
                {"id": "library_module_sql", "classification": "parsed",
                 "path": libmod},
            ],
            "classification_legend": {
                "parsed": "read directly out of a shipped or user file",
                "derived": "computed from parsed data",
                "heuristic": "this tool's judgement",
            },
        },
        "handshake_context": {
            "statement": "Handshake forbids SQLite. This file models the "
                         "catalog CONCEPT - the entity set, the relationships, "
                         "and the bundle decomposition - not a storage "
                         "instruction. Table and column names are evidence of "
                         "what a professional catalog must track.",
        },
        "install_ships_template_catalog": {
            "classification": "parsed",
            "answer": bool(shipped),
            "files_found_in_install": shipped,
            "note": "No .lrcat ships with the product. A real catalog had to "
                    "be located on the machine to read a schema at all.",
        },
        "catalog_bundle_shape": {
            "classification": "derived:observed on the inspected catalog",
            "parts": [
                {"suffix": ".lrcat", "engine": "SQLite 3",
                 "role": "relational core: images, files, folders, metadata, "
                         "keywords, collections, develop settings, history, "
                         "publish, cloud sync"},
                {"suffix": ".lrcat-data/", "engine": "RocksDB",
                 "role": "key-value store for large per-image payloads that "
                         "do not belong in a row"},
                {"suffix": " Previews.lrdata/",
                 "engine": "SQLite index + sharded JPEG files",
                 "role": "multi-resolution preview pyramid cache"},
                {"suffix": " Smart Previews.lrdata/", "engine": "lossy DNG",
                 "role": "offline-editable proxy renditions"},
                {"suffix": " Helper.lrdata/", "engine": "SQLite FTS5",
                 "role": "full-text search indexes and metadata worklists"},
            ],
        },
        "counts": {
            "catalog_tables": len(core["tables"]) if core else 0,
            "catalog_columns_total": total_cols,
            "catalog_indexes": core["index_count"] if core else 0,
            "catalog_triggers": len(core["triggers"]) if core else 0,
            "catalog_views": len(core["views"]) if core else 0,
            "catalog_tables_by_subsystem_heuristic": dict(by_sub),
            "bundle_parts_present": sorted(bundle),
            "sidecar_databases": sum(len(v.get("databases", []))
                                     for v in bundle.values()
                                     if isinstance(v, dict)),
            "xmp_schema_namespaces": (len(xmp_schema["namespaces"])
                                      if xmp_schema else 0),
            "sql_statements_recovered_from_Library_lrmodule": len(sql),
            "catalogs_discovered_in_preferences": len(discovered),
        },
        "catalogs_discovered": discovered,
        "catalog_core_schema": core,
        "catalog_bundle": bundle,
        "asset_xmp_schema": xmp_schema,
        "recovered_catalog_sql": {
            "classification": "parsed",
            "note": "verbatim SQL constants from Library.lrmodule; shows the "
                    "real access patterns against the schema above",
            "statements": [{"sql": k, "occurrences": v}
                           for k, v in sql.most_common(300)],
        },
        "notes": notes,
        "errors": errors,
    }

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)
    print(json.dumps(doc["counts"], indent=1))
    for e in errors:
        print("ERR", e, file=sys.stderr)


if __name__ == "__main__":
    main()
