"""Bounded one-time approved copy authoring; not an execution/validation gate."""
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[3]
BASE = ROOT / '.GOV/spec/master-spec-v02.207'
REPORT = ROOT / '.GOV/docs_repo/studio-collaboration-2026-10-09/spec-mt-team/master-spec-amendment-map.json'
data = json.loads(REPORT.read_text(encoding='utf-8'))
studio = BASE / 'spec-modules/14-studio-creative-suite.md'
text = studio.read_text(encoding='utf-8')
changes = []
for row in data['amendments']:
    new = row['proposed_text'].replace('\ufffd', '-')
    if row['operation'] == 'replace_clause_preserve_id':
        old = row['target']['exact_old_text']
        assert text.count(old) == 1, row['amendment_id']
        if row['amendment_id'] == 'SPEC207-STU-OVR-003':
            new = new.replace('not in `studio-engine`', 'not in any Studio asset crate')
            new += '\n\nAny proposal for an additional database, including a projection engine, MUST first explain the concrete necessity, preferred database and alternatives to the Operator and obtain explicit approval. Historical comparative research is not authorization.'
        if row['amendment_id'] == 'SPEC207-STU-IO-001':
            new = new.replace('Container wire layout/extension/schema are activation-blocking contract choices until specified and independently reviewed.', 'The portable extension is `.handshake`; its bounded streaming ZIP64 transport carries a versioned typed manifest, Studio graph, content-hash asset references, optional granted packed payloads, previews and preservation records. Exact field/schema and migration contracts MUST be executable before container feature activation.')
        text = text.replace(old, new, 1)
    else:
        changes.append(new)
for row in data['unanchored_patches']:
    assert text.count(row['exact_old_text']) == 1
    text = text.replace(row['exact_old_text'], row['proposed_text'], 1)
