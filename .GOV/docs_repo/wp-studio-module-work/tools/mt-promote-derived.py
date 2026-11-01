#!/usr/bin/env python3
"""Promote spec-derived MTs from the green-room staging tree into WP-KERNEL-STUDIO as V2 contracts.

Input per MT: the deriver output (fold_back/spec_mts/MT-<n>.json, schema @1 from
spec-derive-microtasks.py), the cited clause in the named Master Spec version, and an authored
overrides file (research/mt-promote-overrides-<ver>.json) that supplies what the spec does not
determine mechanically: owning module, write scope, related MTs, trigger flags, tier, red-team.

Output per MT: a hsk.microtask_contract@2 file in the instance shape of the newest V2 contracts
(MT-36248..36251: instance_policy, rule_refs, identity, parent, context_capsule, closure_unit,
model_routing, attempt_budget, execution, proof, user_manual_obligation, host_integration,
red_team, lifecycle, submissions, verdicts, blockers, authority_files) plus the spec-derived
fields of the derived corpus (generated_by, derivation, clause, spec_anchor, spec_anchor_status,
domain, scope, acceptance_criteria verbatim from the deriver, hbr_obligations, activation).
host_integration is computed by tools/mt-host-integration.py derive(), the function the bulk write
of 2026-10-30 used. lifecycle.status and validator_verdict are null: the validator owns status.

Index: one entry per new MT is appended to _MT_INDEX.json; nothing else in the index changes. The
index is serialised exactly like HEAD (json.dumps(obj) + '\\n') and written via temp + os.replace.
The write is refused unless the new bytes equal the old bytes with exactly one inserted span that
holds only the new entries.

Usage (from the kernel root):
  python .GOV/docs_repo/wp-studio-module-work/tools/mt-promote-derived.py \
      --overrides .GOV/docs_repo/wp-studio-module-work/research/mt-promote-overrides-v02.227.json \
      --staging .GOV/reference/studio_app_feature_research/_greenroom_20260903/fold_back/spec_mts \
      --spec-commit <sha> [--write]
"""
import argparse, hashlib, json, os, re, sys
from datetime import datetime, timezone
from importlib import util as _u

HERE = os.path.dirname(os.path.abspath(__file__))
KERNEL = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
PACKET = os.path.join(KERNEL, '.GOV', 'task_packets', 'WP-KERNEL-STUDIO')


def _load(name, file):
    s = _u.spec_from_file_location(name, os.path.join(HERE, file))
    m = _u.module_from_spec(s)
    s.loader.exec_module(m)
    return m


mthi = _load('mthi', 'mt-host-integration.py')
bind = _load('bind', 'mt-bind-clauses.py')

RULE_REFS_FROM = 'MT-36251.json'
HBR_REGISTRY = '.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json'
SENT = re.compile(r"(?<=[.;])\s+(?=[A-Z\[`])")


def clause_statements(body, anchor):
    """Split a clause into its own sentences: (statements, declared_gap_sentences).

    The clause anchor and its upper-case title are dropped; everything from 'DECLARED GAP' on is
    a declared gap, not acceptance. Sentences are verbatim spec text, never paraphrase."""
    flat = re.sub(r"\s*\n\s*", " ", body).strip()
    flat = re.sub(r"^\*{0,2}\[" + re.escape(anchor) + r"\]\*{0,2}\s*", "", flat)
    flat = re.sub(r"^[A-Z][A-Z0-9 /,'()-]+\.\s+", "", flat)
    gap = ''
    if 'DECLARED GAP' in flat:
        flat, gap = flat.split('DECLARED GAP', 1)
        gap = 'DECLARED GAP' + gap
    stm = [s.strip() for s in SENT.split(flat.strip()) if s.strip()]
    gaps = [s.strip() for s in SENT.split(gap.strip()) if s.strip()] if gap else []
    return stm, gaps


