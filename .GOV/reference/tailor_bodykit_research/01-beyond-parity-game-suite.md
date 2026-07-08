---
file_id: tailor-bodykit-research-01-beyond-parity-game-suite
topic_id: T-BK-BEYOND-PARITY
title: "Beyond-Parity Requirements: Game-Ready Pipeline, Photoreal Adult Rendering, Animation/Retarget, Suite Workflow + Competitive Feature-Mine"
status: non_normative_research
normative_status: non_normative_context_only
research_lane: web-research sub-agent (game/3D avatar suite requirements), session 2026-07-08
context: "Tailor module = Cloth (MD-class garment engine) + BodyKit (Daz-replacement parametric bodies). Adult/porn production today; game + general 3D work later. This lane inventories what a feature-rich avatar+clothing suite needs BEYOND MD/Daz parity."
updated_at: "2026-07-08"
---

# Beyond-Parity Suite Requirements (Competitive Mine + GREQ/PREQ/AREQ/SREQ)

## Competitive feature-mine

### Reallusion Character Creator 4/5 + iClone 8

| Feature | What it does | Why Tailor/BodyKit should care |
|---|---|---|
| SkinGen dynamic material system | Layered, parametric skin synthesis: muscles, wrinkles, pores, color variation, scalp, body hair, nails, blemishes, tattoos, suntan/wound/scar overlays; all bakeable to flat textures for export | This is the reference model for a non-destructive layered skin editor; bake-on-export is the key trick that keeps portability to Blender/UE |
| Blendable body morph sliders incl. male/female cross-blend | Body mass, muscle, limb length, scale sliders that stack and blend, including gender blending on one mesh | BodyKit's decoupled region morphs should support unrestricted stacking incl. cross-sex blends (futa/androgynous builds are an adult-market staple) |
| Facial Profile Editor + Edit Expressions | Editable expression morph set per character; standard vs extended profiles; per-slider expression tuning | Expression sets must be editable per-character, not fixed — extreme face morphs break canned expressions |
| Dynamic wrinkle system ("one wrinkle for all avatars", CC 4.2) | Pose/expression-driven wrinkle normal maps applied engine-side, shared across avatars | Dynamic tension/compression wrinkles are cheap realism for both face and body (skin folding at waist/thighs in extreme poses) |
| 140 blendshapes from 4D scan (CC4 face) | Scan-derived facial blendshape set for mocap, puppeteering, AccuLips lip-sync | Sets the bar for face-shape counts; scan-derived shapes read more real than hand-sculpted |
| GoZ Plus ZBrush round-trip | Direct mesh+texture round-trip with subdivision handling, diffuse→polypaint and normal→displacement conversion, UDIM bake-back | A sculpt round-trip (to Blender sculpt mode or ZBrush) is mandatory for a body-authoring tool; artists will always want to hand-fix morphs |
| InstaLOD integration (remesher, poly reduction, material merge) | In-app LOD chains, remeshing, texture/material merging producing animation-ready optimized characters | Ship game-export with built-in decimation+atlas — users should not need a second tool |
| Auto Setup plugins for UE/Unity | One-click transfer of characters with shaders/params reconstructed engine-side | Export is not FBX-dump; it is a per-engine companion plugin that rebuilds materials — plan the same for Blender/UE |
| Headshot 2/3 (photo→3D head, AI) | Proprietary AI reconstructs a rigged CC head from one photo; mesh-to-CC wrapping for scans | Photo-to-morph identity capture is an expected on-ramp; also the wrap-scan-to-basemesh path (see Faceform) |
| AccuLips + Audio2Face plugin | Audio→viseme timeline, editable per-viseme; optional NVIDIA A2F neural facial animation ingest | Lip-sync must land as editable data (viseme track), not baked vertex animation |
| Motion Director + Motion LIVE | Gamepad-driven locomotion authoring; unified body/face/hand mocap hub with per-device plugins | A mocap hub abstraction (device → retarget profile → rig) is the right architecture for ingesting anything |
| iClone drag-drop FBX motion retarget | Any FBX motion auto/custom retargets onto CC rig | Frictionless "drop a motion file on the character" is the usability bar |

### Epic MetaHuman (Creator / DNA / Animator / 5.6)

| Feature | What it does | Why Tailor/BodyKit should care |
|---|---|---|
| 2025 licensing change | MetaHumans now usable in any engine/DCC, sellable on FAB and third-party stores; Creator integrated in UE 5.6; free Maya and Houdini plugins | MetaHuman is now a direct competitor in Blender pipelines, not just UE; also validates open-license character ecosystems as a market strategy |
| Parametric bodies (UE 5.6) | Continuous height/proportion/measurement control (chest, waist, leg length), sculpt+blend on a body space learned from real scan data; measurement constraints | Scan-statistics-backed parametric space plus explicit measurement constraints (e.g., "make waist 60cm") is beyond Daz — BodyKit should support numeric measurement targeting |
| Adaptive clothing / Outfit Asset | Garments pre-authored across a body-shape range, auto-interpolated to fit any body; resize is automatic | This is the Cloth↔BodyKit contract: garments must carry fit data across the whole morph space, not one base shape |
| MetaHuman DNA format | Single file storing rig definition, behavior (rig logic), geometry, skin weights, blendshape deltas, per-LOD mappings; DNACalib API to edit (rename/remove joints, move neutrals, edit deltas) programmatically | The strongest prior art for a "character DNA" save format with a public C++/Python edit API — BodyKit's character format should be similarly complete and script-editable |
| MetaHuman Animator | Offline high-fidelity facial solve from iPhone depth video/webcam onto MetaHuman rig curves | Facial capture ingest should be a solve-to-rig-curves pipeline, editable afterwards |
| Control Rig + face board | Full-body and facial control rigs evaluated in-engine; retarget-friendly | Export must preserve enough structure that engine-side control rigs can be auto-generated |
| Groom pipeline | Strand grooms with LOD to cards/meshes, groom binding to skelmesh, Hair Card Generator plugin | Hair should author once as strands and degrade gracefully to cards |
| Mesh to MetaHuman | Wrap any scanned/sculpted head to the MetaHuman topology and rig | Topology-locked wrapping of arbitrary input meshes is the scan on-ramp |

