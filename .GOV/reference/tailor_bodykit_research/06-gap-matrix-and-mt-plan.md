---
file_id: tailor-bodykit-research-06-gap-matrix-and-mt-plan
topic_id: T-BK-GAP-MATRIX
title: "Second-Pass Cloth Parity Gap Matrix (CG-01..29 -> MT-449..477) + BodyKit Fold-In MT Blueprint (MT-478..653)"
status: non_normative_research
normative_status: authoring_blueprint_context_only
author: KERNEL_BUILDER, session 2026-07-08
method: >
  Cross of the independent MD inventory (04-md-feature-inventory.md, full manual corpus) against
  the 448-MT coverage map (02-current-coverage-map.md) and Section 13 normative scope. Only
  features NOT covered by any existing MT or spec clause appear as gaps. GUI-chrome-only MD
  features map to the model-first API + native-shell lane and are marked N/A-GUI with reason.
  BodyKit plan synthesizes 03-operator-body-requirements (OBR), 05-daz REQ-001..044,
  01-beyond-parity GREQ/PREQ/AREQ/SREQ, and the coverage map's body/avatar status quo.
updated_at: "2026-07-08"
---

# Gap Matrix + MT Plan (authoring blueprint)

## Part A — Cloth second-pass parity gaps (CG-01..29 -> MT-449..477, group per row)

Each row becomes one template-exact `hsk.microtask_contract@1` MT. depends_on continues the linear
chain (MT-449 depends on MT-448; each next on the previous) — the packet-wide reconciliation
(reshuffle notes) reorders at create-task-packet time, same as the first parity block.

