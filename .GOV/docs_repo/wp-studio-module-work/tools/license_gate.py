"""Fail if any pinned registry record has a non-permissive or unknown license. Usage: license_gate.py <workspace_root>"""
import glob, json, os, pathlib, re, sys, tarfile, tomllib

root = pathlib.Path(sys.argv[1])
reg = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["metadata"]["studio"]["registry"]
OK = {"MIT", "Apache-2.0", "0BSD", "BSD-2-Clause", "BSD-3-Clause", "Zlib", "Unlicense", "Unicode-3.0",
      "Unicode-DFS-2016", "ISC", "BSL-1.0", "MIT-0", "CC0-1.0", "Apache-2.0 WITH LLVM-exception"}
home = os.path.expanduser("~/.cargo/registry/src")
cache = os.path.expanduser("~/.cargo/registry/cache")


def license_of(n, v):
    """`license` from the extracted crate, else from a .crate archive cargo downloaded but did not extract, else None."""
    c = glob.glob(os.path.join(home, "*", f"{n}-{v}", "Cargo.toml"))
    if c:
        return tomllib.loads(open(c[0], encoding="utf-8").read())["package"].get("license")
    for a in glob.glob(os.path.join(cache, "*", f"{n}-{v}.crate")):
        with tarfile.open(a, "r:gz") as t:
            m = t.extractfile(f"{n}-{v}/Cargo.toml")
            if m:
                return tomllib.loads(m.read().decode("utf-8"))["package"].get("license")
    return None


# Last resort for packages absent from this host: crates.io records in license_evidence.json, accepted only when the
# record's checksum equals the pinned lock checksum (tools never download crates for this gate).
evidence = json.loads((pathlib.Path(__file__).parent / "license_evidence.json").read_text(encoding="utf-8"))["records"]


# Honest kind labels as tools/registry-sync.py derives them: native-ffi for -sys/_sys names and its NATIVE_FFI_EXTRA
# set (owner policy, native-ffi only in native owners, is the workspace probe's job).
m = re.search(r"^NATIVE_FFI_EXTRA: set\[str\] = (.*)$", (root / "tools" / "registry-sync.py").read_text(encoding="utf-8"), re.M)
extra = set(re.findall(r'"([^"]+)"', m.group(1))) if m else set()
bad = []
for owner, pol in reg.items():
    for key, pin in pol["locked"].items():
        n, v = key.split("@")
        lic = license_of(n, v)
        if not lic and key in evidence and evidence[key]["checksum"] == pin.get("checksum"):
            lic = evidence[key]["license"]
        if not lic:
            bad.append(f"{owner} {key}: license unknown")
            continue
        alts = [a.strip(" ()") for a in re.split(r"\bOR\b|/", lic.replace("(", " ").replace(")", " "))]
        # permissive if some OR-alternative consists only of permissive terms (AND-joined)
        if not any(all(t.strip() in OK for t in a.split(" AND ")) for a in alts):
            bad.append(f"{owner} {key}: {lic}")
        want = "native-ffi" if n.endswith(("-sys", "_sys")) or n in extra else "pure-rust"
        if pin.get("kind") != want:
            bad.append(f"{owner} {key}: kind {pin.get('kind')} (expected {want})")
print("LICENSE_GATE", "OK" if not bad else "FAIL", *bad, sep="\n")
sys.exit(1 if bad else 0)
