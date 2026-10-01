"""WP-KERNEL-012 export-current verifier (used by WPV-export-refresh.sh, [VPX-011] step 4).

usage: python WPV-export-verify.py <worktree> <candidate-sha> <export-dir>

Proves export-dir == the candidate exactly:
  * the file set equals `git ls-tree -r <candidate>` (only the MT-165 harness `.git` pointer and the
    export's own */target/nextest round outputs are excluded);
  * every file's bytes equal that blob's bytes as `git archive <candidate>` writes them (same eol
    conversion as the extraction), compared by sha256.
Blob ids are not recomputed from the files: with core.autocrlf=true, blobs that already contain CRLF
convert irreversibly, so `git hash-object` cannot reproduce their ids; the archive stream is the exact
reference for the bytes the refresh extracted.
Prints `VERIFY_OK files=<n> manifest_sha256=<hash of sorted 'blob-id path' lines>` or VERIFY_FAIL lines.
"""
import hashlib, os, subprocess, sys, tarfile

wt, sha, export = sys.argv[1], sys.argv[2], sys.argv[3]

ls = subprocess.run(["git", "-C", wt, "ls-tree", "-r", "-z", sha], capture_output=True, check=True).stdout
expected = {}
for entry in ls.split(b"\0"):
    if not entry:
        continue
    meta, path = entry.split(b"\t", 1)
    mode, kind, oid = meta.split(b" ")
    expected[path.decode("utf-8")] = oid.decode()

archive = subprocess.Popen(["git", "-c", "core.longpaths=true", "-C", wt, "archive", "--format=tar", sha], stdout=subprocess.PIPE)
want = {}
with tarfile.open(fileobj=archive.stdout, mode="r|") as tar:
    for member in tar:
        if member.isfile():
            want[member.name] = hashlib.sha256(tar.extractfile(member).read()).hexdigest()
archive.wait()
if archive.returncode != 0:
    print("VERIFY_FAIL git archive failed"); sys.exit(1)

present = set()
for root, dirs, files in os.walk(export):
    rel_root = os.path.relpath(root, export).replace(os.sep, "/")
    if rel_root.endswith("/target/nextest") or "/target/nextest/" in rel_root + "/":
        continue
    for f in files:
        rel = f if rel_root == "." else f"{rel_root}/{f}"
        if rel == ".git":
            continue
        present.add(rel)

missing = sorted(set(expected) - present)
extra = sorted(present - set(expected))
if missing or extra or set(want) != set(expected):
    print(f"VERIFY_FAIL file set differs: missing={len(missing)} extra={len(extra)} archive_vs_tree={len(set(want) ^ set(expected))}")
    for p in (missing + extra)[:5]:
        print(f"VERIFY_FAIL   {p}")
    sys.exit(1)

bad = []
long_prefix = "\\\\?\\" if os.name == "nt" else ""
for rel in sorted(expected):
    full = os.path.abspath(os.path.join(export, rel))
    with open(long_prefix + full if long_prefix else full, "rb") as fh:
        if hashlib.sha256(fh.read()).hexdigest() != want[rel]:
            bad.append(rel)
if bad:
    print(f"VERIFY_FAIL content differs: {len(bad)} file(s)")
    for p in bad[:5]:
        print(f"VERIFY_FAIL   {p}")
    sys.exit(1)

manifest = "".join(f"{expected[p]} {p}\n" for p in sorted(expected))
print(f"VERIFY_OK files={len(expected)} manifest_sha256={hashlib.sha256(manifest.encode()).hexdigest()}")
