#!/usr/bin/env python3
"""Add the per-MT `host_integration` block to the WP-KERNEL-STUDIO MT contracts.

Authority: research/integration-direction-paperwork-plan-20261030.json (PW-08) and the
Operator approval of 2026-10-30 ("Approve, dry run first"). Schema hsk.studio_mt_host_integration@1.

Usage (from the kernel root):
  python .GOV/docs_repo/wp-studio-module-work/tools/mt-host-integration.py            # dry run, writes a report
  python .GOV/docs_repo/wp-studio-module-work/tools/mt-host-integration.py --write    # applies, writes a report

The pass is derivation-only: nothing is removed, every existing key keeps its order, each file keeps
its indent, newline style and trailing newline. Files that already carry a block with the same
direction_ref are left alone (idempotent). authority_migration.active_execution_authority gains
"host_integration" where that list exists; files without authority_migration (V2-native contracts)
carry the block as a top-level field, which is active there by construction.
"""
import json, os, re, sys, collections
from datetime import datetime, timezone

HERE = os.path.dirname(os.path.abspath(__file__))
KERNEL = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
PACKET = os.path.join(KERNEL, '.GOV', 'task_packets', 'WP-KERNEL-STUDIO')
STUDIO = os.path.join(KERNEL, '.GOV', 'docs_repo', 'wp-studio-module-work')
DIRECTION_REF = 'STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030'
SPEC_VERSION = 'v02.225'
SCHEMA_ID = 'hsk.studio_mt_host_integration@1'

GPU_CLIENT_MODULES = {'STUDIO-MODULE-RENDER_GPU'}
GPU_PROVIDER_MODULES = {'STUDIO-MODULE-RENDER_CPU', 'STUDIO-MODULE-COMPOSITE', 'STUDIO-MODULE-PIGMENT', 'STUDIO-MODULE-PULSE',
                        'STUDIO-MODULE-SCORE', 'STUDIO-MODULE-SCORE_DEVICE', 'STUDIO-MODULE-SCORE_PLUGIN', 'STUDIO-MODULE-REEL',
                        'STUDIO-MODULE-REEL_NATIVE', 'STUDIO-MODULE-MOTION', 'STUDIO-MODULE-MOTION_JS', 'STUDIO-MODULE-DEVELOP',
                        'STUDIO-MODULE-PRISM'}
JOB_MODULES = {'STUDIO-MODULE-REEL', 'STUDIO-MODULE-REEL_NATIVE', 'STUDIO-MODULE-RENDER_GPU', 'STUDIO-MODULE-RENDER_CPU',
               'STUDIO-MODULE-PULSE', 'STUDIO-MODULE-SCORE', 'STUDIO-MODULE-SCORE_DEVICE', 'STUDIO-MODULE-SCORE_PLUGIN',
               'STUDIO-MODULE-SESSION'}
BINDING_MODULES = {'STUDIO-MODULE-SESSION', 'STUDIO-MODULE-KERNEL_BINDINGS', 'STUDIO-MODULE-CHRONICLE'}
HIGH_MODULES = {'STUDIO-MODULE-RENDER_GPU', 'STUDIO-MODULE-CONTROLS', 'STUDIO-MODULE-HARNESS'}
MEDIUM_MODULES = {'STUDIO-MODULE-OBSERVE', 'STUDIO-MODULE-KERNEL_BINDINGS', 'STUDIO-MODULE-SESSION', 'STUDIO-MODULE-CHRONICLE'}
HOST_HIGH = {'MT-36231', 'MT-36233', 'MT-36206', 'MT-36213', 'MT-36215', 'MT-36228', 'MT-36230'}
HOST_GPU_CLIENT = {'MT-36206', 'MT-36228', 'MT-36231'}