| CG | MT | Group | Scope (summary basis) | MD source feature(s) | Spec anchor |
|---|---|---|---|---|---|
| CG-01 | MT-449 | GarmentAuthoring | Internal holes/cutouts in PanelSpec: InternalShape kind Hole; tessellation excludes hole interiors; PANEL_CLOSED + MESH_MANIFOLD updated; vents/eyelet holes | Convert to Hole/Internal Shape | 13.6 PanelSpec/InternalLine + 13.14 GarmentSpec |
| CG-02 | MT-450 | GarmentAuthoring | Spiral panel primitive constructor (turns, inner/outer radius, width) for flounces/ruffles; LLM-emittable params | Spiral tool | 13.6 primitive constructors (MT-338 sibling) |
| CG-03 | MT-451 | GarmentAuthoring | split_panel along an internal line (Cut) with optional auto-seam across the cut (Cut & Sew) + merge_panels inverse across a shared seam | Cut / Cut & Sew, Merge patterns | 13.6 panel ops |
| CG-04 | MT-452 | GarmentAuthoring | Layer Clone Over/Under: linked lining/padding instance panels auto-sewn edge-to-edge with layer offset + sim layer_index; linked-edit propagation | Layer Clone, Linked Editing | 13.6 + 13.4 multi-layer |
| CG-05 | MT-453 | GarmentAuthoring | Persistent symmetric-link editing: half/pair symmetry between panels incl. linked sewing; per-internal-line unlink; CRDT-safe | Unfold Symmetric Editing (with Sewing), Unlink Internal Line Symmetry | 13.6 + 13.11 CRDT |
| CG-06 | MT-454 | Fabric | Region stiffness/thickness override system: painted/region masks applying compliance+thickness overrides with presets (SeamTaping fusible x4/reinforcement x2, Bond, Skive soften, temporary Strengthen incl. partial) | Seam Taping, Bond, Skive, Strengthen | 13.5 material overrides |
| CG-07 | MT-455 | Fabric | Steam brush: localized shrinkage mask painting (add/remove, strength, radius) -> per-region rest-length scaling in solver | Steam brush/Steam Eraser, Shrink local | 13.5 shrinkage + 13.3 rest-length |
| CG-08 | MT-456 | GarmentAuthoring | SeamKind::Turned (two-ply turned seam, disables fold angle) + press_seam operation flattening sewn double plies | Turned sewing type, Press tool | 13.6 SeamSpec + 13.3 seam constraints |
| CG-09 | MT-457 | SolverCore | Seam sublayer ordering: per-seam sublayer int ordering self-folding sewing (seam allowances/pleats) in collision/constraint resolution | Set Sublayer | 13.4 layer ordering + 13.3 |
| CG-10 | MT-458 | UvTexture | Seam puckering: procedural pucker normal/color map generation along seam lines w/ placement control (side/both, width, intensity) | Puckering | 13.9 texture generation |
| CG-11 | MT-459 | TrimRigid | Piping edge treatment: rounded piping geometry along garment edges (radius, fabric ref, closed-end option) as solver-coupled edge trim | Piping | 13.8 edge trims (ElasticInsert/BindingTape sibling) |
| CG-12 | MT-460 | GarmentAuthoring | Fold Arrangement: pre-sim fold poses along internal fold lines (angle per line, symmetric folding) applied at arrangement stage before first drape (collars/plackets/lapels) | Fold Arrangement | 13.6 arrangement + 13.3 initial state |
| CG-13 | MT-461 | GarmentAuthoring | Superimpose (over/under/side) + smart arrangement: place a sewn panel directly onto its counterpart respecting current drape | Superimpose, Smart Arrangement | 13.6 arrangement anchors (MT-120 sibling) |
| CG-14 | MT-462 | SolverCore | Interactive simulation session: long-running cancellable sim job accepting in-loop CRDT panel edits + quick-pinch drag with soft-selection falloff (shape/distance/power); warm-restart preserving drape on 2D edits (incl. preserve-3D-shape-on-2D-scale) | Interactive editing, Quick/Advanced Pinching, Preserve 3D on 2D scaling | 13.3 loop + 13.11 jobs + 13.13 leases |
| CG-15 | MT-463 | Collision | Intersection resolution/untangle pass (normal/flipped-normal recovery) + collision pair-type toggles (avatar-cloth/self/proximity; triangle-vertex vs edge-edge) in SimRunParams + Deactivate scopes (pattern-only vs pattern+sewing) | Intersection Resolution, collision toggles, Deactivate modes | 13.4 + 13.3 SimRunParams |
| CG-16 | MT-464 | Collision | Ground-plane collider + arbitrary static prop mesh colliders (per-prop collision thickness 0-100mm) bound from imported props | Ground Setting, Scene & Props Collision | 13.4 body/rigid collision |
| CG-17 | MT-465 | GarmentAuthoring | Sculpt brush set expansion on the MT-356 sculpt delta layer: wrinkle/release, stamp w/ alpha, pinch brushes; tablet-pressure param pass-through | Sculpt mode brushes | 13.6 sculpt (MT-356 extension) |
| CG-18 | MT-466 | Fabric | Extended calibrated fabric preset pack: grow seeded library from 12 to 40+ presets covering MD-common families (poplin, oxford, twill, gabardine, velvet, corduroy, fleece, terry, lace, mesh, organza, taffeta, tulle, neoprene, vinyl/PU, latex, sequin base...) each with full calibration JSON + drape test | ~77 preset library | 13.5 presets (MT-199 sibling) |
| CG-19 | MT-467 | UvTexture | Per-face material slots: distinct front/back/side PBR material assignment per panel; back-face texture transforms | Fabric Front/Back/Side Setting | 13.9 §5 material assignment |
| CG-20 | MT-468 | UvTexture | Repeating print layer: fabric-level print layer with repeat modes Block/HalfDrop/Brick/Diamond/Stripe, spacing/shift params, PBR blending | Print on Fabric | 13.9 §4 graphic layers |
| CG-21 | MT-469 | ModelFirstApi | Custom tape measures: typed linear/circumference/surface tapes on avatar AND garment; measure_garment MCP tool returning named measurement rows (LLM fit-verification surface) | Avatar/3D Garment Tape Measures | 13.7 §7.1 measurements + 13.13 tools |
| CG-22 | MT-470 | GarmentAuthoring | Numeric edge ops: change_edge_length (with curve-point lock option) + match_edge_lengths between two panels (numeric seam-length reconciliation) | Change Length, Match 2D Pattern Measurements | 13.6 edge ops |
| CG-23 | MT-471 | AutoFit | Flatten options: straight-line-constraint flatten, multi-area merged flatten into one panel, flattened-outline point optimization | Flattening as Straight Line, merged flatten, optimize points | 13.7 ARAP re-unfurl + 13.9 §2 |
| CG-24 | MT-472 | TrimRigid | Auto-distribute trim placements along an edge/outline (count/interval; buttons+buttonholes pairing) + cross-seam 3D graphic projection (graphic spans seam-adjacent panels) | Buttons along outline, 3D graphics over seamline | 13.8 placements + 13.9 §4 |
| CG-25 | MT-473 | ProductionBridge | Export mesh topology options across lanes: weld/unweld sewn vertices, merge-by-proximity distance weld, thin/thick export (pattern thickness extrusion w/ curved side profile + front/back/seam face toggles), triangle-vs-quad output per target | Weld/Unweld, Merge Vertex by Proximity, Thin/Thick, Add Pattern Thickness | 13.12 §6 export + GREQ-020 |
| CG-26 | MT-474 | Animation | Simulation cache post-editing: trim/cut cached ranges, loop-safe cycle creation (seam frame blending), cache blend/merge between takes, retime; loop metadata persisted for export (game/adult loop workflow) | Animation Editor cache keys, Animation Layers cut/merge, Scene Time Warp | 13.10 caching + AREQ-012 |
| CG-27 | MT-475 | RenderViewportExport | Viewport video capture: encode viewport/turntable/animation-range to MP4/WebM headlessly via Handshake-native encoder path (no outside app); size presets; capture manifest row | Video Capture | 13.12 §4 capture + CX-503S |
| CG-28 | MT-476 | ProductionBridge | Garment skin-weight binding export: transfer body skeleton weights to draped garment mesh for non-simulated wear (EveryWear-core basic path; per-garment-class projection profiles; rigidity masks honored); exported in glTF/FBX-bridge skeletal lanes | EveryWear core, MD 2025.2 skin-weight transfer | 13.12 §6 + GREQ-019 + MOAT-7 partial |
| CG-29 | MT-477 | ProductionBridge | VAT (vertex animation texture) export of simulated caches for GPU playback in engines (position/normal textures, looping support, per-target texel budgets, documented limits) | (beyond-MD: GREQ-021 baked-cloth game lane) | 13.12 §6 export + GREQ-021 |