# Remaining legacy names denote the same preserved behavior, now at canonical leaf owners.
text = text.replace('`studio-engine`\'s vector engine', 'the Nib vector owner')
text = text.replace('`studio-engine/src/lib.rs`', 'the Accord typed-port contract owner')
text = text.replace('`studio-engine`', 'the owning modular Studio asset crate')
text = text.replace('`handshake_core::studio::vector`', 'the integrated Studio vector binding')
text = text.replace('No single studio-engine crate owns all compute.', 'A single engine crate owning all compute is prohibited.')
text = text.replace('legal only when `layout_mode = HORIZONTAL`', 'legal only when `layout_mode = HORIZONTAL` or `VERTICAL`; Figma UI supports both flows, while the inspected plugin property page still documents horizontal-only, so importer API-version capabilities MUST be explicit')
text = text.replace('there is no XMP sidecar file, no private raw-develop database, and no SQLite cache;', 'there is no XMP sidecar authority, private raw-develop database or SQLite cache; bounded explicit XMP interchange read/write through CKC is permitted as external transport, never live settings authority;')
additional = [
('[STU-ARC-010]', 'MODULE CATALOG. The canonical asset module roots are `handshake-studio-assets/crates/hsk-studio-{accord,observe,folio,chronicle,prism,pigment,nib,type,layout,develop,components,pulse,score,reel,motion,composite,render-cpu,render-gpu,package,interop-psd,interop-vector,interop-fig,interop-layout,interop-media,controls,session,harness,kernel-bindings,press,web,score-device,score-plugin,reel-native,interop-lrcat}`. The workspace owner coordinates manifests, lockfile, toolchain, closure and mirror provenance; it is not Accord. Every root MUST have its declared semantic owner and public contracts. The asset kernel-bindings root owns host-neutral bridge contracts/consumers; actual AppState/native/SurrealDB consumers live later in the Handshake integration target outside this isolated workspace. No optional path dependency to missing Handshake source may make the mirrored workspace unresolved.'),
('[STU-ARC-011]', 'MUTATION OWNERSHIP. A document-scoped actor MUST serialize accepted commits while compute consumes immutable snapshots and proposes patches. Target/object/property read/write footprints and expected revision vectors MUST admit disjoint changes according to current conflict semantics, including [STU-DS-163]; a blanket whole-document CAS MUST NOT replace them. Commit validation rechecks current grants and target preconditions and writes accepted patch, authority heads, EventLedger and idempotency outcome in one SurrealDB transaction. A lost acknowledgment/restart MUST rediscover the committed receipt rather than apply twice. CRDT draft merge and durable transaction commit are separate boundaries.'),
('[STU-ARC-012]', 'BYTE AND PRODUCER BACKPRESSURE. Admission MUST bound pending callers before large allocation/decode, slots, compressed/inflated bytes, pixels/samples/frames, retained results/snapshot generations and device/process allocations. Byte ownership leases span worker results and snapshot retirement; every cancellation/error/timeout frees or explicitly transfers reservations. Partial multi-resource acquisition MUST release before waiting. Fair per-actor/document service and reserved bounded cancel/completion paths MUST prevent one producer or full data queue from starving stop/commit outcomes.'),
('[STU-ARC-013]', 'OVERLOAD AND REALTIME. Only replaceable uncommitted previews MAY coalesce, with typed superseded outcome and visible stale revision; committed edits, exports and recording MUST NOT silently drop. Stop/deadline/cancel epochs MUST be observed between bounded compute chunks. Audio callbacks MUST use preallocated bounded handoff and MUST NOT allocate, block on locks, decode, save plugin state, retire foreign graphs or call the locking general recorder. Underrun has a defined safe output; overrun has discontinuity/lost-frame outcome. Single-producer recorder invariants MUST remain safe through off-thread aggregation, never by deleting a mutex around unsafe multi-producer access.'),
('[STU-ARC-014]', 'CANONICAL SHARED ASSETS. Controls MUST implement one reusable panel/control mechanic with typed capability configuration for text fields, sliders, swatches, menus, windows, gizmos and timeline values. Reuse or extend existing capability ownership before adding a near-duplicate asset. The same descriptors MUST feed structured tools, AccessKit, UserManual and Argus; agent-local view/selection/navigation MUST not change operator focus or another agent view. Exact actor attribution MUST reconcile authority `agent_label` with the current native `client_session_id` wire field through one adapter.'),
('[STU-ARC-015]', 'VISUAL RECEIPTS. Every visual mutation/check MUST bind Argus observation to exact committed/proposed revision, command/actor, frame/region, renderer/version, color/profile, original/proxy resolution and current resource grant. A screenshot of a previous revision, original cached composite, unlabelled proxy or unsupported fallback MUST NOT prove the edited result. Every render/import error and unavailable device/provider MUST return typed loss/failure; optional early-return test paths are NOT_PROVEN, never PASS.'),
('[STU-ARC-016]', 'FOUNDATION PROOF AND BUILD. All 34 module foundations and workspace coordination MUST precede any feature MT. A foundation implements a narrow real bounded capability with a real consumer and failure path; a missing proprietary decoder MAY expose actual container/opaque/loss validation, while the editable decoder feature remains explicitly blocked. Hollow registries, canned responses, no-op sinks and empty tests do not satisfy foundations. Locked selected-package dependency/feature closure and actual compiler evidence MUST distinguish pure leaves, optional native providers and later embedding. Measured rebuild fanout at a stable source path, external owner-specific artifact root and exact candidate identity are required before claiming compile improvement. ARM-host and x64-product proof remain separate; neither silently substitutes for the other.'),
('[STU-ARC-017]', 'MT PRESERVATION. Architecture migration MUST preserve every prior MT ID, original scope, acceptance payload, rationale and proof obligation with baseline identity and explicit fulfillment mapping to asset-local work plus separately retained kernel embedding. Compound scopes MUST name contributing module owners. Every current HBR rule MUST carry per-MT applicability trigger/reason and proof destination; shared proof is reusable only with unchanged input identity. Foundation completion does not mark old vendor-parity or kernel acceptance complete.'),
('[STU-IO-187]', 'CONTAINER SAFETY. Native snapshots MUST stream ZIP64 through granted ArtifactService provider ports with bounded index/entry/count/compressed/inflated/aggregate limits, duplicate-name/key rejection, traversal/cycle/overflow checks, canonical known-field serialization and explicit unknown-version behavior. Safely retained unknown records preserve verbatim bytes and provenance in quarantine and MUST NOT execute. Known semantic roundtrip and verbatim opaque preservation are distinct checks. Save MUST validate a complete temporary artifact and atomically replace while retaining last-good on failure; deleting the destination before rename is forbidden. Artifact completion is not authority publication: EventLedger/CKC promotion has its own transaction and idempotent orphan/crash reconciliation.'),
('[STU-IO-188]', 'PRESERVATION AND SANITIZED DELIVERY. Preservation snapshots and sanitized deliverables MUST use distinct explicit export policies. Sanitized output MUST exclude disallowed originals, opaque records, thumbnails, history, attachments and metadata from the full reachable payload set; preserving an input file MUST NOT reintroduce removed content. Imports/render/read/retry/export/delivery MUST resolve fresh recipient grants, including linked files and explicitly selected WAL bytes; serialized permission tokens are provenance only. Independent saved/decompressed-byte inspection, fault/revocation and twice-roundtrip edited fixtures are required.'),
('[STU-IO-189]', 'NATIVE AUDIO AND CATALOG INTEROP. `.sesx` editable session transport and Lightroom catalog/develop/WAL provenance MUST have explicit versioned per-entity mapping/loss outcomes alongside INDD/PRPROJ/AEP/FIG requirements [STU-IO-186]. Unsupported process-version/Enable flags, smart rules, archive entities or plugin/effect graphs MUST be retained/translated/approximate/unsupported/missing explicitly; successful parser/count is not full conversion. The .lrcat decoder public API is fixed-format semantic extraction from bounded immutable granted bytes, never a generic table/query/database API.'),
('[STU-PDF-001]', 'PDF REDACTION. Press MUST expose separate reversible marking and irreversible apply-redaction proposal semantics, selective sanitize and full-save disposition, including signed/read-only states and explicit removed-content scope. Accepted destructive application MUST go through normal proposal/promotion and provenance. Full reachable-object rewrite and exclusion of prior revisions/source preservation payloads MUST be independently checked against text, graphics, metadata, attachments and decompressible bytes; exporter self-verification is insufficient.'),
('[STU-PDF-002]', 'PREPRESS AND ACCESSIBILITY. Shared print/export plans MUST preserve selected output profile hash/intent, spot/process ink and overprint/separation behavior, and semantic reading order independent of visual layout. Artifact-produced status, PDF/A object-subset check or approximate preview MUST NOT prove PDF/X, PDF/UA, plate or tagged-order conformity. Independent conformance/separation/structure checks and original-authored edited fixtures are required.'),
('[STU-ARC-018]', 'SELECTIVE REUSE. Adopt upstream Rust leaf algorithms only with pinned source hash, patch/license provenance and explicit dependency/loss/failure review. Upstream app/session/history/catalog/CAS/automation servers and all-engine facades MUST NOT become parallel Handshake owners. Toolkit/GPU/native handles stay behind providers; current host GUI family stays at its inspected approved pins unless a separate scoped upgrade is approved. Budget, cancellation, canonical JSON, streaming storage and diagnostic ports require real consumers and may need extending their existing owners; declarations do not prove wiring.'),
]
for anchor, body in additional:
    assert anchor not in text, anchor
    changes.append(anchor + ' ' + body)