def derive(d, entry):
    ex = d.get('execution') or {}
    module = ex.get('module_ref') or entry.get('active_module_ref') or ''
    kind = entry.get('unit_kind')
    mt = entry['mt_id']
    flags = ((d.get('activation') or {}).get('semantic_trigger_flags')) or {}
    owned_gui = bool(flags.get('owned_gui'))
    inflight = bool(flags.get('queue_or_inflight'))
    collab = bool(flags.get('collaborative_state'))
    anchors = set(d.get('spec_anchor') or [])
    for r in ((d.get('context_capsule') or {}).get('specification_anchor_refs') or []):
        if isinstance(r, dict) and r.get('id'):
            anchors.add(r['id'])
    host = kind == 'host_adoption'
    native_host = host and module.startswith('handshake_native')

    if native_host or module == 'STUDIO-MODULE-CONTROLS' or (owned_gui and module != 'STUDIO-MODULE-HARNESS'):
        gui_host = 'vulkan_shell_client'
    elif module == 'STUDIO-MODULE-HARNESS':
        gui_host = 'asset_local_test_host_only'
    else:
        gui_host = 'none'

    if module in GPU_CLIENT_MODULES or mt in HOST_GPU_CLIENT:
        gpu = 'central_broker_client'
    elif module in GPU_PROVIDER_MODULES:
        gpu = 'via_render_provider_port'
    else:
        gpu = 'none'

    internal = 'none' if module in ('STUDIO-WORKSPACE', 'handshake_core::mex') else 'producer_through_observe'
    fr = 'business_event_ids' if (host or module in BINDING_MODULES or collab or anchors & {'STU-CON-013', 'STU-CON-014'}) else 'none'
    palm = 'ring_visible' if (host or owned_gui or inflight or module in HIGH_MODULES) else 'none'
    descriptors = 'shared_typed_source' if (owned_gui or module in ('STUDIO-MODULE-CONTROLS', 'STUDIO-MODULE-HARNESS') or kind == 'command_table'
                                            or native_host or any(a.startswith('STU-MDL-') or a == 'STU-ARC-014' for a in anchors)) else 'none'
    jobs = 'bounded_admission_via_job_port' if (inflight or module in JOB_MODULES or mt == 'MT-36228') else 'none'
    settings = 'module_tab_feature_subtab' if (owned_gui or native_host) else 'none'

    if module in HIGH_MODULES or mt in HOST_HIGH:
        exposure = 'high'
    elif owned_gui or module in MEDIUM_MODULES or host:
        exposure = 'medium'
    elif gpu == 'via_render_provider_port' or jobs != 'none':
        exposure = 'low'
    else:
        exposure = 'none'

    refs = ['GUI-DESKTOP-049', 'GUI-DESKTOP-050', 'STU-ARC-010', 'STU-VAL-002', 'STU-ARC-015', 'GUI-DESKTOP-041', 'GUI-DESKTOP-045']
    if gui_host == 'vulkan_shell_client':
        refs += ['GUI-DESKTOP-007', 'GUI-DESKTOP-012', 'GUI-DESKTOP-020', 'GUI-DESKTOP-022', 'NATIVE-RENDER-BOOT-001']
    if gpu != 'none':
        refs += ['GUI-DESKTOP-025', 'GUI-DESKTOP-028', 'GUI-DESKTOP-032', 'GPU-QUOTA-010', 'STU-ARC-002', 'STU-ARC-018']
    if descriptors != 'none':
        refs += ['STU-ARC-014', 'STU-MDL-002', 'GUI-DESKTOP-018']
    if jobs != 'none':
        refs += ['STU-ARC-008', 'STU-ARC-012', 'STU-ARC-013', 'GUI-DESKTOP-033', 'GPU-QUOTA-015']
    if settings != 'none':
        refs += ['GUI-DESKTOP-014', 'GUI-DESKTOP-016']
    if fr != 'none':
        refs += ['STU-CON-013', 'STU-CON-014']
    if palm != 'none':
        refs += ['GUI-DESKTOP-051', 'GUI-DESKTOP-054']

    return {
        'schema_id': SCHEMA_ID,
        'direction_ref': DIRECTION_REF,
        'spec_version': SPEC_VERSION,
        'gui_host': gui_host,
        'gpu': gpu,
        'diagnostics': {'internal_diagnostics': internal, 'flight_recorder': fr, 'palmistry': palm},
        'control_descriptors': descriptors,
        'attribution': 'required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing',
        'jobs': jobs,
        'settings': settings,
        'spec_refs': refs,
        'refactor_exposure': exposure,
        'derived_from': {'module_ref': module, 'unit_kind': kind, 'owned_gui': owned_gui, 'queue_or_inflight': inflight, 'collaborative_state': collab},
    }


def detect_style(raw: bytes):
    crlf = b'\r\n' in raw
    m = re.search(rb'\n( +)"', raw)
    indent = len(m.group(1)) if m else None
    single = raw.count(b'\n') <= 1
    return crlf, indent, single, raw.endswith(b'\n'), (b'\\u00' in raw)


def dump(d, style):
    crlf, indent, single, trail, uesc = style
    text = json.dumps(d, indent=(None if single else indent), ensure_ascii=uesc)
    if trail:
        text += '\n'
    if crlf:
        text = text.replace('\n', '\r\n')
    return text.encode('utf-8')