Intentional N/A (recorded, no MT): manual retopo face-editor (auto-quad MT-385 + Blender lane suffice; revisit post-V1), COLLADA import (glTF/FBX/OBJ cover), Maya .mc/.mcx cache (MDD/PC2+Alembic cover), LXO/Sansar/HTML-snapshot/Omniverse/CONNECT/CLO-SET (ecosystem, not core), SBSAR (proprietary Substance runtime; weave generator + AI texture gen cover), GUI chrome items (hotkeys/layouts/rulers/snapping/palettes = native-shell WP-KERNEL-011 + model-first API make these projections), autosave/history (EventLedger + snapshots exceed), 2GB file limit (Postgres authority), USDZ (acceptance addition to MT-250 at implementation), down-fill (CLO-only; Pressure+LayerClone path per MD), grading/DXF-AAMA-import/seam-allowance-tool/notch-tool/colorways/offline-render/print-layout (CLO-only, NOT MD parity; DXF R12 export MT-443 and SeamSpec allowance already exceed MD).

## Part B — BodyKit fold-in MT blueprint (MT-478..653, 176 MTs, 14 groups)

Numbering is authoritative. Every MT cites its governing 13.16+ subsection (authored in bundle
v02.198) + the research REQ/OBR/GREQ/PREQ ids it closes. depends_on: linear within group; first MT
of each group depends on the last MT of the previous group (packet reconciliation reorders later).
All MTs use the CURRENT template (gui_obligation, user_manual_obligation, hbr_int_009_tier_obligations).
Diagnostic tiers: FlightRecorder WIRED (event families exist), internal_diagnostics/Palmistry DEFERRED-with-reason until WP-KERNEL-012/016 ship.

### BkFoundation — MT-478..491 (14) — spec 13.16 (BodyKit overview/architecture/authority)
478 bodykit module scaffold (src/tailor/bodykit/ mod, error types, event_family constants; solver-side body code stays in tailor-solver/src/body/)
479 canonical BodySpec type (hsk.tailor.body_spec@1: channels map, skeleton ref, genital config, skin config, recipe refs; schemars; cm units LLM surface / mm authority)
480 migrations: bodykit_bodies (BDY-) + bodykit_recipes (BRC-) tables (TEXT PKs, event_ledger_event_id FK, guard_authority_write)
481 migrations: bodykit_morph_assets (BMA-) + bodykit_channel_registry (BCH-) tables (sparse delta payload refs, region mask refs, multi-key track metadata, license_tag column)
482 migrations: bodykit_skeleton_defs (BSK-) + bodykit_skin_materials (BSM-) + bodykit_export_profiles (BXP-) tables
483 Tailor* event additions for BodyKit (TAILOR_BODY_* family: BodyCreated/ChannelChanged/RecipeSaved/CorrectiveBaked/GenitalConfigured/SkinBaked/ExportCompleted/...) registered per 13.14 canonical list
484 original base mesh asset ingest: load the commissioned/original watertight quad base mesh (bootstrap reference: one-time MakeHuman CC0 GUI export; NO SMPL, NO Genesis) as versioned artifact with topology contract validation
485 topology contract validator: watertight, single component, quad-dominant, region-mask completeness, genital-region loops present, pole/valence budget, UDIM tile layout (head/torso/limbs/genitals) — BODY_TOPOLOGY_VALID descriptor
486 region mask authority: canonical vertex-region masks (breasts L/R, glutes L/R, thighs inner/outer, shoulders, midriff/belly, hands, feet, arms, legs, neck, head, genital regions) stored versioned with the base mesh
487 SubD cage architecture: Catmull-Clark cage + per-level evaluation in tailor-solver body module (view level vs bake level); open multi-res delta storage format (REQ-037 foundation)
488 license guard tripwire: reject SMPL/.dhdm/Genesis-derived payload imports as authority (magic/URI/metadata heuristics + explicit license_tag required on morph assets) — fail-closed check + test (REQ license findings)
489 BodyKit REST routes (src/api/tailor.rs additions: bodies/recipes/channels CRUD + export trigger) with typed JSON navigation by id
490 BodySpec persistence glue: bodykit storage_glue writes with event-before-row CTE; EventLedger replay reconstructs body authoring state (TAI-OVR-007 parity for bodies)
491 BkFoundation integration test: create body -> set channels -> save recipe -> replay from ledger -> identical BodySpec

