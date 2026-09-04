#!/usr/bin/env python3
"""Regenerate WP-KERNEL-STUDIO microtask contracts at implementable depth from green-room evidence.

Fixes the six defects measured across the existing 509 contracts:
  1. one semicolon-jammed acceptance row  -> multiple rows, each with a stable AC id + evidence_kind
  2. proof_targets always == 1            -> one per acceptance row
  3. resource_privacy_obligation absent   -> emitted (HBR-PRIV has 8 active rules)
  4. validator_focus absent               -> emitted
  5. implementation_notes absent          -> emitted, carrying REAL captured counts/categories/names
  6. stale diagnostic tier reasons        -> internal_diagnostics/palmistry are present in the 012 base

Evidence source: .GOV/reference/studio_app_feature_research/_greenroom_20260903/
Writes contracts to a staging dir; nothing under .GOV/task_packets is touched.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import re
from pathlib import Path

SCHEMA = "hsk.microtask_contract@1"


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def slug(s: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", s.lower()).strip("_")[:48]


def load_evidence(gr: Path) -> dict:
    off = gr / "installed_exports" / "affinity" / "offline"
    presets = json.loads((off / "presets_names_scan.json").read_text(encoding="utf-8"))
    api = json.loads((off / "scripting_api_surface.json").read_text(encoding="utf-8"))
    inv = json.loads((gr / "fold_back" / "affinity-offline-inventory.json").read_text(encoding="utf-8"))
    fam = {}
    for f in presets.get("files", []):
        name = Path(f["file"]).stem
        cats = []
        seen = set()
        for c in f.get("categories_guess", []):
            cn = c.get("name")
            if cn and cn not in seen and c.get("kind") in ("category", "tree_group"):
                seen.add(cn)
                cats.append({"name": cn, "entries": c.get("entries")})
        fam[name] = {
            "file": f["file"],
            "bytes": f.get("bytes"),
            "name_count": f.get("candidate_name_count", 0),
            "categories": cats,
            "sample_names": f.get("candidate_names", [])[:24],
            "header_hex": f.get("header_hex", "")[:32],
        }
    return {"preset_families": fam, "api": api, "inventory": inv}


def ac(mt: str, n: int, text: str, kind: str, evidence: str) -> dict:
    return {"id": f"AC-{mt}-{n:02d}", "criterion": text, "evidence_kind": kind, "status": "PENDING", "evidence": None, "reason": None, "source_evidence": evidence}


def privacy(resources: list[str], note: str) -> dict:
    return {
        "applies": True,
        "touched_resources": resources,
        "identity_context": "authenticated account + acting Principal + session + active AccessSpace; StudioDocument and every derived preset/thumbnail/index row carry owner + ResourceGrant linkage",
        "enforcement_boundaries": ["SurrealDB record-user table/field permissions", "ResourceBroker / ArtifactStore", "studio API surface", "search/index + model-retrieval context assembly", "preview/thumbnail generation", "export/sync", "UI query"],
        "positive_case": "owner in an authorised AccessSpace reads, edits, and exports the resource",
        "negative_case": "a second account in the same project cannot read, list, search, preview, count, or export it; existence metadata does not leak",
        "derived_scope_rule": "thumbnails, indexes, caches, and model context inherit the intersection of contributing source scopes and never widen",
        "revocation_rule": "grant revocation or AccessSpace switch invalidates cached previews and in-flight reads promptly; running sessions stay pinned to immutable context",
        "note": note,
        "hbr": ["HBR-PRIV-001", "HBR-PRIV-002", "HBR-PRIV-004", "HBR-PRIV-005"],
    }


def build_preset_family_mt(mt_id: str, fam_key: str, fam: dict, studio_surface: str, domain: str, anchors: list[str]) -> dict:
    cats = fam["categories"]
    cat_line = ", ".join(f"{c['name']} ({c['entries']})" for c in cats[:14]) or "uncategorised"
    n = fam["name_count"]
    rows = [
        ac(mt_id, 1, f"The {studio_surface} registry ships {n} stock entries organised into the captured category set and every entry is listable, previewable, and applicable.", "runtime_behavior", f"propcol:{fam_key} name_count={n}"),
        ac(mt_id, 2, "Categories support create, rename, reorder, duplicate, delete-with-contents-policy, import, and export as typed commands.", "runtime_behavior", "affinity string table: Preset Manager / Presets panel command set"),
        ac(mt_id, 3, "A user entry is created from current settings, renamed, reordered across categories, and deleted, with name-collision handled by the documented policy rather than silent overwrite.", "runtime_behavior", "affinity strings: 'The preset name you gave is already used by another preset' / '...by one of the stock presets'"),
        ac(mt_id, 4, "Stock entries are immutable: an edit to a stock entry forks a user copy and the stock entry is preserved.", "negative_guard", "affinity strings: stock-preset name-collision message"),
        ac(mt_id, 5, f"Every entry and category row persists in SCHEMAFULL SurrealDB studio_* records with owner/AccessSpace linkage and replays after restart; a STUDIO_* EventLedger event is appended per mutation.", "eventledger_append", "STU-ARC-003 / STU-ARC-004"),
        ac(mt_id, 6, f"The {studio_surface} panel is reachable by stable author_id, renders the category tree and entry grid, and each command is steerable and re-observable through Argus.", "ui_projection", "HBR-VIS-001"),
        ac(mt_id, 7, "A second account in the same project cannot list, search, preview, or export another owner's user entries, and entry counts do not leak.", "negative_guard", "HBR-PRIV-004"),
        ac(mt_id, 8, "Import of a malformed or truncated preset package fails closed with a typed unsupported-feature receipt and a recovery instruction; no partial registry mutation is committed.", "negative_guard", "STU-VAL-* recovery contract"),
    ]
    return {
        "schema_id": SCHEMA,
        "schema_version": "microtask_contract_v2_greenroom",
        "contract_authority": "PRIMARY_MACHINE_READABLE",
        "wp_id": "WP-KERNEL-STUDIO",
        "mt_id": mt_id,
        "created_at_utc": now(),
        "generated_by": "handshake.greenroom.mt_regen.v1",
        "evidence_provenance": {
            "capture": ".GOV/reference/studio_app_feature_research/_greenroom_20260903/installed_exports/affinity/offline/presets_names_scan.json",
            "source_app": "Affinity 3.2.3.4646 (offline package extraction; app never launched)",
            "source_file": fam["file"],
            "source_bytes": fam["bytes"],
            "note": "Vendor data is provenance only. Studio ships Handshake-native names and its own stock content per STU-SECTION-003.",
        },
        "lifecycle": {"status": "PENDING", "depends_on": [], "blocks": [], "active": False, "validator_verdict": "PENDING"},
        "clause": f"{studio_surface}: registry, categories, stock set, user entries, panel, persistence [{','.join(anchors)}]",
        "spec_anchor": anchors,
        "scope": {
            "summary": (
                f"Implement the canonical {studio_surface} preset registry as one Studio primitive: a two-level category/entry tree, a stock set, user entries, "
                f"and the full panel operation set. The reference implementation ships {n} stock entries across {len(cats)} categories ({cat_line}). "
                f"Entries and categories are durable SurrealDB records with EventLedger mutation events, owner-scoped, importable and exportable as a package."
            ),
            "acceptance_criteria": rows,
            "proof_targets": [
                {"for": r["id"], "command_or_check": t}
                for r, t in zip(rows, [
                    f"live: open the {studio_surface} panel, assert stock entry count and category structure",
                    "live: exercise every category command; assert tree state after each",
                    "live: create from current settings, rename, move, delete; collision path asserted",
                    "test: stock entry edit forks a user copy; stock row unchanged",
                    "live: mutate, restart the runtime, assert registry replay + EventLedger event sequence",
                    "argus: panel reachable by author_id; before/after capture per command",
                    "test: cross-account isolation incl. counts, search hits, and previews",
                    "test: malformed package import fails closed with receipt; registry unchanged",
                ])
            ],
            "code_surfaces": [
                f"studio-engine/src/presets/{fam_key}.rs",
                "studio-engine/src/presets/registry.rs",
                f"src/frontend/handshake_native/src/studio/presets/{fam_key}_panel.rs",
                "src/backend/handshake_core/src/studio/presets.rs",
            ],
            "code_surface_status": "PROPOSED_UNRECONCILED: paths must be reconciled against the feat/WP-KERNEL-012 native tree before claiming this MT",
            "allowed_paths": ["studio-engine/**", "src/frontend/handshake_native/src/studio/**", "src/backend/handshake_core/src/studio/**", "tests/studio/**"],
            "forbidden_paths": [".GOV/**", "app/src/** (legacy reference-only)"],
            "expected_tests": [
                f"cargo test -p studio-engine presets::{fam_key}",
                f"argus: studio.presets.{fam_key}_panel category tree + entry grid, before/after per command",
                "live: restart replay of registry state from SurrealDB",
                "negative: cross-account isolation + malformed import",
            ],
        },
        "implementation_notes": {
            "reference_structure": {"stock_entry_count": n, "category_count": len(cats), "categories": cats, "sample_entry_names": fam["sample_names"]},
            "container_format": f"Reference stock content is stored in a Serif KA container ({fam['file']}, header {fam['header_hex']}). Studio does not adopt that format; it ships its own SurrealDB-backed registry and a Handshake-native package format for import/export.",
            "naming": "Do not ship vendor category or entry names verbatim as Studio product content. Use the captured structure to size the registry, the category model, and the panel; author Studio's own stock set.",
            "panel_operations": ["create category", "rename category", "reorder", "duplicate", "delete (with contents policy)", "import presets to category", "export preset", "create entry from current settings", "rename entry", "move entry", "delete entry", "reset to stock"],
            "collision_policy": "user entry name collision prompts; stock entry name collision forks a user copy",
        },
        "gui_obligation": {
            "operator_surface_required": "YES",
            "gui_creation_required": "YES",
            "argus_required": "YES",
            "surfaces": [f"{studio_surface} panel", "category tree", "entry grid", "import/export dialog"],
            "argus_targets": [f"studio.presets.{fam_key}_panel", f"studio.presets.{fam_key}_category_tree", f"studio.presets.{fam_key}_entry_grid"],
            "not_applicable_reason": None,
        },
        "user_manual_obligation": {
            "required": True,
            "same_change_update_required": "YES",
            "manual_version_bump_required": "NO",
            "target_entries": [f"Studio / Presets / {studio_surface}"],
            "must_cover": ["purpose", "panel path", "create/apply/organise workflow", "import/export", "collision and reset behavior", "failure and recovery", "diagnostic linkage"],
            "not_applicable_reason": None,
        },
        "resource_privacy_obligation": privacy(
            [f"{studio_surface} registry rows", "user preset entries", "category rows", "entry thumbnails", "imported preset packages", "search index rows"],
            "User-authored presets are owner-scoped resources; stock content is installation-scoped and readable by every authenticated account.",
        ),
        "hbr_obligations": ["HBR-INT-001", "HBR-INT-003", "HBR-INT-009", "HBR-VIS-001", "HBR-VIS-003", "HBR-MAN-001", "HBR-MAN-004", "HBR-PRIV-001", "HBR-PRIV-002", "HBR-PRIV-004", "HBR-SWARM-001"],
        "hbr_int_009_tier_obligations": [
            {"tier": "flight_recorder", "status": "WIRED", "reason": "FR-EVT-STUDIO-PRESET-* business events on every registry mutation"},
            {"tier": "internal_diagnostics", "status": "WIRED", "reason": "present in the feat/WP-KERNEL-012 base at src/frontend/handshake_native/src/diagnostics/; emit registry load time + panel frame cost"},
            {"tier": "palmistry", "status": "WIRED", "reason": "present in the feat/WP-KERNEL-012 base as the src/frontend/palmistry crate; large registry loads are watched for UI-thread stall"},
        ],
        "validator_focus": [
            "Registry state must survive a real runtime restart from SurrealDB, not from an in-memory fixture.",
            "Stock immutability must be proven by attempting a stock edit, not asserted in prose.",
            "Cross-account isolation must cover metadata: counts, search hits, thumbnails, and export listings.",
            "Reject the MT if the panel is not Argus-observable or if proof is fixture-only (Spec-Realism Gate sub-rule 2).",
            "Confirm no vendor category or entry name shipped as Studio product content.",
        ],
        "risk_if_missed": f"{studio_surface} degrades to a flat unmanaged list: no categories, no user entries, no durable owner-scoped persistence, and no import/export path, which blocks professional asset reuse across documents and projects.",
        "red_team": {
            "required": True,
            "risks": [
                "Registry implemented as an in-memory Vec with a JSON sidecar instead of SurrealDB authority.",
                "Thumbnails generated into a shared cache without owner scoping (metadata side channel).",
                "Large stock registry loaded synchronously on the UI thread, stalling the shell.",
                "Import path trusting package contents and partially mutating the registry before validation.",
            ],
        },
        "handoff": {"coder_session": None, "wp_validator_session": None, "review_request_receipt_id": None, "review_response_receipt_id": None},
    }


FAMILY_PLAN = [
    ("raster_brushes", "Raster Brush Library", "raster", ["STU-RAS-020", "STU-DS-010"]),
    ("vector_brushes", "Vector Brush Library", "vector", ["STU-VEC-030", "STU-DS-010"]),
    ("fills", "Swatch and Fill Library", "color", ["STU-COL-008", "STU-COL-009"]),
    ("adjustments", "Adjustment Preset Library", "raster", ["STU-RAS-030", "STU-COL-012"]),
    ("objectstyles", "Object Style Library", "design_system", ["STU-DS-004"]),
    ("assets", "Asset Library", "design_system", ["STU-DS-011"]),
    ("tool_settings", "Tool Preset Library", "cross_cutting", ["STU-UNI-006"]),
    ("tone_map", "Tone Map Preset Library", "raw", ["STU-RAW-012"]),
    ("grid_presets", "Grid and Guide Preset Library", "layout", ["STU-LAY-020"]),
    ("macros", "Macro Library", "automation", ["STU-AUT-004"]),
    ("livemask_presets", "Live Mask Preset Library", "raster", ["STU-RAS-016"]),
    ("cross_reference_presets", "Cross-Reference Format Library", "layout", ["STU-LAY-040"]),
]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--greenroom", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--start", type=int, default=600)
    args = ap.parse_args()
    ev = load_evidence(args.greenroom)
    args.out.mkdir(parents=True, exist_ok=True)
    made, index = 0, []
    n = args.start
    for key, surface, domain, anchors in FAMILY_PLAN:
        fam = ev["preset_families"].get(key)
        if not fam or not fam["name_count"]:
            print(f"  skip {key}: no captured names")
            continue
        mt_id = f"MT-{n:03d}"
        c = build_preset_family_mt(mt_id, key, fam, surface, domain, anchors)
        (args.out / f"{mt_id}.json").write_text(json.dumps(c, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
        index.append({"mt_id": mt_id, "clause": c["clause"], "domain": domain, "acceptance_rows": len(c["scope"]["acceptance_criteria"]), "stock_entries": fam["name_count"], "categories": len(fam["categories"])})
        made += 1
        n += 1
    (args.out / "_REGEN_INDEX.json").write_text(json.dumps({"schema_id": "hsk.microtask_index@1", "generated_at_utc": now(), "generator": "handshake.greenroom.mt_regen.v1", "staging_only": True, "note": "Staging output. Not yet promoted into .GOV/task_packets/WP-KERNEL-STUDIO.", "mt_count": made, "microtasks": index}, indent=1), encoding="utf-8", newline="\n")
    print(f"[regen] wrote {made} contracts to {args.out}")
    for r in index:
        print(f"   {r['mt_id']}  {r['acceptance_rows']} AC rows  stock={r['stock_entries']:4d} cats={r['categories']:2d}  {r['clause'][:70]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
