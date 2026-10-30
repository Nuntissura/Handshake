#!/usr/bin/env python3
"""Dated host-integration amendments for the WP-KERNEL-STUDIO host-adoption MTs (MT-36189..36233)
and the five foundation MTs that own the host boundary (MT-35546 render-gpu, MT-35553 controls,
MT-35555 harness, MT-35530 observe, MT-35556 kernel-bindings).

Authority: Operator 2026-10-30 "Yes, amend them" (plan PW-09 and PW-11 in
research/integration-direction-paperwork-plan-20261030.json); HBR-HOST-001..008.

Preservation-first: nothing existing is rewritten. Each MT gains closure_unit.amendments[] with one
entry, new AC rows appended to closure_unit.materialized_acceptance and acceptance_refs, one proof
check appended to proof.checks, and updated_at_utc. Superseded wording is recorded in the amendment,
the original text stays. Idempotent on amendment_id. --write applies; default is a dry run.
"""
import json, os, re, sys, subprocess
from datetime import datetime, timezone

HERE = os.path.dirname(os.path.abspath(__file__))
KERNEL = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
PACKET = os.path.join(KERNEL, '.GOV', 'task_packets', 'WP-KERNEL-STUDIO')
STUDIO = os.path.join(KERNEL, '.GOV', 'docs_repo', 'wp-studio-module-work')
DIRECTION_REF = 'STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030'
AUTHORITY = "Operator 2026-10-30 'Yes, amend them' after 'Approve, dry run first' / 'Write it'; HANDSHAKE_BUILD_RULES 1.18.0 HBR-HOST-001..008; Master Spec v02.225"
HOST_DEPS = ("Vulkan backend (WP-KERNEL-012 MT-198/199/212), GPU owner (MT-203), real-time-safe internal_diagnostics handoff, "
             "FR-EVT-STUDIO-* enum registration, Palmistry->Flight Recorder ingestion (WP-KERNEL-016)")

AC_CONTRACT = ("The adopted revision creates no window, viewport, event loop, GPU instance, device, queue, allocator, budget, recorder, "
               "ring or watcher of its own, and no egui, eframe, winit, wgpu or ash type appears in its public contract (HBR-HOST-001, HBR-HOST-003).")
AC_DIAG = ("Every terminal outcome of the module's operations reaches the host internal_diagnostics ring through the single observe adapter as an "
           "allowlisted code with real correlation and session identity, content-free; business outcomes stay on the Flight Recorder (HBR-HOST-005, HBR-HOST-007).")
AC_GPU = ("Every GPU operation of the module is admitted and settled through the central GPU owner grant with module, client and device attribution; "
          "an unavailable or lost device is a typed outcome, never a silent CPU fallback and never a PASS (HBR-HOST-003).")
AC_GUI = ("Every operator-visible surface of the module mounts in the kernel-owned Vulkan shell pane on the real Vulkan path, with shared dividers, "
          "keyboard, focus, IME, AccessKit identity, DPI and multiwindow preserved and truthful loading, empty, denied and error states (HBR-HOST-001, HBR-HOST-008).")
AC_JOBS = ("Every queue, worker or export job of the module is admitted through the host resource owner with typed overload, cancellation, "
           "stale-revision and release outcomes; real-time callbacks use preallocated handoff and never call the recorder (HBR-HOST-004).")
AC_SURVIVAL = ("Acceptance includes the out-of-process watcher surviving a controlled owned freeze and a controlled owned crash of the real application "
               "with this surface mounted, producing survivor records with genuine process and session identity (HBR-HOST-006).")

