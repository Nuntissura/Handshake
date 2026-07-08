---
file_id: tailor-bodykit-research-03-operator-body-requirements
topic_id: T-BK-OPERATOR-REQS
title: "Operator-Stated BodyKit Body Requirements (verbatim intent, 2026-07-08)"
status: operator_requirements_record
normative_status: non_normative_context_only_until_encoded_in_spec_and_MTs
source: "Operator messages, KERNEL_BUILDER session 2026-07-08. These statements are the scope authority for BodyKit morph-architecture design; they get encoded as normative Section 13 BodyKit clauses and MT acceptance criteria."
updated_at: "2026-07-08"
---

# Operator-Stated BodyKit Body Requirements

## Context

The operator has hands-on Daz 3D experience and finds it **very limiting** for the body types below. BodyKit replaces Daz in the pipeline. These are direct production requirements, not aspirations. Adult production (18+ subjects) is the primary use; game/general 3D is the later lane.

## OBR-001 — Full breast-shape space, independently blendable

Operator verbatim intent: "i want to create all kinds of tits, small perky, large natural droopy, extreme large and perky but still natural looking, extreme large fake plastic round and perky and everything in between."

Design consequence — breast shape MUST be a multi-axis continuous space with independent channels, minimally:

- **Volume/size** (small → extreme large, beyond any natural-population prior)
- **Ptosis/droop** (perky → natural droop/sag, gravity-real at every size)
- **Firmness/implant profile** (natural soft tissue → "fake plastic" implant look: round, high, rigid — the bolt-on aesthetic as a reachable, first-class target, not an artifact)
- **Shape profile** (teardrop/natural distribution ↔ round/hemispherical)
- Plus supporting channels: projection, spacing/cleavage gap, nipple/areola placement + size, upper-pole fullness.

Every combination in this space must be reachable ("everything in between"): e.g. extreme-large + perky + natural-looking is a REQUIRED reachable point (this specific combo is where Daz morph stacking visibly breaks). Physics (jiggle/soft-body) and corrective shapes must respect the firmness channel (implant-firm moves differently than natural-soft).

## OBR-002 — Shoulder width decoupled from bust/chest width

Operator verbatim intent: "shoulder width adjustable so i can create shoulders that are not tied to the bust/tits so i can emphasize large tits even more with narrow shoulders/soft. so decoupling shoulders width from chest width."

Design consequence:

- Shoulder width (clavicle length/frame) is its OWN channel, never driven by breast volume or ribcage/chest circumference channels.
- "Soft" narrow shoulders: shoulder morph includes deltoid/trap softness (muscle-tone channel per region), not just skeletal narrowing.
- The signature archetype — narrow soft shoulders + huge tits — is a canonical acceptance test for BodyKit; Daz's ERC-linked full-body morphs make exactly this combo fight itself.

## OBR-003 — Same decoupling for hips, thighs, midriff

Operator verbatim intent: "same for hips, thighs, midriff."

Design consequence — each is an independent region channel group, never auto-driven by any other region:

- **Hips**: hip width (skeletal) separate from glute volume/shape (round ass is its own channel per the earlier requirement) and separate from breast/shoulder anything.
- **Thighs**: thigh circumference/shape independent of hip width and glute volume (narrow thighs + round ass is a REQUIRED reachable combo).
- **Midriff**: waist circumference / belly shape / torso length independent of bust and hip channels (petite/skinny midriff under huge tits is a REQUIRED reachable combo).

## OBR-004 — Prior operator-stated body targets (2026-07-08, earlier message; still binding)

- Petite female bodies with unrealistically large tits; narrow shoulders + narrow hips + small hands simultaneously with huge breasts; skinny; long legs; round ass with narrow thighs.
- Male bodies with oversized penis; muscular variants; slender male/female; fat/obese male and female bodies.
- Breast size commonly linked with hip and shoulder width in existing tools — this coupling MUST be removed (the founding requirement of BodyKit's decoupled architecture).
- Exports usable by Blender and/or Unreal Engine.

## Architecture implications (for spec + MT authoring)

1. **No hidden inter-region drivers.** Region morph channels (breasts, shoulders, hips, glutes, thighs, midriff, hands, legs-length, arms, neck, face, genitals) never write to each other. Cross-region correlation exists ONLY as optional, inspectable, operator-authorable "linked presets" that are syntactic sugar over independent channels — never baked coupling (anti-ERC-link stance).
2. **Skeletal vs soft-tissue split per region.** Frame channels (clavicle width, hip bone width, limb lengths, hand/foot scale) are bone-level; tissue channels (breast volume/ptosis/firmness, glute volume, thigh fat/muscle, belly) are morph/sim-level; both independently addressable.
3. **Correctives are per-body re-bakeable.** JCM-equivalents (RBF pose-space correctives + sim-to-corrective baking) regenerate for the CURRENT channel configuration, so extreme decoupled combos keep working under pose — no fixed-prior JCM set to outgrow.
4. **Collision proxies + physics params derive from channels.** Breast collider decomposition (Cloth §13.4 multi-sphere/SDF) and jiggle parameters (softness/firmness) are derived mechanically from the breast channels (volume/firmness), keeping Cloth drape correct across the whole space.
5. **Acceptance archetypes** (each becomes a seeded test body + validation row): (a) petite frame + extreme-large perky natural breasts + narrow soft shoulders + narrow hips + small hands + long legs + round ass + narrow thighs + skinny midriff; (b) extreme-large fake/plastic round implant look on a slender frame; (c) small perky on athletic frame; (d) large natural droopy on fat/obese female body; (e) obese male; (f) muscular male + oversized penis; (g) slender male + oversized penis; (h) futa/cross-blend build (per beyond-parity competitive finding: unrestricted cross-sex channel stacking).
6. **Numeric measurement targeting** (SREQ-009) applies per channel: "make shoulders X cm, bust Y cm" solves without touching other regions.