block = '\n\n### 14.2.1 Approved modular asset and pressure contracts [ADD v02.207]\n\n' + '\n\n'.join(changes) + '\n\n'
pos = text.index('## 14.3 Unified Document Model and Studio Primitive Set')
text = text[:pos] + block + text[pos:]
studio.write_text(text, encoding='utf-8', newline='\n')
for row in data['crosscut_current_law_amendments']:
    path = BASE / pathlib.Path(row['path']).relative_to('.GOV/spec/master-spec-v02.206')
    s = path.read_text(encoding='utf-8')
    if row.get('operation') == 'preserve_storage_law_add_qualification':
        marker = '#### 2.3.13.0 SurrealDB-exclusive authority supersession [ADD v02.204]'
        assert s.count(marker) == 1
        s = s.replace(marker, marker + '\n\n[ARCH-LRCAT-001] ' + row['proposed_text'], 1)
    else:
        assert s.count(row['exact_old_text']) == 1, row['anchor']
        new = row['proposed_text']
        if row['anchor'] == 'OSS-SDB-001':
        new = new.replace('DuckDB may exist only as an explicitly rebuildable, bounded analytics/diagnostic projection.', 'Any additional database, including DuckDB projections, requires prior explicit Operator approval explaining necessity, preferred database and alternatives; historical comparative inventory is not authorization.')
        s = s.replace(row['exact_old_text'], new, 1)
    path.write_text(s, encoding='utf-8', newline='\n')
cx = data['codex_required_amendment']
path = ROOT / cx['path']
s = path.read_text(encoding='utf-8')
assert s.count(cx['exact_old_text']) == 1
s = s.replace(cx['exact_old_text'], cx['proposed_text'] + ' Any proposed additional database requires prior explicit Operator approval with concrete necessity, preferred database and alternatives.', 1)
path.write_text(s, encoding='utf-8', newline='\n')
print('Authored approved candidate clauses; old bundle and SPEC_CURRENT untouched.')