def build(mt_id, der, ov, common, spec_ver, spec_commit, now, rule_refs, hbr):
    anchor = der['spec_anchor'][0]
    kind = der['derivation']['unit_kind']
    rel = f'.GOV/spec/master-spec-{spec_ver}/spec-modules/14-studio-creative-suite.md'
    md = os.path.join(KERNEL, rel)
    src_sha = hashlib.sha256(open(md, 'rb').read()).hexdigest()
    rng = bind.clause_ranges(md)[anchor]
    lines = open(md, encoding='utf-8').read().split('\n')
    body = '\n'.join(lines[rng['start_line'] - 1:rng['end_line']]).rstrip()
    heading = der['derivation']['spec_heading']
    host = ov['stage'] == 'host_adoption'
    stm, gaps = clause_statements(body, anchor)
    if kind == 'parameter_table':
        mat = [{'id': a['id'], 'text': a['criterion']} for a in der['acceptance_criteria']]
    else:
        mat = [{'id': f'AC-SPEC-{i:03d}', 'text': s} for i, s in enumerate(stm, 1)]
    acc_refs = [m['id'] for m in mat]
    module = ov['module_ref']
    pkg = module.split('::')[0] if host else 'hsk-studio-' + module.split('-')[-1].lower().replace('_', '-')
    forbidden = common['common_forbidden_host'] if host else common['common_forbidden_asset']
    checks = [{
        'check_id': f'{mt_id}-{m["id"]}', 'kind': 'test', 'claim_refs': [m['id'], anchor],
        'command_or_method': (f'After the owning {"host module" if host else "module"} materialises its natural '
                              f'contract target during coding, execute that target in {pkg} for this row; the '
                              f'target, filter and timeout are bound at activation.'),
        'expected': m['text'], 'timeout_seconds': None, 'on_timeout': 'record_timeout',
        'evidence_required': ['Exact candidate commit, toolchain, features and lock',
                              'Nonzero executed case count with independent expected values',
                              'Typed negative path for every refusal the row states'],
    } for m in mat]
    d = {
        'schema_id': 'hsk.microtask_contract@2',
        'schema_version': 'microtask_contract_v2_preservation_asset_first',
        'instance_policy': {'omit_empty_optional_fields': True,
                            'policy_lives_in_rule_refs_not_instance_fields': True, 'schema_ref': None},
        'rule_refs': rule_refs,
        'identity': {'mt_id': mt_id, 'wp_id': 'WP-KERNEL-STUDIO', 'title': ov['title'],
                     'group': 'HOST_ADOPTION' if host else 'STUDIO_ASSET_FEATURE', 'version': 1,
                     'contract_revision': 1},
        'parent': {'wp_contract_revision': 2, 'wp_contract_digest': None},
        'gameplan_ref': '.GOV/task_packets/WP-KERNEL-STUDIO/gameplan.yaml',
        'context_capsule': {
            'specification_anchor_refs': [{
                'id': anchor, 'anchor': f'{rel}:{rng["start_line"]}', 'verbatim': body,
                'heading': heading, 'source_sha256': src_sha, 'start_line': rng['start_line'],
                'end_line': rng['end_line'], 'body_sha256': rng['body_sha256'],
                'hash_encoding': 'UTF8 LF joined body trailing whitespace stripped'}],
            'read_context': ['AGENTS.md', 'CLAUDE.md', '.GOV/codex/Handshake_Codex_v1.4.md', rel,
                             '.GOV/task_packets/WP-KERNEL-STUDIO/STUDIO_MODULES.json',
                             '.GOV/docs_repo/wp-studio-module-work/spec-enrichment/STUDIO-ENRICHMENT-20261030-HOST-INTEGRATION.json']
                            + [f'.GOV/task_packets/WP-KERNEL-STUDIO/{x}.json' for x in ov['host_adoption_mt_refs']],
            'pillar_refs': [],
            'current_hbr_assessment': {'version': hbr['version'], 'registry_sha256': hbr['sha256'],
                                       'assessed_rule_count': 0,
                                       'status': 'PENDING_PER_RULE_ASSESSMENT_AT_ACTIVATION',
                                       'source_capability_ref': f'{mt_id}#/execution', 'host_proof_pending': True},
            'consumed_capability_refs': [{'mt_ref': x} for x in ov['depends_on']],
            'context_digest': None,
        },
        'closure_unit': {
            'outcome': f'{ov["title"]}, as [{anchor}] states it (Master Spec {spec_ver}, {heading}).',
            'requirement_refs': [anchor], 'acceptance_refs': acc_refs, 'materialized_acceptance': mat,
            'declared_spec_gaps': gaps,
            'spec_basis_refs': [f'{rel}:{rng["start_line"]}'], 'operator_decision_refs': [],
        },
        'model_routing': {'model_tier': ov['model_tier'],
                          'tier_reason': 'Sized from the clause: cross-module ownership and declared gaps need original-case reasoning'
                          if ov['model_tier'] == 'strong' else 'Single-owner contract with a bounded acceptance set',
                          'sized_for_declared_tier': True, 'escalate_to_tier_on_blocker': 'strong'},
        'attempt_budget': {'max_attempts_per_failing_check': 2, 'max_fix_rounds_per_mt': 3,
                           'post_diagnosis_runs': {'probe': 1, 'confirm': 1},
                           'diagnosis_location': 'this MT file, blockers[].diagnosis', 'rule_refs': []},
        'execution': {
            'lane': 'implementation', 'module_ref': module, 'build_unit_ref': module,
            'write_scope_source': 'allowed_write_paths', 'depends_on': ov['depends_on'],
            'contract_dependency_kind': 'depends_on_completion',
            'start_precondition': 'PENDING NONEXECUTION preparation: dispatch only after an authorized lane binds '
                                  'the worktree, the natural test target and the immutable pins.',
            'allowed_write_paths': ov['allowed_write_paths'], 'forbidden_write_paths': forbidden,
            'shared_resource_locks': [],
            'inputs': [{'kind': 'verified_canonical_source', 'status': 'INSPECTED_NOT_EXECUTED', 'source_ref': rel,
                        'source_sha256': src_sha, 'start_line': rng['start_line'], 'end_line': rng['end_line'],
                        'clause_sha256': rng['body_sha256'], 'canonical_spec_commit': spec_commit,
                        'clause_body_ref': f'{mt_id}#/context_capsule/specification_anchor_refs/0/verbatim'}],
            'implementation_steps': [
                {'n': 1, 'step': f'Read [{anchor}] and every clause it cites; read the related MTs '
                                 f'{", ".join(ov["host_adoption_mt_refs"] + ov["depends_on"]) or "(none)"}.'},
                {'n': 2, 'step': f'Implement each materialized acceptance row in {pkg} within the allowed write paths.'},
                {'n': 3, 'step': 'Leave every declared spec gap unimplemented and recorded; never choose its values.'},
                {'n': 4, 'step': 'Prove each row through the natural target with an independent oracle and update the canonical UserManual entry in the same change.'},
            ],
            'expected_outputs': [f'{pkg}: the contract of [{anchor}] with one proven case per acceptance row'],
            'stop_conditions': (['A declared spec gap of this clause blocks a row: stop and record the gap, no invented values']
                                if gaps else []) +
                               ['Change needs another module owner path: return to that owner'],
            'may_create_canonical_capability': False,
            'artifact_isolation': {'artifact_root': '../Handshake_Artifacts/handshake-test',
                                   'owner_dir': f'WP-KERNEL-STUDIO/{mt_id}', 'env': {}, 'flags': None,
                                   'reference_runner': None},
            'git_rules': [],
            'module_change_stage': ov['stage'],
        },
        'proof': {'parent_proof_refs': [], 'checks': checks, 'independent_review_required': True,
                  'relevant_input_paths': ov['allowed_write_paths'] + [rel], 'extra_build_proofs': []},
        'user_manual_obligation': {
            'required': True, 'same_change_update_required': True,
            'manual_version_bump_required': 'ASSESS_ACTUAL_DESCRIPTOR_CHANGE',
            'target_entries': [f'{module}: [{anchor}] behaviour, inputs, outputs, failure and recovery'],
            'expected_evidence': ['Canonical UserManual same-change descriptor diff'],
            'location': 'Canonical Handshake UserManual', 'reason': None, 'not_applicable_reason': None},
        'red_team': {'required': True, 'profile': f'[{anchor}] contract fidelity and declared-gap discipline',
                     'risks': [{'risk': r, 'mitigation': m} for r, m in ov['risks']],
                     'minimum_controls': acc_refs},
        'activation': {'disposition': 'NON_AI_CURRENT_SCOPE', 'implementation_state': 'PLANNED_NOT_IMPLEMENTED',
                       'semantic_trigger_flags': ov['flags'], 'current_dispatch_authorized': False},
        'lifecycle': {'phase': 'blocked', 'status': None, 'active': False, 'validator_verdict': None,
                      'contract_preparation_only': True, 'blocked_on': 'AUTHORIZED_LANE_DISPATCH',
                      'updated_at_utc': now},
        'submissions': [], 'verdicts': [], 'blockers': [],
        'wp_id': 'WP-KERNEL-STUDIO', 'mt_id': mt_id,
        'generated_by': der['generated_by'],
        'derivation': dict(der['derivation'], spec_module='14-studio-creative-suite.md',
                           spec_version=spec_ver, deriver_output_ref=f'fold_back/spec_mts/{mt_id}.json'),
        'clause': der['clause'], 'spec_anchor': der['spec_anchor'],
        'spec_anchor_status': der['spec_anchor_status'], 'domain': der['domain'],
        'scope': {'summary': der['scope']['summary'], 'allowed_paths': ov['allowed_write_paths'],
                  'forbidden_paths': forbidden, 'spec_text_is_authority': der['scope']['spec_text_is_authority']},
        'acceptance_criteria': der['acceptance_criteria'],
        'acceptance_criteria_count': der['acceptance_criteria_count'],
        'hbr_obligations': hbr['ids'],
        'authority_files': {'microtask_contract': f'.GOV/task_packets/WP-KERNEL-STUDIO/{mt_id}.json',
                            'packet_contract': '.GOV/task_packets/WP-KERNEL-STUDIO/packet.json',
                            'refinement_contract': '.GOV/task_packets/WP-KERNEL-STUDIO/refinement.json',
                            'codex': '.GOV/codex/Handshake_Codex_v1.4.md'},
        'created_at_utc': now, 'updated_at_utc': now,
    }
    entry = {'mt_id': mt_id, 'file': f'{mt_id}.json', 'clause': d['clause'], 'spec_anchor': d['spec_anchor'],
             'domain': d['domain'], 'unit_kind': kind, 'acceptance_criteria_count': d['acceptance_criteria_count'],
             'status': None, 'active_module_ref': module, 'activation_disposition': 'NON_AI_CURRENT_SCOPE',
             'active_acceptance_count': len(mat), 'module_change_stage': ov['stage'],
             'authoring_contract_revision': 1, 'authoring_proof_status': 'NOT_RUN',
             'current_hbr_assessment_count': 0}
    hi = mthi.derive(d, entry)
    hi['spec_version'] = spec_ver
    hi['recorded_at'] = now
    d['host_integration'] = hi
    return d, entry


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--overrides', required=True)
    ap.add_argument('--staging', required=True)
    ap.add_argument('--spec-commit', required=True)
    ap.add_argument('--write', action='store_true')
    a = ap.parse_args()
    ovs = json.load(open(a.overrides, encoding='utf-8'))
    spec_ver = ovs['spec_version']
    now = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
    rule_refs = json.load(open(os.path.join(PACKET, RULE_REFS_FROM), encoding='utf-8'))['rule_refs']
    reg_raw = open(os.path.join(KERNEL, HBR_REGISTRY), 'rb').read()
    reg = json.loads(reg_raw)
    hbr = {'version': reg['version'], 'sha256': hashlib.sha256(reg_raw).hexdigest(),
           'ids': [r.get('rule_id') or r.get('id') for r in reg['rules']]}
    idx_path = os.path.join(PACKET, '_MT_INDEX.json')
    old_raw = open(idx_path, 'rb').read()
    idx = json.loads(old_raw)
    if (json.dumps(idx) + '\n').encode('utf-8') != old_raw:
        sys.exit('index serialisation differs from json.dumps(obj)+newline; refusing')
    have = {e['mt_id'] for e in idx['entries']}
    files, entries, errors = {}, [], []
    for mt_id, ov in ovs['mts'].items():
        if mt_id in have or os.path.exists(os.path.join(PACKET, f'{mt_id}.json')):
            errors.append(f'{mt_id} already exists; ids are never reused')
            continue
        der = json.load(open(os.path.join(a.staging, f'{mt_id}.json'), encoding='utf-8'))
        d, e = build(mt_id, der, ov, ovs, spec_ver, a.spec_commit, now, rule_refs, hbr)
        files[mt_id] = (json.dumps(d, indent=2, ensure_ascii=False) + '\n').encode('utf-8')
        entries.append(e)
    if errors:
        sys.exit('\n'.join(errors))
    idx['entries'].extend(entries)
    new_raw = (json.dumps(idx) + '\n').encode('utf-8')
    # exactly one inserted span holding only the new entries
    p = 0
    while p < len(old_raw) and old_raw[p] == new_raw[p]:
        p += 1
    s = 0
    while s < len(old_raw) - p and old_raw[-1 - s] == new_raw[-1 - s]:
        s += 1
    ins = new_raw[p:len(new_raw) - s]
    expect = (', ' + ', '.join(json.dumps(e) for e in entries)).encode('utf-8')
    ok = (p + s == len(old_raw)) and ins == expect
    check = json.loads(new_raw)
    ok = ok and check['entries'][:len(check['entries']) - len(entries)] == json.loads(old_raw)['entries'] \
        and {k: v for k, v in check.items() if k != 'entries'} == {k: v for k, v in json.loads(old_raw).items() if k != 'entries'}
    report = {'mode': 'write' if a.write else 'dry_run', 'generated_at': now, 'spec_version': spec_ver,
              'spec_commit': a.spec_commit, 'mts': [e['mt_id'] for e in entries],
              'index_entries_before': len(have), 'index_entries_after': len(check['entries']),
              'index_bytes_before': len(old_raw), 'index_bytes_after': len(new_raw),
              'index_only_new_entries_inserted': ok,
              'entries': entries,
              'host_integration': {e['mt_id']: {k: v for k, v in json.loads(files[e['mt_id']])['host_integration'].items()
                                                if k in ('gui_host', 'gpu', 'jobs', 'settings', 'control_descriptors', 'refactor_exposure')}
                                   for e in entries}}
    print(json.dumps({k: v for k, v in report.items() if k not in ('entries', 'host_integration')}))
    if not ok:
        sys.exit('index diff check failed; nothing written')
    if a.write:
        for mt_id, b in files.items():
            tmp = os.path.join(PACKET, f'{mt_id}.json.tmp')
            open(tmp, 'wb').write(b)
            os.replace(tmp, os.path.join(PACKET, f'{mt_id}.json'))
        tmp = idx_path + '.tmp'
        with open(tmp, 'wb') as f:
            f.write(new_raw)
        os.replace(tmp, idx_path)
    out = os.path.join(KERNEL, '.GOV', 'docs_repo', 'wp-studio-module-work', 'research',
                       f'mt-promote-{report["mode"].replace("_", "")}-{now.replace(":", "").replace("-", "")}.json')
    with open(out, 'w', encoding='utf-8', newline='\n') as f:
        json.dump(report, f, indent=1, ensure_ascii=False)
        f.write('\n')
    print('report:', out)


if __name__ == '__main__':
    main()