### Blender HumGen3D

| Feature | What it does | Why Tailor/BodyKit should care |
|---|---|---|
| One-addon full pipeline | Body, face (100+ adjustments), skin, hair, clothing, pose, expression in a single non-destructive staged workflow | Validates the "one tool, staged workflow" UX for indie/solo users — Handshake's target user |
| Randomize-all + per-slider randomize | One-click plausible random humans; per-parameter dice | Cheap feature, huge for batch content: BodyKit needs seeded randomization at both macro and per-region level |
| Batch generator | Generate many humans with controlled variation; smaller batches improve diversity | Batch body variation is a stated Handshake need |
| Python API (Human class) | Full scripted control of creation/settings | Automation-first API is required for LLM/agent-driven workflows |
| Height slider with real-unit scaling | Height in cm that rescales rig correctly | Real-world units matter for garment grading in Cloth |
| Custom starting humans | Save your own base as a preset seed | Recipe/preset seeding pattern |

### MB-Lab / MakeHuman

| Feature | What it does | Why Tailor/BodyKit should care |
|---|---|---|
| Age/mass/tone macro sliders | ~90% of character defined by 3 macro sliders (age 18-80, mass, tone) driving ~470 morphs | Macro→micro slider cascade is the proven parametric architecture; BodyKit macro sliders should drive its region morph stacks |
| ~470 anatomical morphs | Parametric coverage of anatomical range for body, face, expressions | A concrete floor for morph-library breadth |
| Phenotype classes | Preset population phenotypes blended with morphs | Population/ethnicity presets as first-class blendable assets |
| Finished with proxies, skin/eye shaders, poses | Post-parametric finishing tools inside the same tool | The parametric stage must hand off cleanly to shading/posing stages |
| MakeHuman standalone + morph targeting tech | Standalone app, morph-target-based, controls from gender down to finger length | Cautionary tale: standalone-without-pipeline dies; MPFB (its Blender rebirth) proves in-DCC integration wins |

### VRoid Studio

| Feature | What it does | Why Tailor/BodyKit should care |
|---|---|---|
| Slider-based anime avatar editor | Every preset item (face, hair, outfit) editable via sliders + direct texture painting on UVs in real time | Live texture painting on the character inside the character tool removes a whole DCC round-trip |
| Stroke-based hair authoring | Draw hair chunks with a stroke; per-chunk bounce/physics parameters | Fast stylized-hair authoring lane; per-chunk physics params exportable |
| VRM 0.0/1.0 export | Standardized humanoid avatar format with materials, blendshapes, spring bones, license metadata embedded | VRM is the interchange to study for a self-describing avatar format incl. embedded usage-license metadata |

### Faceform Wrap (R3DS) / Wrap4D

| Feature | What it does | Why Tailor/BodyKit should care |
|---|---|---|
| Non-rigid wrapping of basemesh onto scans | Converts scan sets to one consistent topology; node-graph so one setup batch-processes many scans | The scan on-ramp: BodyKit basemesh must be wrappable onto photogrammetry scans (incl. genital scans) to mine real anatomy |
| Texture/detail transfer | Transfers Texturing.xyz / 3D Scan Store microdetail maps onto custom topology | Enables scan-grade skin on BodyKit's own UV layout |
| Blendshape/animation retarget nodes (2025.11) | Retarget blendshapes and facial animation between characters; stretch/compression masks | Cross-character morph retargeting is exactly what "one morph library, many bodies" needs |
| Wrap4D | 4D (per-frame) scan sequence processing for facial performance | Future lane for scan-driven corrective shapes |

### Texturing.xyz / 3D Scan Store ecosystem

| Feature | What it does | Why Tailor/BodyKit should care |
|---|---|---|
| Multi-channel displacement (RGB = secondary/tertiary/micro) | Channel-packed displacement lets artists re-balance detail frequency in-shader | BodyKit's skin shader should expose 3-band detail weighting, not one displacement slider |
| Utility maps (hemoglobin, melanin, specular) | Biophysical control maps for skin shading | Parametric skin tone via melanin/hemoglobin maps beats RGB-tinting albedo |
| Pre-unwrapped ear-to-ear captures, true (non-baked) detail | Artifact-free full-face texture sources independent of polycount | Source-quality standard for bundled skin content |
| Body-part patches (legs, large skin patches) + SKAP (2026) | Fill/continuity microdetail for full bodies | Full-body microdetail coverage — most competitors only do faces |
| 3D Scan Store MetaHuman texture sets | Scan textures retargeted to a known character topology/UV as a product | Selling textures against a fixed public UV layout creates an asset economy — publish BodyKit's UV layout as a stable target |

### ZBrush + Substance Painter (workflow roles)

| Feature | What it does | Why Tailor/BodyKit should care |
|---|---|---|
| ZBrush subdivision/HD sculpt + multi-map bake | Sculpt tertiary detail at high subdiv; bake diffuse/normal/displacement/cavity per subdiv level | BodyKit must export/import at multiple subdiv levels with map baking, or artists can't inject hand detail |
| ZBrush layers | Non-destructive sculpt layers usable as morph sources | Sculpt-layer→morph-target conversion is the artist path for custom region morphs |
| Substance Painter character texturing | Bakes (AO/curvature/normal) drive smart materials; UDIM painting; CC5 exports OBJ+metadata Painter reads directly | Publish a Painter export template (channels, UDIMs, naming) so texturing round-trips cleanly |

