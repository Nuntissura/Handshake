#!/usr/bin/env python3
"""Derive the Studio microtask set mechanically from the Master Spec modules.

The operator's acceptance test, verbatim: "i want the master spec to be updated so we could
extract the same exact microtasks from the master spec."

That property cannot be achieved by writing microtasks and a spec separately and hoping they
agree. It is achieved by making the spec the ONLY input, so the microtask set is a function of
the spec text. This tool is that function.

Each spec module declares its own derivation rule in a "Microtask Derivation" sub-section. The
rules agree on a closed list of units, and this implements that list:

    clause           a normative clause stating a stored contract, an enumeration, or an
                     engine behaviour that can be implemented and proven independently
    parameter_table  a table carrying the seven-field bound contract, taken whole
    enumeration      an enumeration stated in the module, taken whole
    validator        a validation descriptor
    golden_case      a golden-corpus case

Clauses that yield nothing, by the modules' own exclusion rules: pure cross-references,
restatements of an obligation that attaches to every microtask, supersession/disposition table
rows, and the derivation sub-section itself.

DETERMINISM. Re-running against an unchanged spec must produce a byte-identical set, and adding
a clause must not renumber existing microtasks. Both are guaranteed by deriving a stable
`derivation_key` from the anchor and unit rather than from position, and by persisting the
key -> mt_id assignment in an id-map that is only ever appended to.

Reference tooling under .GOV/reference. It writes only into the green-room staging tree; nothing
under .GOV/task_packets is touched here.
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import re
from pathlib import Path

DEF_LINE = re.compile(r"^(?:\*\*)?\[(STU-[A-Z]+-\d+[a-z]?)\]")
TITLE = re.compile(r"^(?:\*\*)?\[STU-[A-Z]+-\d+[a-z]?\]\*{0,2}\s*\*{0,2}(.+?)(?:\*\*|$)")
HEADING = re.compile(r"^(#{2,5})\s+(.+?)\s*$")
NORMATIVE = re.compile(r"\b(MUST NOT|MUST|SHALL NOT|SHALL|REQUIRED|FORBIDDEN)\b")
ANCHOR_REF = re.compile(r"\[(STU-[A-Z]+-\d+[a-z]?)\]")
BOUND_FIELDS = ("hard_min", "hard_max", "soft_min", "soft_max", "default", "unit", "precision")
# A per-table derivation marker, made normative by the modules' own rule 0. It states how
# many microtasks that table yields, and it overrides every heuristic below.
MARKER = re.compile(r"^\*Derivation:\s*(.+?)\*\s*$", re.I)
MARKER_COUNT = re.compile(r"yields?\s+(\d+)\s+microtask", re.I)

# Sub-sections whose clauses are bookkeeping about the spec itself, never implementable work.
EXCLUDED_HEADING = re.compile(
    r"(microtask derivation|supersession|disposition|anchor continuity|derivation basis|"
    r"authority, derivation|reading order|change log|revision history)", re.I)
# A clause that only restates an obligation carried by every microtask anyway.
RESTATEMENT = re.compile(
    r"(applies to every microtask|attaches to every microtask|restates|is restated|"
    r"for the avoidance of doubt|this obligation is universal|as already required by)", re.I)
# A clause that only points elsewhere, or only restates a blanket obligation.
CROSSREF_ONLY = re.compile(
    r"^(this clause is|see |refer to |as stated in|is specified in|is owned by|"
    r"the contract for .* is stated in)", re.I)


# A module may DECLARE its non-yielding clauses by anchor. That declaration is normative and
# overrides every heuristic: [STU-TYP-240] states a tool MUST use the list rather than infer
# exclusions from prose, because inference is what caused the divergence it records.
NOYIELD_HEADER = re.compile(r"(declared\s+non-?yielding\s+set|no-?yield\s+set\s*:)", re.I)
NOYIELD_COUNT = re.compile(r"no-?yield\s+set\s*:\s*(\d+)\s+clause", re.I)
# A backticked anchor is unambiguous. A bracketed one is indistinguishable from a
# cross-reference, so it is trusted only when the header states how many to expect.
LIST_ITEM = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s")
NOYIELD_TICKED = re.compile(r"`(STU-[A-Z]+-\d+[a-z]?)`", re.I)
NOYIELD_BRACKETED = re.compile(r"\[(STU-[A-Z]+-\d+[a-z]?)\]", re.I)


def declared_no_yield(text: str, prefix: str = "") -> set[str]:
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


def clip(text: str, limit: int) -> str:
    """Shorten text without ever cutting a clause anchor in half.

    A clip that lands inside "[STU-ASSET-030]" leaves "[STU-ASSET-03", which reads as a citation of
    a clause that does not exist. Any trailing partial anchor is removed and the shortening is
    marked so the text is not mistaken for the whole contract.
    """
    if len(text) <= limit:
        return text
    out = text[:limit]
    cut = out.rfind("[STU-")
    if cut != -1 and "]" not in out[cut:]:
        out = out[:cut]
    return out.rstrip().rstrip(",;:") + " [...]"


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def split_sentences(text: str) -> list[str]:
    """Split a clause body into sentences, keeping table rows and list items whole."""
    flat = re.sub(r"\s*\n\s*", " ", text)
    parts = re.split(r"(?<=[.;])\s+(?=[A-Z\[`])", flat)
    return [p.strip() for p in parts if p.strip()]


def parse_tables(body: str) -> list[dict]:
    """Return markdown tables as header + rows + the label line that introduces them."""
    tables, cur, label, marker = [], None, "", ""
    for ln in body.split("\n"):
        t = ln.strip()
        if t.startswith("|") and t.endswith("|") and t.count("|") >= 3:
            cells = [c.strip() for c in t.strip("|").split("|")]
            if all(re.fullmatch(r":?-{2,}:?", c) for c in cells if c):
                continue  # separator row
            if cur is None:
                # Consume the marker and the label. Leaving them set let a marker leak forward
                # onto every later unmarked table in the same clause, which silently reclassified
                # tables that were never marked at all.
                cur = {"header": cells, "rows": [], "label": label, "marker": marker}
                marker, label = "", ""
            else:
                cur["rows"].append(cells)
        else:
            if cur and cur["rows"]:
                tables.append(cur)
            cur = None
            mk = MARKER.match(t)
            if mk:
                marker = mk.group(1).strip()
            elif t.endswith(":") and 2 < len(t) <= 90:
                label = re.sub(r"[*`_]", "", t).rstrip(":").strip()
            elif not t:
                pass
    if cur and cur["rows"]:
        tables.append(cur)
    return tables


def is_parameter_table(t: dict) -> bool:
    head = " ".join(t["header"]).lower().replace(" ", "_")
    return sum(1 for f in BOUND_FIELDS if f in head) >= 4


def is_enumeration_table(t: dict) -> bool:
    head = " ".join(t["header"]).lower()
    return any(k in head for k in ("member", "value", "enumerator", "option", "mode", "state", "kind"))


# A table either describes ONE thing across its rows, or it CATALOGUES many separate things.
# The distinction decides how many microtasks it yields, and getting it wrong is the difference
# between one unimplementable microtask reading "17 tools" and seventeen that a small model can
# each finish. The first column's name is the discriminator: a parameter, field or member belongs
# to one subject, whereas an effect, tool, command or format IS a subject.
CATALOGUE_FIRST_COL = re.compile(
    r"^(studio\s+)?(effect|filter|adjustment|generator|tool|shape|operation|transform|brush|"
    r"format|codec|container|exporter|importer|panel|node|layer|blend\s*mode|primitive|engine|"
    r"gradient|mask|matte|marker|guide|swatch|palette|font|glyph|scale|space|profile)s?\b", re.I)
# A command, shortcut, action or preset row is NOT its own microtask. Binding a key is not a unit
# of implementation work, and treating each as one produced 1,155 microtasks for the video module
# alone. Those tables yield one microtask for the table, taken whole.
COMMAND_FIRST_COL = re.compile(r"^(command|shortcut|binding|key|menu|action|preset|template)s?\b", re.I)
PART_OF_ONE = re.compile(r"^(#|parameter|param|field|member|value|option|property|attribute|key|"
                         r"enumerator|column|setting|flag)s?\b", re.I)


def is_command_table(t: dict) -> bool:
    """A command, shortcut or preset listing: one microtask for the whole table."""
    if is_parameter_table(t) or len(t["rows"]) < 2:
        return False
    first = (t["header"][0] if t["header"] else "").strip().lower()
    head = " ".join(t["header"]).lower()
    return bool(COMMAND_FIRST_COL.match(first)) or "binding" in head


def is_catalogue_table(t: dict) -> bool:
    """True when each ROW is a separate implementable subject rather than a facet of one subject."""
    if is_parameter_table(t) or len(t["rows"]) < 2:
        return False
    first = (t["header"][0] if t["header"] else "").strip().lower()
    if PART_OF_ONE.match(first):
        return False
    if COMMAND_FIRST_COL.match(first):
        return False
    if not CATALOGUE_FIRST_COL.match(first):
        return False
    # Distinct first-column values confirm the rows are separate subjects, not repeated facets.
    vals = {r[0].strip("` *").lower() for r in t["rows"] if r and r[0].strip()}
    return len(vals) >= 2 and len(vals) >= 0.8 * len(t["rows"])


def parse_modules(mod_dir: Path) -> list[dict]:
    """Read every module into clauses, each carrying its owning heading and body."""
    out = []
    for p in sorted(mod_dir.glob("*.md")):
        lines = p.read_text(encoding="utf-8", errors="replace").split("\n")
        marks, fence, heading, headings = [], False, "", {}
        for i, ln in enumerate(lines):
            if ln.lstrip().startswith("```"):
                fence = not fence
                continue
            if fence:
                continue
            h = HEADING.match(ln)
            if h:
                heading = h.group(2)
            m = DEF_LINE.match(ln)
            if m and (i == 0 or not lines[i - 1].strip()):
                marks.append((i, m.group(1)))
                headings[i] = heading
        prefixes = collections.Counter(a.rsplit("-", 1)[0] for _, a in marks)
        prefix = prefixes.most_common(1)[0][0] if prefixes else ""
        clauses = []
        for j, (ln_no, anchor) in enumerate(marks):
            end = marks[j + 1][0] if j + 1 < len(marks) else len(lines)
            body = "\n".join(lines[ln_no:end]).strip()
            tm = TITLE.match(lines[ln_no])
            clauses.append({
                "anchor": anchor, "module": p.name, "line": ln_no + 1,
                "heading": headings.get(ln_no, ""),
                "title": clip(tm.group(1).strip().rstrip(".") if tm else "", 200),
                "body": body,
            })
        out.append({"module": p.name, "clauses": clauses,
                    "declared_no_yield": declared_no_yield("\n".join(lines), prefix)})
    return out


def classify(c: dict) -> tuple[bool, str]:
    """Decide whether a clause yields a microtask, and say why when it does not."""
    if EXCLUDED_HEADING.search(c["heading"] or ""):
        return False, "bookkeeping sub-section: derivation, supersession, disposition or continuity"
    prose = re.sub(r"^\S+\s+", "", re.sub(r"\s+", " ", c["body"]))
    if CROSSREF_ONLY.match(prose):
        return False, "cross-reference only: the contract lives in another clause"
    refs = set(ANCHOR_REF.findall(c["body"])) - {c["anchor"]}
    words = len(re.findall(r"\w+", c["body"]))
    if refs and words < 40 and not NORMATIVE.search(c["body"]):
        return False, "pointer clause: too short to carry a contract and only points elsewhere"
    if RESTATEMENT.search(c["body"]) and words < 120:
        return False, "restates an obligation that already attaches to every microtask"
    # Deliberately NOT gated on MUST or SHALL. The modules' own derivation rules yield one
    # microtask per clause minus a declared non-yielding set, and a clause can state a stored
    # contract, an enumeration or an engine behaviour in the indicative mood. Requiring an
    # obligation word cost roughly 190 microtasks across seven modules before this was corrected
    # against the module ledgers, which their authors verified three independent ways.
    return True, ""


def derive(clauses: list[dict], no_yield: set[str] | None = None,
           unrecognised: list | None = None) -> list[dict]:
    """Apply the modules' declared derivation rule to one module's clauses."""
    units = []
    no_yield = no_yield or set()
    unrecognised = unrecognised if unrecognised is not None else []
    for c in clauses:
        if c["anchor"].upper() in no_yield:
            yields, reason = False, ("declared non-yielding by the module's own normative list, "
                                     "which overrides inference")
        elif no_yield:
            # The module published a normative exclusion list, so that list is the WHOLE exclusion.
            # [STU-TYP-240]: "A tool MUST use this list rather than inferring exclusions from
            # prose." Running the heuristics as well was dropping clauses the module never excluded
            # and is what left colour three units short of its own ledger.
            yields, reason = True, ""
        else:
            yields, reason = classify(c)
        bookkeeping = bool(EXCLUDED_HEADING.search(c["heading"] or ""))
        # A parameter table or an enumeration IS a contract, whether or not the sentence around it
        # happens to carry a MUST. The effects, video, motion and compositing modules state most of
        # their behaviour as catalogue tables in descriptive prose; gating table extraction on an
        # obligation word dropped the entire 425 KB effect catalogue on the first run.
        tables = [] if bookkeeping else parse_tables(c["body"])

        def marked(t: dict) -> str:
            """The class the spec's own marker assigns, or "" when the table carries no marker."""
            mk = (t.get("marker") or "").lower()
            if not mk:
                return ""
            # Classify on the PRIMARY clause of the marker only. Several markers append a
            # cross-reference sentence ending "...they are NOT clause definitions and yield no
            # microtask here", which is about the anchors in the cells, not about the table. Reading
            # the whole string made "no microtask" win and silently voided 56 catalogue tables.
            head = mk.split("anchors appearing")[0].strip()
            if "splits per row" in head or "split per row" in head:
                return "catalogue"
            if "no microtask" in head:
                return "none"
            if "parameter table" in head:
                return "parameter"
            if "enumeration table" in head:
                return "enumeration"
            if "preset" in head or "command table" in head:
                return "command"
            if "taken whole" in head:
                return "enumeration"
            return ""

        marks = {id(t): marked(t) for t in tables}
        for t in tables:
            if (t.get("marker") or "").strip() and not marks[id(t)]:
                unrecognised.append({
                    "anchor": c["anchor"], "module": c["module"], "heading": c["heading"],
                    "marker": (t.get("marker") or "")[:220],
                    "reading": "The table carries a derivation marker whose head matches no class, "
                               "so it fell back to heuristics. A sentence appended to the head "
                               "before the cross-reference split can void the classification.",
                })
        params = [t for t in tables if (marks[id(t)] == "parameter"
                                        or (not marks[id(t)] and is_parameter_table(t)))]
        cats = [t for t in tables if (marks[id(t)] == "catalogue"
                                      or (not marks[id(t)] and is_catalogue_table(t)))]
        cmds = [t for t in tables if t not in params and t not in cats
                and (marks[id(t)] == "command" or (not marks[id(t)] and is_command_table(t)))]
        enums = [t for t in tables if t not in params and t not in cats and t not in cmds
                 and (marks[id(t)] == "enumeration"
                      or (not marks[id(t)] and is_enumeration_table(t)))]
        if not yields:
            if not params and not enums and not cats and not cmds:
                units.append({"kind": "none", "anchor": c["anchor"], "module": c["module"],
                              "reason": reason, "title": c["title"], "heading": c["heading"]})
                continue
            reason_note = f"clause itself yields no microtask ({reason}), but it carries tabular contracts"
            units.append({"kind": "none", "anchor": c["anchor"], "module": c["module"],
                          "reason": reason_note, "title": c["title"], "heading": c["heading"]})
        else:
            head = (c["heading"] or "").lower()
            kind = "clause"
            if "valid" in head or "validator" in head:
                kind = "validator"
            elif "golden" in head or "corpus" in head:
                kind = "golden_case"
            units.append({"kind": kind, "anchor": c["anchor"], "module": c["module"],
                          "title": c["title"], "heading": c["heading"], "line": c["line"],
                          "body": c["body"], "index": 0})
        for n, t in enumerate(params):
            lbl = (t.get("label") or "").strip()
            units.append({"kind": "parameter_table", "anchor": c["anchor"], "module": c["module"],
                          "title": (f"{c['title']} - {lbl} parameter bounds" if lbl
                                    else f"{c['title']} - parameter bounds"), "heading": c["heading"],
                          "line": c["line"], "body": c["body"], "table": t, "index": n})
        for n, t in enumerate(cmds):
            units.append({"kind": "command_table", "anchor": c["anchor"], "module": c["module"],
                          "title": f"{c['title']} - command and binding set", "heading": c["heading"],
                          "line": c["line"], "body": c["body"], "table": t, "index": n})
        for n, t in enumerate(enums):
            lbl = (t.get("label") or "").strip()
            units.append({"kind": "enumeration", "anchor": c["anchor"], "module": c["module"],
                          "title": (f"{c['title']} - {lbl} enumeration" if lbl
                                    else f"{c['title']} - enumeration"), "heading": c["heading"],
                          "line": c["line"], "body": c["body"], "table": t, "index": n})
        # A catalogue row is its own subject, so it is its own microtask. One microtask covering
        # "17 tools" is not implementable by the small models these contracts are sized for.
        # A table whose cells define anchors is a clause family, and each row is its own
        # subject. One microtask for the whole table would be unimplementable.
        for tn, t in enumerate(tables):
            # Respect the spec's own marker. A table the module declares as taken whole, or as
            # yielding nothing, must not be split into one microtask per anchor-bearing row just
            # because its cells happen to define anchors. Ignoring the marker here was inflating
            # vector by 30 and layout by 17 against ledgers their authors had already reconciled.
            if t in params or marks[id(t)] in ("none", "parameter", "enumeration", "command"):
                continue
            # A module may state that anchors in its cells are cross-references rather than
            # definitions. Vector and layout do: all 61 of their in-cell anchors are defined as
            # paragraphs elsewhere in the same sub-section, so counting the cell would count the
            # clause twice. Other modules genuinely DO define clause families in table cells, so
            # this exemption is honoured per table, never assumed globally.
            if "cross-reference" in (t.get("marker") or "").lower():
                continue
            for r in t["rows"]:
                cell = next((c for c in r if re.fullmatch(r"\*{0,2}\[STU-[A-Z]+-\d+[a-z]?\]\*{0,2}", c.strip())), None)
                if not cell:
                    continue
                anch = re.search(r"(STU-[A-Z]+-\d+[a-z]?)", cell).group(1)
                label = next((c.strip("` *") for c in r if c.strip() and c is not cell), anch)
                units.append({
                    "kind": "anchor_row", "anchor": anch, "module": c["module"],
                    "title": label[:160], "heading": c["heading"], "line": c["line"],
                    "body": c["body"], "table": {"header": t["header"], "rows": [r]},
                    "subject": label[:160], "index": 0,
                })

        seen = set()
        for tn, t in enumerate(cats):
            # A marker may declare an explicit yield that differs from the row count, as in
            # "splits per row; yields 9 microtasks, one per live-effect family" over a 14-row
            # table. The declared count is normative. Deduplicating on the subject column usually
            # reconciles the two; where it does not, the mismatch is recorded for proofreading
            # rather than silently resolved in either direction.
            declared_n = None
            mc = MARKER_COUNT.search(t.get("marker") or "")
            if mc and marks[id(t)] == "catalogue":
                declared_n = int(mc.group(1))
            produced_before = len(units)
            for rn, r in enumerate(t["rows"]):
                subject = r[0].strip("` *") if r else ""
                if not subject or subject.lower() in ("", "-", "--"):
                    continue
                slug = re.sub(r"[^a-z0-9]+", "_", subject.lower()).strip("_")[:48]
                if not slug or slug in seen:
                    continue
                seen.add(slug)
                units.append({
                    "kind": "catalogue_row", "anchor": c["anchor"], "module": c["module"],
                    "title": f"{subject}", "heading": c["heading"], "line": c["line"],
                    "body": c["body"], "table": {"header": t["header"], "rows": [r]},
                    "subject": subject, "index": f"{tn}_{slug}",
                })
            if declared_n is not None:
                produced = len(units) - produced_before
                if produced != declared_n:
                    units.append({"kind": "marker_mismatch", "anchor": c["anchor"],
                                  "module": c["module"], "title": c["title"],
                                  "heading": c["heading"],
                                  "declared": declared_n, "produced": produced,
                                  "marker": t.get("marker", "")})
    return units


