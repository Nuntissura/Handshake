#!/usr/bin/env python3
"""Bind published spec clauses to an existing MT as a <name>_field_binding block.

Precedent: the 14.23 foundation MTs carry context_capsule.<module>_current_field_binding blocks
(folio MT-35531, chronicle MT-35532, prism MT-35533, pigment MT-35534, nib MT-35535) that bind the
foundation transport clauses STU-CON-015..036 to the foundation MT with exact source ranges and
hashes. This adds such a block for clauses the spec assigns to an MT that never received one.

Nothing else changes: existing keys keep their order, lifecycle/status is untouched, the file keeps
its indent, newline style and trailing newline; the block is inserted at the end of context_capsule.

Usage (from the kernel root):
  python .GOV/docs_repo/wp-studio-module-work/tools/mt-bind-clauses.py --mt MT-35538 \
      --block develop_current_field_binding --spec-version v02.226 --publication-commit <sha> \
      --anchors STU-CON-042 STU-CON-043 ... --preservation "<one line>" [--write]
"""
import argparse, hashlib, json, os, re, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
KERNEL = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
sys.path.insert(0, HERE)
from importlib import util as _u  # noqa: E402
_spec = _u.spec_from_file_location('mthi', os.path.join(HERE, 'mt-host-integration.py'))
mthi = _u.module_from_spec(_spec)
_spec.loader.exec_module(mthi)

DEF_LINE = re.compile(r"^(?:\*\*)?\[(STU-[A-Z]+-\d+[a-z]?)\]")


def clause_ranges(md_path):
    """Clause ranges in the precedent encoding: a clause runs from its definition line to the line
    before the next clause definition or heading; its body hash is those lines joined with LF and
    right-stripped. Verified against all 26 STU-CON bindings in MT-35531..35535."""
    lines = open(md_path, encoding='utf-8').read().split('\n')
    stops = [i for i, l in enumerate(lines)
             if (DEF_LINE.match(l) and (i == 0 or not lines[i - 1].strip())) or l.startswith('#')]
    out = {}
    for i in stops:
        m = DEF_LINE.match(lines[i])
        if not m:
            continue
        nxt = next((s for s in stops if s > i), len(lines))
        body = '\n'.join(lines[i:nxt]).rstrip()
        out[m.group(1)] = {'start_line': i + 1, 'end_line': nxt,
                           'body_sha256': hashlib.sha256(body.encode('utf-8')).hexdigest()}
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--mt', required=True)
    ap.add_argument('--block', required=True)
    ap.add_argument('--spec-version', required=True)
    ap.add_argument('--publication-commit', required=True)
    ap.add_argument('--anchors', nargs='+', required=True)
    ap.add_argument('--preservation', required=True)
    ap.add_argument('--write', action='store_true')
    a = ap.parse_args()
    rel = f'.GOV/spec/{"master-spec-" + a.spec_version}/spec-modules/14-studio-creative-suite.md'
    md = os.path.join(KERNEL, rel)
    src_sha = hashlib.sha256(open(md, 'rb').read()).hexdigest()
    ptr = subprocess.run(['git', '-C', KERNEL, 'show', f'{a.publication_commit}:.GOV/spec/SPEC_CURRENT.md'],
                         capture_output=True, check=True).stdout
    ranges = clause_ranges(md)
    missing = [x for x in a.anchors if x not in ranges]
    if missing:
        sys.exit(f'anchors not defined in {rel}: {missing}')
    path = os.path.join(KERNEL, '.GOV', 'task_packets', 'WP-KERNEL-STUDIO', a.mt + '.json')
    raw = open(path, 'rb').read()
    d = json.loads(raw)
    cc = d['context_capsule']
    if a.block in cc:
        sys.exit(f'{a.mt} already carries context_capsule.{a.block}; refusing to overwrite')
    status_before = json.dumps(d.get('lifecycle'), sort_keys=True)
    cc[a.block] = {
        'publication_commit': a.publication_commit,
        'version': a.spec_version,
        'pointer_ref': '.GOV/spec/SPEC_CURRENT.md',
        'pointer_sha256': hashlib.sha256(ptr).hexdigest(),
        'preservation': a.preservation,
        'clauses': [{'id': x, 'source_ref': rel, 'start_line': ranges[x]['start_line'],
                     'end_line': ranges[x]['end_line'], 'source_sha256': src_sha,
                     'body_sha256': ranges[x]['body_sha256'],
                     'hash_encoding': 'UTF8 LF joined body trailing whitespace stripped'} for x in a.anchors],
    }
    out = mthi.dump(d, mthi.detect_style(raw))
    assert json.loads(out) == d
    assert json.dumps(json.loads(out).get('lifecycle'), sort_keys=True) == status_before
    before = json.loads(raw)
    del d['context_capsule'][a.block]
    assert d == before, 'only the new block may differ'
    print(json.dumps({'mt': a.mt, 'block': a.block, 'clauses': [c for c in a.anchors], 'write': a.write,
                      'bytes_before': len(raw), 'bytes_after': len(out)}))
    if a.write:
        tmp = path + '.tmp'
        with open(tmp, 'wb') as f:
            f.write(out)
        os.replace(tmp, path)


if __name__ == '__main__':
    main()