### Virt-A-Mate (VaM)

| Feature | What it does | Why Tailor/BodyKit should care |
|---|---|---|
| Runtime soft-body with autocolliders | Breast/glute soft-body reacting to environment (butt flattens on a bed); auto-generated collider chains | The de-facto quality bar for interactive adult soft-body; autocollider generation from mesh regions is a stealable idea |
| Everything-physics scene graph | Hair, cloth, body all physics-enabled and interactable | Adult users expect poke/press/collide interactivity, not just render-time sim |
| Looks / appearance presets / skin presets | Character appearance decomposed into stackable preset layers | Preset layering (shape vs skin vs outfit) mirrors the recipe architecture BodyKit needs |
| .var package + Hub with dependency resolution | Community content packaged with dependencies; "all required files included" filter | Dependency-aware asset packaging is essential for a morph/garment ecosystem (Daz's weakest point) |
| Community morph packs on G2-derived mesh | Thousands of user morphs incl. genital morphs | Proof of demand for an open morph SDK on a fixed topology |

### Koikatsu / Honey Select (Illusion games)

| Feature | What it does | Why Tailor/BodyKit should care |
|---|---|---|
| PNG character card | Entire character (shape, outfit, mods used) steganographically embedded in a shareable PNG; drag-drop to load | Best-in-class shareability: a BodyKit recipe embedded in its own preview render would be a killer distribution format |
| Card databases (BepisDB, koikatsucards) | Searchable community card ecosystems | Character-as-single-file enables third-party marketplaces for free |
| Cross-title card compatibility rules | HS2 reads AI Shoujo cards but not Koikatsu cards | Version/compat metadata must be in the recipe format from day 1 |

### Skyrim adult modding ecosystem (BodySlide/CBBE/3BA/SOS/OStim)

| Feature | What it does | Why Tailor/BodyKit should care |
|---|---|---|
| BodySlide sliders + 0/100 weight interpolation | User-authored slider sets morph body AND all conformed outfits; outfits rebuilt against custom body | 15+ years of proof that body-morph-plus-garment-refit is the feature adult users organize around; Outfit Studio's "conform outfit to slider set" = Cloth's auto-refit contract |
| RaceMenu Build Morphs | In-game live morphing without rebuilding meshes | Runtime morph deltas exported alongside baked meshes |
| Zap sliders | Sliders that delete garment components (bags, skirts) | Garment part toggles belong in the garment format |
| SOS genital rig | Bone-chain rigged penis with scale spells, per-race/character assignment, CBPC soft physics, rigid/floppy state switching per scene | Concrete spec for penis rigging: dedicated bone chain, erection state machine, physics toggle, per-character size — BodyKit should ship this natively instead of leaving it to modders |
| CBPC/HDT-SMP collision physics | Config-driven breast/butt/genital collision physics on vanilla-ish skeletons | Data-driven physics config (per-body-region tuning files) beats hardcoded sim params |
| OStim/SexLab animation frameworks | Sex-scene frameworks consuming community animation packs with alignment/mapping metadata | Export must preserve skeleton conventions + root alignment so animation packs stay interchangeable |

### UE Mutable + VRChat PhysBones (runtime customization prior art)

| Feature | What it does | Why Tailor/BodyKit should care |
|---|---|---|
| UE Mutable (5.5, beta) | Runtime generation of skeletal meshes/materials/textures for customization systems; real-time morphs; removes hidden geometry under clothing; optimizes draw calls/memory | If Handshake characters target games, exporting Mutable-compatible customizable objects (or equivalent data) future-proofs runtime customization; hidden-flesh removal under garments is also a render-time win |
| VRChat PhysBones | Free optimized secondary-motion bones with squash/stretch, grab/pose interaction, 256-transform limit per component | Community-standard jiggle spec incl. user interaction; hard per-component budgets are a good export lint rule |

## Game-ready pipeline requirements

1. GREQ-001: Auto-LOD chain generation for skeletal meshes with skinning weights preserved through decimation (InstaLOD/Simplygon behavior) — LODs that break deformation are useless.
2. GREQ-002: Bone-reduction per LOD (weld minor bones, influence-threshold pruning) with morph-target transfer to reduced LODs — Simplygon exposes both; export should too.
3. GREQ-003: Morph-target stripping per LOD (keep face shapes on LOD0, drop on LOD2+) — standard UE practice for memory.
4. GREQ-004: Configurable max skinning influences per vertex at export: 4 (legacy/mobile), 8 (UE default path), 12 (UE practical cap), unlimited flag for UE Unlimited Bone Influences — engines differ and the exporter must clamp+renormalize.
5. GREQ-005: Target-skeleton export profiles: UE5 Manny/Quinn-compatible skeleton option so retargeting is near one-click in UE 5.4+ — custom skeletons are fine but a UE-native profile removes friction.
6. GREQ-006: Ship IK Rig definition data (chain definitions: spine/arms/legs/root) with exports so UE auto-retargeting and IK Retargeter setup is automatic.
7. GREQ-007: A-pose/T-pose reference pose selection at export with retarget base pose stored — pose mismatch is the #1 retarget failure.
8. GREQ-008: High→low poly baking built in: normal, AO, curvature, displacement, position/thickness from the subdiv/HD body to the game mesh, with bake groups to prevent cross-projection (Marmoset pattern).
9. GREQ-009: Baked morph normal maps / wrinkle maps driven by pose+expression (CC 4.2 style) for real-time skin folding without geometry cost.
10. GREQ-010: UV strategy switch per export target: UDIM set for offline render, single-tile repack (auto atlas + material merge) for game export — polycount consensus: UDIMs for heroes/offline, 0-1 for game assets.
11. GREQ-011: Texel-density tooling: display/enforce px/cm targets (industry anchors: ~512 px/m background, ~1024 px/m hero) and warn on inconsistent density across body/garments.
12. GREQ-012: Blendshape budget linting: warn above ~50 shape keys per real-time mesh, flag >30 when face is merged into body mesh, recommend GPU morph path >10k verts — matches UE community guidance.
13. GREQ-013: Export ARKit-52 blendshape set as the guaranteed facial baseline on every game export (de-facto interchange, drives Live Link Face/Perfect Sync/most AI mocap).
14. GREQ-014: Nanite-aware export: know that Nanite skeletal meshes (stable UE 5.5+) exclude morph targets and Chaos Cloth — offer a "Nanite body + non-Nanite morphing face" split or warn.
15. GREQ-015: Strand groom export as Alembic (.abc) with guides + groom binding target info — Alembic is the only true strand path into UE.
16. GREQ-016: Automatic hair-card generation from strand grooms (UE Hair Card Generator equivalent) plus card/mesh LOD chain — strands don't run on all platforms.
17. GREQ-017: Per-groom LOD schedule (strands→cards→mesh) with curve/point decimation settings — groom cost scales with curve count in every UE pass.
18. GREQ-018: Chaos Cloth export path: garment as USD with panel/seam data, per-panel fabric parameters, and weight maps (max distance etc.) matching the UE 5.4+ Panel Cloth Editor dataflow ingest — MD already proves USD is the carrier.
19. GREQ-019: Skin-weight transfer from body to garment at export (garment skinned to body skeleton for non-simulated wear) — MD EveryWear pattern.
20. GREQ-020: Triangle-mesh option for cloth exports (UE Chaos prefers tris; no quad conversion needed) while keeping quads for DCC targets.
21. GREQ-021: Baked-cloth fallback: vertex animation texture (VAT) export of simulated garments/flesh for GPU playback where runtime sim is too costly; must support looping and document VAT limits (texture memory, no collisions).
22. GREQ-022: Alembic cache export of any simulation (cloth, soft body) for engines/DCCs that ingest caches rather than sims.
23. GREQ-023: Jiggle-bone authoring: optional secondary-motion bone chains (breasts L/R, glutes, belly, penis, testicles, hair, ears/tail) generated from body regions with per-chain params — consumed by Kawaii Physics (UE), PhysBones (VRChat), JigglePhysics/MagicaCloth (Unity), CBPC-style configs.
24. GREQ-024: Physics-config sidecar export: per-region stiffness/damping/gravity/limits in engine-agnostic JSON so each engine plugin maps it (VRChat 256-transform-per-component style budget checks included).
25. GREQ-025: Collision-proxy generation: capsule/sphere autocollider sets for body regions (VaM autocollider pattern) exported with the skeleton for cloth/hair/jiggle collision.
26. GREQ-026: Corrective-shape export as engine-consumable pose-space data: UE Pose Driver / Pose Driver Connect (RBF solver, JSON+FBX poses) so JCM-equivalents run in-engine instead of dying at export.
27. GREQ-027: ML Deformer training-data export (Alembic ground-truth + FBX poses per UE spec) for teams that want neural muscle/flesh/cloth approximation in UE — Fortnite-proven path.
28. GREQ-028: UE Mutable-compatible modular export (body parts, hidden-geometry removal masks under garments, runtime morph flags) for game character-customization systems. (UNVERIFIED: exact Mutable authoring-data interchange surface — needs plugin-level research.)
29. GREQ-029: Unity export profile: Humanoid-avatar-compatible rig, ≤4 influence default with quality tiers up to 32 (Unity skinWeights), Magica Cloth 2-friendly bone/mesh cloth split.
30. GREQ-030: VRM 0.x/1.0 export (humanoid mapping, blendshape clips, spring bones, license metadata) to reach VTuber/VRChat-adjacent markets.
31. GREQ-031: Material merge/atlas on export (InstaLOD merge-material pattern): N skin/garment materials → 1-2 draw-call-friendly materials with rebaked textures.
32. GREQ-032: Per-engine auto-setup companions (Blender addon, UE plugin) that rebuild shaders/physics from exported metadata — Reallusion Auto Setup proves FBX alone never survives the trip.
33. GREQ-033: Genital meshes as optional modular attachments in game exports (separate mesh+bones, own material) with body-seam weld data — mirrors how the modding ecosystem (SOS) retrofits this, but native.
34. GREQ-034: Erection/arousal state as animation-friendly data: bone-chain pose sets + morphs, not shader tricks, so game logic can drive it (SOS rigid/floppy switching precedent).
35. GREQ-035: Export validation report: per-target checklist (influence caps, blendshape counts, bone counts, material counts, texture sizes, missing LODs) emitted machine-readable — models/agents and CI need it, not just humans.
36. GREQ-036: Skeleton naming-convention presets and bone-map export (Mixamo, Rokoko, UE, Unity, VRM naming) so third-party retargeting tools auto-map.
37. GREQ-037: Twist/roll bone generation (forearm/upper-arm/thigh twist chains) as an export option — engines expect them for deformation without pose-space tools. (UNVERIFIED count conventions per engine.)
38. GREQ-038: Root motion / root bone convention handling (root at origin, hips child) per engine profile — retarget and locomotion break without it.
39. GREQ-039: Texture pipeline conformance: normal-map handedness (MikkTSpace, +Y/-Y green channel flip per engine), sRGB/linear tagging, channel-packing presets (ORM etc.) per target.
40. GREQ-040: Per-platform texture budget presets (e.g., 4K hero PC, 2K console, 1K mobile) with automatic downres on export.

## Photoreal adult-render requirements

1. PREQ-001: Three-band skin displacement (secondary/tertiary/micro channel-packed, Texturing.xyz convention) with per-band weight controls in the bundled skin shader.
2. PREQ-002: Tiling micro-normal/detail-normal layer with region masks (nose oilier, lips glossier) — MetaHuman/UE digital-human standard for pore-level closeups.
3. PREQ-003: Cavity/specular-occlusion map support to sell pores in real-time targets where true displacement is absent.
4. PREQ-004: Biophysical skin-tone parameters (melanin/hemoglobin/specular utility maps) so tone changes re-derive albedo instead of tinting it.
5. PREQ-005: Random-walk SSS as the reference shading target in Cycles (closed manifold body mesh required — BodyKit topology must stay watertight) with per-channel radius defaults tuned for skin (red scatters furthest).
6. PREQ-006: UE parity path: Subsurface Profile material preset with sane defaults (UE default radius 1.2cm) and a documented Cycles↔UE translation of SSS params.
7. PREQ-007: 4K default / 8K optional texture workflow for closeups; UDIM body layouts (head/torso/limbs/genitals tiles) as the offline-render default.
8. PREQ-008: Layered wet-look shader stack: independent sweat/oil/water layers as a clearcoat-style specular lobe (IOR ~1.33, low roughness) with mask-driven coverage and droplet/drip normal detail — adult production staple, must be a toggle not a custom node graph every time.
9. PREQ-009: Wetness masking by region + vertex-paint/attribute so saliva/lube/sweat placement is paintable directly on the body.
10. PREQ-010: Genital meshes shading-integrated with body skin: shared/continuous UV strategy or automatic material copy + blend shell (Golden Palace pattern: geograft auto-copies torso materials, blend shell fades the seam) — seamlessness must be automatic, the Daz ecosystem proves manual matching is the pain point.
11. PREQ-011: Single-mesh genital option (not graft) for BodyKit natives: genitals in base topology with LOD/censor toggles, eliminating the geograft seam problem entirely.
12. PREQ-012: Dedicated genital morph library (labia spread/engorgement, penis girth/length/curve, foreskin state, anus gape states) rigged and shaded to the same standard as the face — this is the category Daz outsources to third parties (Golden Palace/Dicktator ecosystem).
13. PREQ-013: Arousal shading states: flush/engorgement color shifts (hemoglobin-map driven), wetness ramp, nipple erection morphs — coupled to one "arousal" controller.
14. PREQ-014: Corrective-shape (JCM-equivalent) coverage explicitly QA'd on porn-range poses: full thigh spread, deep squat, legs-behind-head, deep spinal arch, hip thrust — Daz JCMs target everyday ranges; adult posing exceeds them.
15. PREQ-015: RBF pose-space deformation as the corrective mechanism (interpolated example poses, ramp-in) rather than single-trigger morphs — avoids popping near pose extremes.
16. PREQ-016: Compression/contact shapes: butt-on-surface flattening, thigh-against-torso squash, grip dimples — authored as reusable correctives plus optional sim.
17. PREQ-017: Soft-body simulation lane in-suite (volume-preserving solver on low-res cage driving surface via surface-deform/mesh-deform) for breast/belly/glute jiggle in renders — current field practice is exactly this cage+modifier workflow.
18. PREQ-018: Contact/penetration deformation faking toolkit: collision-driven bulge/indent (proximity-driven shape keys or lattice), because per-frame full soft-body flesh sim remains impractical; VaM-style autocollider flattening is the interactive reference.
19. PREQ-019: Penetration-specific correctives: orifice stretch/conform morphs driven by penetrator proximity/depth — the modding world does this with bone constraints + morphs; make it a first-class driver. (UNVERIFIED: no single canonical field implementation; pattern synthesized from VaM/Skyrim ecosystems.)
20. PREQ-020: Baked jiggle for stills/video: Wiggle-2-class bone physics with collision + bake-to-keyframes so renders don't require live sim.
21. PREQ-021: Body/pubic hair grooming on the curves system (hair curves, sculpt brushes, clump/frizz/interpolate) with region density masks (pubic, chest, arms, brows) and presets — head-hair tooling exists everywhere; body-hair presets are the gap.
22. PREQ-022: Groom portability: pubic/body grooms exportable as Alembic and bindable to morphing bodies (binding must survive extreme body morphs).
23. PREQ-023: Fluid handoff (adjacent pipeline, not core): mark emitter surfaces and collision volumes on bodies/genitals; recommended tools inventory: FLIP Fluids addon (viscosity solver, surface tension/sheeting, APIC/FLIP, cache-resume) for cum/lube/squirt sims; export bodies as clean collision meshes with velocity data. (UNVERIFIED: velocity-data export requirements per solver.)
24. PREQ-024: Pre-made viscous-fluid mesh library (strings, drips, pools as static/animated meshes with shader) — common production shortcut over per-shot sims. (UNVERIFIED prevalence; consistent with marketplace asset behavior.)
25. PREQ-025: Skin microdetail continuity across morphs: displacement/detail maps must not stretch visibly under extreme morphs — needs UV-space compensation or procedural detail re-synthesis (stretch/compression masks per Faceform 2025.11 are prior art).
26. PREQ-026: Extreme-morph texture strategy: regenerate/re-project albedo features (areola size/position, tan lines) when proportions change drastically — texture-follows-morph is unsolved in Daz and a differentiator.
27. PREQ-027: Nipple/areola and genital region get their own UDIM-tile-level texel density priority — closeup shots concentrate there; hero-tier px/cm should apply.
28. PREQ-028: Dynamic wrinkle/fold maps for body (neck folds, waist compression, knuckle stretch) driven by pose, not just face wrinkles.
29. PREQ-029: Eye realism stack (cornea refraction, caustic approximation, wetness meniscus) at parity with CC Digital Human Shader / MetaHuman — face closeups sell arousal expressions.
30. PREQ-030: Expression fidelity for arousal acting: scan-derived (or equivalent quality) blendshape set covering open-mouth/ahegao/bitten-lip/eye-roll extremes beyond ARKit-52, layerable with lip-sync.
31. PREQ-031: Saliva/tears/sweat as groomable mesh+shader elements (strings between lips, sweat beads) — asset library, not sim, for stills.
32. PREQ-032: Blender Cycles reference render profile shipped with the suite (lighting rig, color management, SSS/displacement settings) so "what BodyKit shows" matches "what Cycles renders".
33. PREQ-033: UE path-tracer/Lumen render profile equivalent for video workflows. (UNVERIFIED: UE path tracer skin-shading parity details.)
34. PREQ-034: Bake-down of the whole layered skin (SkinGen-style flatten) into portable PBR sets at export — layered systems must always flatten losslessly for third-party renderers.
35. PREQ-035: Anatomical plausibility guards as soft warnings only (interpenetration hints, joint-limit hints) — never hard limits; extreme proportions (huge tits on petite frame, monster dick) are the product target and must remain fully reachable.

## Animation/retarget requirements

1. AREQ-001: Pose library with thumbnail browser, tag/catalog search, click-to-apply and drag-to-blend (Blender Asset Browser pose library UX as the floor).
2. AREQ-002: Adult pose taxonomy first-class: solo/couple/group poses with multi-character root alignment data (relative transforms between actors) so a "doggy" pose places both bodies — single-actor pose formats can't express sex poses.
3. AREQ-003: Pose assets must store morph-context: a pose authored for extreme proportions must re-solve on other bodies (IK re-target on apply) instead of baking absolute bone transforms.
4. AREQ-004: BVH and FBX mocap ingestion with automatic bone-map detection plus preset maps (Mixamo, Rokoko, BVH standard, UE, Unity) and namespace search/replace — Auto-Rig Pro Remap is the reference UX.
5. AREQ-005: Retargeting engine between arbitrary skeletons (chain-based mapping, retarget base pose, bake-to-target) available both interactively and headless/API.
6. AREQ-006: Live mocap streaming ingest (UDP/Live Link-style) as a later lane; file-based first.
7. AREQ-007: ARKit-52 blendshape set as guaranteed facial animation interchange (input and output); map to/from internal FACS-style rig automatically (ARKit→FACS cheat-sheet mappings exist publicly).
8. AREQ-008: Optional extended facial standard: MetaHuman-rig-curve import path for people with MHA captures. (UNVERIFIED: practical mapping fidelity MetaHuman→custom rig.)
9. AREQ-009: Audio-driven lip-sync: integrate/ingest NVIDIA Audio2Face-3D (now open-source, regression+diffusion models, UE plugin exists) and/or a viseme-timeline generator (AccuLips pattern) producing editable viseme tracks.
10. AREQ-010: Viseme + expression layering: lip-sync must compose with arousal expressions (moans, gasps) — moan/breath non-speech vocalization presets are an adult-specific gap no mainstream tool covers.
11. AREQ-011: Corrective morph triggering must ride along with retargeted animation (pose-driven, not keyframed) so imported mocap gets JCM-equivalents for free.
12. AREQ-012: Loopable sex-animation authoring aids: cycle-safe curve tools, speed ramping, thrust-depth amplitude control as a parameter — game sex frameworks (OStim/SexLab packs) are built from short loops at escalating intensities; export should preserve loop metadata.
13. AREQ-013: Multi-actor animation export with preserved relative alignment + stage/anchor point conventions so engine frameworks can re-assemble scenes.
14. AREQ-014: Skeleton-convention stability across releases — modder ecosystems (Skyrim XPMSSE-style extended skeletons) live or die on stable bone names/hierarchy; publish and version the skeleton spec. (UNVERIFIED: XPMSSE specifics not re-checked this session.)
15. AREQ-015: Genital animation channels as standard rig controls (erection angle, testicle follow, labia morph channels) included in exported animation data, not sidecar hacks.
16. AREQ-016: Physics-vs-keyframe blending: per-region switch between simulated secondary motion and keyframed override, bakeable either direction (SOS rigid/floppy per-scene switching is the user expectation).
17. AREQ-017: IK/FK posing with pinning (hands on hips/partner, knees on ground) for fast adult-pose authoring; pin targets storable in the pose asset.
18. AREQ-018: Animation retarget QA visualization: side-by-side source/target playback with foot-slide and interpenetration flags.
19. AREQ-019: Batch retarget: apply a motion library folder to a character in one operation, headless-capable — supports agent-driven workflows.
20. AREQ-020: Export animation as FBX (engine standard), BVH (mocap interchange), and Alembic cache (deformation-exact) per target.

## Suite workflow requirements

1. SREQ-001: Non-destructive character recipe format: ordered morph-stack references (asset IDs + weights), skin layer stack, groom refs, garment refs, physics config — reconstructable like MetaHuman DNA rebuilds a full rigged mesh from one file.
2. SREQ-002: Recipe layers separable and independently swappable: shape / skin / grooms / outfit / physics (VaM looks+skin presets, Daz character-vs-wearable-vs-scene-subset preset taxonomy as prior art).
3. SREQ-003: Public scripting/API access to the recipe format (DNACalib precedent: rename/remove/edit programmatically) — required for agent-driven and batch pipelines.
4. SREQ-004: Single-file share format with embedded preview: recipe embedded in its own PNG render (Koikatsu card steganography pattern) for drag-drop import and community sharing.
5. SREQ-005: Dependency manifest in every shared asset (morphs/textures/garments referenced) with resolve-or-warn on import — VaM .var dependency model; Daz's silent missing-morph failures are the anti-pattern.
6. SREQ-006: Stable asset IDs + semantic versioning on morphs/garments/grooms; recipes reference ID@version; compat rules explicit (Koikatsu/HS2 cross-title incompatibility shows what happens without it).
7. SREQ-007: Batch generation of body variations from a base recipe: parameter ranges + seed → N characters, deterministic re-generation from seed (HumGen batch mode as floor; seeded determinism as improvement).
8. SREQ-008: Randomization scopes: whole-character, per-region (breasts only, face only), per-slider dice; plausibility-weighted sampling from preset distributions.
9. SREQ-009: Measurement-constrained generation: solve morph stack to hit numeric targets (height, cup size, waist, hip, penis length) — MetaHuman 5.6 measurement constraints prove feasibility on scan-statistical spaces.
10. SREQ-010: Cross-project reuse library: user library with catalogs/tags/thumbnails mounted across all projects (Blender Asset Browser catalog model), for recipes, morphs, poses, grooms, garments, skin presets.
11. SREQ-011: Asset versioning strategy aligned with USD: characters as named, versioned, structured containers; layer-per-department (shape/look/sim) and variantSets for wardrobe/body variants — enables studio pipelines later without redesign.
12. SREQ-012: USD export of assembled characters (mesh, materials, variants, garment sim data) — MD→UE already speaks USD; it is becoming the character-carrier format.
13. SREQ-013: Garment-fit data authored across the morph space: garments store fit targets for body-shape range and auto-interpolate (MetaHuman adaptive clothing; BodySlide conform as the community-proven equivalent) — this is the Cloth↔BodyKit core contract.
14. SREQ-014: One-click "conform outfit library to this body" batch operation (BodySlide batch build pattern).
15. SREQ-015: Morph retarget tooling: transfer a morph authored on base topology vN to vN+1 (or between bodies) via wrapping/RBF (Faceform blendshape-retarget nodes prove the tech) so the morph ecosystem survives basemesh updates.
16. SREQ-016: Sculpt round-trip: send current morphed body to Blender sculpt/ZBrush, diff returned mesh into a new morph target automatically (GoZ pattern).
17. SREQ-017: Scan on-ramp: wrap basemesh onto photogrammetry scans (Wrap-class non-rigid registration) to mint identity morphs + textures from real bodies.
18. SREQ-018: Photo on-ramp (later lane): image→head/body identity morph (Headshot 3 precedent). (UNVERIFIED: buildable quality without proprietary training data.)
19. SREQ-019: Recipe diffing and merging: compare two characters as parameter diffs; apply partial diffs ("take her breasts, keep my face") — trivially enabled by parametric recipes, offered by no mainstream tool.
20. SREQ-020: Headless/CLI generation: full recipe→export pipeline runnable without GUI for render farms and agent swarms — aligns with Handshake's parallel-model workflow requirements.
21. SREQ-021: Machine-readable manifests for every export (what was exported, budgets hit, warnings) — supports validation gates and LLM operators.
22. SREQ-022: Preset marketplace-readiness: license metadata embedded per asset (VRM does this in-format; MetaHuman/FAB shows the marketplace model) including adult-content flagging fields for venue filtering.
23. SREQ-023: Character A/B snapshot and rollback within a project (recipe history), cheap because recipes are small parameter sets — prevents destructive loss during look-dev.
24. SREQ-024: Bundled starter content at credible floor: ≥ MB-Lab-scale morph coverage (~470 morphs incl. macro age/mass/tone cascade), phenotype presets, skin/eye/genital shader presets — empty parametric systems don't demo.
25. SREQ-025: In-app manual + structured diagnostics for no-context models operating the suite (state dumps, deterministic commands) — required by Handshake's own governance policies and enables the agent-parallel workflow the operator runs.

## Sources

- https://www.reallusion.com/character-creator/skingen.html
- https://magazine.reallusion.com/2021/11/05/character-creator-4-new-features-introduction/
- https://magazine.reallusion.com/2023/02/23/character-creator-4-2-feature-highlights-one-wrinkle-for-all-avatars/
- https://magazine.reallusion.com/2022/05/27/immense-innovation-for-character-creator-4-and-iclone-8/
- https://www.reallusion.com/iclone/motion-director.html
- https://www.reallusion.com/iclone/motion-capture/default.html
- https://courses.reallusion.com/home/character-creator/material-and-texture?v=character-creator-3-tutorial-export-with-instalod-optimizing-characters-for-animation
- https://www.reallusion.com/character-creator/zbrush/goz/default.html
- https://magazine.reallusion.com/2026/03/13/headshot-3-is-coming-pre-launch-offer-live-now/
- https://www.reallusion.com/character-creator/hd-animation.html
- https://www.cgchannel.com/2025/06/you-can-now-sell-metahumans-or-use-them-in-unity-or-godot/
- https://www.metahuman.com/license
- https://www.metahuman.com/news/metahuman-leaves-early-access-with-a-feature-packed-new-release
- https://www.biunivoca.com/en/blog/metahuman-5-6-parametric-bodies-and-adaptive-clothing-with-genera-studio
- https://dev.epicgames.com/documentation/unreal-engine/getting-started-with-parametric-clothing
- https://github.com/EpicGames/MetaHuman-DNA-Calibration
- https://github.com/EpicGames/MetaHuman-DNA-Calibration/blob/main/docs/dna.md
- https://humgen3d.com/
- https://help.humgen3d.com/Batch/Overview
- https://github.com/OliverJPost/HumGen3D
- https://en.wikipedia.org/wiki/MB-Lab
- https://aircada.com/blog/makehuman-vs-mb-lab
- https://vroid.com/en/studio
- https://vroid.pixiv.help/hc/en-us/articles/15760756822297-I-want-to-learn-more-about-the-VRM-export-feature
- https://faceform.com/wraporiginal/
- https://www.cgchannel.com/2025/12/faceform-releases-wrap-2025-11-and-wrap4d-2025-11/
- https://texturing.xyz/pages/discover-unwrapped-multi-channel-faces
- https://www.cgchannel.com/2026/03/texturing-xyzs-skap-takes-your-skin-textures-to-the-next-level/
- https://www.3dscanstore.com/metahuman-textures
- https://www.simplygon.com/features/ue
- https://docs.instalod.io/Products/InstaLOD_Studio/Workflows/Optimizing_Skeletal_Meshes
- https://bitsoulhosting.com/marketplace/blog/blend-shapes-morph-targets-game-characters-blender-unity-unreal
- https://polycount.com/discussion/217908/cost-of-morphs-blendshapes-vs-bones
- https://forums.unrealengine.com/t/nanite-skeletal-meshes-morph-targets-cloth/2062994
- https://uhiyama-lab.com/en/notes/ue/animation-retargeting-complete-guide/
- https://dev.epicgames.com/documentation/unreal-engine/groom-scalability-and-performance-with-unreal-engine
- https://dev.epicgames.com/community/learning/tutorials/Mpze/unreal-engine-panel-cloth-editor-walkthrough-updates-5-4
- https://www.docswell.com/s/moyuki/54VVWQ-2024-09-14-231311
- https://www.sidefx.com/tutorials/vertex-animation-textures-for-unreal/
- https://github.com/pafuhana1213/KawaiiPhysics
- https://github.com/naelstrof/JigglePhysics
- https://creators.vrchat.com/common-components/physbones/
- https://gist.github.com/donmccurdy/4cad2039360fbd7cd55d18b3f0428581
- https://docs.unity3d.com/ScriptReference/QualitySettings-skinWeights.html
- https://dev.epicgames.com/documentation/unreal-engine/skeletal-mesh-rendering-paths-in-unreal-engine
- https://nastyrodent.com/uv-unwrapping-for-games/
- https://rebusfarm.net/blog/texel-density-basics-every-artist-should-know
- https://marmoset.co/posts/toolbag-baking-tutorial/
- https://www.daz3d.com/forums/discussion/660011/gen-9-goldenpalace-geograft-texture-issue
- https://zonegfx.com/golden-palace-for-genesis-8-female-updates-2021-10-20/
- https://www.versluis.com/2021/08/jcms/
- https://render.otoy.com/forum/viewtopic.php?f=45&t=66485
- https://blenderartists.org/t/how-do-you-make-realistic-ish-skin-deformations/671883
- http://diffeomorphic.blogspot.com/2022/01/softbody-simulations.html?m=1
- https://docs.blender.org/manual/en/latest/render/shader_nodes/shader/sss.html
- https://dev.epicgames.com/documentation/unreal-engine/ml-deformer-framework-in-unreal-engine
- https://flipfluids.com/features/
- https://studio.blender.org/blog/procedural-hair-nodes/
- https://www.nexusmods.com/skyrimspecialedition/mods/201
- https://www.dracotorre.com/blog/body-conversions-for-skyrim-using-bodyslide/
- https://github.com/LivelyDismay/Learn-To-Mod/blob/main/lessons/Adding%20Schlongs%20to%20Anything%20(NSFW).md
- https://www.nexusmods.com/skyrimspecialedition/mods/108246
- https://pooyadeperson.com/the-ultimate-guide-to-creating-arkits-52-facial-blendshapes/
- https://melindaozel.com/arkit-to-facs-cheat-sheet/
- https://developer.nvidia.com/blog/nvidia-open-sources-audio2face-animation-model/
- https://github.com/NVIDIA/Audio2Face-3D
- https://www.reallusion.com/iclone/nvidia-omniverse/Audio2Face.html
- https://dev.epicgames.com/documentation/unreal-engine/mutable-overview-in-unreal-engine
- https://vrpupu.com/en/2026/01/virt-a-mate-install-and-usage-complete-guide/
- https://github.com/acidbubbles/vam-collider-editor
- https://explore.st-aug.edu/exp/in-depth-analysis-unraveling-the-koikatsu-card-ecosystem
- https://db.bepis.moe/
- https://xanathon.com/2026/04/diffeomorphic-daz-to-blender-why-i-switched-and-never-looked-back/
- https://www.daz3d.com/forums/discussion/690491/how-to-save-my-character-and-morphs-correctly
- https://docs.omniverse.nvidia.com/usd/latest/learn-openusd/independent/asset-structure-principles.html
- https://lf-aswf.atlassian.net/wiki/display/WGUSD/Guidelines+for+Structuring+USD+Assets
- https://openusd.org/dev/glossary.html
- https://www.cgchannel.com/2023/12/get-epic-games-free-pose-driver-connect-for-maya-and-ue5/
- https://docs.unrealengine.com/4.27/en-US/AnimatingObjects/SkeletalMeshAnimation/AnimPose/PoseDriverNode
- https://yelzkizi.org/create-a-hyper-realistic-metahuman-in-unreal-engine-5/
- https://texturing.xyz/pages/saurabh-jethani-creating-realistic-skin-in-ue4
- https://github.com/shteeve3d/blender-wiggle-2
- https://support.marvelousdesigner.com/hc/en-us/articles/47358145573401--Tips-Tricks-Discover-Better-Workflow-with-Marvelous-Designer-and-Unreal-Engine
- https://support.marvelousdesigner.com/hc/en-us/articles/47358220465433-4-Mesh-Optimization-and-Retopology-Toolset
- https://www.lucky3d.fr/auto-rig-pro/doc/remap_doc.html
- https://mocaponline.com/blogs/mocap-news/animation-retargeting-guide
- https://docs.blender.org/manual/en/latest/animation/armatures/posing/editing/pose_library.html
- https://magicasoft.jp/en/magica-cloth-2/
- https://docs.unity3d.com/6000.3/Documentation/Manual/class-Cloth.html
- https://mocaponline.com/blogs/mocap-news/face-capture-game-dev-iphone-arkit-live-link-metahuman
- https://www.patreon.com/adeptussteve
