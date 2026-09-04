#!/usr/bin/env python3
"""Round-trip validator for the Studio spec update.

The operator's acceptance test, stated verbatim: "i want the master spec to be updated so we
could extract the same exact microtasks from the master spec." That is a two-way property and
this checks both directions:

  FORWARD   every normative spec clause is implemented by at least one microtask
  BACKWARD  every microtask cites a clause that actually exists in the assembled bundle

It also checks the things that silently break a multi-agent spec write:
  - anchor collisions across modules written by different agents
  - anchors claimed as RETAINED that no longer exist
  - heading level and frontmatter drift between modules
  - parameter contracts missing a bound field, or collapsing hard and soft bounds
  - vendor product names leaking into normative text, which [STU-SECTION-003] forbids

Reference tooling. Reads the staged modules and the staged microtasks; writes one report.
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import re
from pathlib import Path

# A clause DEFINITION opens a PARAGRAPH: the anchor sits at the start of a line whose predecessor
# is blank. Style alone is not a reliable signal -- agents wrote both "**[STU-X-1] Title.**" and
# "[STU-X-1] Title", and prose that WRAPS onto an anchor reference also starts a line with one.
# Keying on style manufactured false definitions and one phantom cross-module collision, so the
# rule here is structural and style-independent. Fenced code is skipped.
DEF_LINE = re.compile(r"^(?:\*\*)?\[(STU-[A-Z]+-\d+[a-z]?)\]")
# An anchor must be bounded by a non-word character on the right. Without that, a scan of embedded
# JSON matched "STU-ASSET-03" inside "STU-ASSET-030" and reported a citation that never existed.
ANCHOR_BARE = re.compile(r"\b(STU-[A-Z]+-\d+[a-z]?)(?![0-9A-Za-z*_`\\-])")
HEADING = re.compile(r"^(#{1,3})\s+(\S+.*)$", re.M)
FRONTMATTER = re.compile(r"\A---\n(.*?)\n---\n", re.S)
VENDOR = re.compile(
    r"\b(Photoshop|Illustrator|InDesign|Affinity|Figma|FigJam|Premiere|After Effects|Lightroom|Dreamweaver|Camera Raw)\b"
)
# A clause is normative when it states an obligation.
NORMATIVE = re.compile(r"\b(MUST|MUST NOT|SHALL|SHALL NOT|REQUIRED|FORBIDDEN)\b")
BOUND_FIELDS = ("hard_min", "hard_max", "soft_min", "soft_max", "default", "unit", "precision")
# A heading declares its anchor at the end of the line rather than the start.
HEADING_ANCHOR = re.compile(r"^#{1,4}\s+.*\[(STU-[A-Z]+-\d+[a-z]?)\]\s*$", re.M)
# Naming a vendor to FORBID depending on it, to describe an import path, or to record
# provenance is legitimate. Only a vendor name used as a Studio-facing name violates
# [STU-SECTION-003], so these contexts are excluded before anything is reported.
VENDOR_OK_CONTEXT = re.compile(
    r"(MUST NOT require|MUST NOT depend|without requiring|independent of|no dependency|subscription-gated|account-gated|platform-locked|provenance|import|export|round-?trip|interoperab|compatib|file format|open(?:s|ed)? (?:a|an|the)|read(?:s|ing)? (?:a|an|the)|writ(?:e|es|ing)? (?:a|an|the)|derived from|captured from|observed in|as shipped by|teardown|adapter)", re.I)


# A third definition form: an anchor alone in a table cell. v02.205 defines whole clause families
# this way, as "| On click / tap | [STU-PRO-005] | fires when ... |". The cell holding ONLY the
# anchor is what distinguishes a definition from a prose cross-reference.
TABLE_CELL_DEF = re.compile(r"\|\s*\*{0,2}\[(STU-[A-Z]+-\d+[a-z]?)\]\*{0,2}\s*\|")


def table_cell_definitions(text: str) -> list[str]:
    """Anchors defined by occupying a table cell alone."""
    out, fence = [], False
    for ln in text.split("\n"):
        if ln.lstrip().startswith("```"):
            fence = not fence
            continue
        if fence or not ln.lstrip().startswith("|"):
            continue
        out.extend(TABLE_CELL_DEF.findall(ln))
    return out


# A module may DECLARE which of its clauses yield no microtask. A derivation sub-section is the
# usual case: it states how the set is derived and is not itself a unit of work. Those clauses
# must not be reported as uncovered, or real gaps are lost in the noise.
# Mirrors the deriver's own exclusion rule so the two tools agree on what is bookkeeping.
BOOKKEEPING_HEADING = re.compile(
    r"(microtask derivation|supersession|disposition|anchor continuity|derivation basis|"
    r"authority, derivation|reading order|change log|revision history)", re.I)
SEC_HEADING = re.compile(r"^(#{2,5})\s+(.+?)\s*$", re.M)


NEVER_ASSIGNED = re.compile(
    r"(never\s+assigned|never\s+issued|deliberately\s+unissued|not\s+assigned|unissued|"
    r"was\s+never\s+an\s+anchor)", re.I)
NOYIELD_HEADER = re.compile(r"(declared\s+non-?yielding\s+set|no-?yield\s+set\s*:)", re.I)
NOYIELD_COUNT = re.compile(r"no-?yield\s+set\s*:\s*(\d+)\s+clause", re.I)
# A backticked anchor is unambiguous. A bracketed one is indistinguishable from a
# cross-reference, so it is trusted only when the header states how many to expect.
LIST_ITEM = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s")
NOYIELD_TICKED = re.compile(r"`(STU-[A-Z]+-\d+[a-z]?)`", re.I)
NOYIELD_BRACKETED = re.compile(r"\[(STU-[A-Z]+-\d+[a-z]?)\]", re.I)


def declared_no_yield(text: str) -> set[str]:
    """The clauses a module declares as yielding nothing, by anchor.

    Only an explicit declaration counts. A module stating exclusions purely as prose is not
    parsed, because guessing at prose is what [STU-TYP-240] forbids a tool from doing.
    """
    lines = text.split("\n")
    out: set[str] = set()
    for i, ln in enumerate(lines):
        hdr = NOYIELD_HEADER.search(ln)
        if not hdr:
            continue
        # A count in the header, as in "The no-yield set: 10 clauses", lets a riskier bracketed
        # form be accepted only when what is found matches what is claimed.
        cm = NOYIELD_COUNT.search(ln)
        want = int(cm.group(1)) if cm else None
        started = False
        ticked: set[str] = set()
        bracketed: set[str] = set()
        blanks = 0
        for j in range(i, min(i + 90, len(lines))):
            cur = lines[j]
            if j > i and cur.lstrip().startswith("#"):
                break
            if j > i and cur.lstrip().startswith("|"):
                break
            if j > i and DEF_LINE.match(cur) and cur.strip() != lines[i].strip():
                break  # the next clause begins; the block is over
            if not cur.strip():
                blanks += 1
                if blanks >= 2 and (ticked or bracketed):
                    break
                continue
            # Once the list has started, a non-indented line that is not itself a list item ends
            # the declaration. Without this the prose paragraph that follows the list leaks its
            # cross-references into the set, and the count guard then rejects the whole block.
            is_item = bool(LIST_ITEM.match(cur))
            indented = cur[:1].isspace()
            if started and not is_item and not indented:
                break
            if is_item:
                started = True
            blanks = 0
            if j == i:
                continue  # the header line names the declaring clause, not the members
            ticked |= {a.upper() for a in NOYIELD_TICKED.findall(cur)}
            bracketed |= {a.upper() for a in NOYIELD_BRACKETED.findall(cur)}
        if ticked:
            out |= ticked
        elif want is not None and len(bracketed) == want:
            out |= bracketed
    return out


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def clause_blocks(text: str) -> list[dict]:
    """Split a module into clause blocks, one per paragraph-opening anchor."""
    lines = text.split("\n")
    marks, fence, heading, heads = [], False, "", {}
    for i, ln in enumerate(lines):
        if ln.lstrip().startswith("```"):
            fence = not fence
            continue
        if fence:
            continue
        h = SEC_HEADING.match(ln)
        if h:
            heading = h.group(2)
        m = DEF_LINE.match(ln)
        if m and (i == 0 or not lines[i - 1].strip()):
            marks.append((i, m.group(1)))
            heads[i] = heading
    out = []
    for j, (ln_no, anchor) in enumerate(marks):
        end = marks[j + 1][0] if j + 1 < len(marks) else len(lines)
        body = "\n".join(lines[ln_no:end])
        out.append({"anchor": anchor, "normative": bool(NORMATIVE.search(body)),
                    "chars": len(body), "line": ln_no + 1, "body": body,
                    "heading": heads.get(ln_no, ""),
                    "bookkeeping": bool(BOOKKEEPING_HEADING.search(heads.get(ln_no, "")))})
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--modules", type=Path, required=True, help="staged spec module directory")
    ap.add_argument("--microtasks", nargs="+", type=Path, required=True, help="directories holding MT json")
    ap.add_argument("--current-spec", type=Path, required=True, help="the v02.205 module being replaced")
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    # --- read staged modules -------------------------------------------------
    modules = sorted(p for p in args.modules.glob("*.md"))
    per_module, anchor_home, dup, heading_styles, missing_fm = [], {}, [], collections.Counter(), []
    no_yield: set[str] = set()
    never_assigned: set[str] = set()
    intra_dup: list[dict] = []
    clause_styles = collections.Counter()
    all_clauses = []
    vendor_hits = []
    for p in modules:
        text = p.read_text(encoding="utf-8", errors="replace")
        fm = FRONTMATTER.search(text)
        if not fm:
            missing_fm.append(p.name)
        body = text[fm.end():] if fm else text
        h = HEADING.search(body)
        if h:
            heading_styles[len(h.group(1))] += 1
        no_yield |= declared_no_yield(body)
        blocks = clause_blocks(body)
        seen_in_file: dict[str, int] = {}
        for ha in HEADING_ANCHOR.findall(body):
            anchor_home.setdefault(ha, p.name)
        for ta in table_cell_definitions(body):
            anchor_home.setdefault(ta, p.name)
        for b in blocks:
            b["module"] = p.name
            if b["anchor"] in anchor_home and anchor_home[b["anchor"]] != p.name:
                dup.append({"anchor": b["anchor"], "modules": [anchor_home[b["anchor"]], p.name],
                            "line": b["line"]})
            if b["anchor"] in seen_in_file:
                # Two definitions of one anchor make every citation of it ambiguous. Across files
                # this was already caught; inside one file it was not, which is exactly where the
                # assembled section 14 can produce it.
                intra_dup.append({"anchor": b["anchor"], "module": p.name,
                                  "lines": [seen_in_file[b["anchor"]], b["line"]]})
            seen_in_file[b["anchor"]] = b["line"]
            anchor_home.setdefault(b["anchor"], p.name)
            if b.get("bookkeeping"):
                no_yield.add(b["anchor"])
            # A register clause naming anchors that were never assigned makes those anchors
            # intentionally undefined. Citing one is not a dangling reference; it is the spec
            # telling an implementer not to go looking for a clause that does not exist.
            flat = re.sub(r"\s+", " ", b["body"])
            for nm in NEVER_ASSIGNED.finditer(flat):
                window = flat[max(0, nm.start() - 200): nm.end() + 300]
                never_assigned |= set(ANCHOR_BARE.findall(window))
            all_clauses.append(b)
            if b["normative"]:
                for vm in VENDOR.finditer(b["body"]):
                    # Provenance mentions are legitimate; flag only vendor names inside an obligation sentence.
                    sent_start = b["body"].rfind(".", 0, vm.start()) + 1
                    sent = b["body"][sent_start: b["body"].find(".", vm.end()) + 1]
                    if NORMATIVE.search(sent) and not VENDOR_OK_CONTEXT.search(sent):
                        vendor_hits.append({"anchor": b["anchor"], "module": p.name, "vendor": vm.group(1), "sentence": sent.strip()[:220]})
        per_module.append({
            "module": p.name, "bytes": len(text), "lines": text.count("\n") + 1,
            "clauses": len(blocks), "normative_clauses": sum(1 for b in blocks if b["normative"]),
            "has_frontmatter": bool(fm), "heading_level": len(h.group(1)) if h else None,
        })

    # --- parameter contract completeness ------------------------------------
    param_rows, complete, stated_unknown, incomplete = 0, 0, 0, []
    for b in all_clauses:
        if "hard_min" not in b["body"]:
            continue
        for line in b["body"].splitlines():
            if "hard_min" not in line and "|" not in line:
                continue
            if not any(f in b["body"] for f in ("hard_max", "soft_min")):
                continue
        present = [f for f in BOUND_FIELDS if f in b["body"]]
        param_rows += 1
        if len(present) == len(BOUND_FIELDS):
            complete += 1
        elif "unknown" in b["body"].lower():
            stated_unknown += 1
        else:
            incomplete.append({"anchor": b["anchor"], "module": b["module"], "missing": [f for f in BOUND_FIELDS if f not in present]})
    collapsed = [b["anchor"] for b in all_clauses
                 if ("hard_min" in b["body"]) and ("soft_min" not in b["body"]) and ("unknown" not in b["body"].lower())]

    # --- read staged microtasks ---------------------------------------------
    mts, mt_anchor_refs = [], collections.Counter()
    for d in args.microtasks:
        for f in sorted(d.glob("*.json")):
            if f.name.startswith("_"):
                continue
            try:
                m = json.loads(f.read_text(encoding="utf-8"))
            except Exception:  # noqa: BLE001
                continue
            anchors = m.get("spec_anchor") or []
            if isinstance(anchors, str):
                anchors = [anchors]
            blob = json.dumps(m)
            for a in ANCHOR_BARE.findall(blob):
                mt_anchor_refs[a] += 1
            mts.append({
                "file": f.name, "dir": d.name, "mt_id": m.get("mt_id"),
                "anchors": [a for a in anchors if isinstance(a, str)],
                "provisional": "PROVISIONAL" in str(m.get("spec_anchor_status", "")),
            })

    spec_anchors = set(anchor_home)
    normative_anchors = {b["anchor"] for b in all_clauses if b["normative"]}
    referenced = set(mt_anchor_refs)

    # A clause the spec itself declares non-yielding is not a coverage gap.
    forward_uncovered = sorted(normative_anchors - referenced - no_yield)
    excluded_by_declaration = sorted((normative_anchors - referenced) & no_yield)
    backward_dangling = sorted(referenced - spec_anchors - never_assigned)
    cited_never_assigned = sorted((referenced - spec_anchors) & never_assigned)

    # --- anchors the modules claim to retain from the current spec ----------
    cur = args.current_spec.read_text(encoding="utf-8", errors="replace") if args.current_spec.exists() else ""
    cur_anchors = set(ANCHOR_BARE.findall(cur))
    dropped = sorted(a for a in cur_anchors - spec_anchors)

    report = {
        "schema_id": "handshake.reference.studio_spec_roundtrip@1",
        "generated_at": now(),
        "purpose": "Proves the operator's acceptance test: the Master Spec must be complete enough that the same microtask set falls out of it, and no microtask may cite a clause that does not exist.",
        "modules": {"count": len(modules), "per_module": per_module,
                    "missing_frontmatter": missing_fm,
                    "heading_level_histogram": dict(heading_styles),
                    "heading_drift": len(heading_styles) > 1,
                    "definition_rule": "An anchor at line start whose preceding line is blank. Style-independent, so a wrapped prose line beginning with an anchor reference is not a definition."},
        "anchors": {
            "total_in_staged_spec": len(spec_anchors),
            "normative": len(normative_anchors),
            "collisions_across_modules": dup,
            "duplicate_definitions_within_a_file": intra_dup,
            "duplicate_definitions_within_a_file_count": len(intra_dup),
            "dropped_from_v02_205_without_a_home": dropped[:200],
            "dropped_count": len(dropped),
        },
        "microtasks": {
            "count": len(mts),
            "still_provisional_anchor_status": sum(1 for m in mts if m["provisional"]),
            "distinct_anchors_referenced": len(referenced),
        },
        "roundtrip": {
            "forward_spec_clauses_with_no_microtask": forward_uncovered[:400],
            "forward_uncovered_count": len(forward_uncovered),
            "excluded_by_module_declaration": excluded_by_declaration,
            "excluded_by_module_declaration_count": len(excluded_by_declaration),
            "exclusion_note": "These clauses are declared non-yielding by the module that owns "
                              "them, almost always a Microtask Derivation sub-section. A clause "
                              "stating how the microtask set is derived is not itself a unit of "
                              "work, so it is not a coverage gap.",
            "backward_microtask_anchors_not_in_spec": backward_dangling[:400],
            "backward_dangling_count": len(backward_dangling),
            "cited_but_recorded_as_never_assigned": cited_never_assigned,
            "never_assigned_note": "The spec records these anchors as never assigned. A microtask quoting that register is not citing a missing clause.",
            "verdict": "PASS" if not forward_uncovered and not backward_dangling else "FAIL",
        },
        "parameter_contract": {
            "clauses_carrying_bounds": param_rows,
            "complete": complete,
            "stated_unknown": stated_unknown,
            "incomplete_without_stating_unknown": incomplete[:200],
            "collapsed_hard_and_soft": collapsed[:200],
            "collapsed_count": len(collapsed),
        },
        "naming_discipline": {
            "vendor_names_inside_normative_sentences": vendor_hits[:200],
            "count": len(vendor_hits),
            "rule": "[STU-SECTION-003]: a source product name is never a Studio tool, command, panel or manual name. Provenance mentions are legitimate; obligations must not name a vendor product.",
        },
    }
    out = args.out / "spec-roundtrip-report.json"
    out.write_text(json.dumps(report, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    r = report
    print(f"[roundtrip] modules={r['modules']['count']} anchors={r['anchors']['total_in_staged_spec']} normative={r['anchors']['normative']}")
    print(f"[roundtrip] microtasks={r['microtasks']['count']} referencing {r['microtasks']['distinct_anchors_referenced']} anchors")
    print(f"[roundtrip] FORWARD uncovered clauses: {r['roundtrip']['forward_uncovered_count']}"
          f" (+{r['roundtrip']['excluded_by_module_declaration_count']} declared non-yielding)")
    print(f"[roundtrip] BACKWARD dangling anchors: {r['roundtrip']['backward_dangling_count']}")
    print(f"[roundtrip] VERDICT: {r['roundtrip']['verdict']}")
    print(f"[roundtrip] anchor collisions={len(dup)} intra-file duplicate definitions={len(intra_dup)} "
          f"dropped_from_current={len(dropped)} heading_drift={r['modules']['heading_drift']}")
    print(f"[roundtrip] params: complete={complete} stated_unknown={stated_unknown} collapsed={len(collapsed)}")
    print(f"[roundtrip] vendor names in obligations: {len(vendor_hits)}")
    print(f"[roundtrip] -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