SPECIFIC = {
    'MT-36190': "observe exposes a ring-shaped producer port (codes, counters, timing, opaque ids; never text) and maps every Outcome and DeliveryError onto the host DiagEventCode allowlist through one adapter per host; observe never becomes a recorder or watcher authority (STU-ARC-010, STU-VAL-002).",
    'MT-36205': "CPU rendering is admitted through the memory owner with bounded allocations and is never a silent substitute for a refused or lost GPU grant.",
    'MT-36206': "render-gpu's own GpuContext::acquire instance/device and its BackendChoice leave the product path; the module is a typed client of the central GPU owner (grant handle, admission, residency budget issued by the owner, module/client/device attribution, settle-once, device-loss rebind through the owner); Vulkan is the only product backend. Wording superseded: 'wgpu device' and 'device epoch' read as 'central GPU owner grant'.",
    'MT-36213': "controls' Registry, MenuBar and ControlSpec are the single descriptor source for every Studio control and action; the egui adapter lives behind the host adapter module and no toolkit type crosses the public contract; Kumihimo ControlRecord adoption follows Operator decision D1; the same descriptors feed the GUI, state_json, AccessKit DeclaredIdentity, the UserManual and Argus.",
    'MT-36214': "session's JobPort and Admission are the one admission path for every Studio queue; overload, cancellation, stale-revision and release outcomes are typed; CommitRequest expected/successor revision binds to the EventLedger revision.",
    'MT-36215': "the harness binary is an asset-local test host only: its eframe::run_native window is never a product surface and proof taken there is evidence, not host acceptance; harness and harness-view public APIs are toolkit-free.",
    'MT-36216': "the kernel-bindings Envelope carries client_session_id, process birth and real correlation; agent_label and client_session_id reconcile through this one adapter; the 22 FR-EVT-STUDIO-* business events are emitted here, never by a leaf crate.",
    'MT-36224': "dependency pins stay at wgpu 27.0.1 and egui 0.33.3 until Kumihimo's pinned revision is verified (Operator decision D2); the vulkan feature is required and dx12 is not a product path; a wgpu major move is a separately scoped upgrade (STU-ARC-018).",
    'MT-36228': "the host resource owner fronts central GPU owner admission as well: GPU and memory quotas carry module, client and device attribution; claims bind policy, operation, module/client and device; unknown fails closed as HostQuotaUnavailable; cancellation never refunds; settle exactly once; shutdown drains (GPU-QUOTA-010..015).",
    'MT-36230': "the 22 FR-EVT-STUDIO-* identities are registered as closed-enum variants plus the manifest (no runtime registration exists); survivor-record ingestion after recovery is a named dependency on WP-KERNEL-016, recorded as such and never as a no-op proof.",
    'MT-36231': "the Studio pane mounts in the kernel-owned Vulkan shell through PaneRegistry/PaneFactory/EditorSurface on the real Vulkan path (the current DX12-only build is not acceptance); docked panes share edges and dividers; keyboard, focus, IME, AccessKit, DPI, multiwindow and recovery are preserved; Kumihimo shared controls and Handpick search per Operator decision D1; a Studio Settings tab with feature subtabs and Engine Profiles inside Settings.",
    'MT-36232': "Argus receipts identify the actual document revision, renderer and version, frame/region, color profile and resource grant; descriptors come from the single controls source; manual entries carry the three-tier diagnostic posture (STU-MAN-118).",
    'MT-36233': "observe routes onto the host producer through one adapter with a module identity, correlation and PID plus process birth on every event (DiagEvent gains those fields on the host side; the locking record() path needs a preallocated real-time handoff: both recorded as Handshake dependencies, not Studio work); Palmistry freeze (stale heartbeat 5 s plus hung-window probe) and crash survivor records are proved on the real application with the Studio pane mounted; the watcher holds no restart or recovery authority; Flight Recorder ingestion after recovery is the WP-KERNEL-016 dependency.",
    'MT-35546': "render-gpu exposes a typed GPU client port (grant handle, admission request, budget and retained-memory accounting, settle-once release, device-loss rebind) with a transient in-process adapter for asset-local proof only; GpuContext::acquire and BackendChoice leave the public API (test-only), Vulkan is tried first on Windows for asset-local runs, and no wgpu type crosses the public contract. Wording superseded: 'wgpu backend/device lifecycle' reads as 'typed client of the central GPU owner'.",
    'MT-35553': "controls is descriptor-first: Registry, MenuBar and ControlSpec are the single source for every control and action; the egui mechanic is an adapter module behind a toolkit-free public API; descriptors feed the GUI, state_json, AccessKit, the UserManual and Argus. Wording superseded: 'one shared egui control/panel mechanic' reads as 'one shared descriptor source with a host-adapted renderer'.",
    'MT-35555': "the harness is the asset-local test host: proof taken there is evidence and never host acceptance; its window path is excluded from product; harness-view and harness public APIs carry no egui, eframe, winit or wgpu type.",
    'MT-35530': "observe adds a ring-shaped producer port (closed codes, counters, timing, opaque ids; no text) and the Outcome/DeliveryError to DiagEventCode mapping; it stays a bounded adapter and never a recorder, ring or watcher authority.",
    'MT-35556': "kernel-bindings' Envelope and Reply carry client_session_id, process birth and real correlation; the 22 FR-EVT-STUDIO-* business events are emitted by this binding with actor, client session, correlation and causation; actor reconciliation happens here through one adapter.",
}
AUDIO = {'MT-36200', 'MT-36201', 'MT-36219', 'MT-36220'}
INTEROP = {'MT-36208', 'MT-36209', 'MT-36210', 'MT-36211', 'MT-36212', 'MT-36222'}
REEL = {'MT-36202', 'MT-36221'}
FOUNDATIONS = {'MT-35546', 'MT-35553', 'MT-35555', 'MT-35530', 'MT-35556'}
TARGETS = [f'MT-{n}' for n in range(36189, 36234)] + sorted(FOUNDATIONS)


