#!/usr/bin/env python3
"""Fold the patched and spec-derived microtasks into the WP-KERNEL-STUDIO packet.

This is the last step. Everything before it happens in the green-room staging tree; this is the
only tool that writes into .GOV/task_packets.

WHAT LANDS

  patched legacy    the 509 pre-existing contracts, patched against the rebuilt spec. Their
                    original acceptance prose, scope decisions, path lists and obligations are
                    preserved; the jammed rows are split and the missing field groups added.
  spec-derived      one contract per derivation unit the Master Spec declares, so the set is a
                    function of the spec rather than an independent artefact.

The packet is BACKED UP before anything is written, and the backup path is printed. Nothing is
deleted: a legacy contract with no spec-derived counterpart stays exactly where it is.

The index and the activation-readiness record are rewritten to describe what is actually present,
so a no-context model opening the packet sees the real state rather than a stale one.
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import shutil
from pathlib import Path


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--packet", type=Path, required=True)
    ap.add_argument("--patched", type=Path, required=True, help="patched legacy contracts")
    ap.add_argument("--derived", type=Path, required=True, help="spec-derived contracts")
    ap.add_argument("--backup-root", type=Path, required=True)
    ap.add_argument("--spec-version", default="v02.206")
    ap.add_argument("--reports", nargs="*", type=Path, default=[])
    args = ap.parse_args()

    if not args.packet.exists():
        print(f"[fold] FATAL: packet {args.packet} does not exist")
        return 2

    # --- back up first. Never write over governance state without a restorable copy. ----------
    stamp = dt.datetime.now().strftime("%Y%m%d-%H%M%S")
    backup = args.backup_root / f"WP-KERNEL-STUDIO-before-greenroom-fold-{stamp}"
    backup.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(args.packet, backup)
    print(f"[fold] backup -> {backup}")

    stats = collections.Counter()

    for f in sorted(args.patched.glob("MT-*.json")):
        shutil.copy2(f, args.packet / f.name)
        stats["legacy_patched"] += 1

    for f in sorted(args.derived.glob("MT-*.json")):
        if (args.packet / f.name).exists():
            # An id collision would overwrite a legacy contract. The derived band starts well
            # above the legacy range, so this should never fire; if it does, keep the legacy file
            # and report, rather than silently losing a contract that carries scope decisions.
            stats["collision_skipped"] += 1
            continue
        shutil.copy2(f, args.packet / f.name)
        stats["spec_derived_added"] += 1

    # --- rebuild the index over what is actually on disk --------------------------------------
    entries, by_domain, by_unit, anchors = [], collections.Counter(), collections.Counter(), set()
    for f in sorted(args.packet.glob("MT-*.json")):
        try:
            d = json.loads(f.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            stats["unreadable"] += 1
            continue
        a = d.get("spec_anchor") or []
        if isinstance(a, str):
            a = [a]
        anchors.update(x for x in a if isinstance(x, str) and not x.endswith("-*"))
        unit = (d.get("derivation") or {}).get("unit_kind", "legacy_patched")
        by_unit[unit] += 1
        by_domain[d.get("domain", "unassigned")] += 1
        entries.append({
            "mt_id": d.get("mt_id"), "file": f.name, "clause": (d.get("clause") or "")[:220],
            "spec_anchor": a, "domain": d.get("domain"), "unit_kind": unit,
            "acceptance_criteria_count": d.get("acceptance_criteria_count",
                                               len(d.get("acceptance_criteria") or [])),
            "status": (d.get("lifecycle") or {}).get("status"),
        })

    index = {
        "schema_id": "hsk.mt_index@2",
        "wp_id": "WP-KERNEL-STUDIO",
        "generated_at_utc": now(),
        "generated_by": "handshake.greenroom.mt_fold.v1",
        "spec_basis": f"Master Spec {args.spec_version} section 14, proposed bundle awaiting operator promotion",
        "derivation_property": (
            "Every contract whose unit_kind is not legacy_patched was derived mechanically from "
            "the Master Spec by spec-derive-microtasks.py. Re-running that tool against an "
            "unchanged spec reproduces the same set with the same ids, which is the operator's "
            "stated acceptance test for this work."),
        "totals": {
            "microtasks": len(entries),
            "by_unit_kind": dict(by_unit.most_common()),
            "by_domain": dict(by_domain.most_common()),
            "distinct_spec_anchors_cited": len(anchors),
            "acceptance_rows": sum(e["acceptance_criteria_count"] or 0 for e in entries),
        },
        "entries": entries,
    }
    (args.packet / "_MT_INDEX.json").write_text(
        json.dumps(index, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")

    # --- refresh activation readiness so it describes the real state --------------------------
    ar_path = args.packet / "ACTIVATION_READINESS.json"
    ar = json.loads(ar_path.read_text(encoding="utf-8")) if ar_path.exists() else {}
    ar["updated_at_utc"] = now()
    ar["state_source"] = "RECOMPUTED_FROM_GREENROOM_SPEC_DERIVATION"
    ar["verdict"] = "READY_FOR_OPERATOR_REVIEW_OF_SPEC_PROPOSAL"
    ar["ready_for_downstream_launch"] = "NO"
    ar["ready_for_downstream_launch_reason"] = (
        f"The Studio section was rebuilt from an offline teardown of the installed applications "
        f"and assembled as PROPOSED Master Spec {args.spec_version}. That bundle is not promoted: "
        f"SPEC_CURRENT still points at v02.205, and promotion is the operator's decision. The "
        f"microtask set is complete and spec-derived, but no microtask may activate against a "
        f"proposed spec. Operator steps, in order: review the proposed bundle, promote it and move "
        f"SPEC_CURRENT, promote this stub to an official packet, register it in BUILD_ORDER and the "
        f"traceability registry, then create the worktree and branch.")
    ar["greenroom_fold"] = {
        "at": now(),
        "microtasks_total": len(entries),
        "legacy_patched": stats["legacy_patched"],
        "spec_derived_added": stats["spec_derived_added"],
        "backup_of_previous_packet": str(backup),
        "reports": [str(r) for r in args.reports],
    }
    ar_path.write_text(json.dumps(ar, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")

    print(f"[fold] legacy patched={stats['legacy_patched']} "
          f"spec-derived added={stats['spec_derived_added']} "
          f"collisions skipped={stats['collision_skipped']}")
    print(f"[fold] packet now holds {len(entries)} microtasks citing "
          f"{len(anchors)} distinct spec anchors, {index['totals']['acceptance_rows']} acceptance rows")
    print(f"[fold] index -> {args.packet / '_MT_INDEX.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
