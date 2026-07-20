#!/usr/bin/env python3
"""Generate the 2026-07-20 parity-audit register docs (58/59/60) from the
adversarially-verified multi-agent audit findings.

Inputs (committed provenance, repo-relative):
  _audit_20260720/inputs/parity_merged.json    -> per-app surviving feature-parity gaps
  _audit_20260720/inputs/parity_bridges.json   -> per-app bridge opportunities (features the apps lack)
  _audit_20260720/inputs/workflow_needs.json   -> unified workflow-needs registry with corpus coverage verdicts

Outputs (corpus-convention .md, machine data in fenced json/yaml blocks):
  58-parity-feature-gap-register.md
  59-workflow-needs-register.md
  60-bridge-opportunity-register.md

Authority note: reference/provenance only, per index.yaml authority_note and
[STU-SECTION-002]. Not a Work Packet, Master Spec, validator gate, or product authority.
Regenerate after editing the JSON inputs. Disk-agnostic: all paths repo-relative.
"""
import json
import os
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
INPUTS = os.path.join(ROOT, "_audit_20260720", "inputs")
UPDATED = "2026-07-20"

APP_SLUG = {
    "Adobe Photoshop (incl. Camera Raw)": "PS",
    "Adobe Illustrator": "AI",
    "indesign": "ID",
    "Affinity V2 suite (Photo 2, Designer 2, Publisher 2, StudioLink)": "AF",
    "Figma (Design, Draw, Dev Mode; FigJam/Slides/Sites/Buzz/Make secondary)": "FG",
    "Adobe Photoshop": "PS",
    "Adobe InDesign": "ID",
    "Affinity V2 suite (Photo/Designer/Publisher + StudioLink)": "AF",
    "Figma (Design, Draw, Dev Mode)": "FG",
    "photoshop": "PS", "illustrator": "AI", "affinity": "AF", "figma": "FG",
}


def load(name):
    with open(os.path.join(INPUTS, name), "r", encoding="utf-8") as f:
        return json.load(f)


def slug(app):
    return APP_SLUG.get(app, "".join(c for c in app.upper() if c.isalpha())[:3] or "XX")


def fence(obj):
    return "```json\n" + json.dumps(obj, indent=2, ensure_ascii=False) + "\n```"