def detect_style(raw):
    m = re.search(rb'\n( +)"', raw)
    return (b'\r\n' in raw, len(m.group(1)) if m else None, raw.count(b'\n') <= 1, raw.endswith(b'\n'), b'\\u00' in raw)


def dump(d, style):
    crlf, indent, single, trail, uesc = style
    text = json.dumps(d, indent=(None if single else indent), ensure_ascii=uesc) + ('\n' if trail else '')
    return (text.replace('\n', '\r\n') if crlf else text).encode('utf-8')


def amend(mt, d, now):
    hi = d.get('host_integration') or {}
    rows = [AC_CONTRACT, AC_DIAG]
    if hi.get('gpu') != 'none':
        rows.append(AC_GPU)
    if hi.get('gui_host') == 'vulkan_shell_client':
        rows.append(AC_GUI)
    if hi.get('jobs') != 'none' or mt in AUDIO or mt in REEL or mt in INTEROP:
        rows.append(AC_JOBS)
    if mt in ('MT-36231', 'MT-36233'):
        rows.append(AC_SURVIVAL)
    specific = SPECIFIC.get(mt)
    if mt in AUDIO:
        specific = (specific + ' ' if specific else '') + "Real-time audio callbacks use preallocated handoff to off-thread aggregation and never call observe or the host recorder (STU-ARC-013, HSK-PLG-011)."
    if mt in INTEROP:
        specific = (specific + ' ' if specific else '') + "Import and export jobs are admitted through the session JobPort; typed losses are reported content-free."
    if mt in REEL:
        specific = (specific + ' ' if specific else '') + "The export queue is admitted through the session JobPort and encode workers heartbeat through the host ring."
    if specific:
        rows.append(specific)
    cu = d['closure_unit']
    cu.setdefault('amendments', [])
    aid = f'{mt}/HOST-AMEND-001'
    if any(a.get('amendment_id') == aid for a in cu['amendments']):
        return None
    prefix = f'{mt}/AC-HOST-INT-' if not mt in FOUNDATIONS else 'AC-HOST-INT-'
    existing = {r.get('id') for r in cu.get('materialized_acceptance', [])}
    new_rows = []
    for i, text in enumerate(rows, 1):
        rid = f'{prefix}{i:03d}'
        assert rid not in existing, (mt, rid)
        new_rows.append({'id': rid, 'text': text, 'amendment_ref': aid})
    cu.setdefault('materialized_acceptance', []).extend(new_rows)
    cu.setdefault('acceptance_refs', []).extend(r['id'] for r in new_rows)
    cu['amendments'].append({
        'amendment_id': aid, 'recorded_at': now, 'authority': AUTHORITY, 'direction_ref': DIRECTION_REF,
        'hbr_refs': ['HBR-HOST-001', 'HBR-HOST-002', 'HBR-HOST-003', 'HBR-HOST-004', 'HBR-HOST-005', 'HBR-HOST-006', 'HBR-HOST-007', 'HBR-HOST-008'],
        'outcome_addendum': "Adoption binds this unit to the Handshake host under the HOST pillar: the kernel-owned native Vulkan shell is the only GUI host, the central Handshake GPU and memory owner is the only device and budget owner, internal_diagnostics, the Flight Recorder and the out-of-process watcher are wired from the first slice, every command carries attribution, and no module-private window, device, recorder or watcher survives adoption. " + (specific or ''),
        'wording_superseded': ([{'from': 'wgpu device / device epoch', 'to': 'central GPU owner grant'}] if mt in ('MT-36206', 'MT-35546') else []) + ([{'from': 'one shared egui control/panel mechanic', 'to': 'one shared descriptor source with a host-adapted renderer'}] if mt == 'MT-35553' else []),
        'originals_preserved': True, 'added_acceptance_refs': [r['id'] for r in new_rows], 'added_proof_check': f'{mt}/HOST-INT',
        'open_host_dependencies': HOST_DEPS,
    })
    d['proof'].setdefault('checks', []).append({
        'check_id': f'{mt}/HOST-INT', 'kind': 'independent_review', 'claim_refs': [r['id'] for r in new_rows],
        'command_or_method': "Validator reads the pinned revision's public contract and the host adapter wiring against HBR-HOST-001..008 and this MT's host_integration block; where a runtime exists, runs the named proof (real Vulkan path, owner grant admission and settlement, ring codes with identity, survivor records).",
        'expected': f"Each AC-HOST-INT row is PROVED, NOT_APPLICABLE with a reason, or BLOCKED naming the Handshake dependency ({HOST_DEPS}); a DX12, GL or software path never proves a Vulkan row; an asset-local host never proves a host row.",
        'timeout_seconds': 300, 'on_timeout': 'record_timeout',
        'evidence_required': ['public contract grep of the pinned revision', 'host adapter wiring reference', 'runtime log, grant receipt or survivor record where applicable'],
    })
    d['updated_at_utc'] = now
    if isinstance(d.get('acceptance_criteria_count'), int):
        d['acceptance_criteria_count'] += len(new_rows)
    return len(new_rows)


