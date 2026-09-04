#!/usr/bin/env python3
"""Reconcile each spec module's OWN declared microtask yield against what the deriver extracts.

The operator's acceptance test is that the same microtask set falls out of the Master Spec. There
are therefore two numbers per module and they must agree:

  DECLARED   the module's Microtask Derivation sub-section states a yields index ending in a total
  DERIVED    spec-derive-microtasks.py extracts units from the module text and counts them

A gap means one of two defects, and the direction tells you which:

  DECLARED > DERIVED   the module claims work that its own text does not express in a derivable
                       form. The behaviour exists in the author's head, not in the spec. Left
                       alone it silently disappears from the work.
  DERIVED > DECLARED   the deriver is splitting something the module treats as one unit, or the
                       ledger undercounts. Either the ledger or the tool is wrong.

Neither is fixed here. This reports the deltas so a proofreading pass can resolve each one
against the module text.

Reference tooling. Read-only over the staged modules; writes one report.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import re
from pathlib import Path

# The ledger sits either under a markdown heading or inside a numbered clause; both are in use.
DERIV_ANCHOR = re.compile(
    r"^(?:#{2,5}\s+.*microtask derivation|\*{0,2}\[STU-[A-Z]+-\d+[a-z]?\]\*{0,2}\s*\*{0,2}"
    r"microtask derivation)", re.I | re.M)
# A total row carries several numbers, typically "| Module total | | 73 | 77 |" where 73 is the
# clause count and 77 the yields. The yields column is last, so take the LAST number in the row.
TOTAL_ROW = re.compile(r"^\|\s*\**\s*(?:module\s+|grand\s+)?total\b.*$", re.I | re.M)
CELL_NUM = re.compile(r"(\d[\d,]*)")
PROSE_TOTAL = re.compile(r"yields?\s+(?:exactly\s+)?(\d[\d,]*)\s+microtasks", re.I)


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def declared_total(text: str) -> tuple[int | None, str]:
    """Pull the module's own stated yields total out of its derivation sub-section."""
    m = DERIV_ANCHOR.search(text)
    if not m:
        return None, "no Microtask Derivation sub-section"
    tail = text[m.start():]
    rows = TOTAL_ROW.findall(tail)
    # A module may carry more than one total row, for example a module total and a subtotal such
    # as "TOTAL TABLE UNITS". Only a bare total names the module's yield; taking the last row read
    # a subtotal of 97 as typography's total against its real 211.
    bare = [r for r in rows
            if re.fullmatch(r"(module\s+|grand\s+)?total", r.strip("| ").split("|")[0].strip(" *"), re.I)]
    pick = bare[-1] if bare else (rows[-1] if rows else None)
    if pick:
        nums = CELL_NUM.findall(pick)
        if nums:
            return int(nums[-1].replace(",", "")), "yields-index total row, last numeric column"
    pr = PROSE_TOTAL.findall(tail)
    if pr:
        return int(pr[0].replace(",", "")), "stated in prose in the derivation clause"
    return None, "derivation sub-section present but states no total"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--modules", type=Path, required=True)
    ap.add_argument("--manifest", type=Path, required=True,
                    help="studio-mt-derivation-manifest.json from spec-derive-microtasks.py")
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()

    derived = json.loads(args.manifest.read_text(encoding="utf-8"))["per_module_yields"]
    rows, agree, missing, gap_total = [], 0, 0, 0
    for p in sorted(args.modules.glob("*.md")):
        text = p.read_text(encoding="utf-8", errors="replace")
        dec, how = declared_total(text)
        der = derived.get(p.name, 0)
        if dec is None:
            missing += 1
        delta = None if dec is None else der - dec
        if delta == 0:
            agree += 1
        if delta is not None:
            gap_total += abs(delta)
        rows.append({
            "module": p.name,
            "declared_by_module": dec,
            "declared_source": how,
            "derived_by_tool": der,
            "delta_derived_minus_declared": delta,
            "reading": (
                "no declared total to reconcile against" if dec is None else
                "agree" if delta == 0 else
                ("the module claims work its own text does not express in a derivable form, so that "
                 "work silently disappears unless the text is restated or the ledger corrected")
                if delta < 0 else
                ("the deriver splits something the module counts as one unit, or the ledger "
                 "undercounts; one of the two is wrong")),
        })

    report = {
        "schema_id": "handshake.reference.studio_ledger_reconcile@1",
        "generated_at": now(),
        "why": "The Master Spec must yield the microtask set. A module whose declared yield and "
               "derived yield disagree breaks that property in one direction or the other.",
        "modules_total": len(rows),
        "modules_in_agreement": agree,
        "modules_without_a_declared_total": missing,
        "absolute_gap_across_modules": gap_total,
        "per_module": rows,
        "verdict": "PASS" if agree == len(rows) else "FAIL",
    }
    args.out.write_text(json.dumps(report, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    print(f"[ledger] modules={len(rows)} agree={agree} no_total={missing} abs_gap={gap_total}")
    for r in rows:
        d = r["delta_derived_minus_declared"]
        flag = "OK " if d == 0 else ("?? " if d is None else "GAP")
        print(f"  {flag} {r['module'][:34]:36} declared={str(r['declared_by_module']):>6} "
              f"derived={r['derived_by_tool']:>6} delta={str(d):>6}")
    print(f"[ledger] -> {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
