"""pp_adm.py -- parser for the Adobe Dialog Manager (.adm) sheets that define
Premiere's built-in audio effects.

install/adm/<Effect>.adm is a declarative sheet:

    sheet DynamicsUI
    {
      constant:
        decibel_min: -48;
        decibel_max: 48;
        decibel_range: decibel_max - decibel_min;
      input:
        in0: 0;
        in31: 0;                     // make-up gain
      interface:
        unlink attack_time: (in0 * attack_range) + attack_min;  // kParameterIndex_AttackTime
        unlink link_channels: in11 > 0.5;                       // kParameterIndex_CompJoint
      output:
        out0: ...;
    }

That is the whole behavioural contract of the effect's parameter surface:

  * `input` slots in0..inN are the effect's parameters as the host stores them,
    normalised 0..1
  * an `interface` binding names the slot, states the real-world unit range it
    maps onto, and its trailing comment gives the host's own parameter index
    constant (kParameterIndex_*)
  * `constant` supplies the numeric bounds those expressions use

The parser resolves the common expression shapes so a rebuild can reproduce the
mapping exactly, and keeps the raw expression for every binding either way.
"""
import re

_SHEET = re.compile(r"sheet\s+([A-Za-z_]\w*)\s*\{", re.S)
_SECTION = re.compile(r"^\s*(constant|input|interface|output|invariant|logic)\s*:",
                      re.M)
_LINE_COMMENT = re.compile(r"//([^\n]*)")

# (inN * <x>_range) + <x>_min           -> linear map onto [x_min, x_max]
_LINEAR = re.compile(r"^\(\s*(in\d+)\s*\*\s*([A-Za-z_]\w*)_range\s*\)\s*\+\s*\1?\s*([A-Za-z_]\w*)_min\s*$")
_LINEAR2 = re.compile(r"^\(\s*(in\d+)\s*\*\s*([A-Za-z_]\w*)_range\s*\)\s*\+\s*([A-Za-z_]\w*)_min\s*$")
_BOOL = re.compile(r"^(in\d+)\s*>\s*0?\.5\s*$")
_SCALE = re.compile(r"^(in\d+)\s*\*\s*([-\d.]+)\s*$")
_PLAIN = re.compile(r"^(in\d+)$")
_RANGE_DEF = re.compile(r"^([A-Za-z_]\w*)_max\s*-\s*([A-Za-z_]\w*)_min$")


def _num(v):
    try:
        f = float(v)
    except (TypeError, ValueError):
        return None
    return int(f) if f == int(f) and abs(f) < 1e15 else f


_TRAILING_COMMENT = re.compile(r"^[ \t]*//([^\n]*)")


def _split_statements(body):
    """Split a section body on ';' at brace depth zero.

    A statement's explanatory comment is written AFTER its semicolon, on the
    same line, so a naive split hands each comment to the following statement.
    The trailing comment is moved back onto the statement it annotates.
    """
    out = []
    depth = 0
    cur = []
    for ch in body:
        if ch in "{[(":
            depth += 1
        elif ch in "}])":
            depth -= 1
        if ch == ";" and depth <= 0:
            out.append("".join(cur))
            cur = []
            continue
        cur.append(ch)
    if "".join(cur).strip():
        out.append("".join(cur))

    fixed = []
    for i, stmt in enumerate(out):
        m = _TRAILING_COMMENT.match(stmt)
        if m and fixed:
            fixed[-1] = fixed[-1] + "  //" + m.group(1)
            stmt = stmt[m.end():]
        fixed.append(stmt)
    return fixed


