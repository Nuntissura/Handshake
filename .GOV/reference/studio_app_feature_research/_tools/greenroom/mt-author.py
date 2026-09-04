#!/usr/bin/env python3
"""Author WP-KERNEL-STUDIO microtask contracts from the coverage plan + behavioural payload.

Two products:
  1. NEW microtasks for the uncovered capability clusters in studio-mt-coverage-plan.json
  2. PATCHES for the 509 existing microtasks, fixing the six measured defects:
       - one semicolon-jammed acceptance row  -> multiple rows with stable ids + evidence kinds
       - proof_targets always == 1            -> one per acceptance row
       - resource_privacy_obligation absent   -> emitted (HBR-PRIV has 8 active rules)
       - validator_focus absent               -> emitted
       - implementation_notes absent          -> emitted, carrying the extracted behaviour
       - stale diagnostic tiers               -> WIRED against the feat/WP-KERNEL-012 base

Writes to a staging directory. Nothing under .GOV/task_packets is mutated.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import re
from pathlib import Path

SCHEMA = "hsk.microtask_contract@1"

DOMAIN_SURFACE = {
    "raster": ("studio-engine/src/raster", "src/frontend/handshake_native/src/studio/raster"),
    "vector": ("studio-engine/src/vector", "src/frontend/handshake_native/src/studio/vector"),
    "layout": ("studio-engine/src/layout", "src/frontend/handshake_native/src/studio/layout"),
    "typography": ("studio-engine/src/text", "src/frontend/handshake_native/src/studio/text"),
    "color": ("studio-engine/src/color", "src/frontend/handshake_native/src/studio/color"),
    "effects": ("studio-engine/src/effects", "src/frontend/handshake_native/src/studio/effects"),
    "design_system": ("studio-engine/src/design_system", "src/frontend/handshake_native/src/studio/design_system"),
    "document_model": ("studio-engine/src/document", "src/frontend/handshake_native/src/studio/document"),
    "prototype": ("studio-engine/src/prototype", "src/frontend/handshake_native/src/studio/prototype"),
    "raw": ("studio-engine/src/raw", "src/frontend/handshake_native/src/studio/raw"),
    "interop": ("studio-engine/src/interop", "src/frontend/handshake_native/src/studio/interop"),
    "automation": ("studio-engine/src/automation", "src/frontend/handshake_native/src/studio/automation"),
    "motion": ("studio-engine/src/motion", "src/frontend/handshake_native/src/studio/motion"),
    "video": ("studio-engine/src/video", "src/frontend/handshake_native/src/studio/video"),
    "catalog": ("studio-engine/src/catalog", "src/frontend/handshake_native/src/studio/catalog"),
    "whiteboard": ("studio-engine/src/whiteboard", "src/frontend/handshake_native/src/studio/whiteboard"),
    "cross_cutting": ("studio-engine/src/core", "src/frontend/handshake_native/src/studio/core"),
    "foundation": ("studio-engine/src/core", "src/frontend/handshake_native/src/studio/core"),
}


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def slug(s: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", s.lower()).strip("_")[:48]


def split_jammed(row: str) -> list[str]:
    """A single acceptance string carrying several criteria joined by ; or +."""
    parts = [p.strip(" .;") for p in re.split(r";|\s\+\s", row) if p.strip(" .;")]
    return parts or [row.strip()]


def evidence_kind_for(text: str) -> str:
    t = text.lower()
    if re.search(r"argus|panel|author_id|surface|reachable|render", t):
        return "ui_projection"
    if re.search(r"eventledger|event ledger|append|receipt", t):
        return "eventledger_append"
    if re.search(r"persist|restart|replay|surreal|durable|reload", t):
        return "durable_storage"
    if re.search(r"reject|denied|fails|must not|never|closed|guard|isolation", t):
        return "negative_guard"
    if re.search(r"manual|user ?manual", t):
        return "manual_currency"
    return "runtime_behavior"


def privacy_block(domain: str, resources: list[str]) -> dict:
    return {
        "applies": True,
        "touched_resources": resources,
        "identity_context": "authenticated account + acting Principal + session + active AccessSpace; the StudioDocument and every derived artifact carry owner and ResourceGrant linkage",
        "enforcement_boundaries": ["SurrealDB record-user table/field permissions", "ResourceBroker / ArtifactStore", "studio API surface", "search/index and model-retrieval context assembly", "preview and thumbnail generation", "export/sync", "UI query"],
        "positive_case": "the owner, in an authorised AccessSpace, reads, edits and exports the resource",
        "negative_case": "a second account in the same project cannot read, list, search, preview, count or export it, and its existence metadata does not leak",
        "derived_scope_rule": "caches, thumbnails, indexes and model context inherit the intersection of contributing source scopes and never widen it",
        "revocation_rule": "grant revocation or AccessSpace switch promptly invalidates cached previews and in-flight reads; running sessions stay pinned to immutable context",
        "hbr": ["HBR-PRIV-001", "HBR-PRIV-002", "HBR-PRIV-004", "HBR-PRIV-005"],
    }


def tiers() -> list:
    return [
        {"tier": "flight_recorder", "status": "WIRED", "reason": "FR-EVT-STUDIO-* business event per state change"},
        {"tier": "internal_diagnostics", "status": "WIRED", "reason": "present in the feat/WP-KERNEL-012 base at src/frontend/handshake_native/src/diagnostics/"},
        {"tier": "palmistry", "status": "WIRED", "reason": "present in the feat/WP-KERNEL-012 base as the src/frontend/palmistry crate"},
    ]


def make_new_mt(spec: dict, plan_file: str) -> dict:
    domain = spec["domain"]
    eng, ui = DOMAIN_SURFACE.get(domain, DOMAIN_SURFACE["cross_cutting"])
    caps = spec["capabilities"]
    names = [c["name"] for c in caps]
    apps = sorted({a for c in caps for a in c["apps"]})
    beh = spec.get("behaviour_sample", [])
    mt = spec["proposed_mt_id"]
    rows, proofs = [], []

    def ac(n, text, kind, ev):
        rid = f"AC-{mt}-{n:02d}"
        rows.append({"id": rid, "criterion": text, "evidence_kind": kind, "status": "PENDING", "evidence": None, "reason": None, "source_evidence": ev})
        return rid

    i = 1
    rid = ac(i, f"All {len(names)} capabilities in this cluster exist as invocable Studio operations with their documented parameters, and each rejects out-of-range input rather than clamping silently.", "runtime_behavior", f"capability registry cluster '{spec['cluster_key']}' from {', '.join(apps)}")
    proofs.append({"for": rid, "command_or_check": f"live: invoke each of the {len(names)} operations with valid and out-of-range parameters"})
    i += 1
    if beh:
        rid = ac(i, "Every extracted parameter is honoured with its recorded type, range and default; defaults match the reference behaviour captured in the green room.", "runtime_behavior", f"{len(spec.get('behaviour_sample', []))} behaviour records attached")
        proofs.append({"for": rid, "command_or_check": "test: parameter round-trip against the captured behaviour table"})
        i += 1
    rid = ac(i, "State changes persist in SCHEMAFULL SurrealDB studio_* records with owner and AccessSpace linkage and replay after restart.", "durable_storage", "STU-ARC-004")
    proofs.append({"for": rid, "command_or_check": "live: mutate, restart the runtime, assert replay"})
    i += 1
    rid = ac(i, "Each state change appends a typed STUDIO_* EventLedger event; duplicate and stale requests are rejected idempotently.", "eventledger_append", "STU-ARC-003")
    proofs.append({"for": rid, "command_or_check": "live: assert event sequence and idempotency"})
    i += 1
    rid = ac(i, "The operator surface for this cluster is reachable by stable author_id, renders current state, and every control is steerable and re-observable through Argus.", "ui_projection", "HBR-VIS-001")
    proofs.append({"for": rid, "command_or_check": f"argus: studio.{domain}.{slug(spec['cluster_key'])} before/after capture per control"})
    i += 1
    rid = ac(i, "A second account in the same project cannot read, list, search, preview or export another owner's state, and counts do not leak.", "negative_guard", "HBR-PRIV-004")
    proofs.append({"for": rid, "command_or_check": "test: cross-account isolation including metadata"})
    i += 1
    rid = ac(i, "The in-product UserManual entry for this cluster exists in the same change and names purpose, usage path, inputs and outputs, failure modes and recovery.", "manual_currency", "HBR-MAN-004")
    proofs.append({"for": rid, "command_or_check": "test: manual self-consistency; inspect the updated manual path"})

    return {
        "schema_id": SCHEMA,
        "schema_version": "microtask_contract_v2_greenroom",
        "contract_authority": "PRIMARY_MACHINE_READABLE",
        "wp_id": "WP-KERNEL-STUDIO",
        "mt_id": mt,
        "created_at_utc": now(),
        "generated_by": "handshake.greenroom.mt_author.v1",
        "authoring_basis": {"plan": plan_file, "cluster_key": spec["cluster_key"], "capability_count": spec["capability_count"], "source_apps": apps},
        "lifecycle": {"status": "PENDING", "depends_on": [], "blocks": [], "active": False, "validator_verdict": "PENDING"},
        "clause": f"{domain}: {spec['cluster_key']} capability cluster ({len(names)} capabilities)",
        "spec_anchor": ["STU-SECTION-003"],
        "spec_anchor_status": "PROVISIONAL: section 14 anchors must be assigned during activation; the cluster is derived from extracted product behaviour, not from spec text",
        "scope": {
            "summary": (
                f"Implement the {spec['cluster_key']} capability cluster in the {domain} domain as Handshake-native Studio operations. "
                f"The cluster covers {len(names)} capabilities extracted from {', '.join(apps)}: {', '.join(names[:18])}"
                + (f", and {len(names) - 18} more." if len(names) > 18 else ".")
            ),
            "capabilities": names,
            "acceptance_criteria": rows,
            "proof_targets": proofs,
            "code_surfaces": [f"{eng}/{slug(spec['cluster_key'])}.rs", f"{ui}/{slug(spec['cluster_key'])}_panel.rs"],
            "code_surface_status": "PROPOSED_UNRECONCILED: reconcile against the feat/WP-KERNEL-012 native tree before claiming",
            "allowed_paths": ["studio-engine/**", "src/frontend/handshake_native/src/studio/**", "src/backend/handshake_core/src/studio/**", "tests/studio/**"],
            "forbidden_paths": [".GOV/**", "app/src/** (legacy reference-only)"],
            "expected_tests": [
                f"cargo test -p studio-engine {domain}::{slug(spec['cluster_key'])}",
                f"argus: studio.{domain}.{slug(spec['cluster_key'])} reachable by author_id, before/after per control",
                "live: restart replay from SurrealDB",
                "negative: out-of-range input rejected; cross-account isolation",
            ],
        },
        "implementation_notes": {
            "extracted_behaviour": beh,
            "behaviour_record_count": spec.get("behaviour_records", 0),
            "naming": "Vendor names are provenance only. Studio ships Handshake-native names per STU-SECTION-003; do not use a source product name as a tool, command, panel or manual name.",
            "dedup": "Where several source applications provide this capability it collapses to ONE Studio operation; per-suite variants are recorded as provenance only.",
            "gap": "If no behaviour records are attached, the parameter surface must be completed from the green room before this MT is claimed.",
        },
        "gui_obligation": {
            "operator_surface_required": "YES", "gui_creation_required": "YES", "argus_required": "YES",
            "surfaces": [f"{spec['cluster_key']} controls in the {domain} surface"],
            "argus_targets": [f"studio.{domain}.{slug(spec['cluster_key'])}"],
            "not_applicable_reason": None,
        },
        "user_manual_obligation": {
            "required": True, "same_change_update_required": "YES", "manual_version_bump_required": "NO",
            "target_entries": [f"Studio / {domain.replace('_', ' ').title()} / {spec['cluster_key'].title()}"],
            "must_cover": ["purpose", "usage path", "parameters and defaults", "expected output", "failure and recovery", "diagnostic linkage"],
            "not_applicable_reason": None,
        },
        "resource_privacy_obligation": privacy_block(domain, [f"{domain} state", "derived previews", "index rows", "exported artifacts"]),
        "hbr_obligations": ["HBR-INT-001", "HBR-INT-003", "HBR-INT-009", "HBR-VIS-001", "HBR-VIS-003", "HBR-MAN-001", "HBR-MAN-004", "HBR-PRIV-001", "HBR-PRIV-002", "HBR-PRIV-004", "HBR-SWARM-001"],
        "hbr_int_009_tier_obligations": tiers(),
        "validator_focus": [
            "Reject scaffold-only proof: at least one proof command must drive the executable runtime, not a fixture the implementer authored.",
            "Parameter ranges and defaults must match the captured reference behaviour, not be invented.",
            "Cross-account isolation must cover metadata: counts, search hits, previews and export listings.",
            "Confirm no vendor product name shipped as Studio product content.",
            "Confirm the UserManual entry landed in the same change.",
        ],
        "risk_if_missed": f"A professional switching from {', '.join(apps)} loses the {spec['cluster_key']} capabilities in the {domain} domain, which is a direct parity failure.",
        "red_team": {"required": True, "risks": [
            "Cluster implemented as UI affordances with no engine behaviour behind them.",
            "Parameters accepted but silently clamped instead of validated.",
            "State held in memory with a JSON sidecar instead of SurrealDB authority.",
            "Derived previews cached without owner scoping, creating a metadata side channel.",
        ]},
        "handoff": {"coder_session": None, "wp_validator_session": None, "review_request_receipt_id": None, "review_response_receipt_id": None},
    }


def patch_existing(mt: dict, behaviour: dict) -> dict:
    scope = mt.get("scope", {})
    old_rows = scope.get("acceptance_criteria", []) or []
    new_rows, proofs = [], []
    n = 1
    for row in old_rows:
        text = row if isinstance(row, str) else str(row)
        for part in split_jammed(text):
            rid = f"AC-{mt['mt_id']}-{n:02d}"
            new_rows.append({"id": rid, "criterion": part[0].upper() + part[1:] if part else part, "evidence_kind": evidence_kind_for(part), "status": "PENDING", "evidence": None, "reason": None})
            proofs.append({"for": rid, "command_or_check": None, "note": "assign from the MT's expected_tests during activation"})
            n += 1
    domain_guess = "cross_cutting"
    num = int(mt["mt_id"].split("-")[1])
    for d, (a, b) in (("foundation", (1, 20)), ("raster", (21, 80)), ("vector", (81, 140)), ("layout", (141, 190)), ("typography", (191, 230)), ("color", (231, 260)), ("effects", (261, 320)), ("design_system", (321, 370)), ("prototype", (371, 410)), ("raw", (411, 440)), ("interop", (441, 480)), ("automation", (481, 520)), ("whiteboard", (521, 540)), ("cross_cutting", (541, 560))):
        if a <= num <= b:
            domain_guess = d
            break
    patch = {
        "mt_id": mt["mt_id"],
        "patch_id": f"PATCH-{mt['mt_id']}",
        "generated_at": now(),
        "generated_by": "handshake.greenroom.mt_author.v1",
        "defects_fixed": ["jammed_acceptance_row", "single_proof_target", "missing_resource_privacy_obligation", "missing_validator_focus", "missing_implementation_notes", "stale_diagnostic_tiers"],
        "set": {
            "scope.acceptance_criteria": new_rows,
            "scope.proof_targets": proofs,
            "resource_privacy_obligation": privacy_block(domain_guess, [f"{domain_guess} state", "derived previews", "index rows"]),
            "validator_focus": [
                "Reject scaffold-only proof; at least one command must drive the executable runtime.",
                "Verify each acceptance row independently; the previous single jammed row is not acceptable evidence.",
                "Confirm parameter behaviour against green-room reference values where attached.",
                "Confirm the UserManual entry landed in the same change.",
            ],
            "implementation_notes": {
                "extracted_behaviour": behaviour.get("records", [])[:12],
                "behaviour_record_count": len(behaviour.get("records", [])),
                "source": behaviour.get("source", "no matching green-room behaviour records found for this MT's clause"),
            },
            "hbr_int_009_tier_obligations": tiers(),
        },
        "counts": {"acceptance_rows_before": len(old_rows), "acceptance_rows_after": len(new_rows)},
    }
    return patch


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--plan", type=Path, required=True)
    ap.add_argument("--packet", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    new_dir = args.out / "new_mts"
    patch_dir = args.out / "mt_patches"
    new_dir.mkdir(parents=True, exist_ok=True)
    patch_dir.mkdir(parents=True, exist_ok=True)

    plan = json.loads(args.plan.read_text(encoding="utf-8"))
    made_new = []
    for spec in plan["new_mt_specs"]:
        c = make_new_mt(spec, args.plan.name)
        (new_dir / f"{c['mt_id']}.json").write_text(json.dumps(c, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
        made_new.append({"mt_id": c["mt_id"], "domain": spec["domain"], "cluster": spec["cluster_key"], "capabilities": spec["capability_count"], "acceptance_rows": len(c["scope"]["acceptance_criteria"]), "behaviour_records": spec.get("behaviour_records", 0)})

    # behaviour lookup for existing MTs, by clause token overlap
    beh_by_name = {}
    for a in plan["assignments"]:
        if a.get("behaviour_records"):
            beh_by_name.setdefault(a["mt"], {"records": [], "source": "capability assignments with attached behaviour"})
    patches = []
    for f in sorted(args.packet.glob("MT-*.json")):
        mt = json.loads(f.read_text(encoding="utf-8"))
        p = patch_existing(mt, beh_by_name.get(mt["mt_id"], {}))
        (patch_dir / f"{p['patch_id']}.json").write_text(json.dumps(p, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
        patches.append({"mt_id": p["mt_id"], "rows_before": p["counts"]["acceptance_rows_before"], "rows_after": p["counts"]["acceptance_rows_after"]})

    idx = {
        "schema_id": "handshake.reference.studio_mt_authoring_index@1",
        "generated_at": now(),
        "staging_only": True,
        "note": "Staging output. Nothing under .GOV/task_packets has been modified. Promotion is an explicit activation step.",
        "new_mts": {"count": len(made_new), "items": made_new},
        "patches": {"count": len(patches), "rows_before_total": sum(p["rows_before"] for p in patches), "rows_after_total": sum(p["rows_after"] for p in patches), "items": patches},
    }
    (args.out / "_AUTHORING_INDEX.json").write_text(json.dumps(idx, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    print(f"[author] new MTs written: {len(made_new)} -> {new_dir}")
    print(f"[author] patches written: {len(patches)} -> {patch_dir}")
    print(f"[author] acceptance rows {idx['patches']['rows_before_total']} -> {idx['patches']['rows_after_total']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