### BkChannels — MT-492..513 (22) — spec 13.17 (decoupled region channel system) [OBR-001..004, REQ-001..016]
492 channel registry core: BodyChannel type (id, region mask ref, kind skeletal|tissue, range with designed over-range, default; stable snake_case ids addressable by API) (REQ-043)
493 sparse morph delta storage + application: multi-key morph tracks per channel (sculpted keys at 0/50/100/200/300%, interpolated — NOT single-delta scaling) (REQ-009)
494 channel stacking evaluator with volume-preservation post-pass (delta-mush option) (REQ-007)
495 region-mask write enforcement: a channel MUST NOT write outside its declared mask; validator BODY_DECOUPLE_VALID proves zero cross-region deltas (REQ-001/002)
496 explicit coupling-link graph: named, inspectable, per-edge disable, isolate-region override; NO hidden links; links are operator/model-authored presets over independent channels (anti-ERC) (REQ-003, OBR implications 1)
497 breast channel group A (volume, projection, placement height/width, spacing/cleavage gap) with L/R asymmetry sub-channels (OBR-001)
498 breast channel group B (ptosis/droop, firmness natural<->implant-rigid, shape profile teardrop<->round, upper-pole fullness) — full combination space reachable incl. extreme-large+perky+natural and fake-plastic-round (OBR-001)
499 nipple/areola sub-module channels (size, puff, direction, areola diameter/color-zone hook to skin layer) (REQ-010)
500 shoulder frame channels: clavicle width (skeletal) + deltoid/trap softness (tissue), fully independent of all breast/chest channels (OBR-002)
501 hip channels: hip bone width (skeletal) separate from glute volume; pelvis tilt (OBR-003)
502 glute channel group (volume, roundness, lift, width, hip-dip fill, crease sharpness) independent of thigh channels (REQ-012)
503 thigh/leg channels: thigh girth inner/outer split, thigh gap, calf girth, leg length via bone-scale hybrid (REQ-013, OBR-004 long legs)
504 midriff/waist channels: waist circumference, belly size, torso length — independent of bust and hip channels (OBR-003)
505 belly/fat distribution channel group (apron/overhang, rolls count+position, love handles, back fat, arm/thigh fat, double chin) M/F fat-distribution presets (REQ-014 obese/fat bodies)
506 muscle channel group: mass (geometry) separate from definition (detail-map weight) separate from flexion (pose-driven hook) per muscle group (REQ-015)
507 skinny/slender channels: subcutaneous fat removal, rib/hip-bone visibility with explicit breast/glute exclusion masks (REQ-016)
508 hand/foot scale channels (small hands per operator archetype) + neck/arm girth (OBR-004)
509 height + global frame channels decomposed into limb/torso ratios (petite = designed axis set usable with any tissue channel at max) (REQ-005)
510 masc/fem continuum channel (single-base cross-blend incl. futa builds; gender as continuous axis) (Daz G9 pattern + beyond-parity cross-blend)
511 region-pair compensation morphs (breast x ribcage seam, glute x thigh crease, belly x hip overlap) authored as gated pair-correctives, not full-body (REQ-006)
512 measurement solver: numeric targeting per channel (set bust_circ/waist/shoulder_width/penis_length in cm -> solve channel values without touching other regions) (SREQ-009, OBR implications 6)
513 seeded randomization: whole-body/per-region/per-channel scopes with plausibility-weighted distributions + deterministic seeds; batch variation generation (SREQ-007/008)

### BkSkeleton — MT-514..527 (14) — spec 13.18 (skeleton/skinning/posing) [REQ-017..022, GREQ-005/006/023/024/036..038]
514 canonical BodyKit skeleton definition: UE5-convention naming/orientation where anatomy overlaps (pelvis, spine_01.., clavicle_l..) + breast_l/r, glute_l/r, belly chains + full genital chains (penis_01..05, scrotum, labia set, anus) + twist bones; versioned skeleton spec (AREQ-014 stability)
515 per-bone scale channels with child-propagation control (limb length != thickness) exposed as safe channel axes (REQ-021)
516 LBS skinning weights authority on base mesh + optional DQS/blended preview mode (REQ-020)
517 automatic joint re-derivation from mesh landmarks on every channel change, interpolated continuously with channel values (continuous Adjust-Rigging-to-Shape) (REQ-017)
518 skinning weight re-projection at extremes: weights defined in canonical space re-projected as regions grow (giant breasts/ass/belly keep clean deformation) (REQ-019)
519 full-body IK posing with pins (hands/feet/knees pinning; FABRIK/2-bone reuse; drives skeleton not just capsules) (Daz Active-Pose parity, AREQ-017)
520 pose authority + pose library: BodyPose assets (bone transforms + morph-context re-solve on apply; IK retarget on differing proportions) (AREQ-003)
521 multi-actor pose assets: relative root alignment between 2+ bodies (couple/group sex poses placeable as one asset) + stage anchor conventions (AREQ-002/013)
522 pose symmetry mirror + pose blending; puppeteer-style blend between N poses (Daz parity)
523 BVH/FBX/glTF mocap ingestion with bone-map presets (Mixamo/Rokoko/UE/Unity/Daz/VRM naming) + auto-detection (AREQ-004, GREQ-036)
524 retarget engine: chain-based mapping between arbitrary skeletons w/ retarget base pose, bake-to-target, headless API (AREQ-005/019)
525 jiggle-bone secondary-motion chain generation from region channels (breasts/glutes/belly/penis/testicles) with per-chain params (GREQ-023)
526 physics-config sidecar: engine-agnostic JSON per-region stiffness/damping/gravity/limits + per-component budget lint (GREQ-024)
527 twist/roll bone generation option per export profile + root-motion conventions per engine (GREQ-037/038)

