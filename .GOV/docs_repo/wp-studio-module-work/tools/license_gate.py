"""Fail if any pinned registry record has a non-permissive or unknown license.
Usage: license_gate.py <workspace_root>   |   license_gate.py --self-test (SPDX parser checks only; also run before every gate)
SPDX expressions are parsed, not split: parentheses group, WITH binds tightest, then AND, then OR (legacy `/` = OR;
operators case-insensitive). An expression passes when it evaluates true with every permitted term true: an AND fails
when any of its terms is disallowed, an OR passes when one alternative passes."""
import glob, json, os, pathlib, re, sys, tarfile, tomllib

OK = {"MIT", "Apache-2.0", "0BSD", "BSD-2-Clause", "BSD-3-Clause", "Zlib", "Unlicense", "Unicode-3.0",
      "Unicode-DFS-2016", "ISC", "BSL-1.0", "MIT-0", "CC0-1.0", "Apache-2.0 WITH LLVM-exception"}
# Explicit per-package allowances, limited to owners (lead decision 2026-10-14, steward 08f28d0 finding): egui's
# bundled default fonts are permitted only in the UI crates (harness-view: the UI half of the harness after the
# SMB-CRATE-SEAM-SPLITS 3b split, 12e2d46).
PACKAGE_ALLOW = {
    "epaint_default_fonts": {"owners": {"hsk-studio-controls", "hsk-studio-harness", "hsk-studio-harness-view"},
                             "terms": {"OFL-1.1", "Ubuntu-font-1.0"}},
}


def tokens(expr):
    out = []
    for t in re.findall(r"\(|\)|/|[^\s()/]+", expr):
        u = t.upper()
        out.append(u if u in ("AND", "OR", "WITH") else "OR" if t == "/" else t)
    return out


def parse(expr):
    """SPDX expression -> nested tuples ('or'|'and', [children]) or ('term', 'ID' | 'ID WITH EXC'). ValueError on junk."""
    toks, pos = tokens(expr), 0

    def peek():
        return toks[pos] if pos < len(toks) else None

    def take(want=None):
        nonlocal pos
        t = peek()
        if t is None or (want is not None and t != want):
            raise ValueError(f"bad SPDX expression {expr!r} at token {pos}")
        pos += 1
        return t

    def primary():
        if peek() == "(":
            take("(")
            node = disjunction()
            take(")")
            return node
        t = take()
        if t in ("AND", "OR", "WITH", ")"):
            raise ValueError(f"bad SPDX expression {expr!r}: unexpected {t}")
        if peek() == "WITH":
            take("WITH")
            exc = take()
            if exc in ("AND", "OR", "WITH", "(", ")"):
                raise ValueError(f"bad SPDX expression {expr!r}: WITH needs an exception id")
            return ("term", f"{t} WITH {exc}")
        return ("term", t)

    def chain(op, sub):
        kids = [sub()]
        while peek() == op:
            take(op)
            kids.append(sub())
        return kids[0] if len(kids) == 1 else (op.lower(), kids)

    def conjunction():
        return chain("AND", primary)

    def disjunction():
        return chain("OR", conjunction)

    node = disjunction()
    if pos != len(toks):
        raise ValueError(f"bad SPDX expression {expr!r}: trailing tokens")
    return node


def permitted(expr, allowed):
    node = parse(expr)

    def ev(n):
        kind, val = n
        if kind == "term":
            return val in allowed
        return (any if kind == "or" else all)(ev(k) for k in val)

    return ev(node)


def fuzzing_skipped(pol):
    """Lead decision 2026-10-15 (rav1e, option b): entries tools/registry-sync.py reached ONLY through `fuzzing-edges`
    (cfg(fuzzing) target dependencies) are not in `locked` and are skipped here; an edge child that is also in `locked`
    (it has a normal path) is gated like every other record. Returns [(child, via parent)] skipped."""
    out = []
    for edge in pol.get("fuzzing-edges", []):
        parent, child = edge.split(">", 1)
        if child not in pol["locked"]:
            out.append((child, parent))
    return sorted(set(out))