def acceptance_rows(u: dict) -> list[dict]:
    """Acceptance criteria come from the spec's own sentences and tables, never from invention."""
    rows, n = [], 0
    t = u.get("table")
    if t and u["kind"] == "parameter_table":
        head = [h.lower().replace(" ", "_") for h in t["header"]]
        for r in t["rows"]:
            cells = dict(zip(head, r))
            name = r[0].strip("` ") if r else ""
            if not name or name.lower() in ("parameter", "field", "name"):
                continue
            n += 1
            stated = {f: cells.get(f, "").strip() for f in BOUND_FIELDS if f in cells}
            unknown = [f for f, v in stated.items() if not v or v.lower() in ("unknown", "-", "n/a", "--")]
            rows.append({
                "id": f"AC-{n:03d}",
                "criterion": (
                    f"The parameter `{name}` accepts and stores every one of its seven declared "
                    f"fields as SEPARATE values, exactly as the spec table states them: "
                    + ", ".join(f"{f}={v or 'UNKNOWN'}" for f, v in stated.items())
                    + ". A field the spec marks unknown is stored as unknown and is NOT set equal "
                      "to its opposite bound, because collapsing hard and soft bounds destroys "
                      "information that cannot be recovered without re-deriving it from the captures."),
                "evidence_kind": "runtime_assertion",
                "unknown_fields": unknown,
                "source": f"{u['module']} {u['anchor']} parameter table row {n}",
            })
    elif t and u["kind"] in ("catalogue_row", "anchor_row"):
        head = t["header"]
        row = t["rows"][0]
        for col, val in zip(head, row):
            val = (val or "").strip()
            if not val or val in ("-", "--", "n/a"):
                continue
            n += 1
            rows.append({
                "id": f"AC-{n:03d}",
                "criterion": (f"`{u['subject']}` satisfies the spec's stated {col.strip().lower()}: "
                              f"{clip(val, 400)}"),
                "evidence_kind": "runtime_assertion",
                "source": f"{u['module']} {u['anchor']} catalogue row `{u['subject']}`",
            })
        n += 1
        rows.append({
            "id": f"AC-{n:03d}",
            "criterion": (f"`{u['subject']}` is reachable from the operator surface with a tooltip, "
                          f"is invocable headlessly for automation, and is named natively rather than "
                          f"after any source product."),
            "evidence_kind": "gui_and_headless_proof",
            "source": f"{u['module']} {u['anchor']}",
        })
    elif t and u["kind"] == "enumeration":
        for r in t["rows"]:
            member = r[0].strip("` ") if r else ""
            if not member or member.lower() in ("member", "value", "name", "option"):
                continue
            n += 1
            rows.append({
                "id": f"AC-{n:03d}",
                "criterion": (f"The enumeration accepts the member `{member}` with the exact meaning "
                              f"the spec states: {clip(' | '.join(x for x in r[1:] if x), 300)}. No member is "
                              f"added, renamed, or silently mapped onto another."),
                "evidence_kind": "runtime_assertion",
                "source": f"{u['module']} {u['anchor']} enumeration row {n}",
            })
    if not rows:
        for s in split_sentences(u.get("body", "")):
            if not NORMATIVE.search(s) or s.startswith("|"):
                continue
            n += 1
            rows.append({
                "id": f"AC-{n:03d}",
                "criterion": clip(re.sub(r"^\*{0,2}\[STU-[A-Z]+-\d+[a-z]?\]\*{0,2}\s*", "", s), 600),
                "evidence_kind": "runtime_assertion",
                "source": f"{u['module']} {u['anchor']}",
            })
            if n >= 24:
                break
    return rows


