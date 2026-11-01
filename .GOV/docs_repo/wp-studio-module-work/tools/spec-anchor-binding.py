#!/usr/bin/env python3
"""Strict spec-anchor binding check for the WP-KERNEL-STUDIO MT corpus.

spec-roundtrip-check.py counts a clause as covered when its anchor appears ANYWHERE in ANY JSON
file of the MT folder, including packet.json, refinement.json and STUDIO_MODULES.json, and
including incidental mentions (host_integration.spec_refs, prose, notes). [STU-ARC-022] says an
anchor-anywhere textual search is not a valid reconciliation. This check counts only BINDING
citations inside MT-*.json files:

  /spec_anchor[]                                    the derived or authored unit anchor
  /derivation/derivation_key                         spec-derive key (anchor#kind#index)
  /preserved_original/payload/spec_anchor[]          preserved original unit anchor
  /preserved_original/payload/derivation/derivation_key
  /context_capsule/specification_anchor_refs[]/id    V2 verified anchor reference
  /context_capsule/<name>_field_binding/clauses[]/id  foundation transport clause binding
  /implementation_state/canonical_type_transport/anchors[]   Type transport binding (MT-35536)

Usage (from anywhere):
  python spec-anchor-binding.py --mt-dir <WP-KERNEL-STUDIO> --spec <14-studio-creative-suite.md>
         --out <report.json> [--no-yield-manifest <studio-mt-derivation-manifest.json>]

A normative clause with no binding citation and not declared non-yielding is reported. The
optional derivation manifest supplies the deriver's own non-yielding set so the two tools agree.
"""
import argparse, collections, glob, json, os, re

ANCHOR = re.compile(r"\b(STU-[A-Z]+-\d+[a-z]?)(?![0-9A-Za-z*_`\\-])")
DEF_LINE = re.compile(r"^(?:\*\*)?\[(STU-[A-Z]+-\d+[a-z]?)\]")
NORMATIVE = re.compile(r"\b(MUST|MUST NOT|SHALL|SHALL NOT|REQUIRED|FORBIDDEN)\b")
TABLE_CELL_DEF = re.compile(r"\|\s*\*{0,2}\[(STU-[A-Z]+-\d+[a-z]?)\]\*{0,2}\s*\|")
HEADING_ANCHOR = re.compile(r"^#{1,4}\s+.*\[(STU-[A-Z]+-\d+[a-z]?)\]\s*$")
BINDING = re.compile(r"^(/spec_anchor|/derivation/derivation_key|/preserved_original/payload/spec_anchor"
                     r"|/preserved_original/payload/derivation/derivation_key"
                     r"|/context_capsule/specification_anchor_refs/id"
                     r"|/context_capsule/[a-z_]+_field_binding/clauses/id"
                     r"|/implementation_state/canonical_type_transport/anchors)$")


def walk(o, path, out):
    if isinstance(o, dict):
        for k, v in o.items():
            walk(v, path + '/' + k, out)
    elif isinstance(o, list):
        for v in o:
            walk(v, path, out)
    elif isinstance(o, str):
        for a in ANCHOR.findall(o):
            out.append((a, path))


def spec_clauses(md):
    lines = open(md, encoding='utf-8').read().split('\n')
    marks, fence = [], False
    for i, ln in enumerate(lines):
        if ln.lstrip().startswith('```'):
            fence = not fence
            continue
        if fence:
            continue
        m = DEF_LINE.match(ln)
        if m and (i == 0 or not lines[i - 1].strip()):
            marks.append((i, m.group(1)))
    out = {}
    for j, (n, a) in enumerate(marks):
        end = marks[j + 1][0] if j + 1 < len(marks) else len(lines)
        out[a] = {'line': n + 1, 'normative': bool(NORMATIVE.search('\n'.join(lines[n:end])))}
    # Anchors defined alone in a table cell or at the end of a heading are definitions too (the
    # round-trip tool's TABLE_CELL_DEF and HEADING_ANCHOR rules); they are not normative clauses
    # here, but a binding citation of one is not dangling.
    fence = False
    for i, ln in enumerate(lines):
        if ln.lstrip().startswith('```'):
            fence = not fence
            continue
        if fence:
            continue
        for a in TABLE_CELL_DEF.findall(ln) if ln.lstrip().startswith('|') else []:
            out.setdefault(a, {'line': i + 1, 'normative': False})
        h = HEADING_ANCHOR.match(ln)
        if h:
            out.setdefault(h.group(1), {'line': i + 1, 'normative': False})
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--mt-dir', required=True)
    ap.add_argument('--spec', required=True)
    ap.add_argument('--out', required=True)
    ap.add_argument('--no-yield-manifest', default=None)
    a = ap.parse_args()
    clauses = spec_clauses(a.spec)
    no_yield = {}
    if a.no_yield_manifest:
        man = json.load(open(a.no_yield_manifest, encoding='utf-8'))
        no_yield = {x['anchor']: x['reason'] for x in man.get('clauses_yielding_no_microtask', [])}
    binding = collections.defaultdict(set)
    incidental = collections.defaultdict(lambda: collections.defaultdict(set))
    non_mt = collections.defaultdict(set)
    for f in sorted(glob.glob(os.path.join(a.mt_dir, '*.json'))):
        n = os.path.basename(f)
        if n.startswith('_'):
            continue
        hits = []
        walk(json.load(open(f, encoding='utf-8')), '', hits)
        for anc, p in hits:
            if not n.startswith('MT-'):
                non_mt[anc].add(n)
            elif BINDING.match(p):
                binding[anc].add(n[:-5])
            else:
                incidental[anc][p].add(n[:-5])
    rows, gaps, masked = {}, [], []
    for anc, c in clauses.items():
        b = sorted(binding.get(anc, ()))
        inc = {k: sorted(v)[:12] for k, v in incidental.get(anc, {}).items()}
        rows[anc] = {'line': c['line'], 'normative': c['normative'], 'binding': b[:40],
                     'binding_count': len(b), 'declared_non_yielding': anc in no_yield,
                     'incidental_paths': inc, 'non_mt_files': sorted(non_mt.get(anc, ()))}
        if c['normative'] and not b and anc not in no_yield:
            gaps.append(anc)
            if inc or non_mt.get(anc):
                masked.append(anc)
    dangling = sorted(x for x in binding if x not in clauses)
    rep = {
        'schema_id': 'hsk.studio_spec_anchor_binding_report@1',
        'spec': a.spec.replace('\\', '/'),
        'normative_clauses': sum(1 for c in clauses.values() if c['normative']),
        'normative_without_binding': gaps,
        'normative_without_binding_count': len(gaps),
        'masked_by_incidental_or_non_mt_mentions': masked,
        'binding_citations_of_undefined_anchors': dangling,
        'anchors': rows,
    }
    with open(a.out, 'w', encoding='utf-8', newline='\n') as fh:
        json.dump(rep, fh, indent=1, ensure_ascii=False)
        fh.write('\n')
    print(json.dumps({k: rep[k] for k in ('normative_clauses', 'normative_without_binding_count',
                                          'normative_without_binding', 'masked_by_incidental_or_non_mt_mentions',
                                          'binding_citations_of_undefined_anchors')}))


if __name__ == '__main__':
    main()