def self_test():
    pol = {"locked": {"rav1e@0.8.1": {}}, "fuzzing-edges": ["rav1e@0.8.1>libfuzzer-sys@0.4.13"]}
    fuzz_fails = [] if fuzzing_skipped(pol) == [("libfuzzer-sys@0.4.13", "rav1e@0.8.1")] else ["fuzzing-only not skipped"]
    pol["locked"]["libfuzzer-sys@0.4.13"] = {}  # same entry reached by a normal path: locked, so gated, not skipped
    fuzz_fails += [] if fuzzing_skipped(pol) == [] else ["normal-path entry skipped"]
    ui = OK | PACKAGE_ALLOW["epaint_default_fonts"]["terms"]
    cases = [
        ("(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0", OK, False),  # the epaint case without the allowance
        ("(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0", ui, True),   # ... with the UI-crate allowance
        ("(MIT OR Apache-2.0) AND GPL-3.0", OK, False),                      # (A OR B) AND C, C disallowed
        ("(MIT OR Apache-2.0) AND Unicode-3.0", OK, True),
        ("MIT OR Apache-2.0 AND GPL-3.0", OK, True),                          # AND binds tighter: MIT alone suffices
        ("GPL-3.0 OR Apache-2.0 AND LGPL-2.1", OK, False),
        ("Apache-2.0 WITH LLVM-exception", OK, True),                         # WITH: exact listed pairing
        ("Apache-2.0 WITH Classpath-exception-2.0", OK, False),
        ("Apache-2.0 WITH LLVM-exception OR MIT", {"MIT"}, True),
        ("MIT/Apache-2.0", OK, True),                                          # legacy slash = OR
        ("Apache-2.0 or MIT", OK, True),
    ]
    fails = [(e, want) for e, allowed, want in cases if permitted(e, allowed) != want]
    for bad in ("MIT AND", "(MIT OR Apache-2.0", "Apache-2.0 WITH", "MIT Apache-2.0"):
        try:
            parse(bad)
            fails.append((bad, "ValueError"))
        except ValueError:
            pass
    fails += fuzz_fails
    if fails:
        print("LICENSE_GATE_SELF_TEST FAIL", *fails, sep="\n")
        sys.exit(2)
    return len(cases) + 6


if __name__ == "__main__":
    n_checks = self_test()
    if sys.argv[1:] == ["--self-test"]:
        print(f"LICENSE_GATE_SELF_TEST OK ({n_checks} checks)")
        sys.exit(0)
    root = pathlib.Path(sys.argv[1])
    reg = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["metadata"]["studio"]["registry"]
    home = os.path.expanduser("~/.cargo/registry/src")
    cache = os.path.expanduser("~/.cargo/registry/cache")

    def license_of(n, v):
        """`license` from the extracted crate, else from a .crate archive cargo downloaded but did not extract."""
        c = glob.glob(os.path.join(home, "*", f"{n}-{v}", "Cargo.toml"))
        if c:
            return tomllib.loads(open(c[0], encoding="utf-8").read())["package"].get("license")
        for a in glob.glob(os.path.join(cache, "*", f"{n}-{v}.crate")):
            with tarfile.open(a, "r:gz") as t:
                m = t.extractfile(f"{n}-{v}/Cargo.toml")
                if m:
                    return tomllib.loads(m.read().decode("utf-8"))["package"].get("license")
        return None

    # Last resort for packages absent from this host: crates.io records in license_evidence.json, accepted only when
    # the record's checksum equals the pinned lock checksum (tools never download crates for this gate).
    evidence = json.loads((pathlib.Path(__file__).parent / "license_evidence.json").read_text(encoding="utf-8"))["records"]

    # Honest kind labels as tools/registry-sync.py derives them: native-ffi for -sys/_sys names and its
    # NATIVE_FFI_EXTRA set (owner policy, native-ffi only in native owners, is the workspace probe's job).
    m = re.search(r"^NATIVE_FFI_EXTRA: set\[str\] = (.*)$", (root / "tools" / "registry-sync.py").read_text(encoding="utf-8"), re.M)
    extra = set(re.findall(r'"([^"]+)"', m.group(1))) if m else set()
    bad, skipped = [], []
    for owner, pol in reg.items():
        skipped += [f"{owner} {child} (cfg(fuzzing) only, via {parent})" for child, parent in fuzzing_skipped(pol)]
        for key, pin in pol["locked"].items():
            n, v = key.split("@")
            lic = license_of(n, v)
            if not lic and key in evidence and evidence[key]["checksum"] == pin.get("checksum"):
                lic = evidence[key]["license"]
            if not lic:
                bad.append(f"{owner} {key}: license unknown")
                continue
            allowed = OK
            grant = PACKAGE_ALLOW.get(n)
            if grant and owner in grant["owners"]:
                allowed = OK | grant["terms"]
            try:
                ok = permitted(lic, allowed)
            except ValueError as e:
                ok, lic = False, f"{lic} ({e})"
            if not ok:
                bad.append(f"{owner} {key}: {lic}")
            want = "native-ffi" if n.endswith(("-sys", "_sys")) or n in extra else "pure-rust"
            if pin.get("kind") != want:
                bad.append(f"{owner} {key}: kind {pin.get('kind')} (expected {want})")
    print("LICENSE_GATE", "OK" if not bad else "FAIL", *bad, *[f"SKIPPED {s}" for s in skipped], sep="\n")
    sys.exit(1 if bad else 0)