### BkCorrectives — MT-528..537 (10) — spec 13.19 (corrective engine + sim-to-bake) [REQ-018, PREQ-014..016, GREQ-026/027]
528 RBF pose-space deformation engine: corrective shapes interpolated over pose-space example points (ramp-in, no popping) (PREQ-015)
529 corrective authoring pipeline: sculpt-import correctives with reverse-deformation filtering + deltas-only overwrite (Morph Loader Pro parity) (REQ-039)
530 sim-to-corrective baking: pose body at sampled extreme poses, run soft-tissue solver settle, bake skinned-vs-simulated delta as RBF corrective (the JCM leapfrog; per-body machine-generated correctives)
531 per-body corrective re-bake: correctives regenerate for CURRENT channel configuration (extreme decoupled combos keep working under pose) (REQ-018)
532 porn-range pose QA corrective set: full thigh spread, deep squat, legs-behind-head, deep arch, hip thrust sampled poses on all acceptance archetypes (PREQ-014)
533 compression/contact shapes: butt-on-surface flatten, thigh-against-torso squash, grip dimples as reusable proximity-driven correctives (PREQ-016)
534 proximity/contact driver: collision-driven bulge/indent activation (penetrator proximity/depth driver foundation for BkGenitals orifice correctives) (PREQ-018/019)
535 corrective export baking: JCM-equivalents baked as pose-space-named blendshapes (jcm_thigh_fwd_90_l) + machine-readable driver sidecar JSON (bone channel -> morph curve) (Part-3 contract, GREQ-026)
536 UE Pose Driver + Blender shape-key-driver generator emission from the driver sidecar (Diffeo Auto-JCM precedent) (GREQ-026/032)
537 corrective validation: BODY_POSE_RANGE_VALID descriptor proving archetype x QA-pose matrix renders without collapse/pinch/interpenetration

### BkSoftTissue — MT-538..545 (8) — spec 13.20 (unified-solver soft body) [REQ-028, PREQ-017/020, GREQ-025]
538 body soft-tissue regions in tailor-solver: volume-preserving XPBD soft-body on low-res region cages (breasts/glutes/belly) driving surface via cage deform — same solver world as cloth (SoftBodySpec extension)
539 firmness-channel-driven soft-tissue params: natural-soft vs implant-rigid motion derived mechanically from breast firmness channel (OBR-001 physics link)
540 soft-tissue collision vs hands/props/cloth in the unified collision world (squish on contact) (VaM bar)
541 autocollider generation from region masks + channel state (capsule/sphere sets per region, updates with morphs) (GREQ-025; feeds Cloth proxies)
542 bake-to-keyframes: soft-tissue sim results baked to jiggle-bone keyframe curves for export/render (PREQ-020)
543 gravity-direction rest-shape solve (ptosis interacts with pose: lying/leaning rest shapes) + counter-gravity neutralization for export-neutral shapes
544 soft-tissue determinism envelope + settle gate (reuse MeshComparator/settlement policy)
545 BkSoftTissue integration test: archetype (a) petite+extreme breasts jiggle settle deterministic; squish vs prop proven

### BkGenitals — MT-546..561 (16) — spec 13.21 (genital modules) [REQ-022/029..035, PREQ-010..013/019]
546 genital region topology: vulva/penis/anus regions in BASE topology (single-mesh, no seam) with toggleable resolution + censor toggle (PREQ-011)
547 region-graft mechanism for third-party extensions: vertex_pairs weld map + hidden_polys region replace (DSON-graft-equivalent, Handshake-native) (REQ-029)
548 vulva channel group: labia size/shape/spread, clitoris size/hood, mons volume, inner/outer labia balance (REQ-031)
549 vulva rig + gape system: rigged labia/vaginal canal; open/close/gape dials incl. extreme gape; canal depth channels (REQ-031 Golden-Palace bar)
550 penis module mesh + rig: full pose chain base->glans, scrotum rig (REQ-030)
551 penis channel group: length/girth/curve/taper, glans size, foreskin state continuum, vein intensity (displacement-driven), scrotum size/tightness — oversized ranges per multi-key tracks (REQ-030, operator oversized-penis requirement)
552 erection continuum: flaccid<->erect as pose+morph composite; state machine (rigid/floppy per-scene switch) (REQ-032, SOS pattern, GREQ-034)
553 anus module: position, gape states, pucker detail channels (PREQ-012)
554 futa/mixed anatomy: female body + penis module coexistence with no cross-module conflicts; channel namespaces isolated (REQ-035)
555 genital JCM/corrective coverage: erection sweep, insertion poses, extreme spreads via BkCorrectives engine incl. per-size re-bake (REQ-022)
556 orifice penetration correctives: stretch/conform morphs driven by penetrator proximity/depth via MT-534 driver (vaginal/anal/oral) (PREQ-019)
557 genital collision proxies + soft-tissue coupling for insertion scenes (REQ-034)
558 seamless genital skin: shared UV tile + automatic albedo/normal boundary blending sampled from body skin (works on ALL skin tones) (REQ-033, PREQ-010)
559 arousal controller: single channel driving flush/engorgement color (hemoglobin map), wetness ramp, nipple erection, labia engorgement (PREQ-013)
560 genital modular export attachments: separate mesh+bones+material option with body-seam weld data for game lanes (GREQ-033)
561 BkGenitals integration test: archetypes (f)/(g) muscular+slender male oversized penis erection sweep + archetype (a) vulva gape range; visual QA captures