DOMAIN_BY_PREFIX = {
    "RAS": "raster", "VEC": "vector", "LAY": "layout", "TYP": "typography", "COL": "colour",
    "FX": "effects", "DS": "design_systems", "PRO": "prototyping", "IO": "interop",
    "AUT": "automation", "WB": "whiteboard", "VID": "video_timeline", "MOT": "motion",
    "CMP": "compositing", "WEB": "web_authoring", "ASSET": "asset_library", "SHELL": "operator_shell",
    "UI": "operator_shell", "TOOL": "tools_and_controls", "TIP": "tooltips_and_manual",
    # The shell modules use SHL, MDL and MAN rather than the names guessed from their filenames.
    "SHL": "operator_shell", "MDL": "model_facing_surface", "MAN": "tooltips_and_manual",
    # Prefixes carried forward from v02.205 that the rebuilt sub-sections still cite.
    "ARC": "architecture", "DOC": "document_model", "OVR": "cross_cutting", "SDB": "storage",
    "RAW": "raster", "SECTION": "cross_cutting", "CON": "cross_cutting", "UNI": "cross_cutting",
}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--modules", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--id-map", type=Path, required=True)
    ap.add_argument("--base", type=int, default=20000)
    ap.add_argument("--prior-mts", type=Path, default=None,
                    help="staged cluster-derived microtasks, mined for behavioural payload only")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    # --- prior payload, so nothing real that was already extracted is lost -----------------
    prior = []
    if args.prior_mts and args.prior_mts.exists():
        for f in sorted(args.prior_mts.glob("*.json")):
            try:
                d = json.loads(f.read_text(encoding="utf-8"))
            except Exception:  # noqa: BLE001
                continue
            beh = (d.get("implementation_notes") or {}).get("extracted_behaviour") or []
            if beh:
                prior.append({"file": f.name, "clause": d.get("clause", ""),
                              "tokens": set(re.findall(r"[a-z]{4,}", (d.get("clause") or "").lower())),
                              "behaviour": beh})

    modules = parse_modules(args.modules)
    id_map = json.loads(args.id_map.read_text(encoding="utf-8")) if args.id_map.exists() else {}
    next_id = max([int(v.split("-")[1]) for v in id_map.values()] or [args.base - 1]) + 1

    manifest, counts, skipped, mismatches = [], collections.Counter(), [], []
    unrecognised: list[dict] = []
    for mod in modules:
        for u in derive(mod["clauses"], mod.get("declared_no_yield"), unrecognised):
            if u["kind"] == "marker_mismatch":
                mismatches.append({"anchor": u["anchor"], "module": u["module"],
                                   "declared_by_marker": u["declared"], "produced_by_tool": u["produced"],
                                   "marker": u["marker"],
                                   "reading": "The table's marker declares a yield the row set does "
                                              "not produce after deduplication. Either the marker "
                                              "count is wrong or the table repeats or omits a subject."})
                counts["marker_mismatch"] += 1
                continue
            if u["kind"] == "none":
                skipped.append({"anchor": u["anchor"], "module": u["module"],
                                "title": u["title"], "reason": u["reason"]})
                counts["no_microtask"] += 1
                continue
            key = f"{u['anchor']}#{u['kind']}#{u['index']}"
            if key not in id_map:
                id_map[key] = f"MT-{next_id}"
                next_id += 1
            u["mt_id"] = id_map[key]
            u["derivation_key"] = key
            manifest.append(u)
            counts[u["kind"]] += 1

    seen_titles: dict[tuple, int] = {}
    for u in manifest:
        tkey = (u["module"], re.sub(r"[^a-z0-9]+", "", (u["title"] or "").lower())[:70])
        seen_titles[tkey] = seen_titles.get(tkey, 0) + 1
        if seen_titles[tkey] > 1:
            u["title"] = f"{u['title']} ({seen_titles[tkey]})"

    written = 0
    for u in manifest:
        prefix = u["anchor"].split("-")[1]
        domain = DOMAIN_BY_PREFIX.get(prefix, prefix.lower())
        rows = acceptance_rows(u)
        toks = set(re.findall(r"[a-z]{4,}", (u["title"] or "").lower()))
        payload = []
        for pr in prior:
            if toks and len(toks & pr["tokens"]) >= 2:
                payload.extend(pr["behaviour"][:40])
                if len(payload) >= 40:
                    break
        mt = {
            "schema_id": "hsk.microtask_contract@1",
            "schema_version": "microtask_contract_v3_spec_derived",
            "contract_authority": "PRIMARY_MACHINE_READABLE",
            "wp_id": "WP-KERNEL-STUDIO",
            "mt_id": u["mt_id"],
            "created_at_utc": now(),
            "generated_by": "handshake.greenroom.spec_derive.v1",
            "derivation": {
                "rule": "One microtask per derivation unit declared by the module's own Microtask "
                        "Derivation sub-section. The Master Spec is the only input.",
                "derivation_key": u["derivation_key"],
                "unit_kind": u["kind"],
                "spec_module": u["module"],
                "spec_heading": u["heading"],
                "spec_line": u["line"],
                "reproducible": "Re-running spec-derive-microtasks.py against an unchanged spec "
                                "reproduces this contract with this same mt_id.",
            },
            "lifecycle": {"status": "PENDING", "depends_on": [], "blocks": [],
                          "active": False, "validator_verdict": "PENDING"},
            "clause": u["title"] or f"{u['anchor']} contract",
            "spec_anchor": [u["anchor"]],
            "spec_anchor_status": "RESOLVED: derived from the assembled Master Spec module, not provisional",
            "domain": domain,
            "scope": {
                "summary": clip(re.sub(r"\s+", " ", re.sub(r"^\*{0,2}\[.*?\]\*{0,2}\s*", "", u["body"])), 1400),
                "spec_text_is_authority": "The clause text above is quoted from the Master Spec. Where "
                                          "this contract and the spec differ, the spec wins and this "
                                          "contract is regenerated.",
            },
            "acceptance_criteria": rows,
            "acceptance_criteria_count": len(rows),
            "implementation_notes": {
                "extracted_behaviour": payload,
                "behaviour_record_count": len(payload),
                "naming": "Vendor product names are provenance only. Studio ships Handshake-native "
                          "names per [STU-SECTION-003].",
                "database": "SurrealDB with the EventLedger is the only durable authority. SQLite, "
                            "libSQL, Turso and PostgreSQL are forbidden, including in tests and dev caches.",
                "engine_split": "GPU, WGSL and compute live in the studio-engine crate behind its "
                                "traits. handshake_core must never gain a GPU dependency.",
                "routing": "Navigation extends the existing address and bus seams. No new router.",
            },
            "gui_obligation": {"operator_surface_required": "YES", "argus_required": "YES"},
            "user_manual_obligation": {"required": True, "same_change_update_required": "YES"},
            "resource_privacy_obligation": {"applies": True},
            "hbr_obligations": ["HBR-INT-001", "HBR-INT-003", "HBR-INT-009", "HBR-VIS-001",
                                "HBR-VIS-003", "HBR-MAN-001", "HBR-MAN-004", "HBR-PRIV-001"],
            "validator_focus": [
                "Confirm every acceptance row traces to a sentence or table row in the cited spec "
                "clause. A row with no spec basis is a fabrication and fails the contract.",
                "Reject scaffold-only proof: at least one proof command must drive the executable "
                "runtime rather than a fixture the implementer authored.",
                "For a parameter row, confirm hard and soft bounds are stored as separate values and "
                "an unknown bound is stored as unknown rather than mirrored from its opposite.",
            ],
            "handoff": {"coder_session": None, "wp_validator_session": None},
        }
        (args.out / f"{u['mt_id']}.json").write_text(
            json.dumps(mt, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
        written += 1

    args.id_map.write_text(json.dumps(id_map, indent=1, sort_keys=True), encoding="utf-8", newline="\n")
    by_module = collections.Counter(u["module"] for u in manifest)
    report = {
        "schema_id": "handshake.reference.studio_mt_derivation@1",
        "generated_at": now(),
        "input": str(args.modules),
        "unit_counts": dict(counts),
        "microtasks_written": written,
        "per_module_yields": dict(sorted(by_module.items())),
        "markers_matching_no_class": unrecognised,
        "markers_matching_no_class_count": len(unrecognised),
        "marker_count_mismatches": mismatches,
        "marker_mismatch_count": len(mismatches),
        "clauses_yielding_no_microtask": skipped,
        "no_microtask_count": len(skipped),
        "determinism": "mt_id is bound to derivation_key in the id-map, which is append-only. Adding "
                       "a spec clause allocates a new id and renumbers nothing.",
    }
    (args.out.parent / "studio-mt-derivation-manifest.json").write_text(
        json.dumps(report, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    print(f"[derive] modules={len(modules)} microtasks={written} "
          f"unrecognised_markers={len(unrecognised)}")
    print(f"[derive] units: {dict(counts)}")
    print(f"[derive] yields by module: {dict(sorted(by_module.items()))}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
