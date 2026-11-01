#!/usr/bin/env python3
"""Record in the spec-derive id map the derivation keys that existing MTs already materialise.

spec-derive-microtasks.py binds derivation_key -> mt_id in an append-only id map. Units added to
the spec after the last derivation run (v02.207 amendments, v02.209 slide clauses, the 14.23
foundation transports) were materialised by hand as MTs outside the map, so a fresh derivation
would allocate NEW ids for units that already have an MT. This tool finds, for every key a
dry-run derivation yields that the map lacks, the existing MT that binds that unit, and appends
key -> that MT. Existing map entries are never changed. Keys with no materialising MT stay out
of the map; they are the true forward gaps the deriver then allocates fresh ids for.

Resolution order for a key ANCHOR#KIND#INDEX (only a binding citation counts, never prose):
  1. exactly one MT whose top-level spec_anchor is exactly [ANCHOR] and whose index unit_kind is
     KIND, spec207_amendment (for KIND clause) or a clause derived for a later scoped amendment;
  2. a foundation MT that binds ANCHOR in context_capsule.<name>_field_binding.clauses[] or in
     implementation_state.canonical_type_transport.anchors[]; when several do, the one whose
     module short name appears in the clause's spec heading (the section owner);
  3. otherwise unresolved (a forward gap).

Usage:
  python spec-idmap-backfill.py --id-map <map.json> --derived-dir <dry-run spec_mts> \
      --mt-dir <WP-KERNEL-STUDIO> --report <out.json> [--write]
"""
import argparse, glob, json, os, re
from datetime import datetime, timezone


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--id-map', required=True)
    ap.add_argument('--derived-dir', required=True)
    ap.add_argument('--mt-dir', required=True)
    ap.add_argument('--report', required=True)
    ap.add_argument('--write', action='store_true')
    a = ap.parse_args()
    id_map = json.load(open(a.id_map, encoding='utf-8'))
    idx = json.load(open(os.path.join(a.mt_dir, '_MT_INDEX.json'), encoding='utf-8'))
    kind_of = {e['mt_id']: e['unit_kind'] for e in idx['entries']}
    module_of = {e['mt_id']: e.get('active_module_ref') or '' for e in idx['entries']}
    mapped = set(id_map.values())

    derived = {}
    for f in glob.glob(os.path.join(a.derived_dir, 'MT-*.json')):
        d = json.load(open(f, encoding='utf-8'))
        k = d['derivation']['derivation_key']
        if k not in id_map:
            derived[k] = d
    wanted = {k.split('#')[0] for k in derived}

    exact, fb = {}, {}
    for e in idx['entries']:
        p = os.path.join(a.mt_dir, e['file'])
        sa = e.get('spec_anchor') or []
        hit = [x for x in sa if x in wanted]
        d = None
        if len(sa) == 1 and hit:
            exact.setdefault(sa[0], []).append(e['mt_id'])
        if e['unit_kind'] == 'asset_foundation':
            d = json.load(open(p, encoding='utf-8'))
            cc = d.get('context_capsule') or {}
            for key, blk in cc.items():
                if key.endswith('_field_binding') and isinstance(blk, dict):
                    for c in blk.get('clauses') or []:
                        if c.get('id') in wanted:
                            fb.setdefault(c['id'], []).append((e['mt_id'], f'context_capsule/{key}'))
            tt = ((d.get('implementation_state') or {}).get('canonical_type_transport') or {})
            for anc in tt.get('anchors') or []:
                if anc in wanted:
                    fb.setdefault(anc, []).append((e['mt_id'], 'implementation_state/canonical_type_transport'))

    rows, unresolved, ambiguous = [], [], []
    for k in sorted(derived, key=lambda x: derived[x]['derivation']['spec_line']):
        anc, kind, _ = k.split('#')
        heading = derived[k]['derivation']['spec_heading'] or ''
        cand = [m for m in exact.get(anc, []) if m not in mapped and (
            kind_of.get(m) == kind or (kind == 'clause' and kind_of.get(m) == 'spec207_amendment'))]
        how = 'exact_unit_anchor'
        if len(cand) > 1:
            ambiguous.append({'key': k, 'candidates': cand})
            continue
        if not cand and kind == 'clause' and anc in fb:
            owners = fb[anc]
            if len(owners) > 1:
                pick = [o for o in owners if module_of[o[0]].split('-')[-1].lower() in heading.lower()]
                owners = pick if len(pick) == 1 else owners
            if len(owners) == 1:
                cand, how = [owners[0][0]], 'foundation_transport_binding:' + owners[0][1]
            else:
                ambiguous.append({'key': k, 'candidates': owners})
                continue
        if not cand:
            unresolved.append({'key': k, 'spec_heading': heading, 'spec_line': derived[k]['derivation']['spec_line']})
            continue
        rows.append({'key': k, 'mt_id': cand[0], 'resolution': how, 'mt_unit_kind': kind_of.get(cand[0]),
                     'spec_heading': heading})

    many = {}
    for r in rows:
        many.setdefault(r['mt_id'], []).append(r['key'])
    report = {
        'schema_id': 'hsk.studio_spec_idmap_backfill@1',
        'mode': 'write' if a.write else 'dry_run',
        'generated_at': datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
        'id_map': a.id_map.replace('\\', '/'),
        'id_map_entries_before': len(id_map),
        'keys_missing_from_map': len(derived),
        'backfilled': rows,
        'backfilled_count': len(rows),
        'shared_mt_ids': {m: ks for m, ks in many.items() if len(ks) > 1},
        'shared_note': 'A 14.23 foundation transport section declares N source-only transport units that '
                       'bind ONE existing foundation MT (the section derivation markers say so); those '
                       'keys share that MT id by design.',
        'ambiguous': ambiguous,
        'unresolved_forward_gaps': unresolved,
    }
    if a.write and not ambiguous:
        for r in rows:
            assert r['key'] not in id_map
            id_map[r['key']] = r['mt_id']
        tmp = a.id_map + '.tmp'
        with open(tmp, 'w', encoding='utf-8', newline='\n') as fh:
            fh.write(json.dumps(id_map, indent=1, sort_keys=True))
        os.replace(tmp, a.id_map)
        report['id_map_entries_after'] = len(id_map)
    with open(a.report, 'w', encoding='utf-8', newline='\n') as fh:
        json.dump(report, fh, indent=1, ensure_ascii=False)
        fh.write('\n')
    print(json.dumps({k: (v if not isinstance(v, list) else len(v)) for k, v in report.items()
                      if k not in ('shared_mt_ids', 'shared_note')}))
    print('unresolved:', [u['key'] for u in unresolved])
    print('ambiguous:', ambiguous)
    return 1 if ambiguous else 0


if __name__ == '__main__':
    raise SystemExit(main())
