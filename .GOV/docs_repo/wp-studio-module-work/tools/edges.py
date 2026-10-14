"""Add (crate=dep) or remove (remove:crate=dep) allowed intra-Studio dependency edges to root workspace metadata and verify the allowed graph stays acyclic.
Dev-only edges (lead decision B1, 2026-10-14): `dev:crate=dep` / `remove:dev:crate=dep` edit the optional `dev-allowed`
list of the crate (single line right after `allowed`); tools/workspace-probe.rs accepts a `dev-allowed` path dep only
in [dev-dependencies]. The cycle check runs over normal and dev edges together.
Usage: python -X utf8 -I edges.py <workspace_root> [remove:][dev:]crate=dep ...   (short names or hsk-studio-*)"""
import pathlib, re, sys, tomllib

root = pathlib.Path(sys.argv[1])
path = root / "Cargo.toml"
text = path.read_text(encoding="utf-8")
full = lambda n: n if n.startswith("hsk-studio-") else f"hsk-studio-{n}"
for spec in sys.argv[2:]:
    remove = spec.startswith("remove:")
    rest = spec.removeprefix("remove:")
    dev = rest.startswith("dev:")
    crate, dep = (full(x) for x in rest.removeprefix("dev:").split("=", 1))
    header = f"[workspace.metadata.studio.dependencies.{crate}]\n"
    start = text.index(header) + len(header)
    m = re.compile(r"^allowed = \[(.*)\]\n", re.M).match(text, start)
    if not m:
        sys.exit(f"no single-line allowed list for {crate}")
    if dev:
        d = re.compile(r"^dev-allowed = \[(.*)\]\n", re.M).match(text, m.end())
        items = re.findall(r'"([^"]+)"', d.group(1)) if d else []
        span = (d.start(), d.end()) if d else (m.end(), m.end())
        key = "dev-allowed"
    else:
        items = re.findall(r'"([^"]+)"', m.group(1))
        span, key = (m.start(), m.end()), "allowed"
    if remove and dep in items:
        items.remove(dep)
    elif not remove and dep not in items:
        items.append(dep)
    else:
        continue
    line = "" if dev and not items else f"{key} = [" + ", ".join(f'"{x}"' for x in items) + "]\n"
    text = text[: span[0]] + line + text[span[1]:]
deps = tomllib.loads(text)["workspace"]["metadata"]["studio"]["dependencies"]
graph = {k: v["allowed"] + v.get("dev-allowed", []) for k, v in deps.items()}
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
