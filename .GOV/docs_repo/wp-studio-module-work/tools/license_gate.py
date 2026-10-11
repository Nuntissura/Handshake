"""Fail if any pinned registry record has a non-permissive or unknown license. Usage: license_gate.py <workspace_root>"""
import glob, os, pathlib, re, sys, tomllib

root = pathlib.Path(sys.argv[1])
reg = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["metadata"]["studio"]["registry"]
OK = {"MIT", "Apache-2.0", "0BSD", "BSD-2-Clause", "BSD-3-Clause", "Zlib", "Unlicense", "Unicode-3.0",
      "Unicode-DFS-2016", "ISC", "BSL-1.0", "MIT-0", "CC0-1.0", "Apache-2.0 WITH LLVM-exception"}
home = os.path.expanduser("~/.cargo/registry/src")
bad = []
for owner, pol in reg.items():
    for key, pin in pol["locked"].items():
        n, v = key.split("@")
        c = glob.glob(os.path.join(home, "*", f"{n}-{v}", "Cargo.toml"))
        lic = tomllib.loads(open(c[0], encoding="utf-8").read())["package"].get("license") if c else None
        if not lic:
            bad.append(f"{owner} {key}: license unknown")
            continue
        alts = [a.strip(" ()") for a in re.split(r"\bOR\b|/", lic.replace("(", " ").replace(")", " "))]
        # permissive if some OR-alternative consists only of permissive terms (AND-joined)
        if not any(all(t.strip() in OK for t in a.split(" AND ")) for a in alts):
            bad.append(f"{owner} {key}: {lic}")
        if pin.get("kind") != "pure-rust" and not n.endswith(("-sys", "_sys")):
            bad.append(f"{owner} {key}: kind {pin.get('kind')}")
print("LICENSE_GATE", "OK" if not bad else "FAIL", *bad, sep="\n")
sys.exit(1 if bad else 0)