def parse_adm(path):
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        text = fh.read()

    m = _SHEET.search(text)
    sheet_name = m.group(1) if m else None
    body = text[m.end():] if m else text
    if body.rstrip().endswith("}"):
        body = body.rstrip()[:-1]

    # carve into sections
    marks = [(mm.start(), mm.group(1)) for mm in _SECTION.finditer(body)]
    sections = {}
    for i, (pos, name) in enumerate(marks):
        end = marks[i + 1][0] if i + 1 < len(marks) else len(body)
        sections[name] = body[body.index(":", pos) + 1:end]

    constants = {}
    range_pairs = {}
    for stmt in _split_statements(sections.get("constant", "")):
        cm = _LINE_COMMENT.search(stmt)
        comment = cm.group(1).strip() if cm else None
        clean = _LINE_COMMENT.sub("", stmt).strip()
        if ":" not in clean:
            continue
        key, expr = clean.split(":", 1)
        key, expr = key.strip(), expr.strip()
        if not key:
            continue
        val = _num(expr)
        rec = {"name": key, "expression": expr, "value": val}
        if comment:
            rec["comment"] = comment
        rm = _RANGE_DEF.match(expr)
        if rm:
            rec["is_range_of"] = rm.group(1)
            range_pairs[rm.group(1)] = None
        constants[key] = rec

    inputs = {}
    for stmt in _split_statements(sections.get("input", "")):
        cm = _LINE_COMMENT.search(stmt)
        comment = cm.group(1).strip() if cm else None
        clean = _LINE_COMMENT.sub("", stmt).strip()
        if ":" not in clean:
            continue
        key, expr = clean.split(":", 1)
        key = key.strip()
        rec = {"slot": key, "initial": _num(expr.strip()),
               "initial_expression": expr.strip()}
        if comment:
            rec["comment"] = comment
        inputs[key] = rec

    # Pass 1: collect every name -> expression declared in the interface too.
    # Several sheets declare their unit bounds there rather than in `constant`,
    # e.g. Dynamics puts filter_min / filter_max in `interface` because
    # filter_max depends on sample_rate.
    iface_defs = {}
    for stmt in _split_statements(sections.get("interface", "")):
        clean = _LINE_COMMENT.sub("", stmt).strip()
        if ":" not in clean:
            continue
        lhs, expr = clean.split(":", 1)
        lhs = lhs.strip()
        for kw in ("unlink ", "link ", "bind "):
            while lhs.startswith(kw):
                lhs = lhs[len(kw):].strip()
        if lhs:
            iface_defs[lhs] = " ".join(expr.split())

    def resolve_scalar(expr, depth=0):
        """Fold a constant/interface expression down to a number when possible."""
        if expr is None or depth > 6:
            return None
        expr = str(expr).strip()
        n = _num(expr)
        if n is not None:
            return n
        toks = re.findall(r"[A-Za-z_]\w*", expr)
        sub = expr
        for t in toks:
            src = None
            if t in constants:
                src = constants[t].get("expression")
            elif t in iface_defs:
                src = iface_defs[t]
            elif t in inputs:
                return None
            if src is None:
                return None
            r = resolve_scalar(src, depth + 1)
            if r is None:
                return None
            sub = re.sub(r"\b%s\b" % re.escape(t), repr(r), sub)
        if not re.fullmatch(r"[-+*/(). 0-9e']*", sub):
            return None
        try:
            val = eval(sub, {"__builtins__": {}}, {})   # arithmetic only
        except Exception:                               # noqa: BLE001
            return None
        return _num(val)

    def bound(stem, which):
        key = "%s_%s" % (stem, which)
        if key in constants:
            v = constants[key].get("value")
            if v is not None:
                return v
            return resolve_scalar(constants[key].get("expression"))
        if key in iface_defs:
            return resolve_scalar(iface_defs[key]) or iface_defs[key]
        return None

    params = []
    for stmt in _split_statements(sections.get("interface", "")):
        cm = _LINE_COMMENT.search(stmt)
        comment = cm.group(1).strip() if cm else None
        clean = _LINE_COMMENT.sub("", stmt).strip()
        if ":" not in clean:
            continue
        lhs, expr = clean.split(":", 1)
        lhs = lhs.strip()
        expr = " ".join(expr.split())
        modifiers = []
        while True:
            parts = lhs.split(None, 1)
            if len(parts) == 2 and parts[0] in ("unlink", "link", "bind"):
                modifiers.append(parts[0])
                lhs = parts[1].strip()
                continue
            break
        if not lhs:
            continue
        rec = {
            "name": lhs,
            "modifiers": modifiers,
            "expression": expr,
            "value_kind": None,
            "input_slot": None,
        }
        if comment:
            rec["comment"] = comment
            pi = re.search(r"(kParameterIndex_\w+)", comment)
            if pi:
                rec["host_parameter_constant"] = pi.group(1)

        mm = _LINEAR2.match(expr)
        if mm:
            stem = mm.group(3)
            rec.update({
                "value_kind": "linear scalar",
                "input_slot": mm.group(1),
                "normalised_input_range": [0, 1],
                "min": bound(stem, "min"),
                "max": bound(stem, "max"),
                "unit_family": stem,
                "mapping": "value = normalised * (%s_max - %s_min) + %s_min"
                           % (stem, stem, stem),
            })
        elif _BOOL.match(expr):
            rec.update({"value_kind": "boolean",
                        "input_slot": _BOOL.match(expr).group(1),
                        "mapping": "value = normalised > 0.5",
                        "min": False, "max": True})
        elif _SCALE.match(expr):
            g = _SCALE.match(expr)
            rec.update({"value_kind": "scaled scalar",
                        "input_slot": g.group(1),
                        "scale": _num(g.group(2)),
                        "mapping": "value = normalised * %s" % g.group(2)})
        elif _PLAIN.match(expr):
            rec.update({"value_kind": "pass-through",
                        "input_slot": expr,
                        "mapping": "value = normalised"})
        elif expr.startswith("{"):
            rec["value_kind"] = "structured (curve or point set)"
        else:
            rec["value_kind"] = "derived expression"
        if rec["input_slot"] and rec["input_slot"] in inputs:
            rec["input_slot_comment"] = inputs[rec["input_slot"]].get("comment")
            rec["input_initial_normalised"] = inputs[rec["input_slot"]]["initial"]
        params.append(rec)

    bound_slots = {p["input_slot"] for p in params if p.get("input_slot")}
    return {
        "sheet_name": sheet_name,
        "sections_present": sorted(sections),
        "constants": list(constants.values()),
        "constant_count": len(constants),
        "input_slots": list(inputs.values()),
        "input_slot_count": len(inputs),
        "input_slots_bound_to_a_named_parameter": len(bound_slots),
        "parameters": params,
        "parameter_count": len(params),
        "parameters_with_a_host_index_constant": sum(
            1 for p in params if p.get("host_parameter_constant")),
        "parameters_with_resolved_bounds": sum(
            1 for p in params if p.get("min") is not None),
    }
