"""Add allowed intra-Studio dependency edges to root workspace metadata and verify the allowed graph stays acyclic.
Usage: python -X utf8 -I edges.py <workspace_root> crate=dep [crate=dep ...]   (short names or hsk-studio-*)"""
import pathlib, re, sys, tomllib

root = pathlib.Path(sys.argv[1])
path = root / "Cargo.toml"
text = path.read_text(encoding="utf-8")
full = lambda n: n if n.startswith("hsk-studio-") else f"hsk-studio-{n}"
for spec in sys.argv[2:]:
    crate, dep = (full(x) for x in spec.split("=", 1))
    header = f"[workspace.metadata.studio.dependencies.{crate}]\n"
    start = text.index(header) + len(header)
    m = re.compile(r"^allowed = \[(.*)\]$", re.M).match(text, start)
    if not m:
        sys.exit(f"no single-line allowed list for {crate}")
    items = re.findall(r'"([^"]+)"', m.group(1))
    if dep not in items:
        items.append(dep)
        text = text[: m.start()] + "allowed = [" + ", ".join(f'"{x}"' for x in items) + "]" + text[m.end():]
graph = {k: v["allowed"] for k, v in tomllib.loads(text)["workspace"]["metadata"]["studio"]["dependencies"].items()}
state = {}


def visit(n, stack):
    if state.get(n) == 1:
        sys.exit("cycle: " + " -> ".join(stack + [n]))
    if state.get(n) == 2:
        return
    state[n] = 1
    for d in graph.get(n, []):
        visit(d, stack + [n])
    state[n] = 2


for n in graph:
    visit(n, [])
path.write_bytes(text.encode("utf-8"))
print("edges ok:", ", ".join(sys.argv[2:]) or "none")