### BkSkin — MT-562..575 (14) — spec 13.22 (skin/texture system) [REQ-036..038, PREQ-001..009/025..028/034]
562 UV layout authority: UDIM tiles (head/torso/limbs/genitals) published as stable target; per-tile texel priority (genital/areola hero density) (PREQ-007/027)
563 skin material system: PBR slot set + SSS params (Cycles random-walk reference + UE subsurface-profile mapping values) exported per lane (PREQ-005/006)
564 layered texture stack (LIE-equivalent): decals/tattoos/makeup layers with masks + blend modes, non-destructive, bake-on-export flatten (REQ-038, PREQ-034)
565 biophysical tone parameters: melanin/hemoglobin/specular utility maps deriving albedo (tone change re-derives, not tints) (PREQ-004)
566 three-band displacement: secondary/tertiary/micro channel-packed with per-band weights (PREQ-001)
567 micro/detail normal layer with region masks (PREQ-002) + cavity/spec-occlusion support (PREQ-003)
568 wet layer stack: sweat/oil/water clearcoat-style lobes, mask-driven coverage, region + vertex-paint placement (PREQ-008/009)
569 dynamic body wrinkle/fold maps: pose-driven tension/compression maps (neck folds, waist compression) (PREQ-028, CC 4.2 pattern)
570 UV strain compensation: detect texel stretch under extreme channels -> corrected detail maps or per-channel UV adjustment (REQ-036, PREQ-025)
571 texture-follows-morph re-projection: areola/tan-line feature re-projection under drastic proportion change (PREQ-026 differentiator)
572 HD multi-res detail lane: open multi-res deltas on SubD cage (MT-487) + vector-displacement/normal bake at export (REQ-037/040 open HD authoring)
573 skin bake pipeline: full stack flatten to portable PBR sets per export profile w/ per-target budgets (GREQ-040, PREQ-034)
574 body/pubic hair region groom hooks: density masks (pubic/chest/arms/brows) + groom attachment metadata surviving morphs (PREQ-021/022; strand authoring itself deferred to a future groom WP — record deferral)
575 BkSkin integration test: tone sweep incl. dark skins with seamless genital blend; wet layer bake; extreme-morph strain compensation proven on archetype (a)

### BkFace — MT-576..583 (8) — spec 13.23 (face/expression) [PREQ-029/030, AREQ-007..010]
576 face channel set: brow/eye/nose/mouth/jaw/cheek region channels (decoupled, same channel architecture) + face bones hybrid rig
577 ARKit-52 blendshape set authored as the guaranteed baseline; FACS-style extended set mapped to/from ARKit (AREQ-007)
578 expression library: arousal acting extremes (ahegao, bitten lip, eye-roll, open-mouth O, gasp/moan faces) layerable over identity channels (PREQ-030)
579 eye realism stack: cornea refraction geometry, wetness meniscus mesh, eye control channels (PREQ-029)
580 expression correctives: extreme face morphs re-solve canned expressions per-face (CC Facial-Profile pattern)
581 viseme track ingest hook: editable viseme timeline type + ARKit-mapped mouth shapes (audio lip-sync solver itself deferred: stub + typed contract only) (AREQ-009/010)
582 face export: ARKit-52 names on every skeletal export; FACS extended optional; expression bake verification in Blender/UE (Daz FACS-export-gap fix)
583 BkFace integration test: ARKit set drives archetype faces in glTF round-trip; arousal expressions layered with visemes

### BkClothBridge — MT-584..593 (10) — spec 13.24 (BodyKit<->Cloth contract) [REQ-023..027, SREQ-013/014]
584 parametric avatar writer: BodyKit publishes tailor_avatars rows (source_kind='parametric', morph_params_json=channel state) + 25-measurement extraction from generated mesh (closes the coverage-map hole)
585 channel-derived collision proxies: capsule/sphere/SDF body proxies generated mechanically from channel state via MT-541 autocolliders; multi-sphere breast decomposition driven by breast channels; proxy rows auto-update on channel change (REQ-023)
586 grow-into-garment fitting: sim while dialing channels neutral->target (native dForce-timeline-trick) as a RefitMode extension (REQ-024)
587 volume-aware garment auto-follow: garment inherits body channel changes via volume-aware projection (not closest-point); no spikes between/under extreme breasts (REQ-025)
588 garment fix dials: auto-generated per-region ease/expand adjustment channels on fitted garments (Fit Control native) (REQ-027)
589 rigidity masks honored across projection AND sim for garment hardware (REQ-026)
590 fit-across-morph-space: garments store fit targets over channel-space samples, auto-interpolated on any body (MetaHuman adaptive-clothing / BodySlide conform contract) (SREQ-013)
591 conform-wardrobe batch: one-click refit outfit library to a body (batch jobs, parallel lanes) (SREQ-014)
592 pose-driven drape QA hook: archetype bodies x starter garments drape matrix wired into validation
593 BkClothBridge integration test: archetype (a) petite+extreme breasts wearing starter bra+dress: proxy generation, drape, refit after channel change, fix dials, zero interpenetration

### BkModelApi — MT-594..607 (14) — spec 13.25 (model-first body authoring) [REQ-043, SREQ-001..006/019/020]
594 BodyModelAdapter: ModelAdapter impl accepting ContextBundle w/ BodySpec + NL constraints -> BodySpec proposal (TAI-OVR-001 parity for bodies)
595 author_body MCP tool (NL description -> BodySpec draft; sandbox->validate->promote lifecycle, non-bypassable)
596 edit_body MCP tool (RFC 7396 merge patch on BodySpec; CRDT-safe; channel isolation preserved)
597 get_body MCP tool (BodySpec + latest BodyValidationReceipt + capture refs)
598 solve_measurements MCP tool (numeric targets -> channel solution via MT-512)
599 randomize_body MCP tool (scoped randomization w/ seed; batch variation)
600 pose_body MCP tool (NL/reference-image pose -> BodyPose via IK; reuse MT-399/404 lane)
601 configure_genitals MCP tool (typed genital config incl. erection state, gape, arousal controller)
602 export_body MCP tool (profile-driven export trigger returning manifest + validation report)
603 BodyValidationReceipt: typed feedback (decouple violations, topology, pose-range, export budgets) with suggested_fix JSON pointers (self-correction loop)
604 body visual capture integration: capture_view on body viewport + model annotation verdicts (reuse tailor_captures)
605 recipe system: BodyRecipe save/load/diff/partial-apply ("take her breasts, keep my face"), separable shape/skin/genital/physics layers (SREQ-001/002/019)
606 recipe share format: PNG-embedded recipe card + dependency manifest + license/adult-content metadata + compat version rules (SREQ-004/005/006/022)
607 parallel swarm proof: N agents author bodies concurrently with leases/attribution/cancellation (HBR-SWARM parity with garment lane)

