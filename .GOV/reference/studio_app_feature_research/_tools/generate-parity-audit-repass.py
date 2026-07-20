#!/usr/bin/env python3
"""Generate docs 63 (A5 targeted re-pass rows) and 64 (A6 workflow research rows)
from the 2026-07-20 parity-audit follow-up research.

Inputs (committed provenance, repo-relative):
  _audit_20260720/inputs/a5_repass.json     -> targeted parity re-pass rows (5 lanes)
  _audit_20260720/inputs/a6_workflow.json   -> workflow research capabilities (5 lanes)

Outputs:
  63-parity-repass-delta.md
  64-workflow-needs-research-delta.md

Authority: reference/provenance only (index.yaml authority_note, [STU-SECTION-002]).
Disk-agnostic; regenerate after editing inputs.
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
INPUTS = os.path.join(ROOT, "_audit_20260720", "inputs")
UPDATED = "2026-07-20"


def load(name):
    with open(os.path.join(INPUTS, name), "r", encoding="utf-8") as f:
        return json.load(f)


def fence(obj):
    return "```json\n" + json.dumps(obj, indent=2, ensure_ascii=False) + "\n```"


def write(path, text):
    with open(os.path.join(ROOT, path), "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    print("wrote", path, len(text), "bytes")


LANE_TITLE_A5 = {
    "figma-sites-a11y": "Figma Sites Accessibility + SEO + Website Settings (closes the sole CRITICAL gap)",
    "camera-raw": "Adobe Camera Raw 2024-2026 NON-AI develop surface",
    "indesign-fonts": "InDesign variable fonts + OpenType + font management",
    "affinity-option-depth": "Affinity V2 dialog option depth (brush / soft proof / performance / PDF import)",
    "ps-formats-hdr": "Photoshop native AVIF/JXL + HDR/gain-map export",
}
LANE_TITLE_A6 = {
    "review-approval": "Review / approval / sign-off / version-of-record (NEED-004/005/017/022)",
    "workflow-state-pm": "Production workflow-state / project management (NEED-009)",
    "perf-nfr": "Production-volume performance NFRs (NEED-026/047/071/095)",
    "dam-pim-libraries": "DAM/PIM + governed library releases (NEED-045/057/101)",
    "culling-catalog": "Integrated culling / rating / catalog stage (NEED-051/063, sole NOT_COVERED)",
}


def gen_repass():
    data = load("a5_repass.json")
    total = sum(len(l.get("rows", []) or []) for l in data)
    # assign ids per lane
    for lane in data:
        s = lane["lane"].split("-")[0][:3].upper()
        for i, r in enumerate(lane.get("rows", []) or [], 1):
            r_id = f"SFR-REPASS-{lane['lane']}-{i:02d}"
            r["id"] = r_id
    fm = (
        "---\n"
        "file_id: studio-app-feature-research-parity-repass-delta\n"
        "topic_id: SFR-REPASS\n"
        'title: "Parity Re-Pass Delta (2026-07-20, ACTION-A5)"\n'
        "status: draft\n"
        'summary: "Targeted vendor re-pass rows closing the highest-value verified parity gaps from the 58-register: Figma Sites accessibility (CRITICAL), Camera Raw, InDesign fonts, Affinity option depth, PS AVIF/JXL+HDR. Online-source; NON-AI."\n'
        f"sources: {total}\n"
        f'updated_at: "{UPDATED}"\n'
        "---\n\n"
    )
    body = [fm, "## [SFR-REPASS] Parity Re-Pass Delta\n"]
    body.append("### [SFR-REPASS.summary] Summary\n")
    body.append(fence({
        "action": "ACTION-A5 (see 61-parity-audit-action-register.md)",
        "method": "Online-source targeted re-pass per lane; each row cites an authoritative vendor URL. NON-AI scope.",
        "total_rows": total,
        "by_lane": {l["lane"]: len(l.get("rows", []) or []) for l in data},
        "closes_critical": "Figma Sites per-element Accessibility panel + semantic HTML + landmarks + ARIA roles/labels/current/hidden — the sole CRITICAL gap (SFR-PGAP-FG) is now rowed at promotable depth.",
        "authority": "Reference/provenance only; not product authority. Feeds WP-KERNEL-STUDIO refinement Section-14 coverage decisions.",
    }) + "\n")
    for lane in data:
        body.append(f"### [SFR-REPASS.{lane['lane']}] {LANE_TITLE_A5.get(lane['lane'], lane['lane'])}\n")
        body.append(fence({"rows": lane.get("rows", []) or []}) + "\n")
    write("63-parity-repass-delta.md", "\n".join(body))
    return total


def gen_workflow():
    data = load("a6_workflow.json")
    total = sum(len(l.get("capabilities", []) or []) for l in data)
    for lane in data:
        for i, c in enumerate(lane.get("capabilities", []) or [], 1):
            c["id"] = f"SFR-WFRES-{lane['lane']}-{i:02d}"
    fm = (
        "---\n"
        "file_id: studio-app-feature-research-workflow-needs-research-delta\n"
        "topic_id: SFR-WFRES\n"
        'title: "Workflow Needs Research Delta (2026-07-20, ACTION-A6)"\n'
        "status: draft\n"
        'summary: "Research detail for the team/production workflow needs the feature corpus missed: review/approval, workflow-state PM, performance NFRs, DAM/PIM + governed libraries, culling/catalog. How incumbents implement each + the Handshake Studio requirement. NON-AI."\n'
        f"sources: {total}\n"
        f'updated_at: "{UPDATED}"\n'
        "---\n\n"
    )
    body = [fm, "## [SFR-WFRES] Workflow Needs Research Delta\n"]
    body.append("### [SFR-WFRES.summary] Summary\n")
    body.append(fence({
        "action": "ACTION-A6 (see 61-parity-audit-action-register.md)",
        "method": "Per-lane research of how incumbent tools implement each workflow capability (data model, states, mechanics), then the Handshake Studio requirement mapped to CRDT/EventLedger/sessions/permission primitives. NON-AI scope.",
        "total_capabilities": total,
        "by_lane": {l["lane"]: len(l.get("capabilities", []) or []) for l in data},
        "architecture_headline": "These are the coordination surfaces a feature-list audit cannot see. They point at a scale-adaptive review/approval + workflow-state + permission triad on the CRDT/EventLedger substrate, plus an integrated culling/catalog stage (the sole NOT_COVERED need) and DAM/PIM + governed library releases.",
        "authority": "Reference/provenance only; feeds WP-KERNEL-STUDIO refinement and possible Section-14 enrichment decisions.",
    }) + "\n")
    for lane in data:
        body.append(f"### [SFR-WFRES.{lane['lane']}] {LANE_TITLE_A6.get(lane['lane'], lane['lane'])}\n")
        body.append(fence({"capabilities": lane.get("capabilities", []) or []}) + "\n")
    write("64-workflow-needs-research-delta.md", "\n".join(body))
    return total


def gen_capture():
    data = load("a3_capture.json")
    total = sum(len(l.get("groups", []) or []) for l in data)
    fm = (
        "---\n"
        "file_id: studio-app-feature-research-command-shortcut-capture\n"
        "topic_id: SFR-CMDCAP\n"
        'title: "Command / Shortcut / Scripting-DOM Capture (2026-07-20, ACTION-A3)"\n'
        "status: draft\n"
        'summary: "STRUCTURE + representative-sample capture of default shortcuts, Illustrator menu-command IDs, and InDesign scripting DOM from PUBLIC ONLINE sources (no vendor apps). Partial: canonical Adobe SPA pages time out for non-browser clients; full binding tables remain a browser-fetch residual."\n'
        f"sources: {total}\n"
        f'updated_at: "{UPDATED}"\n'
        "---\n\n"
    )
    body = [fm, "## [SFR-CMDCAP] Command / Shortcut / Scripting-DOM Capture\n"]
    body.append("### [SFR-CMDCAP.summary] Summary\n")
    body.append(fence({
        "action": "ACTION-A3, replanned as online-source capture (operator has no vendor apps/subscriptions; installed-export path retired).",
        "coverage_meaning": {
            "FULL_LIST_ON_PAGE": "the source page carries the complete enumeration",
            "REPRESENTATIVE_SAMPLE": "exemplar entries verified verbatim from a reachable source; not exhaustive",
            "INDEX_ONLY": "canonical enumeration source identified (URL + category structure) but full table not transcribed this pass",
        },
        "total_groups": total,
        "by_lane": {f"{l['app']}/{l['surface_kind']}": len(l.get("groups", []) or []) for l in data},
        "fetch_blocker": "The canonical Adobe helpx default-keyboard-shortcuts pages render as slow AEM/JS SPAs that time out for all non-browser clients (WebFetch 60s x4, curl exit 28, Invoke-WebRequest 90s); r.jina.ai returned 422; web.archive.org blocked in this environment. So Adobe shortcut lanes are INDEX_ONLY + REPRESENTATIVE_SAMPLE from third-party cheat-sheets, not verbatim full tables. Illustrator executeMenuCommand: the docsforadobe Application page documents the method but does not enumerate the ID catalog (community/SDK-maintained). Ties to SFR-REMAINING-GAP-003.",
        "residual": "Full verbatim binding tables + the complete executeMenuCommand ID catalog need a browser-capable fetch (or the SFR-REMAINING-GAP-003 browser-export fallback). This capture gives the deterministic source URLs + category structure a later browser pass can complete.",
        "authority": "Reference/provenance only.",
    }) + "\n")
    for lane in data:
        for i, g in enumerate(lane.get("groups", []) or [], 1):
            g["id"] = f"SFR-CMDCAP-{lane['app']}-{lane['surface_kind']}-{i:02d}"
        body.append(f"### [SFR-CMDCAP.{lane['app']}-{lane['surface_kind']}] {lane['app']} / {lane['surface_kind']} ({len(lane.get('groups', []) or [])} groups)\n")
        body.append(fence({"groups": lane.get("groups", []) or []}) + "\n")
    write("65-command-shortcut-capture.md", "\n".join(body))
    return total


if __name__ == "__main__":
    a = gen_repass()
    b = gen_workflow()
    c = gen_capture()
    print("\nSUMMARY  repass_rows:", a, " workflow_capabilities:", b, " capture_groups:", c)
