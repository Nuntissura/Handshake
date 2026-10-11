"""Write workspace-probe provenance TOML. Usage: python -X utf8 -I provenance.py <workspace_root> <revision> <tree> <out_file> <package>..."""
import hashlib, pathlib, sys

root = pathlib.Path(sys.argv[1])
revision, tree, out = sys.argv[2], sys.argv[3], pathlib.Path(sys.argv[4])
files = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"] + [f"crates/{p}/Cargo.toml" for p in sys.argv[5:]]
lines = [
    "[source]",
    'repository="https://github.com/Nuntissura/assets"',
    'subtree="handshake-studio-assets"',
    f'revision="{revision}"',
    f'tree="{tree}"',
    "[files]",
]
for f in sorted(files):
    lines.append(f'"{f}"="{hashlib.sha256((root / f).read_bytes()).hexdigest()}"')
out.parent.mkdir(parents=True, exist_ok=True)
out.write_bytes(("\n".join(lines) + "\n").encode("utf-8"))
print(out)
