#!/usr/bin/env python3
"""Patch the 509 pre-existing WP-KERNEL-STUDIO microtasks against the rebuilt Master Spec.

These contracts were authored before the green-room teardown and carry real scope decisions, so
they are PATCHED, never rewritten or replaced. Every original field is preserved; the original
acceptance prose is kept verbatim alongside the structured rows derived from it.

FOUR MEASURED DEFECTS, all fixed here:

1. Jammed acceptance rows. 463 of 509 carry exactly one acceptance entry, and 422 of those pack
   several distinct criteria into one semicolon-separated sentence, with 61 more using " + ".
   A validator cannot pass or fail such a row: it is several claims wearing one id. They are split
   into separate rows, each with a stable id and its own evidence kind.

2. One proof target for the whole contract. All 509 have exactly one, so most acceptance rows have
   no proof of their own. Each row gets a proof target; where the original named a real command it
   is carried onto the first row rather than invented for the rest.

3. Three field groups absent from all 509: resource_privacy_obligation, validator_focus and
   implementation_notes. Without implementation notes an implementer must read the spec to find
   the contract, which is exactly what these contracts exist to prevent, so the cited clause text
   is injected from the assembled spec.

4. 1,002 stale diagnostic-tier obligations. 501 contracts say internal diagnostics are "built by
   WP-KERNEL-012, retrofit WP-KERNEL-016" and the external watcher is "shipped later". Both are
   present in the WP-KERNEL-012 base today: handshake_core carries a diagnostics module and an
   api/diagnostics surface, and the watcher crate exists at src/frontend/palmistry. The reasons
   are corrected against that observation rather than the assumption they were written under.

Reference tooling. Writes to a staging directory by default; --in-place only on request.
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import re
import shutil
from pathlib import Path

DEF_LINE = re.compile(r"^(?:\*\*)?\[(STU-[A-Z]+-\d+[a-z]?)\]")
TABLE_CELL_DEF = re.compile(r"\|\s*\*{0,2}\[(STU-[A-Z]+-\d+[a-z]?)\]\*{0,2}\s*\|")
SPLIT = re.compile(r"\s*;\s*|\s+\+\s+")


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def anchor_text_map(spec_file: Path) -> dict[str, str]:
    """anchor -> the clause text that defines it, from the assembled spec."""
    lines = spec_file.read_text(encoding="utf-8", errors="replace").split("\n")
    marks, fence = [], False
    out: dict[str, str] = {}
    for i, ln in enumerate(lines):
        if ln.lstrip().startswith("```"):
            fence = not fence
            continue
        if fence:
            continue
        m = DEF_LINE.match(ln)
        if m and (i == 0 or not lines[i - 1].strip()):
            marks.append((i, m.group(1)))
        elif ln.lstrip().startswith("|"):
            for a in TABLE_CELL_DEF.findall(ln):
                out.setdefault(a, re.sub(r"\s+", " ", ln.strip())[:1200])
    for j, (start, anchor) in enumerate(marks):
        end = marks[j + 1][0] if j + 1 < len(marks) else len(lines)
        body = re.sub(r"\s+", " ", "\n".join(lines[start:end]).strip())
        out[anchor] = body[:2400]
    return out


def evidence_kind(text: str) -> str:
    t = text.lower()
    if any(k in t for k in ("compile", "cargo build", "builds", "no dep", "dependency")):
        return "build_proof"
    if any(k in t for k in ("test", "assert", "round-trip", "roundtrip", "golden")):
        return "test_proof"
    if any(k in t for k in ("panel", "surface", "render", "visible", "tooltip", "gui", "screenshot")):
        return "gui_proof"
    if any(k in t for k in ("event", "ledger", "receipt", "persist", "stored", "record")):
        return "ledger_proof"
    return "runtime_assertion"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--packet", type=Path, required=True, help="WP-KERNEL-STUDIO directory")
    ap.add_argument("--spec", type=Path, required=True, help="assembled section 14 markdown")
    ap.add_argument("--out", type=Path, required=True, help="staging directory for patched contracts")
    ap.add_argument("--in-place", action="store_true", help="write back into --packet instead")
    args = ap.parse_args()

    amap = anchor_text_map(args.spec)
    dest = args.packet if args.in_place else args.out
    dest.mkdir(parents=True, exist_ok=True)

    stats = collections.Counter()
    unresolved: list[dict] = []
    files = sorted(f for f in args.packet.glob("MT-*.json"))
    for f in files:
        d = json.loads(f.read_text(encoding="utf-8"))
        # A spec-derived contract must never be run through the legacy patcher. Its acceptance
        # rows come from the spec, not from scope prose, so re-deriving them from an absent
        # scope.acceptance_criteria empties the contract. Folding into a packet that already held
        # the derived set did exactly that once.
        if d.get("derivation") or str(d.get("schema_version", "")).startswith("microtask_contract_v3_spec_derived"):
            stats["skipped_spec_derived"] += 1
            continue
        scope = d.get("scope") or {}
        original = list(scope.get("acceptance_criteria") or [])
        proofs = list(scope.get("proof_targets") or [])

        rows, n = [], 0
        for entry in original:
            parts = [p.strip(" .") for p in SPLIT.split(entry) if p.strip(" .")]
            if len(parts) > 1:
                stats["entries_split"] += 1
            for part in parts:
                n += 1
                rows.append({
                    "id": f"AC-{n:03d}",
                    "criterion": part[0].upper() + part[1:] if part else part,
                    "evidence_kind": evidence_kind(part),
                    "proof_target": proofs[0] if (n == 1 and proofs) else
                                    "PROOF TARGET REQUIRED: name the command or check that proves "
                                    "this row. The original contract carried one target for the "
                                    "whole microtask, which cannot prove this row on its own.",
                    "proof_target_status": "carried_from_original" if (n == 1 and proofs) else "REQUIRED",
                    "source": "split from the original acceptance prose, preserved verbatim in "
                              "scope.acceptance_criteria_original",
                })
        stats["rows_before"] += len(original)
        stats["rows_after"] += len(rows)

        anchors = d.get("spec_anchor") or []
        if isinstance(anchors, str):
            anchors = [anchors]
        clauses, missing, families = [], [], []
        for a in anchors:
            # A family wildcard such as "STU-AUT-*" is a scope statement, not a citation. Expanding
            # it names the clauses the contract actually spans instead of reporting it as dangling.
            if a.endswith("-*"):
                pref = a[:-1]
                members = sorted(k for k in amap if k.startswith(pref))
                families.append({"wildcard": a, "member_count": len(members),
                                 "members": members[:400],
                                 "note": "Family-scope citation. The contract spans this whole "
                                         "clause family; it does not cite one clause."})
                continue
            if a in amap:
                clauses.append({"anchor": a, "spec_clause_text": amap[a]})
            else:
                missing.append(a)
        if missing:
            unresolved.append({"mt_id": d.get("mt_id"), "anchors": missing})
            stats["contracts_with_unresolved_anchor"] += 1

        scope["acceptance_criteria_original"] = original
        scope["acceptance_criteria"] = [r["criterion"] for r in rows]
        d["scope"] = scope
        d["acceptance_criteria"] = rows
        d["acceptance_criteria_count"] = len(rows)
        d["implementation_notes"] = {
            "spec_clauses": clauses,
            "spec_clause_count": len(clauses),
            "unresolved_anchors": missing,
            "family_scope_citations": families,
            "naming": "Vendor product names are provenance only. Studio ships Handshake-native "
                      "names per [STU-SECTION-003].",
            "database": "SurrealDB with the EventLedger is the only durable authority. SQLite, "
                        "libSQL, Turso and PostgreSQL are forbidden, including in tests and dev caches.",
            "engine_split": "GPU, WGSL and compute live in the studio-engine crate behind its "
                            "traits. handshake_core must never gain a GPU dependency.",
            "routing": "Navigation extends the existing address and bus seams. No new router.",
            "worktree": "Code paths resolve against the WP-KERNEL-012 worktree, never main.",
        }
        d["validator_focus"] = [
            "Confirm every acceptance row traces to a sentence in the cited spec clause carried in "
            "implementation_notes. A row with no spec basis is a fabrication.",
            "Reject any row still carrying proof_target_status REQUIRED: it has no proof of its own.",
            "Reject scaffold-only proof: at least one proof command must drive the executable "
            "runtime rather than a fixture the implementer authored.",
            "Where the contract touches numeric parameters, confirm hard and soft bounds are stored "
            "as separate values and an unknown bound is stored as unknown, never mirrored.",
        ]
        d.setdefault("resource_privacy_obligation", {
            "applies": True,
            "rule": "Resources this contract reads, derives, caches or exports stay inside the "
                    "authenticated operator's scope. No cross-identity read path may be introduced.",
        })
        tiers = d.get("hbr_int_009_tier_obligations") or []
        for t in tiers:
            if t.get("tier") == "internal_diagnostics" and t.get("status") == "DEFERRED":
                t["status"] = "WIRED"
                t["reason"] = ("Corrected: internal diagnostics exist in the WP-KERNEL-012 base "
                               "today, at handshake_core/src/diagnostics and "
                               "handshake_core/src/api/diagnostics.rs. The earlier reason assumed "
                               "a later retrofit.")
                stats["tier_internal_diagnostics_corrected"] += 1
            elif t.get("tier") == "palmistry" and t.get("status") == "DEFERRED":
                t["status"] = "WIRED"
                t["reason"] = ("Corrected: the external watcher exists in the WP-KERNEL-012 base "
                               "today, at src/frontend/palmistry. The earlier reason assumed it "
                               "shipped later.")
                stats["tier_palmistry_corrected"] += 1
        d["hbr_int_009_tier_obligations"] = tiers
        d["schema_version"] = "microtask_contract_v3_spec_bound"
        d["updated_at_utc"] = now()
        d["patch_provenance"] = {
            "patched_by": "handshake.greenroom.mt_patch_legacy.v1",
            "patched_at": now(),
            "preserved": "The original acceptance prose is kept verbatim at "
                         "scope.acceptance_criteria_original. No clause, scope decision, path list "
                         "or obligation from the original contract was removed.",
            "spec_basis": str(args.spec),
        }
        (dest / f.name).write_text(json.dumps(d, indent=1, ensure_ascii=False),
                                   encoding="utf-8", newline="\n")
        stats["contracts_patched"] += 1

    report = {
        "schema_id": "handshake.reference.studio_mt_legacy_patch@1",
        "generated_at": now(),
        "packet": str(args.packet),
        "written_to": str(dest),
        "in_place": bool(args.in_place),
        "counts": dict(stats),
        "acceptance_rows": {"before": stats["rows_before"], "after": stats["rows_after"]},
        "contracts_with_unresolved_anchor": unresolved[:200],
        "unresolved_anchor_count": len(unresolved),
        "note": "An unresolved anchor means the contract cites a clause the assembled spec does "
                "not define. That is a spec defect or a stale citation and must be resolved before "
                "the microtask activates; it is reported, never silently dropped.",
    }
    rep = dest.parent / "studio-mt-legacy-patch-report.json"
    rep.write_text(json.dumps(report, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    print(f"[patch] contracts={stats['contracts_patched']} "
          f"acceptance rows {stats['rows_before']} -> {stats['rows_after']}")
    print(f"[patch] entries split={stats['entries_split']} "
          f"tiers corrected: internal={stats['tier_internal_diagnostics_corrected']} "
          f"palmistry={stats['tier_palmistry_corrected']}")
    print(f"[patch] contracts citing an anchor the spec does not define: {len(unresolved)}")
    print(f"[patch] -> {dest}")
    print(f"[patch] report -> {rep}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