def main():
    write = '--write' in sys.argv
    now = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
    idx_path = os.path.join(PACKET, '_MT_INDEX.json')
    with open(idx_path, encoding='utf-8') as f:
        idx = json.load(f)
    entries = {e['mt_id']: e for e in idx['entries']}
    counts = collections.defaultdict(collections.Counter)
    per_module = collections.defaultdict(collections.Counter)
    changed, skipped, errors, no_am, single_line = [], [], [], 0, 0
    samples = {}
    for mt_id, entry in entries.items():
        path = os.path.join(PACKET, entry['file'])
        try:
            raw = open(path, 'rb').read()
            d = json.loads(raw)
        except Exception as e:  # noqa: BLE001
            errors.append({'mt_id': mt_id, 'error': str(e)[:200]})
            continue
        existing = d.get('host_integration')
        if isinstance(existing, dict) and existing.get('direction_ref') == DIRECTION_REF:
            skipped.append(mt_id)
            continue
        hi = derive(d, entry)
        hi['recorded_at'] = now
        for k in ('gui_host', 'gpu', 'control_descriptors', 'jobs', 'settings', 'refactor_exposure'):
            counts[k][hi[k]] += 1
        for k, v in hi['diagnostics'].items():
            counts['diagnostics.' + k][v] += 1
        per_module[hi['derived_from']['module_ref']][hi['refactor_exposure']] += 1
        style = detect_style(raw)
        if style[2]:
            single_line += 1
        am = d.get('authority_migration')
        if isinstance(am, dict):
            lst = am.get('active_execution_authority')
            if isinstance(lst, list) and 'host_integration' not in lst:
                lst.append('host_integration')
        else:
            no_am += 1
        d['host_integration'] = hi
        if mt_id in ('MT-35546', 'MT-35553', 'MT-35555', 'MT-36231', 'MT-36233', 'MT-30042', 'MT-30639', 'MT-560', 'MT-001'):
            samples[mt_id] = {k: v for k, v in hi.items() if k not in ('spec_refs', 'attribution', 'schema_id')}
        out = dump(d, style)
        if json.loads(out) != d:
            errors.append({'mt_id': mt_id, 'error': 'round-trip mismatch'})
            continue
        changed.append(mt_id)
        if write:
            with open(path, 'wb') as f:
                f.write(out)
    report = {
        'schema': 'hsk.studio_mt_host_integration_report@1',
        'mode': 'write' if write else 'dry_run',
        'generated_at': now,
        'direction_ref': DIRECTION_REF,
        'spec_version': SPEC_VERSION,
        'index_entries': len(entries),
        'would_change' if not write else 'changed': len(changed),
        'skipped_already_recorded': len(skipped),
        'errors': errors,
        'files_without_authority_migration_block_is_top_level': no_am,
        'single_line_files': single_line,
        'counts': {k: dict(v) for k, v in counts.items()},
        'exposure_by_module': {k: dict(v) for k, v in sorted(per_module.items())},
        'samples': samples,
    }
    if write and not errors:
        idx['host_integration_direction'] = {
            'schema_id': 'hsk.studio_mt_host_integration_index@1', 'recorded_at': now, 'direction_ref': DIRECTION_REF,
            'spec_version': SPEC_VERSION, 'plan_ref': 'research/integration-direction-paperwork-plan-20261030.json',
            'counts': report['counts'], 'exposure_by_module': report['exposure_by_module'],
            'note': 'host_integration is active authority per MT; gui_obligation and hbr_int_009_tier_obligations stay historical; the host-adoption MT amendments (PW-09..11) are separate dated entries',
        }
        raw = open(idx_path, 'rb').read()
        with open(idx_path, 'wb') as f:
            f.write(dump(idx, detect_style(raw)))
    tag = 'write' if write else 'dryrun'
    rep_path = os.path.join(STUDIO, 'research', f'mt-host-integration-{tag}-{now.replace(":", "").replace("-", "")}.json')
    with open(rep_path, 'w', encoding='utf-8', newline='\n') as f:
        json.dump(report, f, indent=1, ensure_ascii=False)
        f.write('\n')
    print(json.dumps({k: v for k, v in report.items() if k not in ('samples', 'exposure_by_module')}, ensure_ascii=False)[:1500])
    print('report:', rep_path)
    return 1 if errors else 0


if __name__ == '__main__':
    sys.exit(main())