def main():
    write = '--write' in sys.argv
    now = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
    idx_path = os.path.join(PACKET, '_MT_INDEX.json')
    idx_raw = open(idx_path, 'rb').read()
    idx = json.loads(idx_raw)
    entries = {e['mt_id']: e for e in idx['entries']}
    report = {'schema': 'hsk.studio_mt_host_amendments_report@1', 'mode': 'write' if write else 'dry_run', 'generated_at': now, 'amended': {}, 'skipped': [], 'errors': []}
    total_rows = 0
    for mt in TARGETS:
        path = os.path.join(PACKET, f'{mt}.json')
        raw = open(path, 'rb').read()
        d = json.loads(raw)
        n = amend(mt, d, now)
        if n is None:
            report['skipped'].append(mt)
            continue
        out = dump(d, detect_style(raw))
        if json.loads(out) != d:
            report['errors'].append({'mt_id': mt, 'error': 'round-trip mismatch'})
            continue
        report['amended'][mt] = n
        total_rows += n
        e = entries[mt]
        if isinstance(e.get('active_acceptance_count'), int):
            e['active_acceptance_count'] += n
        if isinstance(e.get('acceptance_criteria_count'), int) and mt in FOUNDATIONS:
            e['acceptance_criteria_count'] += n
        if write:
            open(path, 'wb').write(out)
    if write and not report['errors']:
        idx.setdefault('totals', {})['host_integration_amendment_rows'] = idx['totals'].get('host_integration_amendment_rows', 0) + total_rows
        hid = idx.setdefault('host_integration_direction', {})
        hid['amendments'] = {'recorded_at': now, 'tool': '.GOV/docs_repo/wp-studio-module-work/tools/mt-host-amendments.py', 'mts': list(report['amended']), 'rows_added': total_rows,
                             'note': 'closure_unit.amendments[] per MT; AC rows and one proof check appended; originals untouched; superseded wording recorded, not rewritten'}
        open(idx_path, 'wb').write(dump(idx, detect_style(idx_raw)))
    report['rows_added'] = total_rows
    tag = 'write' if write else 'dryrun'
    rep = os.path.join(STUDIO, 'research', f'mt-host-amendments-{tag}-{now.replace(":", "").replace("-", "")}.json')
    with open(rep, 'w', encoding='utf-8', newline='\n') as f:
        json.dump(report, f, indent=1, ensure_ascii=False)
        f.write('\n')
    print(json.dumps({k: (v if k != 'amended' else len(v)) for k, v in report.items()}, ensure_ascii=False))
    print('report:', rep)
    return 1 if report['errors'] else 0


if __name__ == '__main__':
    sys.exit(main())
