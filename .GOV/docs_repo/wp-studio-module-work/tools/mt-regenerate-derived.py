#!/usr/bin/env python3
"""Regenerate the spec-derived surfaces of existing WP-KERNEL-STUDIO MTs after a spec revision.

Mechanism: spec-derive-microtasks.py is re-run on the new spec into the staging tree (same id map,
so every unit keeps its mt_id). For each selected MT the derived surfaces are replaced by the new
deriver output; every other part of the contract is kept.

Selection (--auto): MTs whose deriver output changed between --baseline (previous spec run) and
--staging (new spec run) AND whose live derived surfaces still equal the baseline (so the change
is purely the spec revision). --also <MT ...> adds MTs whose live surfaces are stale against the
new spec for an older reason (for example MT-34110); their live surfaces serve as the baseline.

Replaced (derived surfaces): acceptance_criteria, acceptance_criteria_count, clause, scope.summary,
derivation.spec_line/spec_heading (+ derivation.spec_version), host_integration.spec_version.
Propagated: inside the active contract, a string equal to an old acceptance criterion, old scope
summary or old clause is replaced by the new one (also as a substring for strings >= 40 chars).
New acceptance rows are appended to closure_unit.materialized_acceptance/acceptance_refs with a
proof check. Never touched: lifecycle (status), verdicts, submissions, preserved_original, any
*original* / lineage field (immutable lineage per [STU-ARC-022]). Each MT gains a spec_regeneration
entry recording old -> new per row.

Index: changed MTs' entries get clause / acceptance_criteria_count / active_acceptance_count;
totals.microtasks and totals.by_unit_kind are recounted from the entries. Serialised exactly like
HEAD (json.dumps(obj)+'\\n') via temp + os.replace; refused if anything else differs.

Usage (from the kernel root):
  python .GOV/docs_repo/wp-studio-module-work/tools/mt-regenerate-derived.py --staging <new spec_mts> \
      --baseline <previous spec_mts> --spec-version v02.228 --spec-commit <sha> --auto [--also MT-34110] [--write]
"""
import argparse, collections, glob, json, os, sys
from datetime import datetime, timezone
from importlib import util as _u

HERE = os.path.dirname(os.path.abspath(__file__))
KERNEL = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
PACKET = os.path.join(KERNEL, '.GOV', 'task_packets', 'WP-KERNEL-STUDIO')
_s = _u.spec_from_file_location('mthi', os.path.join(HERE, 'mt-host-integration.py'))
mthi = _u.module_from_spec(_s)
_s.loader.exec_module(mthi)

SKIP_KEYS = {'lifecycle', 'verdicts', 'submissions', 'preserved_original', 'lineage', 'spec_regeneration',
             'acceptance_criteria', 'scope', 'clause', 'derivation', 'host_integration'}


def skip(key):
    return key in SKIP_KEYS or 'original' in key


def propagate(o, repl, hits, path=''):
    if isinstance(o, dict):
        for k in list(o):
            if path == '' and k in SKIP_KEYS or skip(k):
                continue
            v = o[k]
            if isinstance(v, str):
                nv = sub(v, repl)
                if nv != v:
                    o[k] = nv
                    hits.append(path + '/' + k)
            else:
                propagate(v, repl, hits, path + '/' + k)
    elif isinstance(o, list):
        for i, v in enumerate(o):
            if isinstance(v, str):
                nv = sub(v, repl)
                if nv != v:
                    o[i] = nv
                    hits.append(f'{path}[{i}]')
            else:
                propagate(v, repl, hits, f'{path}[{i}]')


def sub(s, repl):
    for old, new in repl:
        if s == old:
            return new
    for old, new in repl:
        if len(old) >= 40 and old in s:
            s = s.replace(old, new)
    return s