def write(path, text):
    with open(os.path.join(ROOT, path), "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    print("wrote", path, len(text), "bytes")


# ---------------------------------------------------------------- 58 gap register
def gen_gap_register():
    data = load("parity_merged.json")
    sev = Counter()
    verd = Counter()
    per_app = []
    total = 0
    for entry in data:
        s = slug(entry["app"])
        gaps = entry.get("surviving_gaps", []) or []
        rows = []
        for i, g in enumerate(gaps, 1):
            gid = f"SFR-PGAP-{s}-{i:03d}"
            sev[g.get("severity", "UNSPEC")] += 1
            verd[g.get("verdict", "UNSPEC")] += 1
            total += 1
            rows.append({
                "id": gid,
                "app": s,
                "title": g.get("title", ""),
                "domain": g.get("domain", ""),
                "severity": g.get("severity", ""),
                "verdict": g.get("verdict", ""),
                "description": g.get("description", ""),
                "external_evidence": g.get("external_evidence", ""),
                "corpus_evidence": g.get("corpus_evidence", ""),
                "search_hints": g.get("search_hints", ""),
            })
        per_app.append({
            "app": entry["app"], "slug": s,
            "surviving_gap_count": len(rows),
            "refuted_count": entry.get("refuted_count", 0),
            "concerns": entry.get("concerns", []) or [],
            "gaps": rows,
        })

    summary = {
        "audit_date": UPDATED,
        "scope": "NON_AI feature parity for Handshake Studio replacing Photoshop(+Camera Raw)/Illustrator/InDesign/Affinity-V2/Figma",
        "method": "Two independent multi-agent workflows (corpus-read -> external feature-universe delta -> adversarial corpus verification). Only gaps that survived a refutation pass are recorded here.",
        "verdict_meaning": {
            "CONFIRMED_MISSING": "no real feature row/card/ledger entry found in the corpus (raw _source_snapshots mention does not count as coverage)",
            "PARTIALLY_COVERED": "adjacent coverage or raw-snapshot-only; not a promotable feature row",
        },
        "total_surviving_gaps": total,
        "by_severity": dict(sev),
        "by_verdict": dict(verd),
        "by_app": {p["slug"]: p["surviving_gap_count"] for p in per_app},
        "refuted_by_app": {slug(e["app"]): e.get("refuted_count", 0) for e in data},
        "root_cause": "The card/row/coverage pipeline (docs 15/39/49) consumes only the leaf indexes, so the 2,330 deep-delta rows (docs 51-55) are invisible to the coverage matrix. The corpus completeness surfaces (08 completeness_audit, 49 missing_required_fields=0) audit field completeness of the leaf pipeline only and cannot detect the semantic omissions recorded here. None of these gaps was acknowledged in 08 or 49 before this register.",
        "cross_app_gap_families": [
            "XAPP-01 installed-app command/shortcut/menu enumeration never captured (export scripts never run; _installed_exports/ absent)",
            "XAPP-02 automation/scripting surfaces count-only (member-level DOM, menu-command IDs uncaptured)",
            "XAPP-03 print-dialog option depth carried by single summary rows",
            "XAPP-04 CJK composition depth (composite fonts / ruby / kinsoku) thin-to-absent",
            "XAPP-05 performance envelopes unresearched (GPU matrices, memory ceilings, scale limits)",
            "XAPP-06 release-currency drift with no vendor re-crawl mechanism (ACR 2025-26, Affinity 2.6.x/v3, Figma post-2026-07-09)",
        ],
    }

    fm = (
        "---\n"
        "file_id: studio-app-feature-research-parity-feature-gap-register\n"
        "topic_id: SFR-PGAP\n"
        'title: "Parity Feature Gap Register (2026-07-20 audit)"\n'
        "status: draft\n"
        'summary: "Adversarially-verified NON-AI feature-parity gaps that survived corpus refutation, per app, with severity, verdict, and corpus/external evidence."\n'
        f"sources: {total}\n"
        f'updated_at: "{UPDATED}"\n'
        "---\n\n"
    )
    body = [fm]
    body.append("## [SFR-PGAP] Parity Feature Gap Register\n")
    body.append("### [SFR-PGAP.summary] Audit Summary\n")
    body.append(fence(summary) + "\n")
    body.append(
        "### [SFR-PGAP.authority] Authority\n\n"
        "Reference/provenance only. This register records where the corpus is missing or thin against the apps' real NON-AI feature universe; it is not a Work Packet, Master Spec, validator gate, or product authority ([STU-SECTION-002], index.yaml authority_note). Section 14 of the Master Spec remains sole Studio authority. Gaps here are inputs the WP-KERNEL-STUDIO refinement must resolve into Section-14 parity coverage.\n"
    )
    for p in per_app:
        body.append(f"### [SFR-PGAP.{p['slug'].lower()}] {p['app']} ({p['surviving_gap_count']} gaps, {p['refuted_count']} refuted)\n")
        if p["concerns"]:
            body.append("Corpus concerns raised for this app:\n\n" + fence({"concerns": p["concerns"]}) + "\n")
        body.append(fence({"gaps": p["gaps"]}) + "\n")
    write("58-parity-feature-gap-register.md", "\n".join(body))
    return summary


# ---------------------------------------------------------------- 59 workflow needs
def gen_needs_register():
    needs = load("workflow_needs.json")
    board = defaultdict(Counter)
    verd = Counter()
    crit = Counter()
    for n in needs:
        v = n.get("coverage_verdict", "UNVERIFIED")
        d = n.get("domain", "unspecified")
        board[d][v] += 1
        verd[v] += 1
        crit[n.get("max_criticality", "UNSPEC")] += 1

    scoreboard = {dom: dict(cnts) for dom, cnts in sorted(board.items())}
    uncovered = [n["id"] for n in needs if n.get("coverage_verdict") != "COVERED"]

    summary = {
        "audit_date": UPDATED,
        "scope": "NON_AI workflow-level needs surfaced by 82 professional scenarios across large-team / small-team / solo scale + cross-app pipelines, for all 5 apps",
        "method": "Scenario research per app x team-scale -> unified deduped needs registry -> each need verified against the research corpus (00-57) and Master Spec module 14 incl. kernel primitives (CRDT/EventLedger/sessions).",
        "verdict_meaning": {
            "COVERED": "an explicit feature row/card/spec section addresses the need",
            "PARTIALLY_COVERED": "adjacent or raw-snapshot-only coverage",
            "NOT_COVERED": "corpus + spec searches came up empty",
            "UNVERIFIED": "coverage verifier did not return a result for this need id",
        },
        "total_needs": len(needs),
        "by_verdict": dict(verd),
        "by_criticality": dict(crit),
        "scoreboard_domain_x_verdict": scoreboard,
        "uncovered_or_partial_need_ids": uncovered,
        "systemic_holes": [
            "review / approval / sign-off / version-of-record provenance chain (NEED-005/017/022)",
            "production workflow-state / project-management surface (NEED-009)",
            "integrated culling / rating / catalog stage (NEED-051, sole NOT_COVERED)",
            "external asset ecosystem: DAM/PIM round-trip + governed versioned library releases (NEED-045/057)",
            "production-volume performance as explicit NFRs (NEED-026/047/071)",
        ],
        "architecture_implication": "Every fully-COVERED need is single-artifact craft; every hole is a team-scale coordination surface. Studio needs a scale-adaptive review/approval + workflow-state + permission triad layered on the CRDT/EventLedger substrate, invisible-for-solo but governable-for-large.",
    }

    fm = (
        "---\n"
        "file_id: studio-app-feature-research-workflow-needs-register\n"
        "topic_id: SFR-WNEED\n"
        'title: "Workflow Needs Register (2026-07-20 scenario audit)"\n'
        "status: draft\n"
        'summary: "Unified NON-AI workflow-level needs from 82 pro scenarios (large-team/small-team/solo + cross-app), each with corpus/spec coverage verdict. Surfaces team-scale coordination needs a feature-list audit cannot see."\n'
        f"sources: {len(needs)}\n"
        f'updated_at: "{UPDATED}"\n'
        "---\n\n"
    )
    body = [fm]
    body.append("## [SFR-WNEED] Workflow Needs Register\n")
    body.append("### [SFR-WNEED.summary] Scenario Audit Summary\n")
    body.append(fence(summary) + "\n")
    body.append(
        "### [SFR-WNEED.authority] Authority\n\n"
        "Reference/provenance only. Complements the feature-parity register (58): 58 covers 'can one person do X to one document', this covers 'can a team of a given scale run a professional production workflow'. Inputs for the WP-KERNEL-STUDIO refinement and Section-14 coverage; not product authority.\n"
    )
    body.append("### [SFR-WNEED.needs] Needs\n")
    body.append(fence({"needs": needs}) + "\n")
    write("59-workflow-needs-register.md", "\n".join(body))
    return summary


# ---------------------------------------------------------------- 60 bridge opportunities
def gen_bridge_register():
    data = load("parity_bridges.json")
    per_app = []
    total = 0
    for entry in data:
        s = slug(entry["app"])
        feats = entry.get("missing_features", []) or []
        rows = []
        for i, f in enumerate(feats, 1):
            total += 1
            rows.append({
                "id": f"SFR-BRIDGE-{s}-{i:03d}",
                "app": s,
                "title": f.get("title", ""),
                "description": f.get("description", ""),
                "demand_evidence": f.get("demand_evidence", ""),
                "bridge_opportunity": f.get("bridge_opportunity", ""),
            })
        per_app.append({"app": entry["app"], "slug": s, "count": len(rows), "features": rows})

    summary = {
        "audit_date": UPDATED,
        "scope": "NON_AI features the 5 source apps themselves lack or do badly, that Handshake Studio could ship as differentiators (bridge opportunities)",
        "method": "Per-app research of long-standing user feature requests, forum/reddit complaints, and switching guides. Licensing/pricing complaints excluded unless they imply a technical capability (offline/local files).",
        "note": "AI/generative differentiators are intentionally excluded; Handshake ships its own AI natively later.",
        "total_bridge_features": total,
        "by_app": {p["slug"]: p["count"] for p in per_app},
        "top_deduped_themes": [
            "local-first files + true offline (Figma structural; Adobe cloud-doc-only)",
            "persistent local version history + real branching in the document format",
            "CRDT co-editing over LAN / self-hosted relay (no vendor cloud)",
            "one modern automation API (loops/conditions/data) doubling as the agent surface",
            "fully non-destructive node graph by default + PSD smart-object preservation",
            "unified codec-forward export (JXL/AVIF/WebP, unlimited size, deterministic batch)",
            "full prepress in every editor + local spot-color infra (never render unlicensed Pantone black)",
            "native tagged PDF/UA that survives re-export (EAA compliance trigger)",
            "universal HarfBuzz-class international text engine (RTL/CJK/composite fonts everywhere)",
            "performance-class engine as marketing (GPU-native, disk-streaming past RAM ceiling, unbounded canvas)",
        ],
    }

    fm = (
        "---\n"
        "file_id: studio-app-feature-research-bridge-opportunity-register\n"
        "topic_id: SFR-BRIDGE\n"
        'title: "Bridge Opportunity Register (2026-07-20 audit)"\n'
        "status: draft\n"
        'summary: "NON-AI features the 5 apps themselves lack that Handshake Studio could ship as differentiators, per app, with demand evidence and bridge framing."\n'
        f"sources: {total}\n"
        f'updated_at: "{UPDATED}"\n'
        "---\n\n"
    )
    body = [fm]
    body.append("## [SFR-BRIDGE] Bridge Opportunity Register\n")
    body.append("### [SFR-BRIDGE.summary] Summary\n")
    body.append(fence(summary) + "\n")
    body.append(
        "### [SFR-BRIDGE.authority] Authority\n\n"
        "Reference/provenance only. Differentiator candidates, not committed scope. Feed the WP-KERNEL-STUDIO refinement and Section-14 'differentiator' framing ([STU-OVR-003]); promotion into product scope requires operator decision.\n"
    )
    for p in per_app:
        body.append(f"### [SFR-BRIDGE.{p['slug'].lower()}] {p['app']} ({p['count']} opportunities)\n")
        body.append(fence({"bridge_features": p["features"]}) + "\n")
    write("60-bridge-opportunity-register.md", "\n".join(body))
    return summary


if __name__ == "__main__":
    g = gen_gap_register()
    n = gen_needs_register()
    b = gen_bridge_register()
    print("\nSUMMARY")
    print("  parity gaps:", g["total_surviving_gaps"], g["by_severity"])
    print("  workflow needs:", n["total_needs"], n["by_verdict"])
    print("  bridge features:", b["total_bridge_features"])