### BkExport — MT-608..625 (18) — spec 13.26 (body export lanes) [Part-3 contract, GREQ-001..016/029..035/039/040]
608 export profile system: per-target profiles (blender_gltf, blender_fbx_bridge, ue_fbx_bridge, ue_gltf, vrm, usd, obj) as typed BXP- rows w/ units/axis contracts (cm/Z-up UE, glTF meters)
609 skeletal mesh export core: skinned body + selected morph targets (morph-export-rules list) via glTF writer (primary lane)
610 FBX delivery via Blender-bridge (reuse MT-419 bridge pattern; no native FBX writer) for body skeletal lane incl. blendshapes
611 corrective/driver sidecar emission per export (MT-535) + generator scripts packaging (MT-536)
612 baked erection/gape as both pose assets and blendshapes in exports (REQ-032)
613 pre-welded single-mesh export option: genitals merged, blended textures baked, one material set (Diffeo merge parity) (Part-3)
614 UE profile: Manny/Quinn-compatible naming + ik_* bone set + IK Rig/Retargeter asset JSON emission (GREQ-005/006) + T0/A-pose reference handling (Daz-bridge pain fix)
615 Unity/VRM profiles: humanoid mapping, blendshape clips, spring-bone config from physics sidecar, license metadata embedded (GREQ-029/030)
616 USD body export (UsdSkel skeleton + blendshapes) behind usd-export feature (strategic lane)
617 LOD chain generation: decimation with skinning preserved + per-LOD bone reduction + morph stripping schedule (GREQ-001..003)
618 skinning influence caps per profile (4/8/12/unlimited) with clamp+renormalize (GREQ-004)
619 high->low bake pipeline: normal/AO/curvature/displacement from SubD+HD to export mesh with bake groups (GREQ-008) + baked morph/wrinkle normal maps option (GREQ-009)
620 UV strategy per profile: UDIM offline vs single-tile atlas repack + material merge for game (GREQ-010/031)
621 texture conformance per profile: MikkTSpace handedness, green-channel flip, sRGB/linear tags, channel packing (ORM), per-platform budgets w/ auto-downres (GREQ-039/040)
622 export validation report: machine-readable per-target checklist (influences, blendshape counts, bones, materials, textures, LODs) EXPORT_BODY_VALID (GREQ-035)
623 DSON read-only import lane: user-owned Daz character migration (channel mapping proposals from .duf/.dsf morph dials; NO .dhdm; NO Genesis mesh payload; license guard MT-488 enforced) (format-strategy P2)
624 Blender/UE round-trip verification harness: headless Blender + UE-import check scripts verifying morphs/skeleton/correctives/scale/orientation survive (Daz-bridge pain list as acceptance tests) (REQ-044)
625 BkExport integration test: archetype set exported through all profiles; validation reports green; round-trip verified

### BkGui — MT-626..635 (10) — spec 13.16 GUI clauses + HBR-VIS [CX-503D1]
626 body editor pane registration in native shell (pane type 'tailor' extension: BodyKit tab) with stable author_ids on all controls
627 channel slider panels grouped by region w/ numeric entry, over-range indication, isolate-region toggle
628 archetype/recipe browser panel (thumbnails, tags, apply/blend)
629 measurement panel: live measurement readout + numeric targeting UI (MT-512 surface)
630 coupling-graph inspector panel: view/disable/author explicit links (MT-496 surface)
631 pose editor UI hooks: IK pin handles, pose library browser, multi-actor alignment display
632 genital config panel (channels, erection state, arousal controller) — same-grain sliders, no special-casing
633 body viewport integration: reuse Tailor wgpu viewport with body render pipelines (skin preview shading, heatmap overlays: weights/strain/decouple-violations)
634 Argus coverage: inspect/steer/screenshot all BodyKit panes headlessly; capture matrix incl. archetype edge states (HBR-VIS)
635 BkGui integration test: no-context model drives channel edit -> capture -> verify via Argus only

### BkBodiesQA — MT-636..645 (10) — spec 13.16 acceptance archetypes [OBR implications 5, REQ-044]
636 archetype seeding framework: seeded archetype bodies as BRC- recipes (deterministic from channel values)
637 archetype (a): petite frame + extreme-large perky natural breasts + narrow soft shoulders + narrow hips + small hands + long legs + round ass + narrow thighs + skinny midriff
638 archetype (b): extreme-large fake/plastic round implant look on slender frame; archetype (c): small perky athletic
639 archetype (d): large natural droopy on fat/obese female; archetype (e): obese male
640 archetype (f): muscular male + oversized penis; archetype (g): slender male + oversized penis; archetype (h): futa cross-blend
641 decouple-proof validation suite: every archetype proves zero cross-region coupling (BODY_DECOUPLE_VALID matrix run)
642 pose-range QA matrix: archetypes x porn-range QA poses (MT-532) x correctives -> BODY_POSE_RANGE_VALID
643 drape QA matrix: archetypes x starter garments via BkClothBridge -> FIT/NO_INTERPENETRATION gates
644 export QA matrix: archetypes x export profiles -> EXPORT_BODY_VALID + round-trip verification
645 visual QA captures: Argus capture set per archetype (front/side/pose extremes) persisted as CAP- rows for validator + operator review