def regenerate(d, new, spec_ver, spec_commit, now):
    old_ac = d.get('acceptance_criteria') or []
    old_by = {a['id']: a['criterion'] for a in old_ac}
    new_by = {a['id']: a['criterion'] for a in new['acceptance_criteria']}
    rows = [{'id': i, 'old': old_by.get(i), 'new': new_by.get(i)} for i in sorted(set(old_by) | set(new_by))
            if old_by.get(i) != new_by.get(i)]
    old_scope = (d.get('scope') or {}).get('summary')
    old_clause = d.get('clause')
    repl = [(r['old'], r['new']) for r in rows if r['old'] and r['new']]
    if old_scope and old_scope != new['scope']['summary']:
        repl.append((old_scope, new['scope']['summary']))
    if old_clause and old_clause != new['clause']:
        repl.append((old_clause, new['clause']))
    lifecycle = json.dumps(d.get('lifecycle'), sort_keys=True)
    hits = []
    propagate(d, repl, hits)
    d['acceptance_criteria'] = new['acceptance_criteria']
    d['acceptance_criteria_count'] = new['acceptance_criteria_count']
    d['clause'] = new['clause']
    d.setdefault('scope', {})['summary'] = new['scope']['summary']
    der = d.setdefault('derivation', {})
    der['spec_line'] = new['derivation']['spec_line']
    der['spec_heading'] = new['derivation']['spec_heading']
    der['spec_version'] = spec_ver
    if isinstance(d.get('host_integration'), dict):
        d['host_integration']['spec_version'] = spec_ver
    cu = d.setdefault('closure_unit', {})
    mat = cu.setdefault('materialized_acceptance', [])
    have = {m['id'] for m in mat}
    appended, removed = [], [r['id'] for r in rows if r['new'] is None]
    for r in rows:
        if r['old'] is None and r['new'] is not None and r['id'] not in have:
            mat.append({'id': r['id'], 'text': r['new']})
            cu.setdefault('acceptance_refs', []).append(r['id'])
            d.setdefault('proof', {}).setdefault('checks', []).append({
                'check_id': f'{d.get("mt_id") or d["identity"]["mt_id"]}-SPEC-{r["id"]}', 'kind': 'test',
                'claim_refs': [r['id']] + (d.get('spec_anchor') or [])[:1],
                'command_or_method': 'Execute the owning module natural contract target for this row once it materialises during coding.',
                'expected': r['new'], 'timeout_seconds': None, 'on_timeout': 'record_timeout',
                'evidence_required': ['Exact candidate commit, toolchain, features and lock',
                                      'Nonzero executed case count with independent expected values']})
            appended.append(r['id'])
    d.setdefault('spec_regeneration', []).append({
        'spec_version': spec_ver, 'spec_commit': spec_commit, 'at': now,
        'mechanism': 'spec-derive-microtasks.py re-run on the new spec, same id map; tools/mt-regenerate-derived.py',
        'acceptance_rows': rows, 'appended_materialized_rows': appended, 'rows_absent_from_new_spec': removed,
        'scope_summary_changed': bool(old_scope and old_scope != new['scope']['summary']),
        'clause_changed': old_clause != new['clause'], 'old_clause': old_clause,
        'propagated_paths': hits,
        'lineage_untouched': 'preserved_original and every *original* field keep the captured text'})
    if 'updated_at_utc' in d:
        d['updated_at_utc'] = now
    assert json.dumps(d.get('lifecycle'), sort_keys=True) == lifecycle
    return d, rows, appended, removed, hits


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--staging', required=True)
    ap.add_argument('--baseline', default=None)
    ap.add_argument('--spec-version', required=True)
    ap.add_argument('--spec-commit', required=True)
    ap.add_argument('--auto', action='store_true')
    ap.add_argument('--also', nargs='*', default=[])
    ap.add_argument('--write', action='store_true')
    a = ap.parse_args()
    now = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
    idx_path = os.path.join(PACKET, '_MT_INDEX.json')
    old_raw = open(idx_path, 'rb').read()
    idx = json.loads(old_raw)
    if (json.dumps(idx) + '\n').encode('utf-8') != old_raw:
        sys.exit('index serialisation differs from json.dumps(obj)+newline; refusing')
    pick, skipped_drift = [], []
    if a.auto:
        for f in sorted(glob.glob(os.path.join(a.staging, 'MT-*.json'))):
            n = os.path.basename(f)
            b = json.load(open(f, encoding='utf-8'))
            base = json.load(open(os.path.join(a.baseline, n), encoding='utf-8'))
            if all(base[k] == b[k] for k in ('acceptance_criteria', 'clause')) and base['scope'] == b['scope']:
                continue
            p = os.path.join(PACKET, n)
            if not os.path.exists(p):
                continue
            d = json.load(open(p, encoding='utf-8'))
            if (d.get('derivation') or {}).get('derivation_key') != b['derivation']['derivation_key']:
                skipped_drift.append({'mt': n[:-5], 'reason': 'MT is not the single derived owner of this key'})
                continue
            live_eq = (d.get('acceptance_criteria') == base['acceptance_criteria'] and d.get('clause') == base['clause']
                       and (d.get('scope') or {}).get('summary') == base['scope']['summary'])
            if not live_eq:
                skipped_drift.append({'mt': n[:-5], 'reason': 'live derived surfaces differ from the baseline run'})
                continue
            pick.append(n[:-5])
    pick += [m for m in a.also if m not in pick]
    files, report_rows = {}, []
    entries = {e['mt_id']: e for e in idx['entries']}
    for m in pick:
        p = os.path.join(PACKET, m + '.json')
        raw = open(p, 'rb').read()
        d = json.loads(raw)
        new = json.load(open(os.path.join(a.staging, m + '.json'), encoding='utf-8'))
        d, rows, appended, removed, hits = regenerate(d, new, a.spec_version, a.spec_commit, now)
        out = mthi.dump(d, mthi.detect_style(raw))
        assert json.loads(out) == d
        files[p] = out
        e = entries[m]
        e['clause'] = d['clause']
        e['acceptance_criteria_count'] = d['acceptance_criteria_count']
        e['active_acceptance_count'] = len(d['closure_unit']['materialized_acceptance'])
        report_rows.append({'mt': m, 'acceptance_rows_changed': rows, 'appended': appended, 'removed': removed,
                            'propagated_path_count': len(hits), 'scope_only': not rows})
    tot = idx.get('totals', {})
    tot_before = json.loads(json.dumps(tot))
    tot['microtasks'] = len(idx['entries'])
    if 'by_unit_kind' in tot:
        tot['by_unit_kind'] = dict(collections.Counter(e['unit_kind'] for e in idx['entries']))
        tot['by_unit_kind'] = {k: tot['by_unit_kind'].get(k, 0) for k in list(tot_before['by_unit_kind']) +
                               [k for k in tot['by_unit_kind'] if k not in tot_before['by_unit_kind']]}
    new_raw = (json.dumps(idx) + '\n').encode('utf-8')
    o, n = json.loads(old_raw), json.loads(new_raw)
    ok = {k: v for k, v in o.items() if k not in ('entries', 'totals')} == {k: v for k, v in n.items() if k not in ('entries', 'totals')}
    changed_entries = [x['mt_id'] for x, y in zip(o['entries'], n['entries']) if x != y]
    ok = ok and len(o['entries']) == len(n['entries']) and set(changed_entries) <= set(pick)
    ok = ok and {k: v for k, v in o['totals'].items() if k not in ('microtasks', 'by_unit_kind')} == \
        {k: v for k, v in n['totals'].items() if k not in ('microtasks', 'by_unit_kind')}
    report = {'schema_id': 'hsk.studio_mt_regeneration_report@1', 'mode': 'write' if a.write else 'dry_run',
              'generated_at': now, 'spec_version': a.spec_version, 'spec_commit': a.spec_commit,
              'regenerated': len(pick), 'acceptance_changed': [r['mt'] for r in report_rows if not r['scope_only']],
              'scope_only': [r['mt'] for r in report_rows if r['scope_only']],
              'skipped_live_drift': skipped_drift, 'index_entries_changed': changed_entries,
              'totals_before': {k: tot_before.get(k) for k in ('microtasks', 'by_unit_kind')},
              'totals_after': {k: tot.get(k) for k in ('microtasks', 'by_unit_kind')},
              'index_only_entries_and_totals_changed': ok, 'mts': report_rows}
    print(json.dumps({k: (v if not isinstance(v, list) else len(v)) for k, v in report.items()
                      if k not in ('mts', 'totals_before', 'totals_after')}))
    if not ok:
        sys.exit('index check failed; nothing written')
    if a.write:
        for p, b in files.items():
            open(p + '.tmp', 'wb').write(b)
            os.replace(p + '.tmp', p)
        with open(idx_path + '.tmp', 'wb') as f:
            f.write(new_raw)
        os.replace(idx_path + '.tmp', idx_path)
    out = os.path.join(KERNEL, '.GOV', 'docs_repo', 'wp-studio-module-work', 'research',
                       f'mt-regenerate-{a.spec_version}-{report["mode"].replace("_", "")}-{now.replace(":", "").replace("-", "")}.json')
    with open(out, 'w', encoding='utf-8', newline='\n') as f:
        json.dump(report, f, indent=1, ensure_ascii=False)
        f.write('\n')
    print('report:', out)


if __name__ == '__main__':
    main()