### BkGovernance — MT-646..653 (8) — spec 13.16/13.27 (validation/HBR/manual)
646 BodyKit ValidationDescriptor catalog registration (BODY_TOPOLOGY_VALID, BODY_DECOUPLE_VALID, BODY_POSE_RANGE_VALID, EXPORT_BODY_VALID, GENITAL_BLEND_VALID, LICENSE_GUARD...) wired into the 13.15 runner + stages
647 BodyKit PromotionGate binding: body/recipe promotion via MeshComparator-class equivalence + CPROM idempotency; no self-approval
648 BodyKit CRDT: channel edits as CRDT ops (LWW per channel), competing body proposals as drafts, ai_edit_proposal lane
649 BodyKit sandbox binding: model-authored bodies run generation/validation in kernel sandbox (process tier, fs-only caps)
650 BodyKit job/scheduler integration: generation/bake/export as Handshake JOBS with leases/backpressure/cancellation; Flight Recorder lifecycle events
651 UserManual: BodyKit chapter (purpose, channel workflow, genital config, export profiles, recipes, MCP tools, failure/recovery, diagnostics posture) per HBR-MAN + CX-982
652 HBR matrix hydration for all BodyKit MTs (INT/SWARM/VIS/QUIET/MAN/STOP rows) + hbr-matrix-check green
653 BodyKit end-to-end lifecycle test: NL description -> author_body -> channels -> genitals -> correctives -> garment drape -> export -> replay from ledger (the full loop, no-context provable)

## Part C — Spec v02.198 edit plan (Section 13)

1. 13.1 amendment: Tailor = one creative module with TWO submodules — Cloth (13.1-13.15) and BodyKit (13.16-13.27); naming law (Cloth*/Body* physics types, Tailor* domain identifiers unchanged); BodyKit build-order note (attaches to same kernel gates; BodyKit implementation may interleave with Cloth groups after BkFoundation).
2. Cloth reconciliation edits (from coverage-map contradictions): 13.9 §13 deferred list revised (UV-space bake, all-quad, toon now IN scope via MT-380/385/390 — feature-gated); 13.10 DEFER-ANIM-001 revised (per-tack TackStrength tracks in scope via MT-412); 13.10/13.12 FBX contradiction resolved (FBX delivery = Blender-bridge bake lane ONLY; native FBX writer remains prohibited; R-ANIM-039 maps to bridge path); 13.14 event list additions (TailorAvatarPoseGenerated, TailorAnimationImported, TailorTopstitchOptimized, TailorCaptureAnnotated, TAILOR_BODY_* family); new CG-feature normative clauses added to owning subsections (holes, turned seams, sublayer, fold arrangement, interactive session, region stiffness overrides, print repeat, per-face materials, tape measures, cache post-edit, video capture, garment skin-weight export, VAT).
3. New subsections 13.16-13.27 (BodyKit normative law): 13.16 overview/architecture/authority/archetypes; 13.17 channels; 13.18 skeleton; 13.19 correctives; 13.20 soft tissue; 13.21 genitals; 13.22 skin; 13.23 face; 13.24 Cloth bridge; 13.25 model-first API; 13.26 export; 13.27 validation/HBR. Each subsection carries [TAI-BK-*] anchors, canonical types/DDL, event additions, MCP tool contracts, prohibitions (no SMPL/Genesis/dhdm; no native FBX write; PostgreSQL-only).
4. Pointer/manifest updates: copy-first v02.197 -> v02.198; manifest hashes/line counts; INDEX.json; spec-changelog.jsonl entry; SPEC_CURRENT.md (+previous_current_spec); archive v02.197 to spec_archive; EOF appendices (12): feature registry rows (Tailor::BodyKit + new Cloth features), primitive matrix, UI guidance (BodyKit panes), interaction matrix (BodyKit x Cloth x Atelier x render lanes).

## Part D — Stub expansion plan (WP-KERNEL-010 contract)

Preservation-first additive edits: lifecycle.spec_status -> v02.198 pointer; microtasks block -> total 653, ranges (core 1-332, MD-parity 333-448, second-pass parity 449-477, BodyKit 478-653 by group), composition updated; draft_scope.intent + scope_sketch extended with BodyKit submodule (verbatim-intent operator requirements OBR-001..004 referenced); KEY_OPEN_QUESTIONS extended (Q10 base-mesh commissioning path, Q11 groom WP split, Q12 lip-sync solver deferral); acceptance criteria extended (BodyKit activation ACs incl. archetype gates); research_basis extended with tailor_bodykit_research package; red_team additions (license guards, decouple enforcement, genital-blend on all skin tones, export round-trip); build_order note (BodyKit groups gate ordering); hbr_directive extended to BodyKit surfaces.
